//! Per-session lifecycle helpers (W2.2-C drain hardening).
//!
//! Owned by task `tracer-w2-drain-lifecycle`. Keeps phase / late-event policy
//! out of the dual-stage drain implementation while remaining control-plane local.

pub mod lifecycle;

pub use lifecycle::{
    is_prompt_terminal_event, is_run_terminal_status, late_event_disposition, LateEventDisposition,
    DrainLifecyclePhase, LATE_DRAIN_JOIN_TIMEOUT, LATE_EVENT_GRACE,
};
