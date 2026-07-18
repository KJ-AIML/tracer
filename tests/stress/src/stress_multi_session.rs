//! W2-C multi-session stress: overlapping live sessions on file SQLite.
//!
//! Time-capped; does not invent production throughput SLAs.

use std::time::{Duration, Instant};

use tempfile::tempdir;
use tracer_vs1_stress::{happy_opts, open_file_cp, stress_lock};

/// Keep wall clock reasonable on developer machines.
const SESSION_CAP: usize = 8;
const BUDGET: Duration = Duration::from_secs(180);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_overlapping_live_sessions() {
    let _g = stress_lock().await;
    let t0 = Instant::now();
    let (_keep, cp) = open_file_cp().await;
    let proj_dir = tempdir().unwrap();
    let proj = cp
        .project_register(proj_dir.path(), Some("ms-stress".into()))
        .await
        .unwrap();

    // Create several live sessions without stopping immediately.
    let mut live = Vec::new();
    for i in 0..SESSION_CAP {
        if t0.elapsed() > BUDGET {
            break;
        }
        match cp
            .session_create(&proj.project_id, Some(format!("ms-stress-{i}")), happy_opts())
            .await
        {
            Ok(s) => live.push(s.session_id),
            Err(e) => {
                eprintln!("[stress_ms] create failed at {i}: {e}");
                break;
            }
        }
    }

    assert!(
        live.len() >= 2,
        "need at least 2 live sessions, got {}",
        live.len()
    );
    assert_eq!(cp.live_session_count(), live.len());

    // Focus switch across all live sessions.
    for sid in &live {
        let snap = cp.presentation_focus(sid).await.expect("focus");
        assert_eq!(snap.active_session_id.as_deref(), Some(sid.as_str()));
    }

    // Sequential prompts while others remain live.
    let mut prompted = 0usize;
    for sid in &live {
        if t0.elapsed() > BUDGET {
            break;
        }
        if cp.session_submit_prompt(sid, "ms-stress ping").await.is_ok() {
            prompted += 1;
        }
    }

    // Deterministic multi-session teardown.
    cp.shutdown_all().await.expect("shutdown_all");
    assert_eq!(cp.live_session_count(), 0);

    // Histories remain listable and session-scoped.
    for sid in &live {
        let events = cp.events_list(sid, 0, 200).await.unwrap();
        assert!(
            events.events.iter().all(|e| {
                e.get("sessionId")
                    .and_then(|s| s.as_str())
                    .map(|s| s == sid)
                    .unwrap_or(false)
            }) || events.events.is_empty()
        );
    }

    eprintln!(
        "[stress_ms_overlap] live_created={} prompted={} duration_ms={}",
        live.len(),
        prompted,
        t0.elapsed().as_millis()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_create_stop_cycle_with_peers() {
    let _g = stress_lock().await;
    let t0 = Instant::now();
    let (_keep, cp) = open_file_cp().await;
    let proj_dir = tempdir().unwrap();
    let proj = cp
        .project_register(proj_dir.path(), Some("ms-cycle".into()))
        .await
        .unwrap();

    // Keep one long-lived peer.
    let peer = cp
        .session_create(&proj.project_id, Some("peer".into()), happy_opts())
        .await
        .expect("peer");

    let mut cycles = 0usize;
    for i in 0..10 {
        if t0.elapsed() > BUDGET {
            break;
        }
        let s = cp
            .session_create(
                &proj.project_id,
                Some(format!("cycle-{i}")),
                happy_opts(),
            )
            .await
            .expect("create");
        let _ = cp.session_submit_prompt(&s.session_id, "cycle").await;
        let _ = cp.session_stop(&s.session_id, false).await;
        cycles += 1;

        // Peer must remain healthy.
        let p = cp.session_get(&peer.session_id).await.unwrap();
        assert!(
            p.process_alive
                || matches!(
                    p.status,
                    tracer_domain::SessionStatus::Ready
                        | tracer_domain::SessionStatus::Running
                        | tracer_domain::SessionStatus::Completed
                ),
            "peer status={:?}",
            p.status
        );
    }

    assert!(cycles >= 3, "expected >=3 cycles, got {cycles}");
    let _ = cp.session_submit_prompt(&peer.session_id, "final").await;
    cp.shutdown_all().await.unwrap();
    assert_eq!(cp.live_session_count(), 0);

    eprintln!(
        "[stress_ms_cycle] cycles={cycles} duration_ms={}",
        t0.elapsed().as_millis()
    );
}
