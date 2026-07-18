//! Tracer control plane (W1-F).
//!
//! Composes:
//! - `tracer-runtime-adapter` (W1-D) for ACP runtime lifecycle
//! - `tracer-storage` (W1-E) as the **sole SQLite writer**
//! - `tracer-heli` (W1-H) for **read-only** workspace status
//!
//! # Architecture
//!
//! ```text
//! ControlPlane
//! ├── RuntimeSupervisor      # W1-D adapter lifecycle only
//! ├── SessionCoordinator     # Tracer session + prompt lifecycle
//! ├── EventIngestor          # continuous drain of adapter events
//! ├── PersistenceCoordinator # SOLE DB writer via tracer-storage
//! ├── ApprovalCoordinator
//! ├── CancellationCoordinator
//! ├── PresentationProjector  # typed snapshots for shell
//! └── RecoveryCoordinator
//! ```
//!
//! # Concurrency
//!
//! Event ingestion continues while `submit_prompt` blocks, while approval is
//! pending, and while cancel runs. Adapter methods take `&self` so cancel and
//! approve do not share a lock with the blocking prompt RPC.
//!
//! # Forbidden
//!
//! - Raw ACP / Grok parsing in command or presentation layers
//! - Auto-approve permissions
//! - Treating process-alive as session/prompt ready
//! - Direct SQLite from Tauri command handlers (use this crate)

// Public modules are documented; field-level docs are optional for DTOs.
#![allow(missing_docs)]

pub mod convert;
pub mod error;
pub mod heli_bridge;
pub mod plane;
pub mod presentation;
pub mod session;
pub mod session_runtime;
pub mod types;

pub use error::{CommandError, ControlPlaneError, ControlPlaneResult};
pub use heli_bridge::probe_heli;
pub use plane::{ControlPlane, ControlPlaneConfig};
pub use presentation::{
    PresentationHub, PresentationMetricsSnapshot, PresentationSubscription, SessionProjectionInput,
    DEFAULT_NOTIFY_CAPACITY,
};
pub use session::{
    is_prompt_terminal_event, late_event_disposition, DrainLifecyclePhase, LateEventDisposition,
    LATE_DRAIN_JOIN_TIMEOUT, LATE_EVENT_GRACE,
};
pub use session_runtime::{
    set_test_force_persist_error, IngestMetrics, IngestMetricsSnapshot, BRIDGE_CAPACITY,
};
pub use types::*;
