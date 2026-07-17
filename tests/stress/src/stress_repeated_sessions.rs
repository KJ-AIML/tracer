//! Time-capped stress: many sequential sessions on file SQLite.
//!
//! Does not invent production throughput SLAs — records observed completion counts.

use std::time::{Duration, Instant};

use tempfile::tempdir;
use tracer_vs1_stress::{happy_opts, open_file_cp, stress_lock};

/// Keep wall clock under ~3 minutes on typical developer machines.
const SESSION_CAP: usize = 20;
const BUDGET: Duration = Duration::from_secs(180);

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stress_sequential_sessions_time_capped() {
    let _g = stress_lock().await;
    let t0 = Instant::now();
    let (_keep, cp) = open_file_cp().await;
    let proj_dir = tempdir().unwrap();
    let proj = cp
        .project_register(proj_dir.path(), Some("stress".into()))
        .await
        .unwrap();

    let mut completed = 0usize;
    for i in 0..SESSION_CAP {
        if t0.elapsed() > BUDGET {
            break;
        }
        let session = cp
            .session_create(&proj.project_id, Some(format!("stress-{i}")), happy_opts())
            .await
            .expect("create");
        let _ = cp
            .session_submit_prompt(&session.session_id, "stress ping")
            .await;
        let _ = cp.session_stop(&session.session_id, false).await;
        completed += 1;
    }

    assert!(
        completed >= 5,
        "expected at least 5 sessions within budget, got {completed}"
    );
    let listed = cp.session_list(&proj.project_id, 100).await.unwrap();
    assert!(listed.len() >= completed);

    eprintln!(
        "[stress_sequential] completed={completed} duration_ms={} listed={}",
        t0.elapsed().as_millis(),
        listed.len()
    );
}
