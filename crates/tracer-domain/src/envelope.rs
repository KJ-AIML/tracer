//! Versioned Tracer Event Protocol v1 envelope.

use crate::adapter::AdapterMetadata;
use crate::event_type::EventType;
use crate::ids::{AgentRunId, EventId, ProjectId, SessionId};
use crate::severity::Severity;
use crate::EVENT_PROTOCOL_VERSION;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use time::OffsetDateTime;

/// Normalized event envelope (control plane → UI / storage).
///
/// Required fields per `TRACER_EVENT_PROTOCOL_V1.md` §2.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    /// Protocol major version (always `1` for v1).
    pub event_version: u32,
    /// Tracer-owned unique identifier for this event instance.
    pub event_id: EventId,
    /// Monotonic sequence within a Tracer `sessionId` (starts at 1).
    pub sequence: u64,
    /// Control-plane observation time (RFC 3339, UTC preferred).
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
    /// Tracer project identifier.
    pub project_id: ProjectId,
    /// Tracer session identifier.
    pub session_id: SessionId,
    /// Active agent run, or `null` if not applicable.
    pub agent_run_id: Option<AgentRunId>,
    /// Dotted event type (catalog or unknown).
    #[serde(rename = "type")]
    pub event_type: EventType,
    /// Type-specific body (empty object allowed).
    pub payload: Map<String, Value>,
    /// Optional adapter metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<AdapterMetadata>,
    /// Presentation hint (default info when omitted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
    /// Unknown envelope-root fields preserved for forward compatibility.
    #[serde(flatten, default, skip_serializing_if = "Map::is_empty")]
    pub unknown_fields: Map<String, Value>,
}

impl EventEnvelope {
    /// Builder for a v1 envelope with required fields.
    pub fn new(
        event_id: EventId,
        sequence: u64,
        timestamp: OffsetDateTime,
        project_id: ProjectId,
        session_id: SessionId,
        agent_run_id: Option<AgentRunId>,
        event_type: EventType,
        payload: Map<String, Value>,
    ) -> Self {
        Self {
            event_version: EVENT_PROTOCOL_VERSION,
            event_id,
            sequence,
            timestamp,
            project_id,
            session_id,
            agent_run_id,
            event_type,
            payload,
            adapter: None,
            severity: None,
            unknown_fields: Map::new(),
        }
    }

    /// Attach adapter metadata.
    pub fn with_adapter(mut self, adapter: AdapterMetadata) -> Self {
        self.adapter = Some(adapter);
        self
    }

    /// Set severity.
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    /// Serialize to JSON value.
    pub fn to_json_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    /// Deserialize from JSON value (tolerates unknown fields via flatten).
    pub fn from_json_value(value: Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }

    /// Deserialize from JSON string.
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Serialize to JSON string.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Pretty JSON string.
    pub fn to_json_string_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate::validate_envelope;
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use time::format_description::well_known::Rfc3339;

    fn sample_envelope() -> EventEnvelope {
        let ts = OffsetDateTime::parse("2026-07-17T12:00:00.123Z", &Rfc3339).unwrap();
        EventEnvelope::new(
            EventId::parse("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            12,
            ts,
            ProjectId::parse("11111111-1111-1111-1111-111111111111").unwrap(),
            SessionId::parse("22222222-2222-2222-2222-222222222222").unwrap(),
            Some(AgentRunId::parse("33333333-3333-3333-3333-333333333333").unwrap()),
            EventType::AgentMessageDelta,
            json!({
                "messageId": "44444444-4444-4444-4444-444444444444",
                "role": "assistant",
                "delta": "Inspecting the repository layout…",
                "contentType": "text/plain"
            })
            .as_object()
            .unwrap()
            .clone(),
        )
        .with_severity(Severity::Info)
        .with_adapter(AdapterMetadata {
            runtime_kind: Some("acp-stdio".into()),
            runtime_session_id: Some("rt-sess-abc".into()),
            raw_ref: Some("optional-opaque-or-truncated-ref".into()),
            ..Default::default()
        })
    }

    #[test]
    fn protocol_example_round_trip() {
        let env = sample_envelope();
        validate_envelope(&env).unwrap();
        let json = env.to_json_string().unwrap();
        let back = EventEnvelope::from_json_str(&json).unwrap();
        assert_eq!(env.event_version, 1);
        assert_eq!(env.sequence, back.sequence);
        assert_eq!(env.event_type, back.event_type);
        assert_eq!(env.payload, back.payload);
        assert_eq!(
            env.adapter.as_ref().unwrap().runtime_kind,
            back.adapter.as_ref().unwrap().runtime_kind
        );
    }

    #[test]
    fn unknown_type_and_fields_preserved() {
        let raw = json!({
            "eventVersion": 1,
            "eventId": "550e8400-e29b-41d4-a716-446655440000",
            "sequence": 1,
            "timestamp": "2026-07-17T12:00:00Z",
            "projectId": "11111111-1111-1111-1111-111111111111",
            "sessionId": "22222222-2222-2222-2222-222222222222",
            "agentRunId": null,
            "type": "vendor.future.event",
            "payload": { "x": 1, "futureField": true },
            "futureEnvelopeField": "keep-me",
            "adapter": {
                "runtimeKind": "acp-stdio",
                "extensions": {
                    "x.ai/method": "x.ai/custom_notify"
                }
            }
        });
        let env = EventEnvelope::from_json_value(raw).unwrap();
        assert!(!env.event_type.is_known());
        assert_eq!(env.event_type.as_str(), "vendor.future.event");
        assert_eq!(env.payload.get("futureField"), Some(&json!(true)));
        assert_eq!(
            env.unknown_fields.get("futureEnvelopeField"),
            Some(&json!("keep-me"))
        );
        let out = env.to_json_value().unwrap();
        assert_eq!(out["futureEnvelopeField"], "keep-me");
        assert_eq!(out["type"], "vendor.future.event");
        assert_eq!(
            out["adapter"]["extensions"]["x.ai/method"],
            "x.ai/custom_notify"
        );
    }

    #[test]
    fn missing_required_field_fails_deserialize() {
        let raw = json!({
            "eventVersion": 1,
            "eventId": "550e8400-e29b-41d4-a716-446655440000",
            "sequence": 1,
            "timestamp": "2026-07-17T12:00:00Z",
            "projectId": "11111111-1111-1111-1111-111111111111",
            // missing sessionId
            "agentRunId": null,
            "type": "session.created",
            "payload": {}
        });
        assert!(EventEnvelope::from_json_value(raw).is_err());
    }
}
