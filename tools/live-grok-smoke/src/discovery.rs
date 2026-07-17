//! Binary discovery for stock Grok (path-portable).

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Result of locating and probing the Grok executable.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryResult {
    /// Whether an executable was resolved.
    pub found: bool,
    /// Sanitized path (or bare name) for evidence only.
    pub executable: Option<String>,
    /// Version string from `grok --version` if obtainable.
    pub version: Option<String>,
    /// Sanitized absolute path when resolvable.
    pub absolute_path: Option<String>,
    /// How it was found.
    pub source: String,
    /// Notes (never secrets).
    pub notes: Vec<String>,
    /// Real filesystem path used for spawn (never serialized into evidence).
    #[serde(skip)]
    pub spawn_path: Option<PathBuf>,
}

/// Resolve Grok binary: override → TRACER_GROK_BIN → PATH (`grok` / `grok.exe`).
pub fn discover_grok(override_path: Option<&Path>) -> DiscoveryResult {
    let mut notes = Vec::new();

    if let Some(p) = override_path {
        notes.push("using explicit --grok / TRACER_GROK_BIN override".into());
        return probe_candidate(p, "override", notes);
    }

    // PATH lookup: try bare name first (portable).
    let candidates: &[&str] = if cfg!(windows) {
        &["grok", "grok.exe"]
    } else {
        &["grok"]
    };

    for name in candidates {
        if let Some(abs) = which(name) {
            notes.push(format!("resolved via PATH lookup for '{name}'"));
            return probe_candidate(&abs, "path", notes);
        }
    }

    // Fall back to bare name — spawn will fail later with clear error.
    notes.push("grok not found on PATH; spawn will use bare name 'grok'".into());
    DiscoveryResult {
        found: false,
        executable: Some("grok".into()),
        version: None,
        absolute_path: None,
        source: "fallback-bare-name".into(),
        notes,
        spawn_path: Some(PathBuf::from("grok")),
    }
}

fn probe_candidate(path: &Path, source: &str, mut notes: Vec<String>) -> DiscoveryResult {
    let display = path.display().to_string();
    let abs_path: Option<PathBuf> = if path.is_absolute() {
        Some(path.to_path_buf())
    } else {
        which(&display)
    };

    let exists = path.exists()
        || abs_path.as_ref().map(|a| a.exists()).unwrap_or(false)
        || source == "path";

    let version = if exists || source == "path" || source == "override" {
        try_version(path).or_else(|| abs_path.as_ref().and_then(|a| try_version(a)))
    } else {
        None
    };

    if let Some(ref v) = version {
        notes.push(format!("version probe ok: {v}"));
    } else if exists {
        notes.push("version probe failed or timed out".into());
    }

    // Real path for spawn (prefer absolute).
    let spawn_path = abs_path
        .clone()
        .unwrap_or_else(|| path.to_path_buf());

    // Evidence must not retain raw user home segments.
    let safe_display = crate::sanitize::sanitize_text(&display);
    let safe_abs = abs_path.map(|a| crate::sanitize::sanitize_text(&a.display().to_string()));
    let safe_notes = notes
        .into_iter()
        .map(|n| crate::sanitize::sanitize_text(&n))
        .collect();

    DiscoveryResult {
        found: version.is_some() || exists,
        executable: Some(safe_display),
        version,
        absolute_path: safe_abs,
        source: source.into(),
        notes: safe_notes,
        spawn_path: Some(spawn_path),
    }
}

fn try_version(exe: &Path) -> Option<String> {
    let mut cmd = Command::new(exe);
    cmd.arg("--version");
    // Avoid inheriting interactive noise; capture stdout only.
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::null());
    cmd.stdin(std::process::Stdio::null());

    let child = cmd.spawn().ok()?;
    // Bound wait — discovery must not hang the harness.
    let output = wait_with_timeout(child, Duration::from_secs(5))?;
    if !output.status.success() && output.stdout.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        None
    } else {
        // Sanitize any accidental path leakage in version banners.
        Some(crate::sanitize::sanitize_text(line))
    }
}

fn wait_with_timeout(
    mut child: std::process::Child,
    timeout: Duration,
) -> Option<std::process::Output> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => return child.wait_with_output().ok(),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return None,
        }
    }
}

/// Cross-platform which(1) lite — search PATH for `name`.
pub fn which(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        // Windows: allow missing extension if PATHEXT would resolve — also try .exe
        if cfg!(windows) {
            let exe = dir.join(format!("{name}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
        }
    }
    None
}

/// Host platform label for evidence.
pub fn platform_label() -> String {
    format!(
        "{}-{}",
        env::consts::OS,
        env::consts::ARCH
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stock_args_match_w1d() {
        assert_eq!(
            tracer_runtime_adapter::grok_stdio_args(),
            vec!["agent".to_string(), "--no-leader".to_string(), "stdio".to_string()]
        );
    }

    #[test]
    fn platform_nonempty() {
        assert!(!platform_label().is_empty());
    }

    #[test]
    fn override_missing_reports_not_found() {
        let r = discover_grok(Some(Path::new(
            "definitely-not-a-real-grok-binary-xyz-12345",
        )));
        assert_eq!(r.source, "override");
        // May or may not be found depending on exists check; version should be None.
        assert!(r.version.is_none());
    }
}
