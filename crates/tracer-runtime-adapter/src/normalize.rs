//! ACP wire → Tracer Event Protocol envelopes.
//!
//! Control plane (W1-F) is the sole SQLite writer. This adapter assigns
//! observation-time envelopes (eventId/sequence) for the live stream so
//! consumers can subscribe immediately; storage may re-key if required.
//!
//! React never needs raw Grok/ACP parsing — only normalized types.

use serde_json::{json, Map, Value};
use time::OffsetDateTime;
use tracer_acp_client::{JsonRpcNotification, JsonRpcRequest};
use tracer_domain::payload::builders;
use tracer_domain::{
    AdapterMetadata, AgentRunId, Capabilities, ErrorClass, EventEnvelope, EventId, EventType,
    ProjectId, SequenceTracker, SessionId, Severity,
};

/// Builds sequenced envelopes for one Tracer session binding.
#[derive(Debug)]
pub struct EnvelopeBuilder {
    project_id: ProjectId,
    session_id: SessionId,
    agent_run_id: Option<AgentRunId>,
    sequences: SequenceTracker,
    runtime_kind: String,
    runtime_session_id: Option<String>,
}

impl EnvelopeBuilder {
    /// Create a builder for a Tracer session.
    pub fn new(project_id: ProjectId, session_id: SessionId) -> Self {
        Self {
            project_id,
            session_id,
            agent_run_id: None,
            sequences: SequenceTracker::new(),
            runtime_kind: "acp-stdio".into(),
            runtime_session_id: None,
        }
    }

    /// Set active agent run.
    pub fn set_agent_run(&mut self, id: Option<AgentRunId>) {
        self.agent_run_id = id;
    }

    /// Set runtime session id for adapter metadata.
    pub fn set_runtime_session_id(&mut self, id: Option<String>) {
        self.runtime_session_id = id;
    }

    /// Current sequence (last assigned).
    pub fn last_sequence(&self) -> u64 {
        self.sequences.last_allocated()
    }

    fn meta(&self, method: Option<&str>, raw: Option<Value>) -> AdapterMetadata {
        let mut m = AdapterMetadata::kind(self.runtime_kind.clone());
        m.runtime_session_id = self.runtime_session_id.clone();
        m.runtime_method = method.map(|s| s.to_string());
        m.raw_fragment = raw;
        m
    }

    /// Emit an envelope.
    pub fn emit(
        &mut self,
        event_type: EventType,
        payload: Map<String, Value>,
        severity: Option<Severity>,
        method: Option<&str>,
        raw: Option<Value>,
    ) -> EventEnvelope {
        let seq = self.sequences.next();
        let mut env = EventEnvelope::new(
            EventId::new(),
            seq,
            OffsetDateTime::now_utc(),
            self.project_id,
            self.session_id,
            self.agent_run_id,
            event_type,
            payload,
        );
        env.adapter = Some(self.meta(method, raw));
        if let Some(s) = severity {
            env.severity = Some(s);
        }
        env
    }

    /// Convenience for info events.
    pub fn emit_info(
        &mut self,
        event_type: EventType,
        payload: Map<String, Value>,
        method: Option<&str>,
    ) -> EventEnvelope {
        self.emit(event_type, payload, Some(Severity::Info), method, None)
    }
}

/// Extract Tracer [`Capabilities`] from an initialize result.
///
/// Prefers `agentCapabilities._meta["tracer/capabilities"]` (fake runtime),
/// then maps standard ACP agentCapabilities heuristics, preserving unknowns.
pub fn capabilities_from_initialize(result: &Value) -> Capabilities {
    let mut caps = Capabilities::none();

    // JSON Pointer encodes '/' in keys as ~1 (RFC 6901).
    if let Some(tracer_caps) = result
        .pointer("/agentCapabilities/_meta/tracer~1capabilities")
        .and_then(|v| v.as_object())
    {
        if let Ok(parsed) =
            serde_json::from_value::<Capabilities>(Value::Object(tracer_caps.clone()))
        {
            caps = parsed;
            caps.preserve_unknown_from(tracer_caps);
            return caps;
        }
    }
    // Fallback: direct map access (also handles keys with '/')
    if let Some(tracer_caps) = result
        .get("agentCapabilities")
        .and_then(|v| v.get("_meta"))
        .and_then(|v| v.get("tracer/capabilities"))
        .and_then(|v| v.as_object())
    {
        if let Ok(parsed) =
            serde_json::from_value::<Capabilities>(Value::Object(tracer_caps.clone()))
        {
            caps = parsed;
            caps.preserve_unknown_from(tracer_caps);
            return caps;
        }
    }

    // Heuristic mapping from standard ACP shape + Grok meta.
    let agent_caps = result.get("agentCapabilities").and_then(|v| v.as_object());
    if let Some(ac) = agent_caps {
        // Streaming: assume true for full agents unless tracer meta says otherwise
        caps.prompt_streaming = true;
        caps.session_resume = ac
            .get("loadSession")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        caps.tool_calls = true;
        caps.approvals = true;
        caps.plan_updates = true;
        // cancellation: Grok cancelRewind / tracer meta
        if let Some(meta) = result.get("_meta").and_then(|v| v.as_object()) {
            if let Some(cr) = meta.get("cancelRewind").and_then(|v| v.as_bool()) {
                caps.cancellation = cr;
            } else {
                caps.cancellation = true;
            }
        } else {
            caps.cancellation = true;
        }
        // Preserve vendor meta keys
        if let Some(meta) = ac.get("_meta").and_then(|v| v.as_object()) {
            for (k, v) in meta {
                if k.starts_with("x.ai/") || k.starts_with("tracer/") {
                    caps.unknown.insert(k.clone(), v.clone());
                }
            }
        }
        caps.preserve_unknown_from(ac);
    }

    // Individual tracer/* keys from fake runtime
    if let Some(meta) = result
        .pointer("/agentCapabilities/_meta")
        .and_then(|v| v.as_object())
    {
        if let Some(v) = meta.get("tracer/cancellation").and_then(|x| x.as_bool()) {
            caps.cancellation = v;
        }
        if let Some(v) = meta.get("tracer/promptStreaming").and_then(|x| x.as_bool()) {
            caps.prompt_streaming = v;
        }
        if let Some(v) = meta.get("tracer/planUpdates").and_then(|x| x.as_bool()) {
            caps.plan_updates = v;
        }
        if let Some(v) = meta.get("tracer/toolCalls").and_then(|x| x.as_bool()) {
            caps.tool_calls = v;
        }
        if let Some(v) = meta.get("tracer/approvals").and_then(|x| x.as_bool()) {
            caps.approvals = v;
        }
    }

    caps
}

/// Normalize a `session/update` (or other) notification into zero or more events.
pub fn normalize_notification(
    builder: &mut EnvelopeBuilder,
    n: &JsonRpcNotification,
) -> Vec<EventEnvelope> {
    let mut out = Vec::new();
    match n.method.as_str() {
        "session/update" => {
            let update = n
                .params
                .as_ref()
                .and_then(|p| p.get("update"))
                .cloned()
                .unwrap_or(Value::Null);
            let kind = update
                .get("sessionUpdate")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match kind {
                "agent_message_chunk" | "user_message_chunk" => {
                    let text = extract_text_content(&update);
                    let role = if kind.starts_with("user") {
                        "user"
                    } else {
                        "assistant"
                    };
                    let payload = json!({
                        "role": role,
                        "delta": text,
                        "contentType": "text/plain"
                    })
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                    out.push(builder.emit_info(
                        EventType::AgentMessageDelta,
                        payload,
                        Some("session/update"),
                    ));
                }
                "agent_thought_chunk" => {
                    let text = extract_text_content(&update);
                    let payload = json!({
                        "role": "assistant",
                        "delta": text,
                        "contentType": "text/thought"
                    })
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                    out.push(builder.emit_info(
                        EventType::AgentMessageDelta,
                        payload,
                        Some("session/update"),
                    ));
                }
                "tool_call" => {
                    let tool_id = update
                        .get("toolCallId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let title = update.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let kind_s = update
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .unwrap_or("other");
                    let status = update
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending");
                    let payload = json!({
                        "toolCallId": tool_id,
                        "title": title,
                        "kind": kind_s,
                        "status": map_tool_status(status),
                    })
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                    out.push(builder.emit_info(
                        EventType::ToolStarted,
                        payload,
                        Some("session/update"),
                    ));
                }
                "tool_call_update" => {
                    let tool_id = update
                        .get("toolCallId")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let status = update
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("in_progress");
                    let mapped = map_tool_status(status);
                    let event_type = match status {
                        "completed" => EventType::ToolCompleted,
                        "failed" => EventType::ToolFailed,
                        _ => EventType::ToolUpdated,
                    };
                    let payload = json!({
                        "toolCallId": tool_id,
                        "status": mapped,
                    })
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                    let sev = if status == "failed" {
                        Some(Severity::Error)
                    } else {
                        Some(Severity::Info)
                    };
                    out.push(builder.emit(event_type, payload, sev, Some("session/update"), None));
                }
                "plan" => {
                    let payload =
                        json!({ "plan": update.get("entries").cloned().unwrap_or(Value::Null) })
                            .as_object()
                            .cloned()
                            .unwrap_or_default();
                    out.push(builder.emit_info(
                        EventType::AgentPlanUpdated,
                        payload,
                        Some("session/update"),
                    ));
                }
                other => {
                    // Unknown sessionUpdate — preserve, do not crash
                    let payload = builders::protocol_unknown(
                        &format!("unknown sessionUpdate: {other}"),
                        Some("session/update"),
                    );
                    let raw = serde_json::to_value(n).ok();
                    out.push(builder.emit(
                        EventType::AdapterProtocolUnknown,
                        payload,
                        Some(Severity::Warn),
                        Some("session/update"),
                        raw,
                    ));
                }
            }
        }
        method if method.starts_with("x.ai/") || method.starts_with("_x.ai/") => {
            let payload =
                builders::protocol_unknown(&format!("vendor notification: {method}"), Some(method));
            let raw = serde_json::to_value(n).ok();
            out.push(builder.emit(
                EventType::AdapterProtocolUnknown,
                payload,
                Some(Severity::Info),
                Some(method),
                raw,
            ));
        }
        other => {
            let payload =
                builders::protocol_unknown(&format!("unknown notification: {other}"), Some(other));
            let raw = serde_json::to_value(n).ok();
            out.push(builder.emit(
                EventType::AdapterProtocolUnknown,
                payload,
                Some(Severity::Warn),
                Some(other),
                raw,
            ));
        }
    }
    out
}

/// Normalize server reverse-request → approval.requested only (never auto-approve).
pub fn normalize_server_request(
    builder: &mut EnvelopeBuilder,
    req: &JsonRpcRequest,
    approval_id: &str,
) -> Option<EventEnvelope> {
    if req.method != "session/request_permission" {
        // Unknown reverse-request
        let payload = builders::protocol_unknown(
            &format!("unknown server request: {}", req.method),
            Some(&req.method),
        );
        return Some(builder.emit(
            EventType::AdapterProtocolUnknown,
            payload,
            Some(Severity::Warn),
            Some(&req.method),
            serde_json::to_value(req).ok(),
        ));
    }

    let params = req.params.as_ref();
    let tool = params.and_then(|p| p.get("toolCall"));
    let options = params
        .and_then(|p| p.get("options"))
        .cloned()
        .unwrap_or(json!([]));
    let payload = json!({
        "approvalId": approval_id,
        "toolCallId": tool.and_then(|t| t.get("toolCallId")).cloned().unwrap_or(json!(null)),
        "title": tool.and_then(|t| t.get("title")).cloned().unwrap_or(json!("")),
        "kind": tool.and_then(|t| t.get("kind")).cloned().unwrap_or(json!("other")),
        "options": options,
        "runtimeRequestId": req.id.as_key(),
    })
    .as_object()
    .cloned()
    .unwrap_or_default();

    Some(builder.emit_info(
        EventType::ApprovalRequested,
        payload,
        Some("session/request_permission"),
    ))
}

fn extract_text_content(update: &Value) -> String {
    if let Some(content) = update.get("content") {
        if let Some(t) = content.get("text").and_then(|v| v.as_str()) {
            return t.to_string();
        }
        if let Some(t) = content.as_str() {
            return t.to_string();
        }
    }
    String::new()
}

fn map_tool_status(status: &str) -> &'static str {
    match status {
        "pending" => "pending",
        "in_progress" => "running",
        "completed" => "completed",
        "failed" => "failed",
        "cancelled" => "cancelled",
        _ => "running",
    }
}

/// Build approval.resolved payload/event.
pub fn approval_resolved(
    builder: &mut EnvelopeBuilder,
    approval_id: &str,
    decision: &str,
    decided_by: &str,
) -> EventEnvelope {
    let payload = json!({
        "approvalId": approval_id,
        "decision": decision,
        "decidedBy": decided_by,
    })
    .as_object()
    .cloned()
    .unwrap_or_default();
    builder.emit_info(
        EventType::ApprovalResolved,
        payload,
        Some("session/request_permission"),
    )
}

/// Protocol error event.
pub fn protocol_error_event(
    builder: &mut EnvelopeBuilder,
    class: ErrorClass,
    message: &str,
) -> EventEnvelope {
    let payload = builders::protocol_error(class, message);
    builder.emit(
        EventType::AdapterProtocolError,
        payload,
        Some(Severity::Error),
        None,
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracer_acp_client::JsonRpcId;

    #[test]
    fn caps_from_fake_initialize() {
        let result = json!({
            "protocolVersion": 1,
            "agentCapabilities": {
                "_meta": {
                    "tracer/capabilities": {
                        "promptStreaming": true,
                        "cancellation": false,
                        "planUpdates": true,
                        "toolCalls": true,
                        "approvals": true,
                        "fileChangeNotifications": false,
                        "terminalOutput": false,
                        "x.ai/extra": true
                    }
                }
            }
        });
        let caps = capabilities_from_initialize(&result);
        assert!(caps.prompt_streaming);
        assert!(!caps.cancellation);
        assert!(caps.tool_calls);
    }

    #[test]
    fn unknown_vendor_maps() {
        let mut b = EnvelopeBuilder::new(ProjectId::new(), SessionId::new());
        let n = JsonRpcNotification::new(
            "x.ai/unknown_vendor_extension",
            Some(json!({"payload": {"note": "x"}})),
        );
        let events = normalize_notification(&mut b, &n);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::AdapterProtocolUnknown);
    }

    #[test]
    fn permission_request_maps_to_approval_only() {
        let mut b = EnvelopeBuilder::new(ProjectId::new(), SessionId::new());
        let req = JsonRpcRequest::new(
            JsonRpcId::Number(42),
            "session/request_permission",
            Some(json!({
                "sessionId": "s",
                "toolCall": { "toolCallId": "t1", "title": "Edit", "kind": "edit" },
                "options": []
            })),
        );
        let ev = normalize_server_request(&mut b, &req, "appr-1").unwrap();
        assert_eq!(ev.event_type, EventType::ApprovalRequested);
        assert_eq!(
            ev.payload.get("approvalId").and_then(|v| v.as_str()),
            Some("appr-1")
        );
    }
}
