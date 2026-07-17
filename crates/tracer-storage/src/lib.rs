//! # tracer-storage
//!
//! SQLite persistence foundation for Tracer (Wave 1 agent **W1-E**).
//!
//! ## Responsibilities
//!
//! - Database initialization and WAL journal settings
//! - Schema migrations
//! - Repository interfaces for Project, Session, Event, RuntimeProcess,
//!   ApprovalDecision, and basic Artifact records
//! - Deterministic per-session event ordering (`sequence`)
//! - Transactional, crash-safe writes
//! - Reload after restart (re-open pool + ordered reads)
//! - Storage error mapping to command-surface `errorClass` values
//!
//! ## Writer policy (normative)
//!
//! The **control plane is the sole planned writer** of the primary Tracer
//! SQLite database. Runtime sidecars and UI code must not open this database
//! for writes (F-S05, ADR-001, event protocol §1 rule 2).
//!
//! This crate exposes repository APIs intended for control-plane composition.
//! It does not embed runtime I/O or ACP.
//!
//! ## Domain IDs
//!
//! Canonical IDs (`EventId`, `ProjectId`, `SessionId`, `AgentRunId`) and
//! session/severity vocabulary come from `tracer-domain` (W1-B). Storage-local
//! IDs (`ProcessId`, `ApprovalId`, `ArtifactId`) remain under [`ids`] until the
//! domain crate expands.
//!
//! ## Paths
//!
//! Database paths are derived from a caller-supplied platform application-data
//! directory via [`path::database_path`] — no hardcoded user homes.

// Public items are documented at the module and type level; field-level docs are optional.
#![allow(missing_docs)]

pub mod db;
pub mod error;
pub mod ids;
pub mod models;
pub mod path;
pub mod repo;
pub mod timeutil;

pub use db::{
    open_database, open_in_memory, run_migrations, schema_logical_version, writer_policy, DbPool,
    OpenOptions,
};
pub use error::{StorageError, StorageErrorClass, StorageResult};
pub use ids::{
    AgentRunId, ApprovalId, ArtifactId, EventId, ProcessId, ProjectId, SessionId, TracerId,
};
pub use models::{
    ApprovalDecision, ApprovalDecisionRecord, ArtifactRecord, EventList, EventRecord,
    ProjectRecord, ProjectStatus, ReconcileReport, RuntimeProcessRecord, RuntimeProcessStatus,
    SessionRecord, SessionStatus, SessionStatusStorageExt, Severity, SeverityStorageExt,
};
pub use path::{database_dir, database_path, ensure_database_dir, TRACER_DATA_DIR, TRACER_DB_FILE};
pub use repo::{
    ApprovalRepository, ArtifactRepository, EventRepository, ProjectRepository,
    RuntimeProcessRepository, SessionRepository, SqliteStorage,
};
pub use timeutil::now_rfc3339;

/// Crate version string.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Logical schema version this crate ships (must match migration seed).
pub const SCHEMA_LOGICAL_VERSION: &str = "1";
