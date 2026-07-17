//! Desktop control-plane composition (W1-F).
//!
//! Owns process of constructing [`tracer_control_plane::ControlPlane`] for Tauri.

use std::path::PathBuf;
use std::sync::Arc;

use tracer_control_plane::{ControlPlane, ControlPlaneConfig};

/// Build control plane for desktop (file DB under app data when provided).
pub async fn build_control_plane(db_path: Option<PathBuf>) -> Result<Arc<ControlPlane>, String> {
    // Resolve fake runtime relative to executable or CARGO workspace when present.
    let fake_js = discover_fake_js();
    let config = ControlPlaneConfig {
        database_path: db_path,
        fake_js,
        node_bin: PathBuf::from("node"),
        heli_probe_path: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        escalate_cancel_to_process_stop: true,
    };
    let plane = ControlPlane::open(config)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Arc::new(plane))
}

fn discover_fake_js() -> Option<PathBuf> {
    // Prefer env override for tests/dev.
    if let Ok(p) = std::env::var("TRACER_FAKE_ACP_JS") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    // Walk up from cwd for monorepo layout.
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..6 {
        let candidate = dir.join("tools/fake-acp-runtime/bin/fake-acp-runtime.js");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    None
}
