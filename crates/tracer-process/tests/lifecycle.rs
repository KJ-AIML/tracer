//! Fake-process lifecycle tests for W1-C.
//!
//! Covers F-P01, F-P05/P06 exit observation, F-P09 graceful stop, F-P10 force kill,
//! F-P11/F-W01 orphan prevention (platform), and F-A05 process≠session ready.

use std::io::Read;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use tracer_process::{
    ExitInfo, ManagedProcess, ProcessErrorClass, ProcessEvent, ProcessManager, ProcessPhase,
    SpawnConfig, StopPolicy,
};

fn helper_exe() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_tracer-process-test-helper"))
}

fn base_config(args: &[&str]) -> SpawnConfig {
    SpawnConfig::new(helper_exe(), std::env::temp_dir()).args(args.iter().map(|s| s.to_string()))
}

fn wait_for_started(proc: &ManagedProcess) -> ProcessEvent {
    // Started is queued at spawn; drain until we see it.
    for _ in 0..50 {
        for ev in proc.drain_events() {
            if matches!(ev, ProcessEvent::Started { .. }) {
                return ev;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("did not observe ProcessEvent::Started");
}

#[test]
fn spawn_emits_started_and_is_process_alive_not_protocol_ready() {
    let mgr = ProcessManager::new();
    let mut proc = mgr
        .spawn(base_config(&["sleep-ms", "500"]))
        .expect("spawn");

    let started = wait_for_started(&proc);
    match started {
        ProcessEvent::Started { pid, executable, .. } => {
            assert_eq!(pid, proc.pid());
            assert!(!executable.is_empty());
        }
        other => panic!("unexpected event: {other:?}"),
    }

    assert!(proc.is_process_alive());
    assert!(matches!(proc.phase(), ProcessPhase::Alive));
    // F-A05: process manager must not claim higher readiness layers.
    assert!(!proc.is_protocol_ready());
    assert!(!proc.is_authenticated());
    assert!(!proc.is_session_ready());
    let view = proc.readiness();
    assert!(view.process_alive);
    assert!(!view.protocol_ready);
    assert!(!view.authenticated);
    assert!(!view.session_ready);
    assert!(!view.may_accept_prompt());

    let info = proc
        .stop(StopPolicy::Force {
            force_wait: Duration::from_secs(2),
        })
        .expect("force stop");
    assert!(info.expected);
}

#[test]
fn executable_missing_maps_to_not_found() {
    let mgr = ProcessManager::new();
    let cfg = SpawnConfig::new(
        PathBuf::from("definitely-not-a-real-tracer-runtime-binary-xyz"),
        std::env::temp_dir(),
    );
    let err = match mgr.spawn(cfg) {
        Ok(_) => panic!("must fail for missing executable"),
        Err(e) => e,
    };
    assert_eq!(err.class, ProcessErrorClass::RuntimeExecutableNotFound);
    assert!(!err.retryable);
}

#[test]
fn invalid_cwd_fails_spawn() {
    let mgr = ProcessManager::new();
    let mut cfg = base_config(&["exit", "0"]);
    cfg.cwd = PathBuf::from("D:/this/path/should/not/exist/tracer-w1-c-cwd");
    let err = match mgr.spawn(cfg) {
        Ok(_) => panic!("must fail for missing cwd"),
        Err(e) => e,
    };
    assert_eq!(err.class, ProcessErrorClass::RuntimeSpawnFailed);
}

#[test]
fn capture_stdout_via_take() {
    let mgr = ProcessManager::new();
    let mut proc = mgr
        .spawn(base_config(&["echo-stdout", "hello-stdout"]))
        .expect("spawn");
    let mut stdout = proc.take_stdout().expect("stdout");
    let mut buf = String::new();
    stdout.read_to_string(&mut buf).expect("read");
    assert_eq!(buf, "hello-stdout");
    let info = proc.wait_timeout(Duration::from_secs(2)).expect("wait");
    assert_eq!(info.exit_code, Some(0));
}

#[test]
fn capture_stderr_events() {
    let mgr = ProcessManager::new();
    let mut proc = mgr
        .spawn(base_config(&["echo-stderr", "warn-line"]))
        .expect("spawn");
    let _ = wait_for_started(&proc);

    let mut saw_stderr = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        for ev in proc.drain_events() {
            if let ProcessEvent::StderrChunk { chunk, truncated, .. } = ev {
                assert!(chunk.contains("warn-line"), "chunk={chunk}");
                assert!(!truncated);
                saw_stderr = true;
            }
        }
        if saw_stderr {
            break;
        }
        if proc.try_wait().ok().flatten().is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(saw_stderr, "expected stderr chunk event");
    let _ = proc.wait_timeout(Duration::from_secs(2));
}

#[test]
fn graceful_stdin_close_exits() {
    let mgr = ProcessManager::new();
    let mut proc = mgr
        .spawn(base_config(&["hang-until-stdin-close"]))
        .expect("spawn");
    let _ = wait_for_started(&proc);
    assert!(proc.is_alive());

    proc.mark_expected_exit();
    proc.close_stdin().expect("close stdin");
    let info = proc
        .wait_timeout(Duration::from_secs(3))
        .expect("exit after stdin close");
    assert_eq!(info.exit_code, Some(0));
    assert!(info.expected);
    assert!(matches!(proc.phase(), ProcessPhase::Exited(_)));
}

#[test]
fn force_kill_long_sleep() {
    let mgr = ProcessManager::new();
    let mut proc = mgr
        .spawn(base_config(&["sleep-ms", "60000"]))
        .expect("spawn");
    let _ = wait_for_started(&proc);

    let info = proc
        .kill_force(Duration::from_secs(3))
        .expect("force kill");
    assert!(info.expected);
    assert!(!proc.is_alive());
}

#[test]
fn nonzero_exit_observed() {
    let mgr = ProcessManager::new();
    let mut proc = mgr.spawn(base_config(&["exit", "7"])).expect("spawn");
    let info = proc.wait_timeout(Duration::from_secs(2)).expect("wait");
    assert_eq!(info.exit_code, Some(7));
    // Natural exit without mark_expected_exit => unexpected from process manager POV
    // unless we mark it — crash-like observation uses expected: false.
    assert!(!info.expected);
}

#[test]
fn graceful_then_force_on_hang() {
    // Process ignores stdin close? sleep-ms doesn't read stdin, so graceful wait times out
    // and force path runs (F-P10).
    let mgr = ProcessManager::new();
    let mut proc = mgr
        .spawn(base_config(&["sleep-ms", "60000"]))
        .expect("spawn");
    let _ = wait_for_started(&proc);

    let info = proc
        .stop(StopPolicy::GracefulThenForce {
            graceful: Duration::from_millis(200),
            force_wait: Duration::from_secs(3),
        })
        .expect("stop");
    assert!(info.expected);
    assert!(!proc.is_alive());
}

#[test]
fn write_stdin_roundtrip() {
    let mgr = ProcessManager::new();
    let mut proc = mgr
        .spawn(base_config(&["read-stdin-line"]))
        .expect("spawn");
    proc.write_stdin(b"ping\n").expect("write");
    let mut stdout = proc.take_stdout().expect("stdout");
    let mut buf = String::new();
    stdout.read_to_string(&mut buf).expect("read");
    assert!(buf.contains("ping"), "got {buf}");
    let _ = proc.wait_timeout(Duration::from_secs(2));
}

#[test]
fn process_event_type_hints_never_ready() {
    // Guard: process manager event surface must not invent runtime.process.ready.
    let hints = [
        ProcessEvent::Started {
            process_id: tracer_process::ProcessId::new(),
            pid: 1,
            executable: "x".into(),
            args: vec![],
            cwd: ".".into(),
        },
        ProcessEvent::StderrChunk {
            process_id: tracer_process::ProcessId::new(),
            chunk: "e".into(),
            truncated: false,
        },
        ProcessEvent::Exited {
            process_id: tracer_process::ProcessId::new(),
            info: ExitInfo::code(0, true),
        },
        ProcessEvent::Failed {
            process_id: tracer_process::ProcessId::new(),
            error_class: ProcessErrorClass::RuntimeSpawnFailed,
            message: "m".into(),
            retryable: true,
        },
    ];
    for h in hints {
        assert_ne!(h.protocol_type_hint(), "runtime.process.ready");
        assert_ne!(h.protocol_type_hint(), "session.ready");
    }
}

#[test]
fn isolation_strategy_is_platform_native_when_enabled() {
    let mgr = ProcessManager::new();
    let mut cfg = base_config(&["exit", "0"]);
    cfg.isolate_process_tree = true;
    let mut proc = mgr.spawn(cfg).expect("spawn");
    let name = proc.isolation_strategy();
    #[cfg(windows)]
    assert_eq!(name, "windows-job-object-kill-on-close");
    #[cfg(unix)]
    assert_eq!(name, "unix-process-group");
    #[cfg(not(any(windows, unix)))]
    assert_eq!(name, "null");
    let _ = proc.wait_timeout(Duration::from_secs(2));
}

/// F-P11 / F-W01: grandchildren must not survive force kill when isolation is on.
#[test]
fn force_kill_reaps_grandchild_no_orphan() {
    let mgr = ProcessManager::new();
    let mut cfg = base_config(&["spawn-child-sleep-ms", "60000"]);
    cfg.isolate_process_tree = true;
    let mut proc = mgr.spawn(cfg).expect("spawn");

    // Read parent_pid / child_pid lines from helper stdout (flushed before sleep).
    let mut stdout = proc.take_stdout().expect("stdout");
    let mut raw = String::new();
    {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(&mut stdout);
        let mut line1 = String::new();
        let mut line2 = String::new();
        reader.read_line(&mut line1).expect("line1");
        reader.read_line(&mut line2).expect("line2");
        raw.push_str(&line1);
        raw.push_str(&line2);
    }

    let mut child_pid: Option<u32> = None;
    for line in raw.lines() {
        if let Some(v) = line.strip_prefix("child_pid=") {
            child_pid = v.trim().parse().ok();
        }
    }
    let child_pid = child_pid.expect("child_pid in helper output");

    // Force kill managed parent (tree).
    let _ = proc
        .kill_force(Duration::from_secs(3))
        .expect("kill force");

    // Allow OS a moment to reap.
    thread::sleep(Duration::from_millis(300));

    assert!(
        !os_pid_alive(child_pid),
        "grandchild pid {child_pid} still alive after tree kill (orphan)"
    );
}

#[cfg(windows)]
fn os_pid_alive(pid: u32) -> bool {
    use std::os::windows::process::CommandExt;
    // tasklist exit 0 always; parse output. Use OpenProcess via PowerShell for reliability.
    let output = std::process::Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/NH"])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .expect("tasklist");
    let text = String::from_utf8_lossy(&output.stdout);
    text.contains(&pid.to_string())
}

#[cfg(unix)]
fn os_pid_alive(pid: u32) -> bool {
    // signal 0 probes existence
    let rc = unsafe { libc::kill(pid as i32, 0) };
    rc == 0
}

#[cfg(not(any(windows, unix)))]
fn os_pid_alive(_pid: u32) -> bool {
    false
}
