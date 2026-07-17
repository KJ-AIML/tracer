//! Contract-oriented serde round-trip and ordering tests for Event Protocol v1.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tracer_domain::payload::builders;
use tracer_domain::sequence::{validate_sequence_order, SequenceTracker};
use tracer_domain::validate::{validate_envelope, validate_session_event_stream};
use tracer_domain::{
    AgentRunId, Capabilities, ErrorCategory, ErrorClass, EventEnvelope,
    EventId, EventType, ProjectId, SessionId, SessionStatus, Severity, TracerError,
    EVENT_PROTOCOL_VERSION,
};

fn fixtures_dir() -> PathBuf {
    // crates/tracer-domain/tests -> repo root tests/contract/event-protocol/fixtures
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/contract/event-protocol/fixtures")
}

fn load_fixture(name: &str) -> Value {
    let path = fixtures_dir().join(name);
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!("failed to read fixture {}: {e}", path.display());
    });
    serde_json::from_str(&raw).expect("fixture JSON")
}

fn load_stream(name: &str) -> Vec<EventEnvelope> {
    let v = load_fixture(name);
    let arr = v
        .as_array()
        .expect("stream fixture must be a JSON array");
    arr.iter()
        .map(|item| EventEnvelope::from_json_value(item.clone()).expect("envelope"))
        .collect()
}

#[test]
fn happy_path_stream_round_trip_and_order() {
    let events = load_stream("happy_prompt_stream.json");
    assert!(events.len() >= 6);
    validate_session_event_stream(&events).expect("stream valid");
    let seqs: Vec<u64> = events.iter().map(|e| e.sequence).collect();
    validate_sequence_order(&seqs, 1).unwrap();

    for env in &events {
        validate_envelope(env).unwrap();
        let json = env.to_json_string().unwrap();
        let back = EventEnvelope::from_json_str(&json).unwrap();
        assert_eq!(env.event_version, back.event_version);
        assert_eq!(env.sequence, back.sequence);
        assert_eq!(env.event_type, back.event_type);
        assert_eq!(env.payload, back.payload);
    }

    // Known type presence
    let types: Vec<_> = events.iter().map(|e| e.event_type.as_str().to_string()).collect();
    assert!(types.iter().any(|t| t == "runtime.process.ready"));
    assert!(types.iter().any(|t| t == "session.prompt.submitted"));
    assert!(types
        .iter()
        .any(|t| t == "agent.message.delta" || t == "agent.message.completed"));
}

#[test]
fn unknown_vendor_fixture_preserves_metadata() {
    let env = EventEnvelope::from_json_value(load_fixture("unknown_vendor_notification.json"))
        .expect("envelope");
    validate_envelope(&env).unwrap();
    assert_eq!(env.event_type, EventType::AdapterProtocolUnknown);
    let adapter = env.adapter.as_ref().expect("adapter metadata");
    assert!(
        adapter.extensions.contains_key("x.ai/method")
            || adapter.runtime_method.is_some()
            || adapter.raw_fragment.is_some()
    );
    let out = env.to_json_value().unwrap();
    // round-trip keeps extensions
    if let Some(ext) = out
        .get("adapter")
        .and_then(|a| a.get("extensions"))
    {
        assert!(ext.as_object().map(|o| !o.is_empty()).unwrap_or(false));
    }
}

#[test]
fn malformed_maps_to_protocol_error_fixture() {
    let env = EventEnvelope::from_json_value(load_fixture("protocol_error.json")).unwrap();
    assert_eq!(env.event_type, EventType::AdapterProtocolError);
    assert_eq!(env.severity, Some(Severity::Error));
    let class = env
        .payload
        .get("errorClass")
        .and_then(|v| v.as_str())
        .unwrap();
    assert_eq!(class, "ProtocolParseError");
    assert_eq!(
        ErrorClass::parse(class).unwrap().category(),
        ErrorCategory::Protocol
    );
}

#[test]
fn crash_exit_fixture_order() {
    let events = load_stream("unexpected_process_exit.json");
    validate_session_event_stream(&events).unwrap();
    let types: Vec<_> = events.iter().map(|e| e.event_type.clone()).collect();
    assert!(types
        .iter()
        .any(|t| matches!(t, EventType::RuntimeProcessExited | EventType::RuntimeProcessFailed)));
    assert!(types.iter().any(|t| matches!(
        t,
        EventType::SessionFailed | EventType::SessionStatusChanged
    )));
}

#[test]
fn cancel_mid_tool_fixture() {
    let events = load_stream("cancel_mid_tool.json");
    validate_session_event_stream(&events).unwrap();
    assert!(events
        .iter()
        .any(|e| e.event_type == EventType::SessionCancelled));
}

#[test]
fn approval_flow_fixture() {
    let events = load_stream("tool_with_approval.json");
    validate_session_event_stream(&events).unwrap();
    assert!(events
        .iter()
        .any(|e| e.event_type == EventType::ApprovalRequested));
    assert!(events
        .iter()
        .any(|e| e.event_type == EventType::ApprovalResolved));
}

#[test]
fn replay_sorted_by_sequence() {
    let mut events = load_stream("happy_prompt_stream.json");
    // Shuffle-like reverse then sort by sequence (simulates storage reload)
    events.reverse();
    events.sort_by_key(|e| e.sequence);
    validate_session_event_stream(&events).unwrap();
    let seqs: Vec<_> = events.iter().map(|e| e.sequence).collect();
    assert_eq!(seqs, (1..=seqs.len() as u64).collect::<Vec<_>>());
}

#[test]
fn reject_missing_required_fields() {
    let incomplete = json!({
        "eventVersion": 1,
        "eventId": "550e8400-e29b-41d4-a716-446655440000",
        "sequence": 1,
        "timestamp": "2026-07-17T12:00:00Z",
        "projectId": "11111111-1111-1111-1111-111111111111",
        "type": "session.created",
        "payload": {}
    });
    assert!(EventEnvelope::from_json_value(incomplete).is_err());
}

#[test]
fn sequence_tracker_assigns_monotonic_ids() {
    let mut tracker = SequenceTracker::new();
    let project = ProjectId::parse("11111111-1111-1111-1111-111111111111").unwrap();
    let session = SessionId::parse("22222222-2222-2222-2222-222222222222").unwrap();
    let ts = OffsetDateTime::parse("2026-07-17T12:00:00Z", &Rfc3339).unwrap();
    let mut events = Vec::new();
    for ty in [
        EventType::SessionCreated,
        EventType::RuntimeProcessStarted,
        EventType::RuntimeProcessReady,
    ] {
        let seq = tracker.next();
        let env = EventEnvelope::new(
            EventId::new(),
            seq,
            ts,
            project,
            session,
            None,
            ty,
            Default::default(),
        );
        validate_envelope(&env).unwrap();
        events.push(env);
    }
    validate_session_event_stream(&events).unwrap();
}

#[test]
fn auth_and_capability_error_categories() {
    let auth = TracerError::new(
        ErrorClass::AuthenticationRequired,
        "Authentication required",
    );
    assert_eq!(auth.category(), ErrorCategory::Authentication);

    let perm = TracerError::new(ErrorClass::PermissionDenied, "Denied by policy");
    assert_eq!(perm.category(), ErrorCategory::Permission);

    let storage = TracerError::new(ErrorClass::StorageError, "disk full");
    assert_eq!(storage.category(), ErrorCategory::Storage);

    let process = TracerError::new(ErrorClass::RuntimeCrashed, "exit 1");
    assert_eq!(process.category(), ErrorCategory::Process);

    let protocol = TracerError::new(ErrorClass::ProtocolParseError, "bad frame");
    assert_eq!(protocol.category(), ErrorCategory::Protocol);
}

#[test]
fn status_payload_builder() {
    let p = builders::status_changed(
        SessionStatus::Running,
        SessionStatus::Cancelling,
        Some("user_cancel"),
    );
    assert_eq!(p["from"], "running");
    assert_eq!(p["to"], "cancelling");
}

#[test]
fn process_ready_payload_with_caps() {
    let caps = Capabilities::all_enabled();
    let p = builders::process_ready(&caps, "acp-negotiated");
    assert_eq!(p["protocolVersion"], "acp-negotiated");
    assert_eq!(p["capabilities"]["promptStreaming"], true);
}

#[test]
fn event_version_constant() {
    assert_eq!(EVENT_PROTOCOL_VERSION, 1);
}

#[test]
fn agent_run_null_allowed() {
    let env = EventEnvelope::new(
        EventId::new(),
        1,
        OffsetDateTime::parse("2026-07-17T12:00:00Z", &Rfc3339).unwrap(),
        ProjectId::new(),
        SessionId::new(),
        None,
        EventType::SessionCreated,
        Default::default(),
    );
    let v = env.to_json_value().unwrap();
    assert!(v.get("agentRunId").unwrap().is_null());
    let _run = AgentRunId::new();
}