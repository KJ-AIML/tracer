//! Bounded presentation delivery (W2-A).
//!
//! # Preferred path
//!
//! ```text
//! persist normalized event
//!   → update canonical presentation projection
//!   → increment snapshot revision (monotonic)
//!   → bounded / coalescing notification signal
//!   → consumer requests latest snapshot
//! ```
//!
//! Slow or absent consumers must not force retention of every intermediate
//! notification. Persisted history remains in SQLite; notifications are not
//! a substitute for storage.
//!
//! # Integrator notes (plane.rs hooks)
//!
//! - [`PresentationHub`] is owned by [`crate::plane::ControlPlane`].
//! - After command-driven state changes, call
//!   [`PresentationHub::publish_snapshot`] (via `refresh_snapshot_for`).
//! - After each successful persist, the ingest pump calls
//!   [`PresentationHub::publish_session_update`].
//! - Consumers: [`PresentationHub::subscribe`] + [`PresentationHub::snapshot`].
//! - Legacy: [`PresentationHub::attach_legacy_sender`] keeps SOAK-03 API shape
//!   without unbounded per-event fan-out growth.

mod hub;

pub use hub::{
    PresentationHub, PresentationMetrics, PresentationMetricsSnapshot, PresentationSubscription,
    SessionProjectionInput, DEFAULT_NOTIFY_CAPACITY,
};
