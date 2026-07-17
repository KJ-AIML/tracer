//! Tracer domain types and Event Protocol v1.
//!
//! Owned by W1-B (`tracer-w1-domain-events`). Implements the normalized envelope,
//! session lifecycle, authentication/capability states, error categories, and
//! sequence validation from `docs/contracts/TRACER_EVENT_PROTOCOL_V1.md`.

#![deny(missing_docs)]

pub mod adapter;
pub mod auth;
pub mod capabilities;
pub mod envelope;
pub mod error;
pub mod event_type;
pub mod ids;
pub mod payload;
pub mod sequence;
pub mod session;
pub mod severity;
pub mod validate;

pub use adapter::AdapterMetadata;
pub use auth::AuthenticationState;
pub use capabilities::Capabilities;
pub use envelope::EventEnvelope;
pub use error::{ErrorCategory, ErrorClass, TracerError};
pub use event_type::{EventType, KNOWN_EVENT_TYPES};
pub use ids::{AgentRunId, EventId, ProjectId, SessionId, TracerId};
pub use sequence::{SequenceError, SequenceTracker, validate_sequence_order};
pub use session::{SessionStatus, StatusTransitionError, is_terminal, is_valid_transition};
pub use severity::Severity;
pub use validate::{EnvelopeValidationError, validate_envelope};

/// Current Tracer Event Protocol major version.
pub const EVENT_PROTOCOL_VERSION: u32 = 1;