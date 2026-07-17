//! Managed process handle: pipes, lifecycle, stop/kill.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::config::{SpawnConfig, StopPolicy};
use crate::error::{ProcessError, ProcessErrorClass};
use crate::event::{ExitInfo, ProcessEvent};
use crate::ids::ProcessId;
use crate::platform::TreeIsolation;
use crate::readiness::{ProcessPhase, ReadinessView};

/// One managed sidecar process.
///
/// # Ownership
///
/// - Stderr is drained on a background thread into [`ProcessEvent::StderrChunk`].
/// - Stdout is left for the ACP adapter via [`ManagedProcess::take_stdout`].
/// - Stdin is available via [`ManagedProcess::stdin_mut`] / [`close_stdin`].
///
/// # Readiness
///
/// [`ManagedProcess::readiness`] never reports protocol/auth/session ready.
pub struct ManagedProcess {
    id: ProcessId,
    pid: u32,
    executable: PathBuf,
    args: Vec<String>,
    cwd: PathBuf,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<ChildStdout>,
    events_rx: Receiver<ProcessEvent>,
    events_tx: Sender<ProcessEvent>,
    phase: ProcessPhase,
    isolation: TreeIsolation,
    stderr_join: Option<JoinHandle<()>>,
    kill_on_drop: bool,
    stop_policy: StopPolicy,
    started_event_emitted: bool,
}

impl std::fmt::Debug for ManagedProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedProcess")
            .field("id", &self.id)
            .field("pid", &self.pid)
            .field("executable", &self.executable)
            .field("args", &self.args)
            .field("cwd", &self.cwd)
            .field("phase", &self.phase)
            .field("isolation", &self.isolation.strategy_name())
            .field("kill_on_drop", &self.kill_on_drop)
            .finish_non_exhaustive()
    }
}

impl ManagedProcess {
    /// Spawn a child according to `config`.
    pub fn spawn(config: SpawnConfig) -> Result<Self, ProcessError> {
        config.validate()?;

        let process_id = ProcessId::new();
        let (events_tx, events_rx) = mpsc::channel();

        let mut command = Command::new(&config.executable);
        command
            .args(&config.args)
            .current_dir(&config.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if config.clear_env {
            command.env_clear();
            // Windows often needs SystemRoot / PATH fragments for some tools;
            // hermetic tests should set what they need via config.env.
        }
        for (k, v) in &config.env {
            command.env(k, v);
        }

        let mut isolation = TreeIsolation::prepare(&mut command, config.isolate_process_tree)?;

        let mut child = match command.spawn() {
            Ok(c) => c,
            Err(err) => {
                let not_found = err.kind() == std::io::ErrorKind::NotFound;
                let class = if not_found {
                    ProcessErrorClass::RuntimeExecutableNotFound
                } else {
                    ProcessErrorClass::RuntimeSpawnFailed
                };
                let message = if not_found {
                    format!(
                        "runtime executable not found: {}",
                        config.executable.display()
                    )
                } else {
                    format!(
                        "failed to spawn {}: {err}",
                        config.executable.display()
                    )
                };
                let pe = ProcessError::new(class, message);
                let _ = events_tx.send(ProcessEvent::Failed {
                    process_id,
                    error_class: pe.class,
                    message: pe.message.clone(),
                    retryable: pe.retryable,
                });
                return Err(pe);
            }
        };

        if let Err(err) = isolation.bind_child(&child) {
            // Best-effort cleanup if job assignment fails.
            let _ = child.kill();
            let _ = child.wait();
            return Err(err);
        }

        let pid = child.id();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let stderr_join = stderr.map(|pipe| {
            let tx = events_tx.clone();
            let limit = config.stderr_chunk_limit;
            let pid_for_log = process_id;
            thread::spawn(move || drain_stderr(pipe, tx, pid_for_log, limit))
        });

        let started = ProcessEvent::Started {
            process_id,
            pid,
            executable: config.executable.display().to_string(),
            args: config.args.clone(),
            cwd: config.cwd.display().to_string(),
        };
        let _ = events_tx.send(started);

        Ok(Self {
            id: process_id,
            pid,
            executable: config.executable,
            args: config.args,
            cwd: config.cwd,
            child: Some(child),
            stdin,
            stdout,
            events_rx,
            events_tx,
            phase: ProcessPhase::Alive,
            isolation,
            stderr_join,
            kill_on_drop: config.kill_on_drop,
            stop_policy: config.stop_policy,
            started_event_emitted: true,
        })
    }

    /// Process manager id.
    pub fn id(&self) -> ProcessId {
        self.id
    }

    /// OS pid at spawn time.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Configured executable.
    pub fn executable(&self) -> &std::path::Path {
        &self.executable
    }

    /// Configured args.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// Configured cwd.
    pub fn cwd(&self) -> &std::path::Path {
        &self.cwd
    }

    /// Isolation strategy name.
    pub fn isolation_strategy(&self) -> &'static str {
        self.isolation.strategy_name()
    }

    /// Current phase.
    pub fn phase(&self) -> &ProcessPhase {
        &self.phase
    }

    /// OS process believed alive (Alive or Stopping).
    pub fn is_alive(&self) -> bool {
        self.phase.is_alive()
    }

    /// Explicit process-alive check (pipes may still be open).
    pub fn is_process_alive(&self) -> bool {
        self.phase.is_process_alive()
    }

    /// **Always `false`.** Protocol readiness is adapter-owned (`runtime.process.ready`).
    pub fn is_protocol_ready(&self) -> bool {
        false
    }

    /// **Always `false`.** Authentication is adapter/control-plane owned.
    pub fn is_authenticated(&self) -> bool {
        false
    }

    /// **Always `false`.** Session readiness is control-plane owned.
    pub fn is_session_ready(&self) -> bool {
        false
    }

    /// Readiness view that never confuses process-alive with session-ready.
    pub fn readiness(&self) -> ReadinessView {
        ReadinessView::from_phase(&self.phase)
    }

    /// Whether the started event was emitted (always true after successful spawn).
    pub fn started_event_emitted(&self) -> bool {
        self.started_event_emitted
    }

    /// Take stdout for the ACP adapter. Only once.
    pub fn take_stdout(&mut self) -> Option<ChildStdout> {
        self.stdout.take()
    }

    /// Mutable stdin for writing ACP frames.
    pub fn stdin_mut(&mut self) -> Option<&mut ChildStdin> {
        self.stdin.as_mut()
    }

    /// Write bytes to stdin.
    pub fn write_stdin(&mut self, data: &[u8]) -> Result<(), ProcessError> {
        let Some(stdin) = self.stdin.as_mut() else {
            return Err(ProcessError::disconnected("stdin already closed"));
        };
        stdin
            .write_all(data)
            .map_err(|e| ProcessError::disconnected(format!("stdin write failed: {e}")))?;
        stdin
            .flush()
            .map_err(|e| ProcessError::disconnected(format!("stdin flush failed: {e}")))?;
        Ok(())
    }

    /// Close stdin (graceful shutdown signal for stdio ACP runtimes).
    pub fn close_stdin(&mut self) -> Result<(), ProcessError> {
        self.stdin.take();
        Ok(())
    }

    /// Non-blocking event poll.
    pub fn try_recv_event(&self) -> Option<ProcessEvent> {
        match self.events_rx.try_recv() {
            Ok(ev) => Some(ev),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => None,
        }
    }

    /// Drain currently queued events (non-blocking).
    pub fn drain_events(&self) -> Vec<ProcessEvent> {
        let mut out = Vec::new();
        while let Some(ev) = self.try_recv_event() {
            out.push(ev);
        }
        out
    }

    /// Wait for a process event with timeout (does not wait on child exit itself).
    pub fn recv_event_timeout(&self, timeout: Duration) -> Result<ProcessEvent, ProcessError> {
        match self.events_rx.recv_timeout(timeout) {
            Ok(ev) => Ok(ev),
            Err(RecvTimeoutError::Timeout) => Err(ProcessError::timeout(
                "timed out waiting for process event",
            )),
            Err(RecvTimeoutError::Disconnected) => Err(ProcessError::internal(
                "process event channel disconnected",
            )),
        }
    }

    /// Poll OS for exit without blocking long.
    pub fn try_wait(&mut self) -> Result<Option<ExitInfo>, ProcessError> {
        self.poll_exit(false)
    }

    /// Block until exit or timeout.
    pub fn wait_timeout(&mut self, timeout: Duration) -> Result<ExitInfo, ProcessError> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(info) = self.poll_exit(false)? {
                return Ok(info);
            }
            if Instant::now() >= deadline {
                return Err(ProcessError::timeout(format!(
                    "process {} did not exit within {:?}",
                    self.pid, timeout
                )));
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// Mark that the next exit is expected (user stop / cancel).
    pub fn mark_expected_exit(&mut self) {
        if matches!(self.phase, ProcessPhase::Alive) {
            self.phase = ProcessPhase::Stopping { expected: true };
        }
    }

    /// Stop using the configured default policy.
    pub fn stop_default(&mut self) -> Result<ExitInfo, ProcessError> {
        self.stop(self.stop_policy)
    }

    /// Stop with an explicit policy.
    pub fn stop(&mut self, policy: StopPolicy) -> Result<ExitInfo, ProcessError> {
        match policy {
            StopPolicy::GracefulThenForce {
                graceful,
                force_wait,
            } => self.stop_graceful_then_force(graceful, force_wait),
            StopPolicy::Force { force_wait } => self.kill_force(force_wait),
        }
    }

    /// Close stdin, wait for voluntary exit, then force-kill the tree.
    pub fn stop_graceful_then_force(
        &mut self,
        graceful: Duration,
        force_wait: Duration,
    ) -> Result<ExitInfo, ProcessError> {
        self.mark_expected_exit();
        let _ = self.close_stdin();
        match self.wait_timeout(graceful) {
            Ok(info) => Ok(info),
            Err(err) if err.class == ProcessErrorClass::Timeout => {
                self.kill_force(force_wait)
            }
            Err(err) => Err(err),
        }
    }

    /// Force-kill the process tree (Job Object / process group) and wait briefly.
    pub fn kill_force(&mut self, force_wait: Duration) -> Result<ExitInfo, ProcessError> {
        if matches!(self.phase, ProcessPhase::Exited(_) | ProcessPhase::Failed) {
            if let ProcessPhase::Exited(info) = &self.phase {
                return Ok(info.clone());
            }
            return Err(ProcessError::disconnected("process already failed"));
        }

        self.mark_expected_exit();
        // Prefer tree kill so grandchildren die (F-P11 / F-W01).
        let _ = self.isolation.force_kill_tree();
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
        }

        match self.wait_timeout(force_wait) {
            Ok(info) => Ok(info),
            Err(_) => {
                // Last chance poll.
                if let Some(info) = self.poll_exit(true)? {
                    return Ok(info);
                }
                Err(ProcessError::new(
                    ProcessErrorClass::CancellationFailed,
                    format!(
                        "force kill issued but process {} exit was not observed",
                        self.pid
                    ),
                ))
            }
        }
    }

    fn poll_exit(&mut self, force_expected: bool) -> Result<Option<ExitInfo>, ProcessError> {
        if let ProcessPhase::Exited(info) = &self.phase {
            return Ok(Some(info.clone()));
        }

        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };

        match child.try_wait() {
            Ok(Some(status)) => {
                let expected = force_expected
                    || matches!(self.phase, ProcessPhase::Stopping { expected: true });
                let info = exit_info_from_status(status, expected);
                self.phase = ProcessPhase::Exited(info.clone());
                let _ = self.events_tx.send(ProcessEvent::Exited {
                    process_id: self.id,
                    info: info.clone(),
                });
                // Drop child to release OS resources.
                let _ = self.child.take();
                Ok(Some(info))
            }
            Ok(None) => Ok(None),
            Err(err) => Err(ProcessError::internal(format!("try_wait failed: {err}"))),
        }
    }
}

impl Drop for ManagedProcess {
    fn drop(&mut self) {
        if !self.kill_on_drop {
            return;
        }
        if self.phase.is_alive() {
            let _ = self.isolation.force_kill_tree();
            if let Some(child) = self.child.as_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
        if let Some(handle) = self.stderr_join.take() {
            let _ = handle.join();
        }
    }
}

fn drain_stderr(
    mut pipe: impl Read,
    tx: Sender<ProcessEvent>,
    process_id: ProcessId,
    limit: usize,
) {
    let mut buf = [0u8; 4096];
    loop {
        match pipe.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                let raw = &buf[..n];
                let truncated = raw.len() > limit;
                let slice = if truncated { &raw[..limit] } else { raw };
                let chunk = String::from_utf8_lossy(slice).into_owned();
                if chunk.is_empty() {
                    continue;
                }
                let _ = tx.send(ProcessEvent::StderrChunk {
                    process_id,
                    chunk,
                    truncated,
                });
            }
            Err(_) => break,
        }
    }
}

fn exit_info_from_status(status: std::process::ExitStatus, expected: bool) -> ExitInfo {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return ExitInfo::new(None, Some(format!("SIG{sig}")), expected);
        }
    }
    ExitInfo::new(status.code(), None, expected)
}

/// Process manager entry point (stateless for now).
#[derive(Debug, Default, Clone, Copy)]
pub struct ProcessManager;

impl ProcessManager {
    /// Create a manager.
    pub fn new() -> Self {
        Self
    }

    /// Spawn a managed process.
    pub fn spawn(&self, config: SpawnConfig) -> Result<ManagedProcess, ProcessError> {
        ManagedProcess::spawn(config)
    }
}
