//! Read-only HeliHarness workspace status adapter for Tracer (W1-H).
//!
//! # Scope
//!
//! This crate discovers a parent HeliHarness workspace by walking upward for
//! `.heli-harness/HARNESS.md` and parses concurrent-mode state files:
//! tasks, sessions, write leases, worktree bindings, targets, and path-claim
//! conflicts.
//!
//! It is intentionally **read-only**. It does not claim leases, mutate tasks,
//! or shell out to the `heli` CLI. Callers that need mutations must use the
//! installed HeliHarness CLI.
//!
//! # Library path decision
//!
//! W1-H places this adapter under `crates/tracer-heli/` (Rust) rather than
//! `packages/heli-client/` so it can compose with other Wave 1 Rust crates
//! (`tracer-domain`, control plane) without introducing a second JS client.
//! See `docs/modules/w1-h/LIBRARY_CHOICE.md`.
//!
//! # Safe missing-workspace behavior
//!
//! ```rust
//! use tracer_heli::{try_load_workspace_status, WorkspaceProbe};
//! use std::path::Path;
//!
//! let start = Path::new(".");
//! match WorkspaceProbe::probe(start) {
//!     WorkspaceProbe::Missing { .. } => {
//!         // No harness installed above this path — do not panic.
//!     }
//!     WorkspaceProbe::Found { root } => {
//!         let _ = root;
//!     }
//! }
//!
//! // Or: Option-returning loader
//! let _ = try_load_workspace_status(start);
//! ```

#![deny(missing_docs)]

pub mod conflict;
pub mod error;
pub mod paths;
pub mod status;
pub mod types;

pub use conflict::{
    detect_path_claim_conflicts, ConflictKind, ConflictSeverity, PathClaimConflict,
};
pub use error::HeliError;
pub use paths::{
    canonical_path_string, canonicalize_path, find_workspace_root, path_to_forward_slash,
    require_workspace_root, HeliPaths, HARNESS_MARKER, HELI_DIR_NAME,
};
pub use status::{
    load_workspace_status, try_load_workspace_status, TaskStatusView, WorkspaceProbe,
    WorkspaceStatus,
};
pub use types::{
    HostBinding, LeaseRecord, LeaseState, PathClaims, RepoEntry, SessionMode, SessionRecord,
    TargetFile, TaskRecord, TaskSource, TaskTarget, WorkspaceIndexFile, WorkspaceMode,
    WorkspaceSchemaFile, WorktreeBinding, WorktreeSource,
};
