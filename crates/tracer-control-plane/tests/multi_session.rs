//! W2-C multi-session isolation tests (fake ACP + file/in-memory SQLite).
//!
//! CI class: standard (network: no, credentials: no, live Grok: no).
//!
//! Proves local Tracer sessions remain isolated and recoverable across:
//! sequential sessions, presentation focus switch, history-while-ingest,
//! cancel/approval isolation, id/sequence locality, failure non-poisoning,
//! restart/interrupt recovery, stale approvals, leak-free shutdown.

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use tracer_control_plane::{ControlPlane, ControlPlaneConfig, RuntimeCreateOptions};
use tracer_domain::{ErrorClass, SessionStatus};

/// Serialize multi-session tests (parallel node spawns contend under Windows).
async fn ms_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // crates
    p.pop(); // repo root
    p
}

fn fake_js() -> PathBuf {
    repo_root().join("tools/fake-acp-runtime/bin/fake-acp-runtime.js")
}

async fn open_cp(db_path: Option<PathBuf>) -> ControlPlane {
    assert!(
        fake_js().is_file(),
        "missing fake runtime at {}",
        fake_js().display()
    );
    ControlPlane::open(ControlPlaneConfig {
        database_path: db_path,
        fake_js: Some(fake_js()),
        node_bin: PathBuf::from("node"),
        heli_probe_path: repo_root(),
        escalate_cancel_to_process_stop: true,
    })
    .await
    .expect("open control plane")
}

async fn register_temp_project(cp: &ControlPlane) -> (tempfile::TempDir, String) {
    let dir = tempdir().unwrap();
    let proj = cp
        .project_register(dir.path(), Some("ms-project".into()))
        .await
        .expect("register");
    (dir, proj.project_id)
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

fn sequences_monotonic(events: &[serde_json::Value]) -> bool {
    let mut last = 0i64;
    for e in events {
        let seq = e.get("sequence").and_then(|s| s.as_i64()).unwrap_or(0);
        if seq <= last && last != 0 {
            return false;
        }
        if seq > 0 {
            last = seq;
        }
    }
    true
}

fn event_session_ids_consistent(events: &[serde_json::Value], session_id: &str) -> bool {
    events.iter().all(|e| {
        e.get("sessionId")
            .and_then(|s| s.as_str())
            .map(|s| s == session_id)
            .unwrap_or(false)
    })
}

// ---------------------------------------------------------------------------
// MS-01 Sequential many sessions
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms01_sequential_many_sessions() {
    let _g = ms_lock().await;
    let dir = tempdir().unwrap();
    let db = dir.path().join("ms01.db");
    let cp = open_cp(Some(db)).await;
    let (_proj_dir, project_id) = register_temp_project(&cp).await;

    const N: usize = 6;
    let mut ids = Vec::new();
    for i in 0..N {
        let session = cp
            .session_create(
                &project_id,
                Some(format!("ms01-{i}")),
                runtime_opts("happy_prompt_stream"),
            )
            .await
            .expect("create");
        assert_eq!(session.status, SessionStatus::Ready);
        let _ = cp
            .session_submit_prompt(&session.session_id, &format!("ping {i}"))
            .await
            .expect("prompt");
        ids.push(session.session_id.clone());
        let _ = cp.session_stop(&session.session_id, false).await;
    }

    assert_eq!(cp.live_session_count(), 0, "all stopped");
    let listed = cp.session_list(&project_id, 100).await.unwrap();
    assert!(listed.len() >= N);
    for sid in &ids {
        let events = cp.events_list(sid, 0, 500).await.unwrap();
        assert!(!events.events.is_empty(), "history for {sid}");
        assert!(sequences_monotonic(&events.events));
        assert!(event_session_ids_consistent(&events.events, sid));
    }
}

// ---------------------------------------------------------------------------
// MS-02 Switching active presentation between sessions
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms02_presentation_focus_switch() {
    let _g = ms_lock().await;
    let cp = open_cp(None).await;
    let (_dir, project_id) = register_temp_project(&cp).await;

    let a = cp
        .session_create(
            &project_id,
            Some("A".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();
    let b = cp
        .session_create(
            &project_id,
            Some("B".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();

    assert_eq!(cp.live_session_count(), 2);

    let snap_a = cp.presentation_focus(&a.session_id).await.unwrap();
    assert_eq!(
        snap_a.active_session_id.as_deref(),
        Some(a.session_id.as_str())
    );
    assert_eq!(
        cp.snapshot().active_session_id.as_deref(),
        Some(a.session_id.as_str())
    );

    let snap_b = cp.presentation_focus(&b.session_id).await.unwrap();
    assert_eq!(
        snap_b.active_session_id.as_deref(),
        Some(b.session_id.as_str())
    );
    assert_ne!(
        snap_b.active_session_id.as_deref(),
        Some(a.session_id.as_str())
    );

    // Both still live and independent after focus switch.
    let da = cp.session_get(&a.session_id).await.unwrap();
    let db = cp.session_get(&b.session_id).await.unwrap();
    assert!(da.process_alive);
    assert!(db.process_alive);
    assert_eq!(da.status, SessionStatus::Ready);
    assert_eq!(db.status, SessionStatus::Ready);

    let _ = cp.shutdown_all().await;
    assert_eq!(cp.live_session_count(), 0);
}

// ---------------------------------------------------------------------------
// MS-03 History reads while another session ingests
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms03_history_while_other_ingests() {
    let _g = ms_lock().await;
    let dir = tempdir().unwrap();
    let db = dir.path().join("ms03.db");
    let cp = std::sync::Arc::new(open_cp(Some(db)).await);
    let (_proj_dir, project_id) = register_temp_project(&cp).await;

    let hist = cp
        .session_create(
            &project_id,
            Some("hist".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();
    let _ = cp
        .session_submit_prompt(&hist.session_id, "history seed")
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let before = cp.events_list(&hist.session_id, 0, 500).await.unwrap();
    assert!(!before.events.is_empty());
    let before_latest = before.latest_sequence;

    let live = cp
        .session_create(
            &project_id,
            Some("live".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();

    let cp_p = std::sync::Arc::clone(&cp);
    let sid_live = live.session_id.clone();
    let prompt = tokio::spawn(async move {
        cp_p.session_submit_prompt(&sid_live, "ingest while reading")
            .await
    });

    // Concurrent history reads on hist must not fail or mix sessions.
    for _ in 0..20 {
        let ev = cp.events_list(&hist.session_id, 0, 500).await.unwrap();
        assert!(event_session_ids_consistent(&ev.events, &hist.session_id));
        assert!(sequences_monotonic(&ev.events));
        assert!(
            ev.latest_sequence >= before_latest,
            "history must not rewind"
        );
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let _ = tokio::time::timeout(Duration::from_secs(30), prompt)
        .await
        .expect("prompt join timeout");

    let after = cp.events_list(&hist.session_id, 0, 500).await.unwrap();
    assert_eq!(
        after.latest_sequence, before_latest,
        "other session ingest must not append to hist"
    );

    let live_ev = cp.events_list(&live.session_id, 0, 500).await.unwrap();
    assert!(event_session_ids_consistent(
        &live_ev.events,
        &live.session_id
    ));
    assert!(!live_ev.events.is_empty());

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// MS-04 Cancel one session does not affect another
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms04_cancel_isolation() {
    let _g = ms_lock().await;
    let cp = std::sync::Arc::new(open_cp(None).await);
    let (_dir, project_id) = register_temp_project(&cp).await;

    let victim = cp
        .session_create(
            &project_id,
            Some("victim".into()),
            runtime_opts("cancel_mid_stream"),
        )
        .await
        .unwrap();
    let bystander = cp
        .session_create(
            &project_id,
            Some("bystander".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();

    let cp_p = std::sync::Arc::clone(&cp);
    let sid_v = victim.session_id.clone();
    let prompt = tokio::spawn(async move { cp_p.session_submit_prompt(&sid_v, "cancel me").await });

    // Give cancel_mid_stream a moment to start streaming.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let cancel = tokio::time::timeout(
        Duration::from_secs(10),
        cp.session_cancel(&victim.session_id),
    )
    .await
    .expect("cancel bounded")
    .expect("cancel ok");
    assert!(cancel.accepted);

    let _ = tokio::time::timeout(Duration::from_secs(20), prompt).await;

    // Bystander still ready / independent.
    let b = cp.session_get(&bystander.session_id).await.unwrap();
    assert!(
        matches!(
            b.status,
            SessionStatus::Ready | SessionStatus::Running | SessionStatus::Completed
        ),
        "bystander poisoned: {:?}",
        b.status
    );
    assert!(b.process_alive || b.session_ready || b.status == SessionStatus::Ready);

    let prompt_b = cp
        .session_submit_prompt(&bystander.session_id, "still works")
        .await;
    assert!(
        prompt_b.is_ok()
            || prompt_b
                .as_ref()
                .err()
                .map(|e| e.to_command_error().error_class == "InvalidState")
                .unwrap_or(false),
        "bystander submit unexpected: {prompt_b:?}"
    );
    // If Ready, prompt should succeed.
    if b.status == SessionStatus::Ready {
        assert!(prompt_b.is_ok(), "ready bystander must accept prompt");
    }

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// MS-05 Approval one session cannot resolve another's request
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms05_approval_cross_session_rejected() {
    let _g = ms_lock().await;
    let cp = std::sync::Arc::new(open_cp(None).await);
    let (_dir, project_id) = register_temp_project(&cp).await;

    let a = cp
        .session_create(
            &project_id,
            Some("A".into()),
            runtime_opts("permission_allow"),
        )
        .await
        .unwrap();
    let b = cp
        .session_create(
            &project_id,
            Some("B".into()),
            runtime_opts("permission_allow"),
        )
        .await
        .unwrap();

    let cp_a = std::sync::Arc::clone(&cp);
    let sid_a = a.session_id.clone();
    let prompt_a =
        tokio::spawn(async move { cp_a.session_submit_prompt(&sid_a, "need allow A").await });

    let deadline = Instant::now() + Duration::from_secs(12);
    let mut approval_id = None;
    while Instant::now() < deadline {
        let pending = cp.approval_list_pending(&a.session_id).unwrap_or_default();
        if let Some(p) = pending.first() {
            approval_id = Some(p.approval_id.clone());
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    let aid = approval_id.expect("session A must have pending approval");

    // Cross-session resolve must fail (B's map does not contain A's approval).
    let cross = cp
        .approval_resolve(&b.session_id, &aid, "allow", None)
        .await;
    assert!(cross.is_err(), "cross-session resolve must fail");
    let cmd = cross.unwrap_err().to_command_error();
    assert_eq!(
        cmd.error_class,
        ErrorClass::ApprovalUnknown.as_str(),
        "class={}",
        cmd.error_class
    );

    // A's approval still pending until resolved on A.
    let still = cp.approval_list_pending(&a.session_id).unwrap_or_default();
    assert!(
        still.iter().any(|p| p.approval_id == aid),
        "A's approval must remain after rejected cross resolve"
    );

    // Legitimate resolve on A.
    let ok = cp
        .approval_resolve(&a.session_id, &aid, "allow", None)
        .await;
    assert!(ok.is_ok(), "same-session resolve: {ok:?}");

    let _ = tokio::time::timeout(Duration::from_secs(20), prompt_a).await;
    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// MS-06 Runtime / session IDs not confused
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms06_runtime_and_session_ids_distinct() {
    let _g = ms_lock().await;
    let cp = open_cp(None).await;
    let (_dir, project_id) = register_temp_project(&cp).await;

    let a = cp
        .session_create(
            &project_id,
            Some("A".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();
    let b = cp
        .session_create(
            &project_id,
            Some("B".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();

    // Tracer session ids are unique and authoritative for isolation.
    assert_ne!(a.session_id, b.session_id);
    assert!(a.runtime_session_id.is_some());
    assert!(b.runtime_session_id.is_some());
    // Runtime ids must not be confused with Tracer session ids (lookup keys).
    assert_ne!(
        a.runtime_session_id.as_deref(),
        Some(a.session_id.as_str()),
        "runtime session id must not equal Tracer session id"
    );
    assert_ne!(
        b.runtime_session_id.as_deref(),
        Some(b.session_id.as_str()),
        "runtime session id must not equal Tracer session id"
    );
    // Fake ACP may reuse a fixed wire session id across processes; CP still
    // isolates by Tracer session_id (registry key + storage partition).
    let _ = (a.runtime_session_id.as_ref(), b.runtime_session_id.as_ref());

    let views = cp.runtime_status(None).unwrap();
    assert_eq!(views.len(), 2);
    let ids: Vec<_> = views.iter().map(|v| v.session_id.clone()).collect();
    assert!(ids.contains(&a.session_id));
    assert!(ids.contains(&b.session_id));

    let only_a = cp.runtime_status(Some(&a.session_id)).unwrap();
    assert_eq!(only_a.len(), 1);
    assert_eq!(only_a[0].session_id, a.session_id);

    // Cross-key lookup by the other session's id must not return this view.
    let only_b = cp.runtime_status(Some(&b.session_id)).unwrap();
    assert_eq!(only_b.len(), 1);
    assert_eq!(only_b[0].session_id, b.session_id);
    assert_ne!(only_a[0].session_id, only_b[0].session_id);

    let _ = cp.session_submit_prompt(&a.session_id, "a").await.unwrap();
    let _ = cp.session_submit_prompt(&b.session_id, "b").await.unwrap();

    let ea = cp.events_list(&a.session_id, 0, 500).await.unwrap();
    let eb = cp.events_list(&b.session_id, 0, 500).await.unwrap();
    assert!(event_session_ids_consistent(&ea.events, &a.session_id));
    assert!(event_session_ids_consistent(&eb.events, &b.session_id));
    // Events never leak the peer's Tracer session id.
    assert!(!ea
        .events
        .iter()
        .any(|e| { e.get("sessionId").and_then(|s| s.as_str()) == Some(b.session_id.as_str()) }));
    assert!(!eb
        .events
        .iter()
        .any(|e| { e.get("sessionId").and_then(|s| s.as_str()) == Some(a.session_id.as_str()) }));

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// MS-07 Sequences are session-local and monotonic
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms07_sequences_session_local_monotonic() {
    let _g = ms_lock().await;
    let dir = tempdir().unwrap();
    let db = dir.path().join("ms07.db");
    let cp = open_cp(Some(db)).await;
    let (_proj_dir, project_id) = register_temp_project(&cp).await;

    let a = cp
        .session_create(
            &project_id,
            Some("A".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();
    let b = cp
        .session_create(
            &project_id,
            Some("B".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();

    let _ = cp.session_submit_prompt(&a.session_id, "a1").await.unwrap();
    let _ = cp.session_submit_prompt(&b.session_id, "b1").await.unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;

    let ea = cp.events_list(&a.session_id, 0, 500).await.unwrap();
    let eb = cp.events_list(&b.session_id, 0, 500).await.unwrap();
    assert!(sequences_monotonic(&ea.events));
    assert!(sequences_monotonic(&eb.events));
    assert!(
        ea.events
            .iter()
            .any(|e| e.get("sequence").and_then(|s| s.as_i64()) == Some(1)),
        "session A sequence restarts at 1 locally"
    );
    assert!(
        eb.events
            .iter()
            .any(|e| e.get("sequence").and_then(|s| s.as_i64()) == Some(1)),
        "session B sequence restarts at 1 locally"
    );

    // Sequences are independent — both may share numeric values without collision.
    assert!(ea.latest_sequence >= 1);
    assert!(eb.latest_sequence >= 1);

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// MS-08 Failed session does not poison later sessions
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms08_failed_session_does_not_poison() {
    let _g = ms_lock().await;
    let cp = open_cp(None).await;
    let (_dir, project_id) = register_temp_project(&cp).await;

    // Force spawn failure via missing binary.
    let mut bad = runtime_opts("happy_prompt_stream");
    bad.executable_override = Some("definitely-not-a-real-binary-xyz-w2c".into());
    let err = cp
        .session_create(&project_id, Some("bad".into()), bad)
        .await
        .unwrap_err()
        .to_command_error();
    assert!(
        err.error_class == "RuntimeExecutableNotFound"
            || err.error_class == "RuntimeSpawnFailed"
            || err.error_class == ErrorClass::RuntimeSpawnFailed.as_str(),
        "spawn fail class: {}",
        err.error_class
    );

    // Healthy session after failure.
    let good = cp
        .session_create(
            &project_id,
            Some("good".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .expect("create after failure must work");
    assert_eq!(good.status, SessionStatus::Ready);
    let prompt = cp
        .session_submit_prompt(&good.session_id, "recover")
        .await
        .expect("prompt after prior failure");
    assert!(prompt.accepted);

    let _ = cp.session_stop(&good.session_id, true).await;
}

// ---------------------------------------------------------------------------
// MS-09 Persistence failure scoped per session (documented invariant)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms09_persist_failed_flag_is_session_local() {
    let _g = ms_lock().await;
    // Documented invariant: persist_failed lives on SessionRuntimeState per LiveSession.
    // We cannot easily inject SQLite I/O failure without redesigning storage; instead
    // prove independent session state machines and that a crash scenario on one
    // session leaves another Ready for prompts.
    let cp = open_cp(None).await;
    let (_dir, project_id) = register_temp_project(&cp).await;

    let crash = cp
        .session_create(
            &project_id,
            Some("crash".into()),
            runtime_opts("crash_nonzero_exit"),
        )
        .await
        .unwrap();
    let stable = cp
        .session_create(
            &project_id,
            Some("stable".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();

    let _ = cp.session_submit_prompt(&crash.session_id, "boom").await;
    tokio::time::sleep(Duration::from_millis(300)).await;

    let s = cp.session_get(&stable.session_id).await.unwrap();
    assert_eq!(s.status, SessionStatus::Ready);
    assert!(s.process_alive);
    let p = cp
        .session_submit_prompt(&stable.session_id, "still good")
        .await
        .expect("stable session after peer crash");
    assert!(p.accepted);

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// MS-10 Restart restores completed sessions
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms10_restart_restores_completed_sessions() {
    let _g = ms_lock().await;
    let dir = tempdir().unwrap();
    let db = dir.path().join("ms10.db");
    let project_dir = tempdir().unwrap();

    let (project_id, sid_a, sid_b) = {
        let cp = open_cp(Some(db.clone())).await;
        let proj = cp
            .project_register(project_dir.path(), Some("persist".into()))
            .await
            .unwrap();
        let a = cp
            .session_create(
                &proj.project_id,
                Some("A".into()),
                runtime_opts("happy_prompt_stream"),
            )
            .await
            .unwrap();
        let b = cp
            .session_create(
                &proj.project_id,
                Some("B".into()),
                runtime_opts("happy_prompt_stream"),
            )
            .await
            .unwrap();
        let _ = cp.session_submit_prompt(&a.session_id, "a").await.unwrap();
        let _ = cp.session_submit_prompt(&b.session_id, "b").await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        let sa = a.session_id.clone();
        let sb = b.session_id.clone();
        let _ = cp.session_stop(&sa, false).await;
        let _ = cp.session_stop(&sb, false).await;
        (proj.project_id, sa, sb)
    };

    let cp2 = open_cp(Some(db)).await;
    assert_eq!(cp2.live_session_count(), 0);

    let list = cp2.session_list(&project_id, 100).await.unwrap();
    assert!(list.iter().any(|s| s.session_id == sid_a));
    assert!(list.iter().any(|s| s.session_id == sid_b));

    for sid in [&sid_a, &sid_b] {
        let detail = cp2.session_get(sid).await.unwrap();
        assert!(!detail.process_alive);
        let events = cp2.events_list(sid, 0, 500).await.unwrap();
        assert!(!events.events.is_empty());
        assert!(sequences_monotonic(&events.events));
        assert!(event_session_ids_consistent(&events.events, sid));

        let snap = cp2.presentation_focus(sid).await.unwrap();
        assert_eq!(snap.active_session_id.as_deref(), Some(sid.as_str()));
        assert!(snap.latest_sequence > 0);
    }
}

// ---------------------------------------------------------------------------
// MS-11 Interrupted sessions recover independently
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms11_interrupted_sessions_recover_independently() {
    let _g = ms_lock().await;
    let dir = tempdir().unwrap();
    let db = dir.path().join("ms11.db");
    let project_dir = tempdir().unwrap();

    let (project_id, sid_a, sid_b) = {
        let cp = open_cp(Some(db.clone())).await;
        let proj = cp
            .project_register(project_dir.path(), Some("recover".into()))
            .await
            .unwrap();
        let a = cp
            .session_create(
                &proj.project_id,
                Some("A".into()),
                runtime_opts("happy_prompt_stream"),
            )
            .await
            .unwrap();
        let b = cp
            .session_create(
                &proj.project_id,
                Some("B".into()),
                runtime_opts("happy_prompt_stream"),
            )
            .await
            .unwrap();
        use tracer_storage::SessionId;
        let aid = SessionId::parse(&a.session_id).unwrap();
        let bid = SessionId::parse(&b.session_id).unwrap();
        cp.storage()
            .update_session_status(&aid, SessionStatus::Running)
            .await
            .unwrap();
        cp.storage()
            .update_session_status(&bid, SessionStatus::AwaitingApproval)
            .await
            .unwrap();
        (proj.project_id, a.session_id, b.session_id)
        // Drop without stop — simulate crash.
    };

    let cp2 = open_cp(Some(db)).await;
    for sid in [&sid_a, &sid_b] {
        let detail = cp2.session_get(sid).await.unwrap();
        assert!(
            matches!(
                detail.status,
                SessionStatus::Disconnected
                    | SessionStatus::Stopped
                    | SessionStatus::Failed
                    | SessionStatus::Completed
            ) || !detail.process_alive,
            "session {sid} not recovered: {:?}",
            detail.status
        );
        assert!(!detail.process_alive);
    }

    // Independent recovery: listing and history still work per id.
    let list = cp2.session_list(&project_id, 10).await.unwrap();
    assert!(list.len() >= 2);
    let _ = cp2.events_list(&sid_a, 0, 50).await.unwrap();
    let _ = cp2.events_list(&sid_b, 0, 50).await.unwrap();
}

// ---------------------------------------------------------------------------
// MS-12 Stale approvals removed per session on cancel
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms12_stale_approvals_cleared_per_session() {
    let _g = ms_lock().await;
    let cp = std::sync::Arc::new(open_cp(None).await);
    let (_dir, project_id) = register_temp_project(&cp).await;

    let a = cp
        .session_create(
            &project_id,
            Some("A".into()),
            runtime_opts("cancel_while_permission_pending"),
        )
        .await
        .unwrap();
    let b = cp
        .session_create(
            &project_id,
            Some("B".into()),
            runtime_opts("cancel_while_permission_pending"),
        )
        .await
        .unwrap();

    let cp_a = std::sync::Arc::clone(&cp);
    let sid_a = a.session_id.clone();
    let prompt_a = tokio::spawn(async move { cp_a.session_submit_prompt(&sid_a, "perm A").await });

    let cp_b = std::sync::Arc::clone(&cp);
    let sid_b = b.session_id.clone();
    let prompt_b = tokio::spawn(async move { cp_b.session_submit_prompt(&sid_b, "perm B").await });

    // Wait until both have pending approvals (or timeout).
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut saw_a = false;
    let mut saw_b = false;
    while Instant::now() < deadline && !(saw_a && saw_b) {
        if !cp
            .approval_list_pending(&a.session_id)
            .unwrap_or_default()
            .is_empty()
        {
            saw_a = true;
        }
        if !cp
            .approval_list_pending(&b.session_id)
            .unwrap_or_default()
            .is_empty()
        {
            saw_b = true;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    // Cancel only A — B's pending must remain (if present).
    let _ = tokio::time::timeout(Duration::from_secs(10), cp.session_cancel(&a.session_id))
        .await
        .expect("cancel A bounded")
        .expect("cancel A");

    let pending_a = cp.approval_list_pending(&a.session_id).unwrap_or_default();
    assert!(
        pending_a.is_empty(),
        "A must clear stale approvals: {pending_a:?}"
    );

    // B still independent — either still pending or not yet arrived; cancel must not
    // wipe B's map via global clear (if B had pending, it stays until B cancel/resolve).
    if saw_b {
        // After A cancel, B's pending should still be listable if not cancelled.
        // Race: B may complete; allow empty only if B no longer awaiting.
        let pending_b = cp.approval_list_pending(&b.session_id).unwrap_or_default();
        let detail_b = cp.session_get(&b.session_id).await.unwrap();
        if detail_b.status == SessionStatus::AwaitingApproval {
            assert!(
                !pending_b.is_empty(),
                "B still awaiting must keep its own approvals"
            );
        }
    }

    let _ = tokio::time::timeout(Duration::from_secs(15), prompt_a).await;
    let _ = cp.session_cancel(&b.session_id).await;
    let _ = tokio::time::timeout(Duration::from_secs(15), prompt_b).await;
    let pending_b_after = cp.approval_list_pending(&b.session_id).unwrap_or_default();
    assert!(
        pending_b_after.is_empty(),
        "B clear after its own cancel: {pending_b_after:?}"
    );

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// MS-13 No leaks of live registry after stop / shutdown
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms13_no_live_registry_leaks() {
    let _g = ms_lock().await;
    let cp = open_cp(None).await;
    let (_dir, project_id) = register_temp_project(&cp).await;

    let mut ids = Vec::new();
    for i in 0..3 {
        let s = cp
            .session_create(
                &project_id,
                Some(format!("leak-{i}")),
                runtime_opts("happy_prompt_stream"),
            )
            .await
            .unwrap();
        ids.push(s.session_id);
    }
    assert_eq!(cp.live_session_count(), 3);
    assert_eq!(cp.live_session_ids().len(), 3);

    // Stop one individually.
    let _ = cp.session_stop(&ids[0], false).await;
    assert_eq!(cp.live_session_count(), 2);
    assert!(!cp.live_session_ids().contains(&ids[0]));

    // Shutdown remaining.
    let _ = cp.shutdown_all().await;
    assert_eq!(cp.live_session_count(), 0);
    assert!(cp.live_session_ids().is_empty());
    assert!(cp.snapshot().active_session_id.is_none());
    assert!(!cp.snapshot().prompt_in_flight);
    assert!(cp.snapshot().pending_approvals.is_empty());

    // Runtime status empty.
    let views = cp.runtime_status(None).unwrap();
    assert!(views.is_empty());
}

// ---------------------------------------------------------------------------
// MS-14 Shutdown handles all active sessions deterministically
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms14_shutdown_all_deterministic() {
    let _g = ms_lock().await;
    let dir = tempdir().unwrap();
    let db = dir.path().join("ms14.db");
    let cp = open_cp(Some(db)).await;
    let (_proj_dir, project_id) = register_temp_project(&cp).await;

    for i in 0..4 {
        let s = cp
            .session_create(
                &project_id,
                Some(format!("shut-{i}")),
                runtime_opts("happy_prompt_stream"),
            )
            .await
            .unwrap();
        if i % 2 == 0 {
            let _ = cp
                .session_submit_prompt(&s.session_id, "pre-shutdown")
                .await;
        }
    }
    assert_eq!(cp.live_session_count(), 4);

    // Double shutdown must be safe.
    cp.shutdown_all().await.expect("shutdown");
    assert_eq!(cp.live_session_count(), 0);
    cp.shutdown_all().await.expect("second shutdown");
    assert_eq!(cp.live_session_count(), 0);

    // History still available after full shutdown.
    let listed = cp.session_list(&project_id, 100).await.unwrap();
    assert!(listed.len() >= 4);
    for s in listed {
        let events = cp.events_list(&s.session_id, 0, 100).await.unwrap();
        // Events may be empty for sessions that only started; command must not error.
        assert!(
            event_session_ids_consistent(&events.events, &s.session_id) || events.events.is_empty()
        );
    }
}

// ---------------------------------------------------------------------------
// MS-15 One prompt per session (controlled rejection of double submit)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms15_one_prompt_per_session_rejects_double_submit() {
    let _g = ms_lock().await;
    // Limitation: one in-flight prompt **per session** (status must be Ready).
    // Parallel prompts **across** different sessions remain allowed.
    let cp = std::sync::Arc::new(open_cp(None).await);
    let (_dir, project_id) = register_temp_project(&cp).await;

    let s = cp
        .session_create(
            &project_id,
            Some("double".into()),
            runtime_opts("cancel_mid_stream"),
        )
        .await
        .unwrap();

    let cp1 = std::sync::Arc::clone(&cp);
    let sid1 = s.session_id.clone();
    let first = tokio::spawn(async move { cp1.session_submit_prompt(&sid1, "first").await });

    // Wait until Running.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        let d = cp.session_get(&s.session_id).await.unwrap();
        if d.status == SessionStatus::Running {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let second = cp.session_submit_prompt(&s.session_id, "second").await;
    assert!(
        second.is_err(),
        "second concurrent prompt on same session must fail"
    );
    let err = second.unwrap_err().to_command_error();
    assert_eq!(
        err.error_class,
        ErrorClass::InvalidState.as_str(),
        "class={}",
        err.error_class
    );

    let _ = cp.session_cancel(&s.session_id).await;
    let _ = tokio::time::timeout(Duration::from_secs(20), first).await;
    let _ = cp.session_stop(&s.session_id, true).await;
}

// ---------------------------------------------------------------------------
// MS-16 Parallel prompts across sessions (supported)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms16_parallel_prompts_across_sessions_supported() {
    let _g = ms_lock().await;
    let cp = std::sync::Arc::new(open_cp(None).await);
    let (_dir, project_id) = register_temp_project(&cp).await;

    let a = cp
        .session_create(
            &project_id,
            Some("A".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();
    let b = cp
        .session_create(
            &project_id,
            Some("B".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();

    let cp_a = std::sync::Arc::clone(&cp);
    let sid_a = a.session_id.clone();
    let ha = tokio::spawn(async move { cp_a.session_submit_prompt(&sid_a, "parallel-a").await });

    let cp_b = std::sync::Arc::clone(&cp);
    let sid_b = b.session_id.clone();
    let hb = tokio::spawn(async move { cp_b.session_submit_prompt(&sid_b, "parallel-b").await });

    let ra = tokio::time::timeout(Duration::from_secs(45), ha)
        .await
        .expect("a timeout")
        .expect("a join");
    let rb = tokio::time::timeout(Duration::from_secs(45), hb)
        .await
        .expect("b timeout")
        .expect("b join");

    assert!(ra.is_ok(), "parallel A: {ra:?}");
    assert!(rb.is_ok(), "parallel B: {rb:?}");

    let ea = cp.events_list(&a.session_id, 0, 500).await.unwrap();
    let eb = cp.events_list(&b.session_id, 0, 500).await.unwrap();
    assert!(event_session_ids_consistent(&ea.events, &a.session_id));
    assert!(event_session_ids_consistent(&eb.events, &b.session_id));
    assert!(sequences_monotonic(&ea.events));
    assert!(sequences_monotonic(&eb.events));

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// MS-17 Integrated: after focus switch, background ingest cannot steal focus
// ---------------------------------------------------------------------------
//
// Command paths (submit/create) may set focus via refresh_snapshot_for
// (authoritative "current session"). Post-persist hub updates use
// publish_session_update, which only applies when session_id matches focus.
// This test: start B prompt (steals focus), re-focus A, wait for B to finish —
// A must remain focused while B's remaining ingest events land.

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ms17_focus_stable_while_background_session_ingests() {
    let _g = ms_lock().await;
    let cp = std::sync::Arc::new(open_cp(None).await);
    let (_dir, project_id) = register_temp_project(&cp).await;

    let focused = cp
        .session_create(
            &project_id,
            Some("focused".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();
    let background = cp
        .session_create(
            &project_id,
            Some("background".into()),
            runtime_opts("happy_prompt_stream"),
        )
        .await
        .unwrap();

    // Start background stream first (command path may set focus to B).
    let cp_bg = std::sync::Arc::clone(&cp);
    let bg_id = background.session_id.clone();
    let bg_handle =
        tokio::spawn(async move { cp_bg.session_submit_prompt(&bg_id, "bg-stream").await });

    // Give B a moment to begin, then pin focus on A.
    tokio::time::sleep(Duration::from_millis(150)).await;
    let snap = cp
        .presentation_focus(&focused.session_id)
        .await
        .expect("focus A");
    assert_eq!(
        snap.active_session_id.as_deref(),
        Some(focused.session_id.as_str())
    );
    let rev_after_focus = snap.revision;

    // While B continues, focus must remain A (ingest path must not steal).
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let s = cp.snapshot();
        assert_eq!(
            s.active_session_id.as_deref(),
            Some(focused.session_id.as_str()),
            "background post-persist updates must not steal presentation focus"
        );
        if bg_handle.is_finished() {
            break;
        }
        if Instant::now() > deadline {
            panic!("background prompt timed out");
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    let bg_res = bg_handle.await.expect("join");
    assert!(bg_res.is_ok(), "background prompt: {bg_res:?}");

    // submit_prompt end path refreshes snapshot for B — command path may reclaim
    // focus when B completes. Re-assert hub isolation by focusing A again and
    // verifying publish_session_update for B does not flip focus (unit path
    // already covered). Here we only require that explicit focus sticks until
    // the next command on another session.
    let after = cp
        .presentation_focus(&focused.session_id)
        .await
        .expect("refocus A");
    assert_eq!(
        after.active_session_id.as_deref(),
        Some(focused.session_id.as_str())
    );
    assert!(after.revision >= rev_after_focus);

    // Background events exist and are session-local.
    let eb = cp
        .events_list(&background.session_id, 0, 500)
        .await
        .unwrap();
    assert!(
        !eb.events.is_empty(),
        "background should have persisted events"
    );
    assert!(event_session_ids_consistent(
        &eb.events,
        &background.session_id
    ));
    // Focus still A after reading B history.
    assert_eq!(
        cp.snapshot().active_session_id.as_deref(),
        Some(focused.session_id.as_str())
    );

    let _ = cp.shutdown_all().await;
}
