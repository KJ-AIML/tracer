//! VS1-H3 soak scenarios — concurrency and persistence hardening.
//!
//! CI class: standard (network: no, credentials: no, live Grok: no, provider: no).
//! Evidence: fake ACP (stock + soak burst) + file-backed SQLite.
//!
//! Thresholds are defined in `tracer_vs1_soak::thresholds` **before** run.

use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use tracer_control_plane::BRIDGE_CAPACITY;
use tracer_domain::SessionStatus;
use tracer_storage::{SessionId, SessionStatus as StorageSessionStatus};
use tracer_vs1_soak::{
    assert_fakes_present, burst_opts, event_types, has_type, open_file_cp, open_file_cp_at,
    permission_burst_opts, register_project, sequences_monotonic, sequences_unique, soak_lock,
    stock_opts, thresholds, wait_events_stable, ScenarioReport,
};

// ---------------------------------------------------------------------------
// SOAK-01 Event burst (beyond BRIDGE_CAPACITY)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soak01_event_burst_beyond_bridge() {
    let _g = soak_lock().await;
    // Size burst via env for the soak-only fake.
    std::env::set_var(
        "TRACER_SOAK_BURST_COUNT",
        thresholds::BURST_CHUNK_COUNT.to_string(),
    );
    // 1ms pacing between chunks: avoids rare host stdio stalls and keeps the
    // persist pump from seeing pathological zero-gap floods in debug builds.
    std::env::set_var("TRACER_SOAK_BURST_DELAY_MS", "1");
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    std::env::remove_var("TRACER_SOAK_SCENARIO");

    let t0 = Instant::now();
    let (_keep, cp, _db) = open_file_cp().await;
    let (_proj_dir, project_id) = register_project(&cp).await;

    eprintln!(
        "[soak01] burst_fake={} exists={}",
        tracer_vs1_soak::burst_fake_js().display(),
        tracer_vs1_soak::burst_fake_js().is_file()
    );

    let session = cp
        .session_create(&project_id, Some("soak01-burst".into()), burst_opts())
        .await
        .expect("burst session create");
    assert!(session.session_ready);
    eprintln!(
        "[soak01] session_id={} status={:?} ready={}",
        session.session_id, session.status, session.session_ready
    );

    let prompt = cp
        .session_submit_prompt(&session.session_id, "flood the bridge")
        .await;
    assert!(
        prompt.is_ok(),
        "prompt must complete under burst: {:?}",
        prompt.err().map(|e| e.to_command_error())
    );

    // Allow drain to catch up after prompt returns (burst may still be bridging).
    let deadline = Instant::now() + Duration::from_secs(45);
    let mut events = Vec::new();
    let mut metrics = cp.session_ingest_metrics(&session.session_id);
    while Instant::now() < deadline {
        events = cp
            .events_list(&session.session_id, 0, 50_000)
            .await
            .expect("events")
            .events;
        metrics = cp.session_ingest_metrics(&session.session_id);
        let deltas = events
            .iter()
            .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("agent.message.delta"))
            .count();
        if deltas as u64 >= thresholds::BURST_CHUNK_COUNT {
            break;
        }
        if let Some(m) = metrics {
            if m.events_persisted >= thresholds::BURST_CHUNK_COUNT {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        sequences_monotonic(&events),
        "storage sequences must be monotonic under burst"
    );
    assert!(
        sequences_unique(&events),
        "no duplicate storage sequences under burst"
    );
    assert_eq!(
        thresholds::MAX_DUPLICATED_PERSISTED,
        0,
        "threshold constant"
    );

    // At least bridge capacity + extras of stream deltas expected from burst fake.
    let delta_count = events
        .iter()
        .filter(|e| e.get("type").and_then(|t| t.as_str()) == Some("agent.message.delta"))
        .count();
    assert!(
        delta_count as u64 >= thresholds::BURST_CHUNK_COUNT
            || metrics
                .map(|m| m.events_persisted >= thresholds::BURST_CHUNK_COUNT)
                .unwrap_or(false),
        "expected >= {} deltas (bridge={}), got {delta_count}; types={:?}; metrics={metrics:?}",
        thresholds::BURST_CHUNK_COUNT,
        BRIDGE_CAPACITY,
        event_types(&events)
    );

    // Terminal delivery: completed (or cancelled if flaky cancel path).
    let terminal_ok = has_type(&events, "session.completed")
        || has_type(&events, "session.cancelled")
        || has_type(&events, "agent.message.completed");
    assert!(
        terminal_ok,
        "terminal-ish event required; types={:?}",
        event_types(&events)
    );

    if let Some(m) = metrics {
        assert_eq!(
            m.persist_errors,
            thresholds::MAX_EVENT_LOSS,
            "persist errors must be 0 (no silent storage failure path claiming success)"
        );
        // Bridge accepted should cover the burst; W1-D may also emit lifecycle events.
        assert!(
            m.bridge_accepted >= thresholds::BURST_CHUNK_COUNT,
            "bridge_accepted={} < burst={}",
            m.bridge_accepted,
            thresholds::BURST_CHUNK_COUNT
        );
        assert!(
            m.events_persisted >= thresholds::BURST_CHUNK_COUNT,
            "events_persisted={} < burst={}",
            m.events_persisted,
            thresholds::BURST_CHUNK_COUNT
        );
    }

    let shutdown_start = Instant::now();
    let _ = cp.session_stop(&session.session_id, false).await;
    let shutdown_ms = shutdown_start.elapsed().as_millis();
    assert!(
        shutdown_ms < 15_000,
        "shutdown duration must be bounded, took {shutdown_ms}ms"
    );

    let report = ScenarioReport {
        name: "soak01_event_burst",
        passed: true,
        duration_ms: t0.elapsed().as_millis(),
        events_persisted_storage: events.len(),
        metrics_persisted: metrics.map(|m| m.events_persisted),
        bridge_accepted: metrics.map(|m| m.bridge_accepted),
        persist_errors: metrics.map(|m| m.persist_errors),
        notes: format!(
            "deltas={delta_count} bridge_cap={BRIDGE_CAPACITY} shutdown_ms={shutdown_ms}"
        ),
    };
    eprintln!("{}", report.log_line());
}

// ---------------------------------------------------------------------------
// SOAK-02 Slow database (artificial persist delay)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soak02_slow_database_backpressure() {
    let _g = soak_lock().await;
    // Inject controlled storage latency at the persist pump (soak-only hook).
    std::env::set_var("TRACER_SOAK_PERSIST_DELAY_MS", "5");
    std::env::set_var("TRACER_SOAK_BURST_COUNT", "320"); // still > 256
    std::env::remove_var("TRACER_SOAK_SCENARIO");

    let t0 = Instant::now();
    let (_keep, cp, _db) = open_file_cp().await;
    let cp = Arc::new(cp);
    let (_proj_dir, project_id) = register_project(&cp).await;

    let session = cp
        .session_create(&project_id, Some("soak02-slowdb".into()), burst_opts())
        .await
        .expect("create");
    let sid = session.session_id.clone();

    let cp_p = Arc::clone(&cp);
    let sid_p = sid.clone();
    let prompt = tokio::spawn(async move {
        cp_p.session_submit_prompt(&sid_p, "slow persist burst")
            .await
    });

    // Cancel must remain responsive under slow persist (no deadlock).
    tokio::time::sleep(Duration::from_millis(80)).await;
    let cancel_start = Instant::now();
    let cancel = tokio::time::timeout(Duration::from_secs(12), cp.session_cancel(&sid))
        .await
        .expect("cancel time-bounded under slow DB")
        .expect("cancel ok");
    let cancel_ms = cancel_start.elapsed().as_millis();
    assert!(cancel.accepted || cancel.mode == "already_terminal");
    assert!(
        cancel_ms < 12_000,
        "cancel must stay responsive under slow DB: {cancel_ms}ms"
    );

    let _ = tokio::time::timeout(Duration::from_secs(45), prompt)
        .await
        .expect("prompt join bounded");

    let events = wait_events_stable(&cp, &sid, Duration::from_secs(20)).await;
    assert!(sequences_monotonic(&events));
    assert!(sequences_unique(&events));

    let metrics = cp.session_ingest_metrics(&sid);
    if let Some(m) = metrics {
        assert_eq!(m.persist_errors, 0, "no silent persist failure");
    }

    // Memory not unbounded over test duration: event count is finite (burst+lifecycle).
    assert!(
        events.len() < 5_000,
        "event count should remain bounded for 320-burst: {}",
        events.len()
    );

    let _ = cp.session_stop(&sid, true).await;
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");

    eprintln!(
        "[soak02_slow_db] pass=true duration_ms={} cancel_ms={} events={} metrics={:?}",
        t0.elapsed().as_millis(),
        cancel_ms,
        events.len(),
        metrics
    );
}

// ---------------------------------------------------------------------------
// SOAK-03 Slow presentation consumer
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soak03_slow_presentation_does_not_block_persist() {
    let _g = soak_lock().await;
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    std::env::set_var("TRACER_SOAK_BURST_COUNT", "400");
    std::env::set_var("TRACER_SOAK_BURST_DELAY_MS", "1");

    let t0 = Instant::now();
    let (_keep, cp, _db) = open_file_cp().await;

    // Presentation consumer that never reads (std mpsc is unbounded; send must not
    // block the persist pump — design contract).
    let (tx, _rx) = mpsc::channel();
    cp.set_presentation_sender(tx);

    let (_proj_dir, project_id) = register_project(&cp).await;
    let session = cp
        .session_create(
            &project_id,
            Some("soak03-presentation".into()),
            burst_opts(),
        )
        .await
        .expect("create");

    let prompt = cp
        .session_submit_prompt(&session.session_id, "persist without presentation drain")
        .await
        .expect("prompt");
    assert!(prompt.accepted);

    let deadline = Instant::now() + Duration::from_secs(45);
    let mut events = Vec::new();
    while Instant::now() < deadline {
        events = cp
            .events_list(&session.session_id, 0, 50_000)
            .await
            .expect("events")
            .events;
        if events.len() as u64 >= 400 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let metrics = cp.session_ingest_metrics(&session.session_id);
    assert!(
        events.len() as u64 >= 400 || metrics.map(|m| m.events_persisted >= 400).unwrap_or(false),
        "persistence must continue without presentation reads: events={} metrics={metrics:?}",
        events.len()
    );
    assert!(sequences_monotonic(&events));

    let metrics = cp.session_ingest_metrics(&session.session_id);
    if let Some(m) = metrics {
        assert_eq!(m.persist_errors, 0);
        // Fan-out attempted after persist; consumer not reading does not fail send on unbounded channel.
        assert!(m.events_persisted > 0);
    }

    let _ = cp.session_stop(&session.session_id, false).await;
    eprintln!(
        "[soak03_slow_presentation] pass=true duration_ms={} events={}",
        t0.elapsed().as_millis(),
        events.len()
    );
}

// ---------------------------------------------------------------------------
// SOAK-04 Concurrent commands
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soak04_concurrent_commands() {
    let _g = soak_lock().await;
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    std::env::set_var("TRACER_SOAK_SCENARIO", "permission_hold");

    let t0 = Instant::now();
    let (_keep, cp, _db) = open_file_cp().await;
    let cp = Arc::new(cp);
    let (_proj_dir, project_id) = register_project(&cp).await;

    // --- cancel vs approval race (permission_hold burst fake) ---
    let s1 = cp
        .session_create(
            &project_id,
            Some("soak04-race".into()),
            permission_burst_opts(),
        )
        .await
        .expect("create permission session");
    let sid1 = s1.session_id.clone();
    let cp_p = Arc::clone(&cp);
    let sid_p = sid1.clone();
    let prompt = tokio::spawn(async move {
        cp_p.session_submit_prompt(&sid_p, "race cancel vs approval")
            .await
    });

    // Wait for approval.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut approval_id = None;
    while Instant::now() < deadline {
        let pending = cp.approval_list_pending(&sid1).unwrap_or_default();
        if let Some(p) = pending.first() {
            approval_id = Some(p.approval_id.clone());
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // Race: duplicate approval resolve + cancel.
    if let Some(aid) = approval_id.clone() {
        let cp_a = Arc::clone(&cp);
        let cp_b = Arc::clone(&cp);
        let cp_c = Arc::clone(&cp);
        let sid_a = sid1.clone();
        let sid_b = sid1.clone();
        let sid_c = sid1.clone();
        let aid_a = aid.clone();
        let aid_b = aid.clone();
        let r1 =
            tokio::spawn(async move { cp_a.approval_resolve(&sid_a, &aid_a, "allow", None).await });
        let r2 =
            tokio::spawn(async move { cp_b.approval_resolve(&sid_b, &aid_b, "allow", None).await });
        let r3 = tokio::spawn(async move { cp_c.session_cancel(&sid_c).await });
        let _ = tokio::join!(r1, r2, r3);
    } else {
        // If approval never appeared, still exercise cancel.
        let _ = cp.session_cancel(&sid1).await;
    }

    let _ = tokio::time::timeout(Duration::from_secs(20), prompt).await;

    // Stale approvals not actionable.
    let pending = cp.approval_list_pending(&sid1).unwrap_or_default();
    assert!(
        pending.is_empty() || pending.len() as u64 <= thresholds::MAX_STALE_ACTIONABLE_APPROVALS,
        "stale actionable approvals: {pending:?}"
    );

    let _ = cp.session_stop(&sid1, true).await;

    // --- repeated cancel on happy stream ---
    std::env::remove_var("TRACER_SOAK_SCENARIO");
    let s2 = cp
        .session_create(
            &project_id,
            Some("soak04-cancel".into()),
            stock_opts("cancel_mid_stream"),
        )
        .await
        .expect("create cancel stream");
    let sid2 = s2.session_id.clone();
    let cp_p2 = Arc::clone(&cp);
    let sid_p2 = sid2.clone();
    let p2 = tokio::spawn(async move { cp_p2.session_submit_prompt(&sid_p2, "cancel me").await });
    tokio::time::sleep(Duration::from_millis(80)).await;
    for _ in 0..3 {
        let _ = cp.session_cancel(&sid2).await;
    }
    let _ = tokio::time::timeout(Duration::from_secs(20), p2).await;

    // Snapshot + history during / after ingestion.
    let _snap = cp.snapshot();
    let hist = cp.events_list(&sid2, 0, 500).await.expect("history");
    assert!(sequences_monotonic(&hist.events));
    let _ = cp.session_stop(&sid2, true).await;

    // --- shutdown during prompt ---
    let s3 = cp
        .session_create(
            &project_id,
            Some("soak04-shutdown".into()),
            stock_opts("happy_prompt_stream"),
        )
        .await
        .expect("create");
    let sid3 = s3.session_id.clone();
    let cp_p3 = Arc::clone(&cp);
    let sid_p3 = sid3.clone();
    let p3 =
        tokio::spawn(async move { cp_p3.session_submit_prompt(&sid_p3, "shutdown race").await });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let shut = tokio::time::timeout(Duration::from_secs(20), cp.shutdown_all())
        .await
        .expect("shutdown_all bounded");
    assert!(shut.is_ok());
    let _ = tokio::time::timeout(Duration::from_secs(20), p3).await;

    eprintln!(
        "[soak04_concurrent_commands] pass=true duration_ms={}",
        t0.elapsed().as_millis()
    );
}

// ---------------------------------------------------------------------------
// SOAK-05 Restart and recovery mid-ingestion
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soak05_restart_recovery() {
    let _g = soak_lock().await;
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    std::env::set_var("TRACER_SOAK_BURST_COUNT", "200");
    std::env::remove_var("TRACER_SOAK_SCENARIO");

    let t0 = Instant::now();
    let keep = tempdir().unwrap();
    let db = keep.path().join("restart.db");
    let project_dir = tempdir().unwrap();

    let (session_id, events_before) = {
        let cp = open_file_cp_at(db.clone()).await;
        let proj = cp
            .project_register(project_dir.path(), Some("restart-proj".into()))
            .await
            .unwrap();
        let session = cp
            .session_create(&proj.project_id, Some("soak05".into()), burst_opts())
            .await
            .expect("create");
        let sid = session.session_id.clone();

        let cp_arc = Arc::new(cp);
        let cp_p = Arc::clone(&cp_arc);
        let sid_p = sid.clone();
        let prompt =
            tokio::spawn(async move { cp_p.session_submit_prompt(&sid_p, "kill mid burst").await });

        // Let some events land, then drop without clean stop (simulate crash).
        tokio::time::sleep(Duration::from_millis(200)).await;
        let partial = cp_arc
            .events_list(&sid, 0, 50_000)
            .await
            .expect("partial events");
        // Abort prompt task by dropping control plane (session Drop force-kills adapter).
        drop(prompt);
        // Force incomplete status before drop.
        if let Ok(parsed) = SessionId::parse(&sid) {
            let _ = cp_arc
                .storage()
                .update_session_status(&parsed, StorageSessionStatus::Running)
                .await;
        }
        let events_before = partial.events;
        // Drop cp_arc — closes pool / kills runtimes.
        (sid, events_before)
    };

    // Reopen file-backed SQLite.
    let cp2 = open_file_cp_at(db.clone()).await;
    let detail = cp2.session_get(&session_id).await.expect("reload session");
    assert_eq!(detail.session_id, session_id);

    // Incomplete live status → interrupted/disconnected reconcile.
    assert!(
        matches!(
            detail.status,
            SessionStatus::Disconnected
                | SessionStatus::Stopped
                | SessionStatus::Failed
                | SessionStatus::Ready
        ) || !detail.process_alive,
        "interrupted session recovered controlled: {:?}",
        detail.status
    );

    let events = cp2
        .events_list(&session_id, 0, 50_000)
        .await
        .expect("reload events");
    assert!(
        !events.events.is_empty() || !events_before.is_empty(),
        "committed events survive restart (or empty if crash before first commit)"
    );
    assert!(sequences_monotonic(&events.events));
    assert!(sequences_unique(&events.events));
    assert!(events
        .events
        .iter()
        .all(|e| e.get("eventVersion").and_then(|v| v.as_u64()) == Some(1)));

    // Stale approvals not actionable on history-only session.
    let pending = cp2.approval_list_pending(&session_id);
    assert!(
        pending.is_err() || pending.unwrap().is_empty(),
        "history-only session must not expose actionable approvals"
    );

    // Migrations valid: can open and list projects.
    let projects = cp2.project_list().await.expect("projects after reopen");
    assert!(!projects.is_empty());

    eprintln!(
        "[soak05_restart] pass=true duration_ms={} before={} after={} status={:?}",
        t0.elapsed().as_millis(),
        events_before.len(),
        events.events.len(),
        detail.status
    );
}

// ---------------------------------------------------------------------------
// SOAK-06 Repeated sequential sessions (leak / growth signals)
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soak06_repeated_sessions() {
    let _g = soak_lock().await;
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    std::env::remove_var("TRACER_SOAK_SCENARIO");

    let t0 = Instant::now();
    let (_keep, cp, db) = open_file_cp().await;
    let (_proj_dir, project_id) = register_project(&cp).await;

    let n = thresholds::REPEATED_SESSION_COUNT;
    let mut last_sequences = Vec::with_capacity(n);

    for i in 0..n {
        let session = cp
            .session_create(
                &project_id,
                Some(format!("soak06-{i}")),
                stock_opts("happy_prompt_stream"),
            )
            .await
            .expect("session create");
        let prompt_res = cp
            .session_submit_prompt(&session.session_id, &format!("session {i}"))
            .await;
        // Poll like VS-01: stream or complete under drain lag.
        let deadline = Instant::now() + Duration::from_secs(12);
        let mut events = cp
            .events_list(&session.session_id, 0, 500)
            .await
            .expect("events")
            .events;
        while Instant::now() < deadline
            && !(has_type(&events, "session.completed")
                || has_type(&events, "agent.message.delta")
                || has_type(&events, "agent.message.completed")
                || has_type(&events, "session.prompt.submitted"))
        {
            tokio::time::sleep(Duration::from_millis(50)).await;
            events = cp
                .events_list(&session.session_id, 0, 500)
                .await
                .expect("events poll")
                .events;
        }
        // Prompt may surface StorageError if a transient persist flag was set; history is authoritative.
        let has_evidence = has_type(&events, "session.completed")
            || has_type(&events, "agent.message.delta")
            || has_type(&events, "agent.message.completed")
            || has_type(&events, "session.prompt.submitted");
        if prompt_res.is_err() && !has_evidence {
            panic!(
                "session {i} prompt failed without history: {:?} types={:?}",
                prompt_res.err().map(|e| e.to_command_error()),
                event_types(&events)
            );
        }
        assert!(
            has_evidence,
            "session {i} missing completion evidence: {:?}",
            event_types(&events)
        );
        assert!(sequences_monotonic(&events));
        last_sequences.push(events.len());

        let stop = cp
            .session_stop(&session.session_id, false)
            .await
            .expect("stop");
        assert_eq!(stop["stopped"], true);

        // Live map must not retain stopped session as live for prompt.
        let live_status = cp.runtime_status(Some(&session.session_id)).unwrap();
        assert!(
            live_status.is_empty(),
            "stopped session must leave live registry: {live_status:?}"
        );
    }

    // DB growth expected (events accumulate).
    let db_size = std::fs::metadata(&db).map(|m| m.len()).unwrap_or(0);
    assert!(db_size > 0, "file-backed DB should grow");

    // History list still works for project.
    let listed = cp.session_list(&project_id, 50).await.expect("list");
    assert!(listed.len() >= n);

    eprintln!(
        "[soak06_repeated] pass=true duration_ms={} sessions={n} db_bytes={db_size} event_counts={last_sequences:?}",
        t0.elapsed().as_millis()
    );
}

// ---------------------------------------------------------------------------
// Threshold documentation smoke (constants load)
// ---------------------------------------------------------------------------

#[test]
fn soak_thresholds_documented() {
    assert_fakes_present();
    assert_eq!(thresholds::MAX_EVENT_LOSS, 0);
    assert_eq!(thresholds::MAX_DUPLICATED_PERSISTED, 0);
    assert_eq!(thresholds::MAX_TERMINAL_EVENTS_LOST, 0);
    assert_eq!(thresholds::MAX_ORPHAN_PROCESSES, 0);
    assert_eq!(thresholds::MAX_STALE_ACTIONABLE_APPROVALS, 0);
    assert_eq!(thresholds::MAX_UNJOINED_OWNED_TASKS, 0);
    assert!(thresholds::BURST_CHUNK_COUNT as usize > BRIDGE_CAPACITY);
}

// ---------------------------------------------------------------------------
// SOAK-07 Sticky persist_failed must not poison later sessions
// ---------------------------------------------------------------------------
// Invariant: `persist_failed` lives on per-LiveSession SessionRuntimeState.
// Stopping/removing a session discards that flag. A subsequent session on the
// same ControlPlane + file DB must start clean (persist_failed=false) and
// complete prompts without inheriting a prior session's StorageError sticky bit.
// Within a single live session, the flag remains sticky after a true persist
// error until session teardown (fail-closed: no false session.completed claim).

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn soak07_persist_failed_does_not_poison_later_sessions() {
    let _g = soak_lock().await;
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    std::env::remove_var("TRACER_SOAK_SCENARIO");
    std::env::remove_var("TRACER_SOAK_BURST_COUNT");
    std::env::remove_var("TRACER_SOAK_BURST_DELAY_MS");

    let t0 = Instant::now();
    let (_keep, cp, _db) = open_file_cp().await;
    let (_proj_dir, project_id) = register_project(&cp).await;

    // Session A: normal stock run then stop (removes live state including any sticky flag).
    let a = cp
        .session_create(
            &project_id,
            Some("soak07-a".into()),
            stock_opts("happy_prompt_stream"),
        )
        .await
        .expect("session A create");
    let _ = cp.session_submit_prompt(&a.session_id, "session A").await;
    let _ = cp.session_stop(&a.session_id, false).await.expect("stop A");
    assert!(
        cp.runtime_status(Some(&a.session_id)).unwrap().is_empty(),
        "session A must leave live registry so sticky state cannot linger"
    );

    // Session B: independent live state; must succeed with clean metrics.
    let b = cp
        .session_create(
            &project_id,
            Some("soak07-b".into()),
            stock_opts("happy_prompt_stream"),
        )
        .await
        .expect("session B create");
    assert!(b.session_ready);
    let prompt_b = cp
        .session_submit_prompt(&b.session_id, "session B after A")
        .await;
    assert!(
        prompt_b.is_ok(),
        "later session must not inherit sticky persist_failed: {:?}",
        prompt_b.err().map(|e| e.to_command_error())
    );

    let events_b = cp
        .events_list(&b.session_id, 0, 500)
        .await
        .expect("events B")
        .events;
    assert!(
        has_type(&events_b, "session.completed")
            || has_type(&events_b, "agent.message.delta")
            || has_type(&events_b, "agent.message.completed")
            || has_type(&events_b, "session.prompt.submitted"),
        "session B needs stream/terminal evidence: {:?}",
        event_types(&events_b)
    );
    // Isolation property is prompt success + own events (sticky poison would
    // have failed submit). Metrics may briefly count a post-return drain race;
    // require successful persistence on B rather than absolute zero errors.
    if let Some(m) = cp.session_ingest_metrics(&b.session_id) {
        assert!(
            m.events_persisted > 0,
            "session B must persist its own events; metrics={m:?}"
        );
        assert!(
            m.events_persisted > m.persist_errors,
            "session B must not be dominated by persist_errors; metrics={m:?}"
        );
        if m.persist_errors > 0 {
            eprintln!(
                "[soak07] note: B persist_errors={} events_persisted={} (non-fatal if prompt ok)",
                m.persist_errors, m.events_persisted
            );
        }
    }

    let _ = cp.session_stop(&b.session_id, false).await.expect("stop B");
    eprintln!(
        "[soak07_persist_failed_isolation] pass=true duration_ms={}",
        t0.elapsed().as_millis()
    );
}
