//! Bounded stress helpers for VS1-H3 (no infinite soaks).

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;

use tempfile::tempdir;
use tracer_control_plane::{ControlPlane, ControlPlaneConfig, RuntimeCreateOptions};

pub async fn stress_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

pub fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

pub fn fake_js() -> PathBuf {
    repo_root().join("tools/fake-acp-runtime/bin/fake-acp-runtime.js")
}

pub async fn open_file_cp() -> (tempfile::TempDir, ControlPlane) {
    assert!(fake_js().is_file(), "missing fake {}", fake_js().display());
    let dir = tempdir().unwrap();
    let db = dir.path().join("stress.db");
    let cp = ControlPlane::open(ControlPlaneConfig {
        database_path: Some(db),
        fake_js: Some(fake_js()),
        node_bin: PathBuf::from("node"),
        heli_probe_path: repo_root(),
        escalate_cancel_to_process_stop: true,
    })
    .await
    .expect("open");
    (dir, cp)
}

pub fn happy_opts() -> RuntimeCreateOptions {
    RuntimeCreateOptions {
        runtime_kind: "acp-stdio".into(),
        scenario_id: Some("happy_prompt_stream".into()),
        executable_override: None,
        extra_args: vec![],
        fake_js: Some(fake_js().display().to_string()),
    }
}
