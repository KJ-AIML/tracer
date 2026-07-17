//! Error types for the read-only HeliHarness adapter.

use std::path::PathBuf;

use thiserror::Error;

/// Errors produced while discovering or parsing HeliHarness state.
#[derive(Debug, Error)]
pub enum HeliError {
    /// No parent directory containing `.heli-harness/HARNESS.md` was found.
    #[error("heli workspace not found from {start}")]
    WorkspaceNotFound {
        /// Directory the upward search started from.
        start: PathBuf,
    },

    /// A required harness file is missing under a discovered workspace.
    #[error("missing heli path: {path}")]
    MissingPath {
        /// Absolute or relative path that was expected.
        path: PathBuf,
    },

    /// JSON could not be parsed.
    #[error("invalid json at {path}: {source}")]
    InvalidJson {
        /// Path of the file that failed to parse.
        path: PathBuf,
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
    },

    /// I/O failure while reading harness files.
    #[error("io error at {path}: {source}")]
    Io {
        /// Path involved in the I/O operation.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Lease file exists but is missing required fields.
    #[error("malformed lease for task {task_id}: {reason}")]
    MalformedLease {
        /// Task the lease belongs to.
        task_id: String,
        /// Human-readable reason.
        reason: String,
    },
}

impl HeliError {
    /// True when the error is specifically "no workspace found".
    pub fn is_workspace_not_found(&self) -> bool {
        matches!(self, Self::WorkspaceNotFound { .. })
    }
}
