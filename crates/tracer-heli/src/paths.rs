//! Workspace discovery and path helpers (no fixed absolute machine paths).

use std::path::{Component, Path, PathBuf};

use crate::error::HeliError;

/// Marker file that identifies a HeliHarness workspace root.
pub const HARNESS_MARKER: &str = "HARNESS.md";

/// Directory name for the harness installation under a workspace root.
pub const HELI_DIR_NAME: &str = ".heli-harness";

/// Canonicalize a path for Heli binding identity:
/// - absolute (best-effort via `std::fs::canonicalize` when path exists)
/// - strip Windows `\\?\` / `//?/` extended prefixes (keeps paths usable with `join`/`is_file`)
/// - forward slashes in the identity string
/// - lowercase drive letter / path on Windows
/// - strip trailing slash (except drive roots)
///
/// The returned [`PathBuf`] is built from a normalized forward-slash string so it
/// remains usable for filesystem operations on Windows and Unix.
pub fn canonicalize_path(input: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(canonical_path_string(input))
}

/// String form of [`canonicalize_path`] (forward slashes; Windows lowercased).
pub fn canonical_path_string(input: impl AsRef<Path>) -> String {
    let input = input.as_ref();
    let abs = if input.is_absolute() {
        input.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(input))
            .unwrap_or_else(|_| input.to_path_buf())
    };

    let resolved = std::fs::canonicalize(&abs).unwrap_or(abs);
    let stripped = strip_extended_path_prefix(resolved);
    let mut s = path_to_forward_slash(&stripped);

    if cfg!(windows) {
        s = s.to_lowercase();
        if s.len() > 3 && s.ends_with('/') {
            s.pop();
        }
    } else if s.len() > 1 && s.ends_with('/') {
        s.pop();
    }

    s
}

/// Remove Windows extended-length prefixes so paths stay joinable.
fn strip_extended_path_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    // `\\?\C:\...` or `//?/C:/...` or `\\?\UNC\server\share\...`
    let trimmed = if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{rest}")
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else if let Some(rest) = s.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = s.strip_prefix("//?/") {
        rest.to_string()
    } else {
        return path;
    };
    PathBuf::from(trimmed)
}

/// Convert a path to a forward-slash string.
pub fn path_to_forward_slash(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    let mut out = String::new();
    for (i, c) in path.components().enumerate() {
        match c {
            Component::Prefix(p) => {
                out.push_str(&p.as_os_str().to_string_lossy());
            }
            Component::RootDir => {
                if out.is_empty() || !out.ends_with('/') {
                    // Unix root, or after Windows prefix like `C:`
                    if out.ends_with(':') || out.is_empty() {
                        out.push('/');
                    }
                }
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.is_empty() && !out.ends_with('/') {
                    out.push('/');
                }
                out.push_str("..");
            }
            Component::Normal(s) => {
                if i > 0 && !out.ends_with('/') {
                    out.push('/');
                }
                out.push_str(&s.to_string_lossy());
            }
        }
    }
    // Fallback for empty edge cases
    if out.is_empty() {
        path.to_string_lossy().replace('\\', "/")
    } else {
        out
    }
}

/// Walk upward from `start` looking for `.heli-harness/HARNESS.md`.
///
/// Returns the workspace root (parent of `.heli-harness`) when found.
pub fn find_workspace_root(start: impl AsRef<Path>) -> Option<PathBuf> {
    let start = start.as_ref();
    let mut dir = if start.is_absolute() {
        start.to_path_buf()
    } else {
        std::env::current_dir()
            .ok()
            .map(|cwd| cwd.join(start))
            .unwrap_or_else(|| start.to_path_buf())
    };

    // If start is a file, begin at its parent.
    if dir.is_file() {
        dir = dir.parent()?.to_path_buf();
    }

    loop {
        let marker = dir.join(HELI_DIR_NAME).join(HARNESS_MARKER);
        if marker.is_file() {
            return Some(canonicalize_path(&dir));
        }
        if !dir.pop() {
            break;
        }
    }
    None
}

/// Like [`find_workspace_root`] but returns a typed error when missing.
pub fn require_workspace_root(start: impl AsRef<Path>) -> Result<PathBuf, HeliError> {
    let start = start.as_ref().to_path_buf();
    find_workspace_root(&start).ok_or(HeliError::WorkspaceNotFound { start })
}

/// Paths under a HeliHarness workspace root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeliPaths {
    /// Absolute workspace root (canonicalized when possible).
    pub workspace_root: PathBuf,
}

impl HeliPaths {
    /// Build path helpers for a known workspace root.
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    /// Discover from an arbitrary starting directory.
    pub fn discover(start: impl AsRef<Path>) -> Result<Self, HeliError> {
        Ok(Self::new(require_workspace_root(start)?))
    }

    /// `.heli-harness` directory.
    pub fn heli_dir(&self) -> PathBuf {
        self.workspace_root.join(HELI_DIR_NAME)
    }

    /// `workspace/schema.json`
    pub fn schema_path(&self) -> PathBuf {
        self.heli_dir().join("workspace").join("schema.json")
    }

    /// `workspace/index.json`
    pub fn index_path(&self) -> PathBuf {
        self.heli_dir().join("workspace").join("index.json")
    }

    /// `workspace/target.json`
    pub fn target_path(&self) -> PathBuf {
        self.heli_dir().join("workspace").join("target.json")
    }

    /// `tasks/` directory.
    pub fn tasks_dir(&self) -> PathBuf {
        self.heli_dir().join("tasks")
    }

    /// `tasks/<task_id>/task.json`
    pub fn task_json(&self, task_id: &str) -> PathBuf {
        self.tasks_dir().join(task_id).join("task.json")
    }

    /// `sessions/` directory.
    pub fn sessions_dir(&self) -> PathBuf {
        self.heli_dir().join("sessions")
    }

    /// `sessions/<session_id>.json`
    pub fn session_json(&self, session_id: &str) -> PathBuf {
        self.sessions_dir().join(format!("{session_id}.json"))
    }

    /// `bindings/worktrees/` directory.
    pub fn bindings_dir(&self) -> PathBuf {
        self.heli_dir().join("bindings").join("worktrees")
    }

    /// `locks/tasks/` directory.
    pub fn locks_dir(&self) -> PathBuf {
        self.heli_dir().join("locks").join("tasks")
    }

    /// `locks/tasks/<task_id>.write.lock/lease.json`
    pub fn lease_json(&self, task_id: &str) -> PathBuf {
        self.locks_dir()
            .join(format!("{task_id}.write.lock"))
            .join("lease.json")
    }

    /// `manifest.json`
    pub fn manifest_path(&self) -> PathBuf {
        self.heli_dir().join("manifest.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn forward_slash_normalizes_backslashes() {
        let p = PathBuf::from(r"C:\foo\bar");
        let s = path_to_forward_slash(&p);
        assert!(s.contains('/'));
        assert!(!s.contains('\\'));
    }

    #[test]
    fn discover_walks_upward() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("ws");
        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir_all(root.join(HELI_DIR_NAME)).unwrap();
        fs::write(root.join(HELI_DIR_NAME).join(HARNESS_MARKER), "# harness\n").unwrap();

        let found = find_workspace_root(&nested).expect("workspace");
        let expected = canonicalize_path(&root);
        assert_eq!(found, expected);
    }

    #[test]
    fn missing_workspace_is_none() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(find_workspace_root(tmp.path()).is_none());
    }
}
