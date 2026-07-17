//! Envelope ↔ storage record conversion.
//!
//! # Sequence policy
//!
//! Adapter sequences are observation-only (monotonic per adapter start for stream
//! consumers). **Storage sequences are authoritative** and assigned by
//! `SqliteStorage::append_event` when `sequence == 0`. Presentation and history
//! reload always use storage sequence order.

use serde_json::{json, Map, Value};
use time::format_description::well_known::Rfc3339;
use tracer_domain::{EventEnvelope, EventType, SessionStatus};
use tracer_storage::EventRecord;

/// Convert a domain envelope into a storage record ready for append.
///
/// - `sequence = 0` so storage assigns the next monotonic value (authoritative).
/// - `event_id` is **newly generated** for storage (sole-writer authority). Adapter
///   observation ids are preserved under `adapter.rawRef` when present so live-stream
///   correlation remains possible without PK collisions on redelivery.
pub fn envelope_to_event_record(env: &EventEnvelope) -> EventRecord {
    let timestamp = env
        .timestamp
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into());

    let payload = Value::Object(env.payload.clone());
    let mut adapter = env
        .adapter
        .as_ref()
        .and_then(|a| serde_json::to_value(a).ok())
        .unwrap_or_else(|| Value::Object(Default::default()));
    // Preserve adapter observation event id without using it as the storage PK.
    if let Some(obj) = adapter.as_object_mut() {
        obj.entry("adapterEventId".to_string())
            .or_insert_with(|| Value::String(env.event_id.to_string()));
        obj.entry("adapterSequence".to_string())
            .or_insert_with(|| Value::from(env.sequence));
    }

    EventRecord {
        event_version: env.event_version,
        event_id: tracer_storage::EventId::new(),
        sequence: 0, // storage assigns (authoritative)
        timestamp,
        project_id: env.project_id,
        session_id: env.session_id,
        agent_run_id: env.agent_run_id,
        event_type: env.event_type.as_str().to_string(),
        payload,
        adapter: Some(adapter),
        severity: env.severity,
    }
}

/// Session status inferred from a normalized event type (best-effort).
pub fn status_hint_from_event_type(event_type: &str) -> Option<SessionStatus> {
    match event_type {
        "session.created" => Some(SessionStatus::Creating),
        "runtime.process.started" => Some(SessionStatus::StartingRuntime),
        "session.ready" => Some(SessionStatus::Ready),
        "session.prompt.submitted" => Some(SessionStatus::Running),
        "approval.requested" => Some(SessionStatus::AwaitingApproval),
        "session.completed" => Some(SessionStatus::Ready), // run complete → accept next prompt
        "session.cancelled" => Some(SessionStatus::Stopped),
        "session.failed" => Some(SessionStatus::Failed),
        "runtime.process.exited" => None, // inspect payload expected flag
        "runtime.process.failed" => Some(SessionStatus::Failed),
        "adapter.protocol.error" => None,
        _ => None,
    }
}

/// Whether this event type implies a terminal failure for the active run.
pub fn is_terminal_failure_event(event_type: &str) -> bool {
    matches!(
        event_type,
        "session.failed" | "runtime.process.failed" | "adapter.protocol.error"
    )
}

/// Extract approval id from an approval.requested payload.
pub fn approval_id_from_payload(payload: &Value) -> Option<String> {
    payload
        .get("approvalId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Build a pending approval view from envelope payload.
pub fn pending_from_payload(
    session_id: &str,
    payload: &Value,
    created_at: &str,
) -> Option<crate::types::PendingApprovalView> {
    let approval_id = approval_id_from_payload(payload)?;
    Some(crate::types::PendingApprovalView {
        approval_id,
        session_id: session_id.to_string(),
        action: payload
            .get("action")
            .or_else(|| payload.get("kind"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        description: payload
            .get("description")
            .or_else(|| payload.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        risk: payload
            .get("risk")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        created_at: created_at.to_string(),
    })
}

/// Map event type string to known [`EventType`] or Unknown.
pub fn parse_event_type(s: &str) -> EventType {
    EventType::parse(s)
}

/// Last-error JSON from an adapter/storage failure.
pub fn error_payload(class: &str, message: &str) -> Value {
    json!({
        "errorClass": class,
        "message": message,
    })
}

/// Runtime observation string for UI (presentation only; not raw ACP).
pub fn runtime_observation(
    process_alive: bool,
    protocol_ready: bool,
    session_ready: bool,
    status: SessionStatus,
) -> String {
    use SessionStatus::*;
    if matches!(status, Failed) {
        return "failed".into();
    }
    if matches!(status, Disconnected) {
        return "disconnected".into();
    }
    if matches!(status, Stopped | Completed) {
        return "stopped".into();
    }
    if matches!(status, Cancelling) {
        return "cancelling".into();
    }
    if matches!(status, AwaitingApproval) {
        return "awaiting_approval".into();
    }
    if matches!(status, Running) {
        return "running".into();
    }
    if session_ready {
        return "ready".into();
    }
    if protocol_ready {
        return "protocol_ready".into();
    }
    if process_alive {
        return "starting".into();
    }
    "unknown".into()
}

/// Merge unknown vendor payload fields without requiring UI parse.
pub fn preserve_unknown_payload(payload: Map<String, Value>) -> Value {
    Value::Object(payload)
}
