//! Tracer runtime adapter (W1-D).
//!
//! Composes:
//!
//! ```text
//! ACP transport → codec → session protocol SM → runtime adapter → Tracer events
//! ```
//!
//! Uses `tracer-process` for process ownership (no second process manager) and
//! `tracer-acp-client` for framing + protocol state. Emits normalized
//! [`tracer_domain::EventEnvelope`] values so React / W1-F never parse raw Grok
//! or ACP frames.
//!
//! # Public surface for W1-F
//!
//! See `docs/modules/w1-d/W1_D_PUBLIC_INTERFACE.md`.

#![deny(missing_docs)]

pub mod adapter;
pub mod config;
pub mod error;
pub mod normalize;
pub mod readiness;

pub use adapter::{
    AdapterEvent, AdapterHandle, ApprovalDecisionRequest, PromptRequest, RuntimeAdapter,
    RuntimeAdapterState, SessionCreateParams, ShutdownOptions, DEFAULT_CANCEL_TIMEOUT,
    DEFAULT_RPC_TIMEOUT, PERMISSION_CANCEL_DEADLOCK_BUDGET,
};
pub use config::{
    fake_acp_spawn_config, grok_stdio_args, grok_stdio_spawn_config, RuntimeKind, RuntimeSpawnSpec,
};
pub use error::AdapterError;
pub use normalize::{
    capabilities_from_initialize, normalize_notification, normalize_server_request, EnvelopeBuilder,
};
pub use readiness::AdapterReadiness;
