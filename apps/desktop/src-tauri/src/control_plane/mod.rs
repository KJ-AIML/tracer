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

/// Keys allowed from `--tracer-e2e-env=` / `TRACER_E2E_ENV_FILE` (harness isolation only).
/// Arbitrary process env (PATH, credentials, HOME, user profile DB paths via free-form
/// keys) is never applied from the file.
pub const E2E_ENV_ALLOWLIST: &[&str] = &[
    "TRACER_DATABASE_PATH",
    "TRACER_FAKE_ACP_JS",
    "TRACER_HELI_PROBE_PATH",
    "TRACER_NODE_BIN",
    "TRACER_E2E_READY_MARKER",
    "TRACER_E2E_PROFILE",
    "TRACER_E2E_ENV_FILE",
];

/// Parse dotenv-style body into allowlisted KEY=VALUE pairs.
///
/// Disallowed keys, blank lines, comments, and malformed lines are ignored.
/// Values may be double-quoted; quotes are stripped.
pub fn parse_e2e_env_body(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
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
        if E2E_ENV_ALLOWLIST.contains(&k) {
            out.push((k.to_string(), v.to_string()));
        }
    }
    out
}

/// Validate e2e env file path: absolute + existing regular file.
///
/// Relative paths are rejected so a cwd-relative surprise cannot silently open a
/// developer/user DB under the wrong working directory.
pub fn validate_e2e_env_path(path: &std::path::Path) -> Result<(), String> {
    if !path.is_absolute() {
        return Err(format!(
            "TRACER E2E env path must be absolute (got {})",
            path.display()
        ));
    }
    if !path.is_file() {
        return Err(format!(
            "TRACER E2E env path must be an existing file (got {})",
            path.display()
        ));
    }
    Ok(())
}

/// Apply allowlisted keys from a validated absolute env file into process env.
pub fn apply_e2e_env_from_path(path: &std::path::Path) -> Result<usize, String> {
    validate_e2e_env_path(path)?;
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("TRACER E2E env file unreadable ({}): {e}", path.display()))?;
    let pairs = parse_e2e_env_body(&text);
    for (k, v) in &pairs {
        // Single-threaded before Tauri loop; harness-owned keys only.
        std::env::set_var(k, v);
    }
    Ok(pairs.len())
}

/// Load test-only environment from a dotenv-style file before control-plane open.
///
/// Why: some WebDriver/`tauri-driver` hosts do not reliably forward `tauri:options.env`
/// into the child process. L3-J therefore passes:
///
/// ```text
/// tracer-desktop.exe --tracer-e2e-env=<absolute-path>
/// ```
///
/// File format: one `KEY=VALUE` per line (`#` comments, blank lines ignored).
/// Only allowlisted keys are applied. Path must be absolute and exist.
/// Never required for normal product use (no flag → no-op).
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
    if let Err(e) = apply_e2e_env_from_path(&path) {
        eprintln!("{e}");
    }
}

#[cfg(test)]
mod e2e_env_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_allowlists_only_known_keys() {
        let body = r#"
# comment
TRACER_DATABASE_PATH=/tmp/tracer-e2e.sqlite
PATH=/evil
HOME=/evil-home
API_KEY=secret
TRACER_FAKE_ACP_JS="/tmp/fake.js"
MALFORMED_LINE
=no-key
TRACER_NODE_BIN=node
"#;
        let pairs = parse_e2e_env_body(body);
        let keys: Vec<_> = pairs.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            vec![
                "TRACER_DATABASE_PATH",
                "TRACER_FAKE_ACP_JS",
                "TRACER_NODE_BIN"
            ]
        );
        assert_eq!(pairs[1].1, "/tmp/fake.js");
        assert!(!pairs.iter().any(|(k, _)| k == "PATH" || k == "API_KEY" || k == "HOME"));
    }

    #[test]
    fn parse_ignores_empty_and_comments() {
        assert!(parse_e2e_env_body("").is_empty());
        assert!(parse_e2e_env_body("# only comment\n\n").is_empty());
    }

    #[test]
    fn relative_path_rejected() {
        let err = validate_e2e_env_path(std::path::Path::new("relative.env")).unwrap_err();
        assert!(err.contains("absolute"), "{err}");
    }

    #[test]
    fn missing_file_rejected() {
        let p = if cfg!(windows) {
            PathBuf::from(r"C:\tracer-e2e-definitely-missing-env-file-xyz.env")
        } else {
            PathBuf::from("/tmp/tracer-e2e-definitely-missing-env-file-xyz.env")
        };
        let err = validate_e2e_env_path(&p).unwrap_err();
        assert!(err.contains("existing file"), "{err}");
    }

    #[test]
    fn absolute_existing_file_accepted_and_allowlist_only() {
        let dir = std::env::temp_dir().join(format!(
            "tracer-e2e-env-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("tracer-e2e.env");
        {
            let mut f = std::fs::File::create(&path).expect("create");
            writeln!(
                f,
                "TRACER_E2E_PROFILE=unit-test-profile\nPATH=should-not-apply\nAPI_KEY=nope"
            )
            .expect("write");
        }
        assert!(path.is_absolute());
        validate_e2e_env_path(&path).expect("valid path");
        let text = std::fs::read_to_string(&path).unwrap();
        let pairs = parse_e2e_env_body(&text);
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "TRACER_E2E_PROFILE");
        assert_eq!(pairs[0].1, "unit-test-profile");
        let _ = std::fs::remove_dir_all(&dir);
    }
}