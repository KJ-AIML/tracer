//! Tracer runtime process manager (W1-C).
//!
//! Owns OS-level sidecar lifecycle for ACP-compatible runtimes:
//!
//! - spawn configuration (executable, args, env, cwd)
//! - independent stdout / stderr handling
//! - process-alive signals and process-exit observation
//! - graceful termination (stdin close) + forced tree kill
//! - orphan prevention via Windows Job Object or Unix process groups
//!
//! # Explicit non-goals
//!
//! - ACP protocol parsing / normalization
//! - SQLite / session persistence
//! - Claiming protocol-ready, authenticated, or session-ready
//!
//! Process-alive (`ProcessPhase::Alive` / `runtime.process.started`) is **distinct**
//! from `runtime.process.ready` (adapter initialize + caps) and from session-ready
//! after auth + `session/new`. Confusing these is F-A05.

#![deny(missing_docs)]

pub mod config;
pub mod error;
pub mod event;
pub mod handle;
pub mod ids;
pub mod platform;
pub mod readiness;

pub use config::{
    SpawnConfig, StopPolicy, DEFAULT_FORCE_TIMEOUT, DEFAULT_GRACEFUL_TIMEOUT,
    DEFAULT_STDERR_CHUNK_LIMIT,
};
pub use error::{ProcessError, ProcessErrorClass};
pub use event::{ExitInfo, ProcessEvent};
pub use handle::{ManagedProcess, ProcessManager};
pub use ids::ProcessId;
pub use platform::TreeIsolation;
pub use readiness::{ProcessPhase, ReadinessView};
