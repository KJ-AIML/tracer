//! Desktop control-plane composition (W1-F + W2-B E2E hooks).
//!
//! Owns construction of [`tracer_control_plane::ControlPlane`] for Tauri.
//! Test/E2E env overrides (never required for normal product use):
//! - `TRACER_DATABASE_PATH` — file SQLite path (temp DB for automated journeys)
//! - `TRACER_FAKE_ACP_JS` — absolute path to fake-acp-runtime.js
//! - `TRACER_HELI_PROBE_PATH` — directory to probe for Heli workspace
//! - `TRACER_NODE_BIN` — node executable (default: `node`)

use std::path::PathBuf;
use std::sync::Arc;

use tracer_control_plane::{ControlPlane, ControlPlaneConfig};

/// Build control plane for desktop (file DB under app data when provided).
///
/// When `db_path` is `None`, reads `TRACER_DATABASE_PATH` if set; otherwise
/// opens an in-memory store (dev fallback).
pub async fn build_control_plane(db_path: Option<PathBuf>) -> Result<Arc<ControlPlane>, String> {
    let database_path = db_path.or_else(|| {
        std::env::var("TRACER_DATABASE_PATH")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
    });

    let heli_probe_path = std::env::var("TRACER_HELI_PROBE_PATH")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let node_bin = std::env::var("TRACER_NODE_BIN")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("node"));

    let fake_js = discover_fake_js();
    let config = ControlPlaneConfig {
        database_path,
        fake_js,
        node_bin,
        heli_probe_path,
        escalate_cancel_to_process_stop: true,
    };
    let plane = ControlPlane::open(config)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Arc::new(plane))
}

/// Resolve fake ACP script for tests/dev (env override then monorepo walk).
pub fn discover_fake_js() -> Option<PathBuf> {
    // Prefer env override for tests/dev.
    if let Ok(p) = std::env::var("TRACER_FAKE_ACP_JS") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Some(pb);
        }
    }
    // Walk up from cwd for monorepo layout.
    let mut dir = std::env::current_dir().ok()?;
    for _ in 0..8 {
        let candidate = dir.join("tools/fake-acp-runtime/bin/fake-acp-runtime.js");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    // Also try from CARGO_MANIFEST_DIR when available (desktop package).
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut dir = PathBuf::from(manifest);
        for _ in 0..6 {
            let candidate = dir.join("tools/fake-acp-runtime/bin/fake-acp-runtime.js");
            if candidate.is_file() {
                return Some(candidate);
            }
            if !dir.pop() {
                break;
            }
        }
    }
    None
}

/// Resolve database path the same way the app does (for harness diagnostics).
pub fn resolve_database_path_for_e2e(explicit: Option<PathBuf>) -> Option<PathBuf> {
    explicit.or_else(|| {
        std::env::var("TRACER_DATABASE_PATH")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from)
    })
}
