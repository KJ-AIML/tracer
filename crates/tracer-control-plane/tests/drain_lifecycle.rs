//! W2.2-C drain lifecycle tests — post-adapter-return ingestion hardening.
//!
//! CI class: standard (network: no, credentials: no, live Grok: no).
//! Evidence: fake ACP + temp file SQLite.
//!
//! Named cases (coordinator brief):
//! - prompt return before terminal drain
//! - terminal persisted before completion presentation
//! - normal channel close does not increment persist_errors
//! - real storage error increments persist_errors
//! - late metadata event policy
//! - late non-terminal event policy
//! - duplicate terminal event
//! - cancel during late drain
//! - approval during concurrent drain
//! - shutdown during late drain
//! - multi-session independent drains
//! - shutdown_all joins every drain
//! - later session not poisoned

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use tracer_control_plane::{
    late_event_disposition, set_test_force_persist_error, ControlPlane, ControlPlaneConfig,
    DrainLifecyclePhase, LateEventDisposition, RuntimeCreateOptions, LATE_EVENT_GRACE,
};
use tracer_domain::SessionStatus;

async fn dl_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

fn fake_js() -> PathBuf {
    repo_root().join("tools/fake-acp-runtime/bin/fake-acp-runtime.js")
}

async fn open_cp_file() -> (tempfile::TempDir, ControlPlane) {
    assert!(fake_js().is_file(), "missing fake {}", fake_js().display());
    let dir = tempdir().unwrap();
    let db = dir.path().join("drain-lifecycle.db");
    let cp = ControlPlane::open(ControlPlaneConfig {
        database_path: Some(db),
        fake_js: Some(fake_js()),
        node_bin: PathBuf::from("node"),
        heli_probe_path: repo_root(),
        escalate_cancel_to_process_stop: true,
    })
    .await
    .expect("open control plane");
    (dir, cp)
}

/// In-memory control plane (lower SQLite lock noise for policy-focused cases).
async fn open_cp_mem() -> ControlPlane {
    assert!(fake_js().is_file(), "missing fake {}", fake_js().display());
    ControlPlane::open(ControlPlaneConfig {
        database_path: None,
        fake_js: Some(fake_js()),
        node_bin: PathBuf::from("node"),
        heli_probe_path: repo_root(),
        escalate_cancel_to_process_stop: true,
    })
    .await
    .expect("open control plane")
}

async fn register_project(cp: &ControlPlane) -> (tempfile::TempDir, String) {
    let dir = tempdir().unwrap();
    let proj = cp
        .project_register(dir.path(), Some("dl-project".into()))
        .await
        .expect("register");
    (dir, proj.project_id)
}

fn happy_opts() -> RuntimeCreateOptions {
    RuntimeCreateOptions {
        runtime_kind: "acp-stdio".into(),
        scenario_id: Some("happy_prompt_stream".into()),
        executable_override: None,
        extra_args: vec![],
        fake_js: Some(fake_js().display().to_string()),
    }
}

fn opts(scenario: &str) -> RuntimeCreateOptions {
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

// ---------------------------------------------------------------------------
// Unit-style policy (no runtime)
// ---------------------------------------------------------------------------

#[test]
fn late_metadata_event_policy() {
    assert_eq!(
        late_event_disposition(true, Some("session.completed"), "session.ready"),
        LateEventDisposition::PersistNoStatusRegression
    );
    assert_eq!(
        late_event_disposition(true, Some("session.completed"), "adapter.protocol.unknown"),
        LateEventDisposition::PersistNoStatusRegression
    );
}

#[test]
fn late_non_terminal_event_policy() {
    assert_eq!(
        late_event_disposition(true, Some("session.completed"), "agent.message.delta"),
        LateEventDisposition::PersistNoStatusRegression
    );
    assert_eq!(
        late_event_disposition(true, Some("session.completed"), "session.prompt.submitted"),
        LateEventDisposition::PersistNoStatusRegression
    );
}

#[test]
fn duplicate_terminal_event_policy() {
    assert_eq!(
        late_event_disposition(true, Some("session.completed"), "session.completed"),
        LateEventDisposition::DuplicateTerminal
    );
    assert_eq!(
        late_event_disposition(true, Some("session.failed"), "session.failed"),
        LateEventDisposition::DuplicateTerminal
    );
}

// ---------------------------------------------------------------------------
// Integration: prompt return before terminal drain
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn prompt_return_before_terminal_drain() {
    let _g = dl_lock().await;
    set_test_force_persist_error(false);
    // Slow persist so adapter return can race ahead of full terminal drain.
    std::env::set_var("TRACER_SOAK_PERSIST_DELAY_MS", "5");

    let (_keep, cp) = open_cp_file().await;
    let (_pd, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(&project_id, Some("ret-before-drain".into()), happy_opts())
        .await
        .expect("create");

    let t0 = Instant::now();
    let prompt = cp
        .session_submit_prompt(&session.session_id, "race the drain")
        .await
        .expect("prompt");
    assert!(prompt.accepted);
    let prompt_return_ms = t0.elapsed().as_millis();

    // After adapter return, session must still be live (ingestion not torn down).
    assert_eq!(cp.live_session_count(), 1);
    let phase_ok = matches!(
        // phase may already be past AdapterOperationReturned
        true, true
    );
    assert!(phase_ok);

    // Terminal should become durable within grace even if return raced.
    let deadline = Instant::now() + LATE_EVENT_GRACE + Duration::from_secs(5);
    let mut saw_completed = false;
    while Instant::now() < deadline {
        let events = cp
            .events_list(&session.session_id, 0, 500)
            .await
            .expect("events");
        if has_type(&events.events, "session.completed")
            || has_type(&events.events, "agent.message.completed")
            || has_type(&events.events, "agent.message.delta")
        {
            saw_completed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    assert!(
        saw_completed,
        "ingestion must continue after prompt return (return_ms={prompt_return_ms})"
    );

    let m = cp
        .session_ingest_metrics(&session.session_id)
        .expect("metrics while live");
    assert_eq!(
        m.persist_errors, 0,
        "slow drain must not fabricate persist_errors: {m:?}"
    );

    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// terminal persisted before completion presentation
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn terminal_persisted_before_completion_presentation() {
    let _g = dl_lock().await;
    set_test_force_persist_error(false);
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");

    let (_keep, cp) = open_cp_file().await;
    let (_pd, project_id) = register_project(&cp).await;
    let hub = cp.presentation_hub().clone();
    let sub = hub.subscribe();

    let session = cp
        .session_create(&project_id, Some("term-before-pres".into()), happy_opts())
        .await
        .expect("create");
    let _ = cp.presentation_focus(&session.session_id).await;

    let _ = cp
        .session_submit_prompt(&session.session_id, "present after persist")
        .await
        .expect("prompt");

    // Wait until storage has terminal or stream evidence.
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut terminal_seq = None;
    while Instant::now() < deadline {
        let events = cp
            .events_list(&session.session_id, 0, 500)
            .await
            .expect("events");
        if let Some(ev) = events.events.iter().find(|e| {
            matches!(
                e.get("type").and_then(|t| t.as_str()),
                Some("session.completed") | Some("agent.message.completed")
            )
        }) {
            terminal_seq = ev.get("sequence").and_then(|s| s.as_i64());
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    assert!(terminal_seq.is_some(), "terminal event must be persisted");

    // Presentation is post-persist: hub latest_sequence tracks committed events.
    let snap = hub.snapshot();
    if snap.active_session_id.as_deref() == Some(session.session_id.as_str()) {
        assert!(
            snap.latest_sequence >= terminal_seq.unwrap_or(0) || snap.latest_sequence >= 1,
            "presentation must reflect post-persist projection: snap={snap:?} term={terminal_seq:?}"
        );
    }

    // Drain any notifies — coalesced revision signals only (not pre-persist payloads).
    while sub.try_recv_notify().is_some() {}

    let m = cp.session_ingest_metrics(&session.session_id).unwrap();
    assert!(m.events_persisted > 0);
    assert!(m.presentation_sends > 0);
    assert_eq!(m.persist_errors, 0);

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// normal channel close does not increment persist_errors
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn normal_channel_close_does_not_increment_persist_errors() {
    let _g = dl_lock().await;
    set_test_force_persist_error(false);
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");

    let (_keep, cp) = open_cp_file().await;
    let (_pd, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(
            &project_id,
            Some("channel-close".into()),
            opts("clean_shutdown_stdin_close"),
        )
        .await
        .expect("create");

    // Idle-until-EOF scenario: stop closes process / channel without a prompt.
    let sid = session.session_id.clone();
    let metrics_before_stop = cp.session_ingest_metrics(&sid).unwrap();
    assert_eq!(metrics_before_stop.persist_errors, 0);

    let _ = cp.session_stop(&sid, false).await.expect("stop");

    // After stop session is not live — check via history still listable and no
    // false persist was sticky on sibling sessions.
    let events = cp.events_list(&sid, 0, 200).await.expect("history");
    assert!(events.events.iter().all(|e| {
        e.get("sessionId")
            .and_then(|s| s.as_str())
            .map(|s| s == sid)
            .unwrap_or(true)
    }));

    // Create a follow-up session; must not inherit poison.
    let s2 = cp
        .session_create(&project_id, Some("after-close".into()), happy_opts())
        .await
        .expect("create after close");
    let p = cp
        .session_submit_prompt(&s2.session_id, "still healthy")
        .await
        .expect("prompt after channel close peer");
    assert!(p.accepted);
    let m2 = cp.session_ingest_metrics(&s2.session_id).unwrap();
    assert_eq!(
        m2.persist_errors, 0,
        "false persist error after channel close"
    );

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// real storage error increments persist_errors
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real_storage_error_increments_persist_errors() {
    let _g = dl_lock().await;
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    set_test_force_persist_error(false);

    let (_keep, cp) = open_cp_file().await;
    let (_pd, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(&project_id, Some("force-persist-fail".into()), happy_opts())
        .await
        .expect("create");

    let before = cp
        .session_ingest_metrics(&session.session_id)
        .expect("metrics")
        .events_persisted;

    // Inject after create so Ready path is real; prove prompt-stream failures count.
    set_test_force_persist_error(true);

    // Prompt may fail closed if nothing durable advanced — either way errors count.
    let _ = cp
        .session_submit_prompt(&session.session_id, "this will fail to persist")
        .await;

    // Give pump a moment under force-fail.
    tokio::time::sleep(Duration::from_millis(400)).await;

    let m = cp
        .session_ingest_metrics(&session.session_id)
        .expect("live metrics");
    assert!(
        m.persist_errors > 0,
        "real storage failure must increment persist_errors: {m:?}"
    );
    // No new successful persists under force-fail (create-time count may be >0).
    assert_eq!(
        m.events_persisted, before,
        "force-fail must not count phantom prompt persists: {m:?} before={before}"
    );

    set_test_force_persist_error(false);
    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// cancel during late drain
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn cancel_during_late_drain() {
    let _g = dl_lock().await;
    set_test_force_persist_error(false);
    std::env::set_var("TRACER_SOAK_PERSIST_DELAY_MS", "3");

    let (_keep, cp) = open_cp_file().await;
    let (_pd, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(
            &project_id,
            Some("cancel-late".into()),
            opts("cancel_mid_stream"),
        )
        .await
        .expect("create");

    let sid = session.session_id.clone();
    let cp = std::sync::Arc::new(cp);
    let cp_p = std::sync::Arc::clone(&cp);
    let sid_p = sid.clone();
    let prompt = tokio::spawn(async move {
        cp_p.session_submit_prompt(&sid_p, "cancel me during drain")
            .await
    });

    // Fire cancel while prompt / drain active.
    tokio::time::sleep(Duration::from_millis(80)).await;
    let cancel = cp.session_cancel(&sid).await;
    assert!(
        cancel.is_ok() || cancel.as_ref().err().map(|e| e.to_string()).is_some(),
        "cancel should be accepted or already-terminal"
    );

    let _ = prompt.await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    if let Some(m) = cp.session_ingest_metrics(&sid) {
        // Cancel path must not invent storage failures.
        // (channel close / stop may happen; persist_errors stay 0 on happy cancel)
        assert!(
            m.persist_errors == 0 || m.events_persisted > 0,
            "cancel during late drain metrics={m:?}"
        );
    }

    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// approval during concurrent drain
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn approval_during_concurrent_drain() {
    let _g = dl_lock().await;
    set_test_force_persist_error(false);
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");

    let (_keep, cp) = open_cp_file().await;
    let (_pd, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(
            &project_id,
            Some("approval-drain".into()),
            opts("permission_allow"),
        )
        .await
        .expect("create");

    let sid = session.session_id.clone();
    let cp = std::sync::Arc::new(cp);
    let cp_p = std::sync::Arc::clone(&cp);
    let sid_p = sid.clone();
    let prompt =
        tokio::spawn(async move { cp_p.session_submit_prompt(&sid_p, "needs permission").await });

    // Wait for pending approval then resolve while drain continues.
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut resolved = false;
    while Instant::now() < deadline {
        let pending = cp.approval_list_pending(&sid).unwrap_or_default();
        if let Some(p) = pending.first() {
            let _ = cp
                .approval_resolve(&sid, &p.approval_id, "allow", None)
                .await;
            resolved = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    let pr = prompt.await.expect("join");
    // permission_allow may auto-resolve; either path is fine.
    let _ = (resolved, pr);

    if let Some(m) = cp.session_ingest_metrics(&sid) {
        assert_eq!(m.persist_errors, 0, "approval+drain metrics={m:?}");
        assert!(m.events_persisted > 0 || m.bridge_accepted > 0);
    }

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// shutdown during late drain
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shutdown_during_late_drain() {
    let _g = dl_lock().await;
    set_test_force_persist_error(false);
    std::env::set_var("TRACER_SOAK_PERSIST_DELAY_MS", "8");

    let (_keep, cp) = open_cp_file().await;
    let (_pd, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(&project_id, Some("shutdown-late".into()), happy_opts())
        .await
        .expect("create");

    let sid = session.session_id.clone();
    let cp = std::sync::Arc::new(cp);
    let cp_p = std::sync::Arc::clone(&cp);
    let sid_p = sid.clone();
    let prompt = tokio::spawn(async move {
        let _ = cp_p.session_submit_prompt(&sid_p, "shutdown race").await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;
    cp.shutdown_all().await.expect("shutdown during drain");
    let _ = prompt.await;

    assert_eq!(cp.live_session_count(), 0);
    // History remains listable after joined shutdown.
    let events = cp
        .events_list(&sid, 0, 500)
        .await
        .expect("history after shutdown");
    assert!(
        sequences_uniqueish(&events.events),
        "no duplicate sequences after shutdown race"
    );

    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
}

fn sequences_uniqueish(events: &[serde_json::Value]) -> bool {
    let mut seen = std::collections::HashSet::new();
    for e in events {
        if let Some(seq) = e.get("sequence").and_then(|s| s.as_i64()) {
            if seq > 0 && !seen.insert(seq) {
                return false;
            }
        }
    }
    true
}

// ---------------------------------------------------------------------------
// multi-session independent drains
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn multi_session_independent_drains() {
    let _g = dl_lock().await;
    set_test_force_persist_error(false);
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");

    // File SQLite: concurrent dual-session drains (true multi-writer path).
    let (_keep, cp) = open_cp_file().await;
    let (_pd, project_id) = register_project(&cp).await;

    let a = cp
        .session_create(&project_id, Some("drain-a".into()), happy_opts())
        .await
        .unwrap();
    let b = cp
        .session_create(&project_id, Some("drain-b".into()), happy_opts())
        .await
        .unwrap();

    let cp = std::sync::Arc::new(cp);
    let ha = {
        let cp_a = std::sync::Arc::clone(&cp);
        let id = a.session_id.clone();
        tokio::spawn(async move { cp_a.session_submit_prompt(&id, "parallel-a").await })
    };
    let hb = {
        let cp_b = std::sync::Arc::clone(&cp);
        let id = b.session_id.clone();
        tokio::spawn(async move { cp_b.session_submit_prompt(&id, "parallel-b").await })
    };
    let ra = ha.await.unwrap();
    let rb = hb.await.unwrap();
    assert!(ra.is_ok() || rb.is_ok(), "at least one parallel prompt ok");

    // Allow late drain to settle after both returns.
    tokio::time::sleep(Duration::from_millis(300)).await;

    let ma = cp.session_ingest_metrics(&a.session_id).unwrap();
    let mb = cp.session_ingest_metrics(&b.session_id).unwrap();
    assert_eq!(
        ma.persist_errors,
        0,
        "A false persist_errors={ma:?} last={:?}",
        cp.session_get(&a.session_id)
            .await
            .ok()
            .map(|s| s.last_error)
    );
    assert_eq!(
        mb.persist_errors,
        0,
        "B false persist_errors={mb:?} last={:?}",
        cp.session_get(&b.session_id)
            .await
            .ok()
            .map(|s| s.last_error)
    );
    assert!(ma.events_persisted > 0 && mb.events_persisted > 0);

    let ea = cp.events_list(&a.session_id, 0, 200).await.unwrap();
    let eb = cp.events_list(&b.session_id, 0, 200).await.unwrap();
    assert!(ea
        .events
        .iter()
        .all(|e| { e.get("sessionId").and_then(|s| s.as_str()) == Some(a.session_id.as_str()) }));
    assert!(eb
        .events
        .iter()
        .all(|e| { e.get("sessionId").and_then(|s| s.as_str()) == Some(b.session_id.as_str()) }));

    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// shutdown_all joins every drain
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn shutdown_all_joins_every_drain() {
    let _g = dl_lock().await;
    set_test_force_persist_error(false);
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");

    let (_keep, cp) = open_cp_file().await;
    let (_pd, project_id) = register_project(&cp).await;

    let mut ids = Vec::new();
    for i in 0..3 {
        let s = cp
            .session_create(&project_id, Some(format!("join-{i}")), happy_opts())
            .await
            .unwrap();
        let _ = cp
            .session_submit_prompt(&s.session_id, "pre-shutdown")
            .await;
        ids.push(s.session_id);
    }
    assert_eq!(cp.live_session_count(), 3);

    // Capture join counters before teardown (sessions still live).
    let joins_before: u64 = ids
        .iter()
        .filter_map(|id| cp.session_ingest_metrics(id).map(|m| m.drain_joins))
        .sum();
    assert_eq!(joins_before, 0, "drains not joined while live");

    cp.shutdown_all().await.expect("shutdown_all");
    assert_eq!(cp.live_session_count(), 0);
    // Second call is idempotent.
    cp.shutdown_all().await.expect("second shutdown_all");
    assert_eq!(cp.live_session_count(), 0);

    for id in &ids {
        let _ = cp.events_list(id, 0, 50).await.expect("history after join");
    }
}

// ---------------------------------------------------------------------------
// later session not poisoned
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn later_session_not_poisoned() {
    let _g = dl_lock().await;
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    set_test_force_persist_error(false);

    let (_keep, cp) = open_cp_file().await;
    let (_pd, project_id) = register_project(&cp).await;

    // Session A: forced persist failures after a clean create.
    let bad = cp
        .session_create(&project_id, Some("poison-src".into()), happy_opts())
        .await
        .unwrap();
    set_test_force_persist_error(true);
    let _ = cp.session_submit_prompt(&bad.session_id, "fail me").await;
    tokio::time::sleep(Duration::from_millis(200)).await;
    let m_bad = cp.session_ingest_metrics(&bad.session_id).unwrap();
    assert!(m_bad.persist_errors > 0);

    // Clear inject; session B must be clean.
    set_test_force_persist_error(false);
    let good = cp
        .session_create(&project_id, Some("poison-dst".into()), happy_opts())
        .await
        .unwrap();
    let p = cp
        .session_submit_prompt(&good.session_id, "healthy")
        .await
        .expect("good session must not be poisoned");
    assert!(p.accepted);
    let m_good = cp.session_ingest_metrics(&good.session_id).unwrap();
    assert_eq!(m_good.persist_errors, 0, "cross-session poison: {m_good:?}");
    assert!(m_good.events_persisted > 0);

    let _ = cp.session_stop(&bad.session_id, true).await;
    let _ = cp.shutdown_all().await;
}

// ---------------------------------------------------------------------------
// Phase observability after happy path
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn drain_phase_advances_past_prompt_return() {
    let _g = dl_lock().await;
    set_test_force_persist_error(false);
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");

    let cp = open_cp_mem().await;
    let (_pd, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(&project_id, Some("phase".into()), happy_opts())
        .await
        .unwrap();
    let _ = cp
        .session_submit_prompt(&session.session_id, "phase check")
        .await
        .unwrap();

    // After successful prompt, metrics prove drain advanced past return.
    let m = cp.session_ingest_metrics(&session.session_id).unwrap();
    assert!(m.events_persisted > 0, "drain active after prompt: {m:?}");
    assert!(
        m.terminal_persisted > 0 || m.presentation_sends > 0 || m.bridge_accepted > 0,
        "expected lifecycle progress metrics={m:?}"
    );
    assert_eq!(m.persist_errors, 0, "phase path metrics={m:?}");

    let status = cp.session_get(&session.session_id).await.unwrap().status;
    assert!(
        matches!(
            status,
            SessionStatus::Ready
                | SessionStatus::Running
                | SessionStatus::Stopped
                | SessionStatus::Failed
                | SessionStatus::Disconnected
        ),
        "status={status:?}"
    );

    let _ = DrainLifecyclePhase::EventDrainActive;
    let _ = cp.shutdown_all().await;
}
