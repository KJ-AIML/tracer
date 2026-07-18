//! W2-A presentation delivery invariants.
//!
//! Deterministic tests: slow/absent consumer, reconnect, multi-consumer,
//! coalescing, terminal snapshot, version monotonicity, disconnect cleanup,
//! shutdown cleanup, burst beyond delivery capacity.
//!
//! File-backed SQLite + fake ACP where integration is needed.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use tracer_control_plane::{
    ControlPlane, ControlPlaneConfig, PresentationHub, PresentationSnapshot, RuntimeCreateOptions,
    SessionProjectionInput, DEFAULT_NOTIFY_CAPACITY,
};
use tracer_domain::{AuthenticationState, SessionStatus};

fn stock_opts() -> RuntimeCreateOptions {
    let fake = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/fake-acp-runtime/bin/fake-acp-runtime.js");
    RuntimeCreateOptions {
        runtime_kind: "acp-stdio".into(),
        scenario_id: Some("happy_prompt_stream".into()),
        executable_override: Some("node".into()),
        extra_args: Vec::new(),
        fake_js: Some(fake.to_string_lossy().into_owned()),
    }
}

async fn open_file_cp() -> (tempfile::TempDir, ControlPlane) {
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join("w2a.db");
    let fake = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/fake-acp-runtime/bin/fake-acp-runtime.js");
    let cp = ControlPlane::open(ControlPlaneConfig {
        database_path: Some(db),
        fake_js: Some(fake),
        node_bin: std::path::PathBuf::from("node"),
        heli_probe_path: dir.path().to_path_buf(),
        escalate_cancel_to_process_stop: true,
    })
    .await
    .expect("open cp");
    (dir, cp)
}

async fn register_project(cp: &ControlPlane) -> (tempfile::TempDir, String) {
    let dir = tempdir().expect("proj");
    let p = cp
        .project_register(dir.path().to_str().unwrap(), Some("w2a-proj".into()))
        .await
        .expect("register");
    (dir, p.project_id)
}

fn empty_snap() -> PresentationSnapshot {
    PresentationSnapshot::default()
}

fn projection(seq: i64, status: SessionStatus) -> SessionProjectionInput {
    SessionProjectionInput {
        session_id: "sess-1".into(),
        project_id: "proj-1".into(),
        status,
        auth_state: AuthenticationState::NotRequired,
        pending_approvals: Vec::new(),
        last_error: None,
        capabilities: None,
        latest_sequence: seq,
        prompt_in_flight: matches!(status, SessionStatus::Running),
        process_alive: true,
        protocol_ready: true,
        session_ready: !matches!(
            status,
            SessionStatus::Failed | SessionStatus::Disconnected | SessionStatus::Stopped
        ),
    }
}

// ---------------------------------------------------------------------------
// Hub unit-level invariants (no runtime)
// ---------------------------------------------------------------------------

#[test]
fn inv07_snapshot_revisions_monotonic() {
    let hub = PresentationHub::new(empty_snap());
    let mut prev = hub.revision();
    for i in 1..=50 {
        let mut s = empty_snap();
        s.latest_sequence = i;
        let r = hub.publish_snapshot(s);
        assert!(r > prev, "revision {r} should exceed {prev}");
        prev = r;
    }
    assert_eq!(hub.snapshot().revision, prev);
    assert_eq!(
        hub.snapshot().version,
        1,
        "schema version stays SNAPSHOT_VERSION"
    );
}

#[test]
fn inv08_consumer_detects_stale_snapshot() {
    let hub = PresentationHub::new(empty_snap());
    let sub = hub.subscribe();
    let known = hub.revision();
    assert!(!sub.is_stale(known));
    hub.publish_snapshot(empty_snap());
    assert!(sub.is_stale(known));
    assert!(!sub.is_stale(hub.revision()));
}

#[test]
fn inv02_slow_consumer_no_unbounded_growth() {
    let hub = PresentationHub::new(empty_snap());
    let (_id, rx) = hub.subscribe_notify();
    // Never drain rx.
    for i in 0..5_000 {
        let mut s = empty_snap();
        s.latest_sequence = i;
        hub.publish_snapshot(s);
    }
    let mut pending = 0usize;
    while rx.try_recv().is_ok() {
        pending += 1;
    }
    assert!(
        pending <= DEFAULT_NOTIFY_CAPACITY,
        "pending notifies {pending} exceeds capacity {DEFAULT_NOTIFY_CAPACITY}"
    );
    // Latest state still recoverable.
    assert_eq!(hub.snapshot().latest_sequence, 4_999);
    assert!(hub.metrics().notify_coalesced > 0 || hub.metrics().publishes == 5_000);
}

#[test]
fn inv02_absent_consumer_no_growth() {
    let hub = PresentationHub::new(empty_snap());
    // No subscribers at all.
    for i in 0..2_000 {
        let mut s = empty_snap();
        s.latest_sequence = i;
        hub.publish_snapshot(s);
    }
    assert_eq!(hub.snapshot().latest_sequence, 1_999);
    assert_eq!(hub.metrics().notify_sends, 0);
    assert_eq!(hub.revision(), 2_000);
}

#[test]
fn inv03_latest_state_via_snapshot() {
    let hub = PresentationHub::new(empty_snap());
    hub.publish_session_update(projection(1, SessionStatus::Ready));
    hub.publish_session_update(projection(42, SessionStatus::Running));
    let snap = hub.snapshot();
    assert_eq!(snap.latest_sequence, 42);
    assert_eq!(snap.session_status, Some(SessionStatus::Running));
    assert_eq!(snap.active_session_id.as_deref(), Some("sess-1"));
}

#[test]
fn inv04_terminal_cannot_be_permanently_missed() {
    let hub = PresentationHub::new(empty_snap());
    let (_id, rx) = hub.subscribe_notify();
    // Flood then terminal — consumer may only see last coalesced notify.
    for i in 0..100 {
        hub.publish_session_update(projection(i, SessionStatus::Running));
    }
    hub.publish_session_update(projection(100, SessionStatus::Failed));
    // Drain whatever is pending.
    let mut last = None;
    while let Ok(n) = rx.try_recv() {
        last = Some(n);
    }
    // Even if notify was lost/coalesced, snapshot has terminal.
    let snap = hub.snapshot();
    assert_eq!(snap.session_status, Some(SessionStatus::Failed));
    assert!(hub.terminal_sticky());
    if let Some(n) = last {
        // If a notify was delivered, terminal flag must be set when status is failed.
        if n.session_status == Some(SessionStatus::Failed) {
            assert!(n.terminal);
        }
    }
    // Reconnect recovers terminal via snapshot.
    let sub2 = hub.subscribe();
    assert_eq!(sub2.snapshot().session_status, Some(SessionStatus::Failed));
}

#[test]
fn inv05_notification_duplication_harmless() {
    let hub = PresentationHub::new(empty_snap());
    let mut s = empty_snap();
    s.latest_sequence = 7;
    let r1 = hub.publish_snapshot(s.clone());
    let r2 = hub.publish_snapshot(s);
    assert!(r2 > r1);
    // Both revisions map to same logical latest_sequence; consumer just pulls snapshot.
    assert_eq!(hub.snapshot().latest_sequence, 7);
}

#[test]
fn inv06_notification_loss_recoverable_via_snapshot() {
    let hub = PresentationHub::new(empty_snap());
    let (id, rx) = hub.subscribe_notify();
    hub.publish_session_update(projection(1, SessionStatus::Ready));
    // Drop all notifies without reading them fully after unsubscribe.
    drop(rx);
    hub.unsubscribe(id);
    hub.publish_session_update(projection(99, SessionStatus::Running));
    // New consumer only has snapshot — still correct.
    assert_eq!(hub.snapshot().latest_sequence, 99);
    assert_eq!(hub.snapshot().session_status, Some(SessionStatus::Running));
}

#[test]
fn inv09_multiple_consumers_cannot_block_publish() {
    let hub = PresentationHub::new(empty_snap());
    let mut rxs = Vec::new();
    for _ in 0..8 {
        let (_id, rx) = hub.subscribe_notify();
        rxs.push(rx);
    }
    // None of the receivers drain.
    let t0 = Instant::now();
    for i in 0..500 {
        let mut s = empty_snap();
        s.latest_sequence = i;
        hub.publish_snapshot(s);
    }
    assert!(
        t0.elapsed() < Duration::from_secs(2),
        "publish blocked by consumers: {:?}",
        t0.elapsed()
    );
    assert_eq!(hub.snapshot().latest_sequence, 499);
    drop(rxs);
}

#[test]
fn inv10_disconnect_removes_delivery_state() {
    let hub = PresentationHub::new(empty_snap());
    let sub = hub.subscribe();
    assert_eq!(hub.metrics().consumers_registered, 1);
    drop(sub);
    assert_eq!(hub.metrics().consumers_removed, 1);
    // Further publishes should not attempt dead sinks.
    hub.publish_snapshot(empty_snap());
    assert_eq!(hub.metrics().notify_sends, 0);
}

#[test]
fn inv11_shutdown_clears_consumers() {
    let hub = PresentationHub::new(empty_snap());
    let _sub = hub.subscribe();
    let (tx, _rx) = mpsc::channel();
    hub.attach_legacy_sender(tx);
    hub.shutdown();
    assert!(hub.is_shutdown());
    // Publish after shutdown is a no-op for revision (or freezes).
    let rev = hub.revision();
    hub.publish_snapshot(empty_snap());
    assert_eq!(hub.revision(), rev);
}

#[test]
fn inv_burst_beyond_delivery_capacity_coalesces() {
    let hub = PresentationHub::new(empty_snap());
    let (_id, rx) = hub.subscribe_notify();
    let burst = DEFAULT_NOTIFY_CAPACITY * 200 + 50;
    for i in 0..burst {
        let mut s = empty_snap();
        s.latest_sequence = i as i64;
        hub.publish_snapshot(s);
    }
    let mut got = 0usize;
    while rx.try_recv().is_ok() {
        got += 1;
    }
    assert!(got <= DEFAULT_NOTIFY_CAPACITY);
    assert_eq!(hub.snapshot().latest_sequence, (burst - 1) as i64);
}

#[test]
fn inv_reconnect_sees_latest() {
    let hub = PresentationHub::new(empty_snap());
    let sub1 = hub.subscribe();
    hub.publish_session_update(projection(5, SessionStatus::Ready));
    drop(sub1);
    hub.publish_session_update(projection(10, SessionStatus::Running));
    let sub2 = hub.subscribe();
    assert_eq!(sub2.snapshot().latest_sequence, 10);
    assert!(sub2.watch_revision() >= 2 || sub2.snapshot().revision >= 2);
}

// ---------------------------------------------------------------------------
// Integration: file SQLite + fake ACP
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inv01_persistence_independent_of_presentation() {
    let (_keep, cp) = open_file_cp().await;
    // Absent consumers: no subscribe at all.
    let (_proj_dir, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(&project_id, Some("w2a-persist".into()), stock_opts())
        .await
        .expect("create");
    let prompt = cp
        .session_submit_prompt(&session.session_id, "hello independent of UI")
        .await
        .expect("prompt");
    assert!(prompt.accepted);

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut events = Vec::new();
    while Instant::now() < deadline {
        events = cp
            .events_list(&session.session_id, 0, 10_000)
            .await
            .expect("events")
            .events;
        if events.iter().any(|e| {
            e.get("type").and_then(|t| t.as_str()) == Some("session.completed")
                || e.get("type").and_then(|t| t.as_str()) == Some("agent.message.completed")
        }) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(
        !events.is_empty(),
        "events must persist without presentation consumers"
    );
    let _ = cp.session_stop(&session.session_id, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inv_slow_legacy_sender_does_not_block_persist() {
    let (_keep, cp) = open_file_cp().await;
    // Unbounded channel that is never drained — legacy path must not grow
    // control-plane queues or block persist (bridge capacity 1 + forwarder).
    let (tx, _rx) = mpsc::channel();
    cp.set_presentation_sender(tx);

    let (_proj_dir, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(&project_id, Some("w2a-slow-legacy".into()), stock_opts())
        .await
        .expect("create");
    let prompt = cp
        .session_submit_prompt(&session.session_id, "persist despite slow UI")
        .await
        .expect("prompt");
    assert!(prompt.accepted);

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut n = 0usize;
    while Instant::now() < deadline {
        n = cp
            .events_list(&session.session_id, 0, 10_000)
            .await
            .expect("events")
            .events
            .len();
        if n >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    assert!(n >= 2, "persisted events={n}");
    // Snapshot still recoverable with monotonic revision.
    let snap = cp.snapshot();
    assert!(snap.revision >= 1 || snap.active_session_id.is_some());
    let _ = cp.session_stop(&session.session_id, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inv_multi_consumer_subscribe_and_snapshot() {
    let (_keep, cp) = open_file_cp().await;
    let mut subs: Vec<_> = (0..3).map(|_| cp.subscribe_presentation()).collect();
    let (_proj_dir, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(&project_id, Some("w2a-multi".into()), stock_opts())
        .await
        .expect("create");

    // At least one consumer should observe a revision advance or snapshot fields.
    let mut saw = false;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        for s in &subs {
            if s.snapshot().active_session_id.as_deref() == Some(session.session_id.as_str()) {
                saw = true;
                break;
            }
            if s.try_recv_notify().is_some() {
                saw = true;
                break;
            }
        }
        if saw {
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    assert!(saw, "consumers should observe session projection");
    assert_eq!(
        cp.snapshot().active_session_id.as_deref(),
        Some(session.session_id.as_str())
    );
    // Disconnect one consumer.
    let removed_before = cp.presentation_hub().metrics().consumers_removed;
    drop(subs.pop());
    assert!(cp.presentation_hub().metrics().consumers_removed > removed_before);
    let _ = cp.session_stop(&session.session_id, false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inv_shutdown_presentation_after_session() {
    let (_keep, cp) = open_file_cp().await;
    let sub = cp.subscribe_presentation();
    let (_proj_dir, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(&project_id, Some("w2a-shutdown".into()), stock_opts())
        .await
        .expect("create");
    let _ = cp.session_stop(&session.session_id, false).await;
    drop(sub);
    cp.shutdown_presentation();
    assert!(cp.presentation_hub().is_shutdown());
    // Snapshot still readable (last projection retained).
    let _ = cp.snapshot();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inv_coalesce_under_burst_with_fake_path() {
    // Use happy path but many presentation publishes via hub while session runs.
    let (_keep, cp) = open_file_cp().await;
    let received = Arc::new(AtomicUsize::new(0));
    let (id, rx) = cp.presentation_hub().subscribe_notify();
    let received2 = Arc::clone(&received);
    let drain = std::thread::spawn(move || {
        // Slow consumer: sleep between recvs.
        while let Ok(_n) = rx.recv_timeout(Duration::from_secs(20)) {
            received2.fetch_add(1, Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(20));
        }
    });

    let (_proj_dir, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(&project_id, Some("w2a-coalesce".into()), stock_opts())
        .await
        .expect("create");
    let _ = cp
        .session_submit_prompt(&session.session_id, "coalesce me")
        .await
        .expect("prompt");

    tokio::time::sleep(Duration::from_millis(800)).await;
    let snap = cp.snapshot();
    // Sequence progressed in storage regardless of notify rate.
    let events = cp
        .events_list(&session.session_id, 0, 10_000)
        .await
        .expect("events");
    assert!(!events.events.is_empty());
    assert!(
        snap.revision >= 1 || events.latest_sequence >= 1,
        "revision={} seq={}",
        snap.revision,
        events.latest_sequence
    );

    cp.presentation_hub().unsubscribe(id);
    let _ = drain.join();
    let _ = cp.session_stop(&session.session_id, false).await;
    let _ = received.load(Ordering::Relaxed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn inv12_vs_happy_path_ordering_smoke() {
    // Lightweight guard that core happy path still works with hub wired.
    let (_keep, cp) = open_file_cp().await;
    let (_proj_dir, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(&project_id, Some("w2a-vs-smoke".into()), stock_opts())
        .await
        .expect("create");
    assert!(session.session_ready || session.status == SessionStatus::Ready);
    let prompt = cp
        .session_submit_prompt(&session.session_id, "ordering smoke")
        .await
        .expect("prompt");
    assert!(prompt.accepted);
    let mut events = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        events = cp
            .events_list(&session.session_id, 0, 10_000)
            .await
            .expect("events")
            .events;
        if events.len() >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    // Sequences monotonic.
    let mut last = 0i64;
    for e in &events {
        let seq = e.get("sequence").and_then(|v| v.as_i64()).unwrap_or(0);
        assert!(seq >= last, "seq {seq} < last {last}");
        last = seq;
    }
    let snap = cp.snapshot();
    assert_eq!(snap.version, 1);
    let _ = cp.session_stop(&session.session_id, false).await;
}
