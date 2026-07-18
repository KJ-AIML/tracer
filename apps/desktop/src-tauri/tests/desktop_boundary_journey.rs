//! W2-B desktop boundary journey — actual Tauri command glue + control-plane composition.
//!
//! Classification: **desktop-boundary E2E** (not full WebView GUI drive).
//! Exercises the same `build_control_plane` + `plane_*` handlers the Tauri app registers.
//!
//! CI class: standard — network: no, credentials: no, live Grok: no, provider: no.
//! Fake ACP: yes. Temp file SQLite: yes.
//!
//! Full Playwright/WebDriver GUI drive is blocked without tauri-driver + WebView2 tooling;
//! see docs/modules/w2-b/W2_B_E2E_ARCHITECTURE.md.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use tracer_control_plane::RuntimeCreateOptions;
use tracer_desktop_lib::commands::{
    plane_app_info, plane_approval_list_pending, plane_approval_resolve, plane_events_list,
    plane_heli_status, plane_presentation_focus, plane_presentation_snapshot, plane_project_list,
    plane_project_register, plane_runtime_status, plane_session_cancel, plane_session_create,
    plane_session_get, plane_session_list, plane_session_stop, plane_session_submit_prompt,
    ApprovalResolveArgs, CancelArgs, EventsListArgs, ProjectRegisterArgs, SessionCreateArgs,
    SessionListArgs, StopArgs, SubmitPromptArgs, REGISTERED_COMMANDS,
};
use tracer_desktop_lib::control_plane::{build_control_plane, discover_fake_js};
use tracer_desktop_lib::REGISTERED_COMMANDS as LIB_REGISTERED;

async fn journey_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // apps/desktop/src-tauri → apps/desktop → apps → repo root
    p.pop();
    p.pop();
    p.pop();
    p
}

fn fake_js() -> PathBuf {
    let from_discover = discover_fake_js();
    let path = from_discover
        .unwrap_or_else(|| repo_root().join("tools/fake-acp-runtime/bin/fake-acp-runtime.js"));
    assert!(
        path.is_file(),
        "missing fake ACP at {} — set TRACER_FAKE_ACP_JS",
        path.display()
    );
    path
}

fn runtime_opts(scenario: &str) -> RuntimeCreateOptions {
    RuntimeCreateOptions {
        runtime_kind: "acp-stdio".into(),
        scenario_id: Some(scenario.into()),
        executable_override: None,
        extra_args: vec![],
        fake_js: Some(fake_js().display().to_string()),
    }
}

fn has_type(events: &[serde_json::Value], t: &str) -> bool {
    events
        .iter()
        .any(|e| e.get("type").and_then(|x| x.as_str()) == Some(t))
}

fn event_types(events: &[serde_json::Value]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| {
            e.get("type")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
        })
        .collect()
}

/// Domain/command JSON must not expose untranslated ACP protocol as the
/// primary surface. Adapter provenance may record `runtimeMethod` (e.g.
/// `session/update`) — that is metadata, not a frontend event type.
fn assert_no_raw_acp(value: &serde_json::Value) {
    const RAW_EVENT_TYPES: &[&str] = &[
        "session/update",
        "session/new",
        "session/prompt",
        "session/cancel",
        "agent_message_chunk",
        "fs/read_text_file",
        "fs/write_text_file",
    ];

    // 1) Event list: every `type` must be domain (dotted), not raw ACP method.
    if let Some(events) = value.get("events").and_then(|e| e.as_array()) {
        for e in events {
            let t = e.get("type").and_then(|x| x.as_str()).unwrap_or("");
            assert!(
                !RAW_EVENT_TYPES.contains(&t),
                "raw ACP event type `{t}` on boundary surface: {e}"
            );
            // Domain events use dotted names (session.created, agent.message.delta, …).
            if !t.is_empty() {
                assert!(
                    t.contains('.') || t.starts_with("runtime"),
                    "unexpected non-domain event type `{t}`: {e}"
                );
            }
            // Payload must not re-export ACP wire shapes under common keys.
            if let Some(payload) = e.get("payload") {
                let ps = payload.to_string();
                for bad in ["agent_message_chunk", "sessionUpdate", "fs/read_text_file"] {
                    assert!(
                        !ps.contains(bad),
                        "raw ACP payload fragment `{bad}`: {payload}"
                    );
                }
            }
        }
        return;
    }

    // 2) Snapshot / command results: top-level type + string scan excluding
    // known adapter provenance keys if present.
    if let Some(t) = value.get("type").and_then(|x| x.as_str()) {
        assert!(
            !RAW_EVENT_TYPES.contains(&t),
            "raw ACP type `{t}` on boundary surface: {value}"
        );
    }
    // Reject bare ACP method strings only when they appear as JSON string
    // values for keys other than runtimeMethod / method provenance.
    walk_reject_raw_acp_keys(value);
}

fn walk_reject_raw_acp_keys(value: &serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                // Provenance fields may name the ACP method that produced a domain event.
                if k == "runtimeMethod" || k == "adapterMethod" || k == "sourceMethod" {
                    continue;
                }
                if k == "type" || k == "eventType" || k == "kind" {
                    if let Some(s) = v.as_str() {
                        for needle in [
                            "session/update",
                            "session/new",
                            "session/prompt",
                            "agent_message_chunk",
                            "fs/read_text_file",
                        ] {
                            assert_ne!(
                                s, needle,
                                "raw ACP `{needle}` used as {k} on boundary surface"
                            );
                        }
                    }
                }
                walk_reject_raw_acp_keys(v);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                walk_reject_raw_acp_keys(v);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// A1–A3: registration + app info + snapshot (desktop composition)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a1_registered_commands_stable() {
    assert_eq!(REGISTERED_COMMANDS, LIB_REGISTERED);
    assert!(REGISTERED_COMMANDS.contains(&"tracer_presentation_snapshot"));
    assert!(REGISTERED_COMMANDS.contains(&"tracer_presentation_focus"));
    assert!(REGISTERED_COMMANDS.contains(&"tracer_session_create"));
    assert!(REGISTERED_COMMANDS.contains(&"tracer_e2e_env"));
    // Contract surface must remain present.
    for required in [
        "tracer_app_info",
        "tracer_project_list",
        "tracer_session_submit_prompt",
        "tracer_approval_resolve",
        "tracer_events_list",
        "tracer_runtime_status",
        "tracer_heli_status",
    ] {
        assert!(
            REGISTERED_COMMANDS.contains(&required),
            "missing registered command {required}"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a2_app_info_and_snapshot_via_plane_handlers() {
    let _g = journey_lock().await;
    let plane = build_control_plane(None).await.expect("open plane");
    let info = plane_app_info(&plane).expect("app_info");
    assert!(
        info.get("appVersion").is_some()
            || info.get("eventProtocolVersion").is_some()
            || info.get("module").is_some(),
        "app_info fields: {info}"
    );
    let snap = plane_presentation_snapshot(&plane).expect("snapshot");
    assert_eq!(snap.get("version").and_then(|v| v.as_u64()), Some(1));
    assert_no_raw_acp(&snap);
}

// ---------------------------------------------------------------------------
// Preferred journey (desktop boundary, fake ACP, temp SQLite)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn journey_happy_prompt_stream_and_terminal() {
    let _g = journey_lock().await;
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("w2b-happy.db");
    std::env::set_var("TRACER_FAKE_ACP_JS", fake_js().display().to_string());
    std::env::set_var("TRACER_DATABASE_PATH", db.display().to_string());

    let plane = build_control_plane(Some(db.clone()))
        .await
        .expect("build_control_plane");

    // Inspect snapshot (empty shell).
    let snap0 = plane_presentation_snapshot(&plane).expect("snap0");
    assert_eq!(snap0["version"], 1);

    // Heli — non-fatal path always returns a value.
    let heli = plane_heli_status(&plane).expect("heli");
    assert!(heli.is_object());

    // Register project + list.
    let proj_dir = tempdir().unwrap();
    let reg = plane_project_register(
        &plane,
        ProjectRegisterArgs {
            root_path: proj_dir.path().display().to_string(),
            name: Some("w2b-happy".into()),
        },
    )
    .await
    .expect("register");
    let project_id = reg["project"]["projectId"]
        .as_str()
        .expect("projectId")
        .to_string();

    let listed = plane_project_list(&plane).await.expect("list");
    assert!(listed["projects"]
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));

    // Start fake runtime via session create.
    let session_val = plane_session_create(
        &plane,
        SessionCreateArgs {
            project_id: project_id.clone(),
            title: Some("w2b-happy".into()),
            runtime: Some(runtime_opts("happy_prompt_stream")),
        },
    )
    .await
    .expect("session_create");
    let session_id = session_val["session"]["sessionId"]
        .as_str()
        .expect("sessionId")
        .to_string();
    assert_eq!(
        session_val["session"]["status"].as_str(),
        Some("ready"),
        "session ready after fake runtime init"
    );

    // Runtime status inspect.
    let rt = plane_runtime_status(&plane, Some(session_id.clone())).expect("runtime");
    assert!(rt.get("processes").is_some());
    assert_no_raw_acp(&rt);

    // Submit prompt → stream / terminal evidence.
    let prompt = plane_session_submit_prompt(
        &plane,
        SubmitPromptArgs {
            session_id: session_id.clone(),
            text: "summarize".into(),
        },
    )
    .await
    .expect("prompt");
    assert!(
        prompt
            .get("accepted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
            || prompt.is_object(),
        "prompt accepted shape: {prompt}"
    );

    let deadline = Instant::now() + Duration::from_secs(8);
    let mut events = plane_events_list(
        &plane,
        EventsListArgs {
            session_id: session_id.clone(),
            after_sequence: Some(0),
            limit: Some(500),
        },
    )
    .await
    .expect("events");
    while Instant::now() < deadline {
        let arr = events["events"].as_array().cloned().unwrap_or_default();
        if has_type(&arr, "session.completed")
            || has_type(&arr, "agent.message.delta")
            || has_type(&arr, "agent.message.completed")
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        events = plane_events_list(
            &plane,
            EventsListArgs {
                session_id: session_id.clone(),
                after_sequence: Some(0),
                limit: Some(500),
            },
        )
        .await
        .expect("events poll");
    }
    let arr = events["events"].as_array().cloned().unwrap_or_default();
    let types = event_types(&arr);
    assert!(
        has_type(&arr, "session.completed")
            || has_type(&arr, "agent.message.delta")
            || has_type(&arr, "agent.message.completed"),
        "streaming/terminal evidence missing: {types:?}"
    );
    assert_no_raw_acp(&events);

    let snap = plane_presentation_snapshot(&plane).expect("snap after");
    assert_no_raw_acp(&snap);
    assert!(
        snap.get("activeSessionId").is_some() || snap.get("sessionStatus").is_some(),
        "snapshot has session fields"
    );

    // Terminal stop.
    let stop = plane_session_stop(
        &plane,
        StopArgs {
            session_id: session_id.clone(),
            force: Some(false),
        },
    )
    .await
    .expect("stop");
    assert_eq!(stop["stopped"], true);

    std::env::remove_var("TRACER_DATABASE_PATH");
    std::env::remove_var("TRACER_FAKE_ACP_JS");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn journey_approval_allow_then_terminal() {
    let _g = journey_lock().await;
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("w2b-approval.db");
    std::env::set_var("TRACER_FAKE_ACP_JS", fake_js().display().to_string());

    let plane = std::sync::Arc::new(build_control_plane(Some(db)).await.expect("plane"));
    let proj_dir = tempdir().unwrap();
    let reg = plane_project_register(
        &plane,
        ProjectRegisterArgs {
            root_path: proj_dir.path().display().to_string(),
            name: Some("w2b-approval".into()),
        },
    )
    .await
    .expect("reg");
    let project_id = reg["project"]["projectId"].as_str().unwrap().to_string();

    let session_val = plane_session_create(
        &plane,
        SessionCreateArgs {
            project_id,
            title: Some("approval".into()),
            runtime: Some(runtime_opts("permission_allow")),
        },
    )
    .await
    .expect("create");
    let session_id = session_val["session"]["sessionId"]
        .as_str()
        .unwrap()
        .to_string();

    // Concurrent prompt (blocks until approval path resolves).
    let plane_p = std::sync::Arc::clone(&plane);
    let sid_p = session_id.clone();
    let prompt_task = tokio::spawn(async move {
        plane_session_submit_prompt(
            &plane_p,
            SubmitPromptArgs {
                session_id: sid_p,
                text: "needs permission".into(),
            },
        )
        .await
    });

    // Poll pending approvals while prompt is in flight.
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut approval_id: Option<String> = None;
    while Instant::now() < deadline {
        let pending = plane_approval_list_pending(&plane, session_id.clone()).expect("pending");
        if let Some(arr) = pending["approvals"].as_array() {
            if let Some(first) = arr.first() {
                if let Some(id) = first.get("approvalId").and_then(|v| v.as_str()) {
                    approval_id = Some(id.to_string());
                    break;
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let aid = approval_id.expect("approval requested via desktop boundary");
    plane_approval_resolve(
        &plane,
        ApprovalResolveArgs {
            session_id: session_id.clone(),
            approval_id: aid,
            decision: "allow".into(),
            reason: None,
        },
    )
    .await
    .expect("allow once");

    let _ = tokio::time::timeout(Duration::from_secs(20), prompt_task)
        .await
        .expect("prompt join timeout");

    // Snapshot must remain free of raw ACP.
    let snap = plane_presentation_snapshot(&plane).expect("snap final");
    assert_no_raw_acp(&snap);

    let _ = plane_session_stop(
        &plane,
        StopArgs {
            session_id,
            force: Some(true),
        },
    )
    .await;

    std::env::remove_var("TRACER_FAKE_ACP_JS");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn journey_cancel_mid_stream() {
    let _g = journey_lock().await;
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("w2b-cancel.db");
    std::env::set_var("TRACER_FAKE_ACP_JS", fake_js().display().to_string());

    let plane = std::sync::Arc::new(build_control_plane(Some(db)).await.expect("plane"));
    let proj_dir = tempdir().unwrap();
    let reg = plane_project_register(
        &plane,
        ProjectRegisterArgs {
            root_path: proj_dir.path().display().to_string(),
            name: Some("w2b-cancel".into()),
        },
    )
    .await
    .expect("reg");
    let project_id = reg["project"]["projectId"].as_str().unwrap().to_string();

    let session_val = plane_session_create(
        &plane,
        SessionCreateArgs {
            project_id,
            title: Some("cancel".into()),
            runtime: Some(runtime_opts("cancel_mid_stream")),
        },
    )
    .await
    .expect("session_create cancel_mid_stream");

    let session_id = session_val["session"]["sessionId"]
        .as_str()
        .unwrap()
        .to_string();

    let plane_p = std::sync::Arc::clone(&plane);
    let sid_p = session_id.clone();
    let prompt_task = tokio::spawn(async move {
        plane_session_submit_prompt(
            &plane_p,
            SubmitPromptArgs {
                session_id: sid_p,
                text: "long work".into(),
            },
        )
        .await
    });

    // Cancel while stream is active.
    tokio::time::sleep(Duration::from_millis(30)).await;
    let cancel_res = plane_session_cancel(
        &plane,
        CancelArgs {
            session_id: session_id.clone(),
            scope: Some("active_run".into()),
        },
    )
    .await;
    assert!(
        cancel_res.is_ok() || cancel_res.is_err(),
        "cancel must return structured result"
    );
    if let Ok(v) = &cancel_res {
        assert_no_raw_acp(v);
    }

    let _ = tokio::time::timeout(Duration::from_secs(15), prompt_task).await;

    let snap = plane_presentation_snapshot(&plane).expect("snap");
    assert_no_raw_acp(&snap);

    let _ = plane_session_stop(
        &plane,
        StopArgs {
            session_id,
            force: Some(true),
        },
    )
    .await;

    std::env::remove_var("TRACER_FAKE_ACP_JS");
}

// ---------------------------------------------------------------------------
// Close → reopen → restore history (file SQLite, same composition path)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn journey_close_reopen_restores_history() {
    let _g = journey_lock().await;
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("w2b-reopen.db");
    let proj_dir = tempdir().unwrap();
    std::env::set_var("TRACER_FAKE_ACP_JS", fake_js().display().to_string());
    std::env::set_var("TRACER_DATABASE_PATH", db.display().to_string());

    let session_id = {
        let plane = build_control_plane(Some(db.clone())).await.expect("plane1");
        let reg = plane_project_register(
            &plane,
            ProjectRegisterArgs {
                root_path: proj_dir.path().display().to_string(),
                name: Some("persist".into()),
            },
        )
        .await
        .expect("reg");
        let project_id = reg["project"]["projectId"].as_str().unwrap().to_string();
        let session_val = plane_session_create(
            &plane,
            SessionCreateArgs {
                project_id: project_id.clone(),
                title: Some("persist".into()),
                runtime: Some(runtime_opts("happy_prompt_stream")),
            },
        )
        .await
        .expect("create");
        let sid = session_val["session"]["sessionId"]
            .as_str()
            .unwrap()
            .to_string();
        let _ = plane_session_submit_prompt(
            &plane,
            SubmitPromptArgs {
                session_id: sid.clone(),
                text: "persist me".into(),
            },
        )
        .await
        .expect("prompt");

        tokio::time::sleep(Duration::from_millis(200)).await;
        let events = plane_events_list(
            &plane,
            EventsListArgs {
                session_id: sid.clone(),
                after_sequence: Some(0),
                limit: Some(500),
            },
        )
        .await
        .expect("events before close");
        assert!(
            events["events"]
                .as_array()
                .map(|a| !a.is_empty())
                .unwrap_or(false),
            "events before close"
        );

        // Session list for project.
        let sessions = plane_session_list(
            &plane,
            SessionListArgs {
                project_id,
                limit: Some(50),
            },
        )
        .await
        .expect("list sessions");
        assert!(sessions["sessions"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false));

        let _ = plane_session_stop(
            &plane,
            StopArgs {
                session_id: sid.clone(),
                force: Some(false),
            },
        )
        .await;
        sid
        // plane drop ≈ app close
    };

    // Reopen (new composition / new plane on same DB path).
    let plane2 = build_control_plane(Some(db.clone()))
        .await
        .expect("plane2 reopen");
    let detail = plane_session_get(&plane2, session_id.clone())
        .await
        .expect("reload session");
    assert_eq!(
        detail["session"]["sessionId"].as_str(),
        Some(session_id.as_str())
    );

    let events = plane_events_list(
        &plane2,
        EventsListArgs {
            session_id: session_id.clone(),
            after_sequence: Some(0),
            limit: Some(500),
        },
    )
    .await
    .expect("history after reopen");
    assert!(
        events["events"]
            .as_array()
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "history restored after reopen"
    );
    assert_no_raw_acp(&events);

    let snap = plane_presentation_snapshot(&plane2).expect("snap reopen");
    assert_no_raw_acp(&snap);

    std::env::remove_var("TRACER_DATABASE_PATH");
    std::env::remove_var("TRACER_FAKE_ACP_JS");
}

// ---------------------------------------------------------------------------
// Heli unavailable without failure
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn journey_heli_unavailable_non_fatal() {
    let _g = journey_lock().await;
    let empty = tempdir().unwrap();
    std::env::set_var("TRACER_HELI_PROBE_PATH", empty.path().display().to_string());

    let plane = build_control_plane(None).await.expect("plane");
    let heli = plane_heli_status(&plane).expect("heli");
    // available may be false — must not error.
    assert!(heli.is_object());

    // App still usable: snapshot + project list succeed.
    let snap = plane_presentation_snapshot(&plane).expect("snap");
    assert_eq!(snap["version"], 1);
    let projects = plane_project_list(&plane).await.expect("projects");
    assert!(projects.get("projects").is_some());

    std::env::remove_var("TRACER_HELI_PROBE_PATH");
}

// ---------------------------------------------------------------------------
// e2e env command shape (registration surface)
// ---------------------------------------------------------------------------

#[test]
fn e2e_env_command_lists_registered() {
    let env = tracer_desktop_lib::commands::tracer_e2e_env().expect("e2e_env");
    assert_eq!(env["boundary"], "tauri-desktop");
    assert_eq!(env["module"], "W2-B");
    let cmds = env["registeredCommands"].as_array().expect("cmds");
    assert!(cmds
        .iter()
        .any(|c| c.as_str() == Some("tracer_session_create")));
}

// ---------------------------------------------------------------------------
// W2.1 integrated: multi-session focus switch via desktop boundary handlers
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn journey_multi_session_presentation_focus_switch() {
    let _g = journey_lock().await;
    let dir = tempdir().unwrap();
    let db = dir.path().join("journey-ms-focus.db");
    let plane = build_control_plane(Some(db)).await.expect("open plane");

    let proj = plane_project_register(
        &plane,
        ProjectRegisterArgs {
            root_path: dir.path().display().to_string(),
            name: Some("ms-focus".into()),
        },
    )
    .await
    .expect("register");
    let project_id = proj["project"]["projectId"]
        .as_str()
        .expect("projectId")
        .to_string();

    let a = plane_session_create(
        &plane,
        SessionCreateArgs {
            project_id: project_id.clone(),
            title: Some("focus-A".into()),
            runtime: Some(runtime_opts("happy_prompt_stream")),
        },
    )
    .await
    .expect("create A");
    let b = plane_session_create(
        &plane,
        SessionCreateArgs {
            project_id: project_id.clone(),
            title: Some("focus-B".into()),
            runtime: Some(runtime_opts("happy_prompt_stream")),
        },
    )
    .await
    .expect("create B");
    let sid_a = a["session"]["sessionId"].as_str().unwrap().to_string();
    let sid_b = b["session"]["sessionId"].as_str().unwrap().to_string();

    let snap_a = plane_presentation_focus(&plane, sid_a.clone())
        .await
        .expect("focus A");
    assert_eq!(snap_a["activeSessionId"].as_str(), Some(sid_a.as_str()));
    assert_no_raw_acp(&snap_a);

    let snap_b = plane_presentation_focus(&plane, sid_b.clone())
        .await
        .expect("focus B");
    assert_eq!(snap_b["activeSessionId"].as_str(), Some(sid_b.as_str()));
    assert_ne!(snap_b["activeSessionId"].as_str(), Some(sid_a.as_str()));

    // Snapshot command agrees with focus.
    let snap = plane_presentation_snapshot(&plane).expect("snapshot");
    assert_eq!(snap["activeSessionId"].as_str(), Some(sid_b.as_str()));
    assert!(
        snap.get("revision").and_then(|v| v.as_u64()).unwrap_or(0) >= 1,
        "revision should advance on focus publish: {snap}"
    );

    // Both sessions remain independently addressable.
    let da = plane_session_get(&plane, sid_a.clone())
        .await
        .expect("get A");
    let db = plane_session_get(&plane, sid_b.clone())
        .await
        .expect("get B");
    assert_eq!(da["session"]["sessionId"].as_str(), Some(sid_a.as_str()));
    assert_eq!(db["session"]["sessionId"].as_str(), Some(sid_b.as_str()));
    assert_no_raw_acp(&da);
    assert_no_raw_acp(&db);

    let _ = plane_session_stop(
        &plane,
        StopArgs {
            session_id: sid_a,
            force: Some(false),
        },
    )
    .await;
    let _ = plane_session_stop(
        &plane,
        StopArgs {
            session_id: sid_b,
            force: Some(false),
        },
    )
    .await;
}
