//! Envelope validation beyond serde shape checks.

use crate::envelope::EventEnvelope;
use crate::EVENT_PROTOCOL_VERSION;
use thiserror::Error;

/// Envelope validation failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EnvelopeValidationError {
    /// Unsupported or zero protocol version.
    #[error("unsupported eventVersion: {0} (expected {EVENT_PROTOCOL_VERSION})")]
    UnsupportedVersion(u32),
    /// Sequence must be ≥ 1.
    #[error("sequence must be >= 1, got {0}")]
    InvalidSequence(u64),
    /// Type string empty.
    #[error("event type must be non-empty")]
    EmptyType,
}

/// Validate semantic rules for a deserialized envelope.
pub fn validate_envelope(env: &EventEnvelope) -> Result<(), EnvelopeValidationError> {
    if env.event_version != EVENT_PROTOCOL_VERSION {
        return Err(EnvelopeValidationError::UnsupportedVersion(
            env.event_version,
        ));
    }
    if env.sequence < 1 {
        return Err(EnvelopeValidationError::InvalidSequence(env.sequence));
    }
    if env.event_type.as_str().is_empty() {
        return Err(EnvelopeValidationError::EmptyType);
    }
    Ok(())
}

/// Validate a stream of envelopes for a single session: same sessionId + monotonic sequence.
pub fn validate_session_event_stream(
    events: &[EventEnvelope],
) -> Result<(), EnvelopeValidationError> {
    for (i, env) in events.iter().enumerate() {
        validate_envelope(env)?;
        if i > 0 {
            let prev = &events[i - 1];
            if prev.session_id != env.session_id {
                // Cross-session mix is a caller error; treat as invalid sequence context.
                return Err(EnvelopeValidationError::InvalidSequence(env.sequence));
            }
            if env.sequence != prev.sequence + 1 {
                return Err(EnvelopeValidationError::InvalidSequence(env.sequence));
            }
        } else if env.sequence != 1 {
            // First event in a full stream should be 1; partial windows may use validate_sequence_order with custom start.
            // Here we only enforce pairing consistency when sequence is non-empty stream from start.
            // Allow non-1 only if single partial check is desired — for full streams require 1.
            return Err(EnvelopeValidationError::InvalidSequence(env.sequence));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_type::EventType;
    use crate::ids::{EventId, ProjectId, SessionId};
    use serde_json::Map;
    use time::OffsetDateTime;

    fn env(seq: u64) -> EventEnvelope {
        EventEnvelope::new(
            EventId::new(),
            seq,
            OffsetDateTime::UNIX_EPOCH,
            ProjectId::new(),
            SessionId::parse("22222222-2222-2222-2222-222222222222").unwrap(),
            None,
            EventType::SessionCreated,
            Map::new(),
        )
    }

    #[test]
    fn rejects_zero_sequence() {
        let e = env(0);
        assert!(matches!(
            validate_envelope(&e),
            Err(EnvelopeValidationError::InvalidSequence(0))
        ));
    }

    #[test]
    fn stream_ok() {
        let events = vec![env(1), env(2), env(3)];
        assert!(validate_session_event_stream(&events).is_ok());
    }

    #[test]
    fn stream_gap_fails() {
        let events = vec![env(1), env(3)];
        assert!(validate_session_event_stream(&events).is_err());
    }
}
