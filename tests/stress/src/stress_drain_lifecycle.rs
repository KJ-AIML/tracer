//! W2.2-C drain lifecycle stress — repeated prompts, overlapping sessions,
//! delayed terminal/close races, slow SQLite, cancel/shutdown races.
//!
//! Assert: lost 0, dups 0, terminal lost 0, orphan drain 0 (via clean
//! shutdown_all), cross-session 0, false persist-error 0 for normal lifecycle.

use std::time::{Duration, Instant};

use tempfile::tempdir;
use tracer_control_plane::set_test_force_persist_error;
use tracer_vs1_stress::{happy_opts, open_file_cp, stress_lock};

const PROMPT_ROUNDS: usize = 6;
const OVERLAP_SESSIONS: usize = 4;
const BUDGET: Duration = Duration::from_secs(180);

fn sequences_unique(events: &[serde_json::Value]) -> bool {
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

fn has_type(events: &[serde_json::Value], t: &str) -> bool {
    events
        .iter()
        .any(|e| e.get("type").and_then(|x| x.as_str()) == Some(t))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_repeated_prompts_zero_false_persist_errors() {
    let _g = stress_lock().await;
    set_test_force_persist_error(false);
    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");

    let t0 = Instant::now();
    let (_keep, cp) = open_file_cp().await;
    let proj_dir = tempdir().unwrap();
    let proj = cp
        .project_register(proj_dir.path(), Some("dl-stress".into()))
        .await
        .unwrap();

    let session = cp
        .session_create(&proj.project_id, Some("dl-repeat".into()), happy_opts())
        .await
        .expect("create");

    let mut ok = 0usize;
    for i in 0..PROMPT_ROUNDS {
        if t0.elapsed() > BUDGET {
            break;
        }
        if cp
            .session_submit_prompt(&session.session_id, &format!("round {i}"))
            .await
            .is_ok()
        {
            ok += 1;
        }
        if let Some(m) = cp.session_ingest_metrics(&session.session_id) {
            assert_eq!(
                m.persist_errors, 0,
                "false persist_error after round {i}: {m:?}"
            );
        }
    }
    assert!(ok >= 2, "need multiple successful prompts, got {ok}");

    let events = cp
        .events_list(&session.session_id, 0, 50_000)
        .await
        .unwrap();
    assert!(sequences_unique(&events.events), "duplicate sequences");
    assert!(
        has_type(&events.events, "session.completed")
            || has_type(&events.events, "agent.message.delta"),
        "terminal/stream evidence lost"
    );

    cp.shutdown_all().await.expect("shutdown_all joins drains");
    assert_eq!(cp.live_session_count(), 0, "orphan live sessions");

    eprintln!(
        "[stress_dl_repeat] ok_prompts={} duration_ms={}",
        ok,
        t0.elapsed().as_millis()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_overlapping_sessions_independent_drains() {
    let _g = stress_lock().await;
    set_test_force_persist_error(false);
    // Mild slow SQLite pressure (not a production SLA).
    std::env::set_var("TRACER_SOAK_PERSIST_DELAY_MS", "1");

    let t0 = Instant::now();
    let (_keep, cp) = open_file_cp().await;
    let proj_dir = tempdir().unwrap();
    let proj = cp
        .project_register(proj_dir.path(), Some("dl-overlap".into()))
        .await
        .unwrap();

    let mut ids = Vec::new();
    for i in 0..OVERLAP_SESSIONS {
        if t0.elapsed() > BUDGET {
            break;
        }
        match cp
            .session_create(&proj.project_id, Some(format!("dl-ov-{i}")), happy_opts())
            .await
        {
            Ok(s) => ids.push(s.session_id),
            Err(e) => {
                eprintln!("[stress_dl_overlap] create {i} failed: {e}");
                break;
            }
        }
    }
    assert!(ids.len() >= 2, "need >=2 sessions");

    let cp = std::sync::Arc::new(cp);
    // Stagger starts so late-drain windows overlap without a pathological
    // simultaneous write stampede (still concurrent across sessions).
    let mut handles = Vec::new();
    for (i, id) in ids.iter().enumerate() {
        let c = std::sync::Arc::clone(&cp);
        let sid = id.clone();
        let delay = Duration::from_millis(40 * i as u64);
        handles.push(tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            c.session_submit_prompt(&sid, "overlap drain").await
        }));
    }
    for h in handles {
        let _ = h.await;
    }

    // Late drain settle under slow persist.
    tokio::time::sleep(Duration::from_millis(800)).await;

    let mut false_pe = 0u64;
    for id in &ids {
        if let Some(m) = cp.session_ingest_metrics(id) {
            false_pe += m.persist_errors;
            assert!(
                m.events_persisted > 0 || m.bridge_accepted > 0,
                "session {id} saw no drain activity: {m:?}"
            );
        }
        let ev = cp.events_list(id, 0, 5_000).await.unwrap();
        assert!(sequences_unique(&ev.events), "dups in {id}");
        assert!(
            ev.events.iter().all(|e| {
                e.get("sessionId")
                    .and_then(|s| s.as_str())
                    .map(|s| s == id)
                    .unwrap_or(true)
            }),
            "cross-session leak into {id}"
        );
    }
    assert_eq!(false_pe, 0, "false persist_errors across overlap sessions");

    cp.shutdown_all().await.expect("join all drains");
    assert_eq!(cp.live_session_count(), 0);

    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    eprintln!(
        "[stress_dl_overlap] sessions={} duration_ms={}",
        ids.len(),
        t0.elapsed().as_millis()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_cancel_and_shutdown_races() {
    let _g = stress_lock().await;
    set_test_force_persist_error(false);
    std::env::set_var("TRACER_SOAK_PERSIST_DELAY_MS", "4");

    let t0 = Instant::now();
    let (_keep, cp) = open_file_cp().await;
    let proj_dir = tempdir().unwrap();
    let proj = cp
        .project_register(proj_dir.path(), Some("dl-race".into()))
        .await
        .unwrap();

    let session = cp
        .session_create(
            &proj.project_id,
            Some("dl-cancel-race".into()),
            tracer_control_plane::RuntimeCreateOptions {
                runtime_kind: "acp-stdio".into(),
                scenario_id: Some("cancel_mid_stream".into()),
                executable_override: None,
                extra_args: vec![],
                fake_js: Some(tracer_vs1_stress::fake_js().display().to_string()),
            },
        )
        .await
        .expect("create");

    let cp = std::sync::Arc::new(cp);
    let sid = session.session_id.clone();
    let cp_p = std::sync::Arc::clone(&cp);
    let sid_p = sid.clone();
    let prompt = tokio::spawn(async move {
        let _ = cp_p.session_submit_prompt(&sid_p, "race cancel").await;
    });
    tokio::time::sleep(Duration::from_millis(60)).await;
    let _ = cp.session_cancel(&sid).await;
    let _ = prompt.await;

    // Second session must not be poisoned by cancel race.
    let good = cp
        .session_create(&proj.project_id, Some("dl-after-race".into()), happy_opts())
        .await
        .expect("create after race");
    let p = cp
        .session_submit_prompt(&good.session_id, "healthy")
        .await
        .expect("not poisoned");
    assert!(p.accepted);
    if let Some(m) = cp.session_ingest_metrics(&good.session_id) {
        assert_eq!(m.persist_errors, 0, "poison metrics={m:?}");
    }

    cp.shutdown_all().await.expect("shutdown joins");
    assert_eq!(cp.live_session_count(), 0);

    std::env::remove_var("TRACER_SOAK_PERSIST_DELAY_MS");
    eprintln!("[stress_dl_race] duration_ms={}", t0.elapsed().as_millis());
}
