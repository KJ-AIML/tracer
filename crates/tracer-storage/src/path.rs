//! Database path derivation through platform application-data directories.
//!
//! The storage crate does **not** hardcode user home paths. Callers (Tauri
//! control plane) supply the platform app-data root obtained from the host
//! (e.g. Tauri `path::app_data_dir`). Storage only joins well-known relative
//! segments under that root.

use std::path::{Path, PathBuf};

/// Relative directory under the platform application data root.
pub const TRACER_DATA_DIR: &str = "tracer";

/// Primary database file name.
pub const TRACER_DB_FILE: &str = "tracer.db";

/// Resolve the primary Tracer SQLite path under a platform app-data directory.
///
/// ```text
/// {app_data_dir}/tracer/tracer.db
/// ```
///
/// `app_data_dir` must come from OS/platform APIs (Tauri path plugin, etc.).
/// Tests may pass a temporary directory.
pub fn database_path(app_data_dir: impl AsRef<Path>) -> PathBuf {
    app_data_dir
        .as_ref()
        .join(TRACER_DATA_DIR)
        .join(TRACER_DB_FILE)
}

/// Directory that should contain the database file.
pub fn database_dir(app_data_dir: impl AsRef<Path>) -> PathBuf {
    app_data_dir.as_ref().join(TRACER_DATA_DIR)
}

/// Ensure the database parent directory exists.
pub async fn ensure_database_dir(app_data_dir: impl AsRef<Path>) -> std::io::Result<PathBuf> {
    let dir = database_dir(app_data_dir);
    tokio::fs::create_dir_all(&dir).await?;
    Ok(dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn joins_relative_segments_only() {
        let root = PathBuf::from("app-data-root");
        let path = database_path(&root);
        assert_eq!(
            path,
            PathBuf::from("app-data-root")
                .join("tracer")
                .join("tracer.db")
        );
        // No drive-letter or home path baked into the crate logic itself.
        let s = path.to_string_lossy();
        assert!(!s.contains("/Users/"));
        assert!(!s.contains("C:\\Users"));
    }
}
