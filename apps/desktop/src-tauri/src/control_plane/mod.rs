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

/// Write a test-only readiness marker file when `TRACER_E2E_READY_MARKER` is set.
/// Used by L3-J harness process-level readiness (optional; DOM marker is primary).
pub fn write_e2e_ready_marker() {
    let Ok(path) = std::env::var("TRACER_E2E_READY_MARKER") else {
        return;
    };
    let path = path.trim();
    if path.is_empty() {
        return;
    }
    let pb = PathBuf::from(path);
    if let Some(parent) = pb.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body = format!(
        "ready=1\npid={}\ndatabase={}\nfakeJs={}\n",
        std::process::id(),
        std::env::var("TRACER_DATABASE_PATH").unwrap_or_default(),
        std::env::var("TRACER_FAKE_ACP_JS").unwrap_or_default(),
    );
    let _ = std::fs::write(&pb, body);
}

/// Load test-only environment from a dotenv-style file before control-plane open.
///
/// Why: some WebDriver/`tauri-driver` hosts do not reliably forward `tauri:options.env`
/// into the child process. L3-J therefore passes:
///
/// ```text
/// tracer-desktop.exe --tracer-e2e-env=<path>
/// ```
///
/// File format: one `KEY=VALUE` per line (`#` comments, blank lines ignored).
/// Only applies when the path exists. Never required for normal product use.
pub fn apply_e2e_env_from_cli() {
    let mut path: Option<PathBuf> = None;
    if let Ok(p) = std::env::var("TRACER_E2E_ENV_FILE") {
        if !p.trim().is_empty() {
            path = Some(PathBuf::from(p.trim()));
        }
    }
    if path.is_none() {
        for arg in std::env::args().skip(1) {
            if let Some(rest) = arg.strip_prefix("--tracer-e2e-env=") {
                path = Some(PathBuf::from(rest));
                break;
            }
            if arg == "--tracer-e2e-env" {
                // next arg form not required; support only = form for harness simplicity
                continue;
            }
        }
    }
    let Some(path) = path else {
        return;
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("TRACER E2E env file unreadable: {}", path.display());
        return;
    };
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim().trim_matches('"');
        if k.is_empty() {
            continue;
        }
        // Only set harness-owned vars (do not clobber arbitrary process env from file).
        const ALLOW: &[&str] = &[
            "TRACER_DATABASE_PATH",
            "TRACER_FAKE_ACP_JS",
            "TRACER_HELI_PROBE_PATH",
            "TRACER_NODE_BIN",
            "TRACER_E2E_READY_MARKER",
            "TRACER_E2E_PROFILE",
            "TRACER_E2E_ENV_FILE",
        ];
        if ALLOW.contains(&k) {
            std::env::set_var(k, v);
        }
    }
}
