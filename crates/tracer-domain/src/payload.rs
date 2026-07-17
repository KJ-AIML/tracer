//! Common payload helpers and typed fragments for catalog events.

use crate::capabilities::Capabilities;
use crate::error::ErrorClass;
use crate::session::SessionStatus;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Helpers to build JSON-safe payloads without depending on ACP types.
pub mod builders {
    use super::*;
    use serde_json::json;

    /// `runtime.process.ready` payload.
    pub fn process_ready(
        capabilities: &Capabilities,
        protocol_version: &str,
    ) -> Map<String, Value> {
        json!({
            "capabilities": capabilities,
            "protocolVersion": protocol_version,
        })
        .as_object()
        .cloned()
        .unwrap_or_default()
    }

    /// `runtime.process.exited` payload.
    pub fn process_exited(
        exit_code: Option<i32>,
        signal: Option<&str>,
        expected: bool,
        message: Option<&str>,
    ) -> Map<String, Value> {
        let mut m = json!({
            "exitCode": exit_code,
            "signal": signal,
            "expected": expected,
        });
        if let Some(msg) = message {
            m.as_object_mut()
                .unwrap()
                .insert("message".into(), json!(msg));
        }
        m.as_object().cloned().unwrap_or_default()
    }

    /// `runtime.process.failed` / `session.failed` style error payload.
    pub fn error_payload(
        error_class: ErrorClass,
        message: &str,
        retryable: bool,
    ) -> Map<String, Value> {
        json!({
            "errorClass": error_class.as_str(),
            "message": message,
            "retryable": retryable,
        })
        .as_object()
        .cloned()
        .unwrap_or_default()
    }

    /// `session.status.changed` payload.
    pub fn status_changed(
        from: SessionStatus,
        to: SessionStatus,
        reason: Option<&str>,
    ) -> Map<String, Value> {
        let mut m = json!({
            "from": from.as_str(),
            "to": to.as_str(),
        });
        if let Some(r) = reason {
            m.as_object_mut().unwrap().insert("reason".into(), json!(r));
        }
        m.as_object().cloned().unwrap_or_default()
    }

    /// `adapter.protocol.unknown` summary payload.
    pub fn protocol_unknown(summary: &str, runtime_method: Option<&str>) -> Map<String, Value> {
        let mut m = json!({ "summary": summary });
        if let Some(method) = runtime_method {
            m.as_object_mut()
                .unwrap()
                .insert("runtimeMethod".into(), json!(method));
        }
        m.as_object().cloned().unwrap_or_default()
    }

    /// `adapter.protocol.error` payload.
    pub fn protocol_error(error_class: ErrorClass, message: &str) -> Map<String, Value> {
        error_payload(error_class, message, error_class.typically_retryable())
    }
}

/// Tool status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolStatus {
    /// Queued.
    Pending,
    /// In progress.
    Running,
    /// Finished ok.
    Completed,
    /// Finished with error.
    Failed,
    /// Cancelled.
    Cancelled,
}

/// Approval decision values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Allow the action.
    Allow,
    /// Deny the action.
    Deny,
    /// Cancel the request / run.
    Cancel,
}

/// Who decided an approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecidedBy {
    /// End user.
    User,
    /// Policy engine.
    Policy,
    /// System / control plane.
    System,
}

/// Message role for agent messaging events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    /// User.
    User,
    /// Assistant / agent.
    Assistant,
    /// System.
    System,
}

/// Plan step status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStepStatus {
    /// Not started.
    Pending,
    /// In progress.
    Running,
    /// Done.
    Completed,
    /// Failed.
    Failed,
    /// Skipped.
    Skipped,
}

/// File change kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeKind {
    /// Created.
    Created,
    /// Modified.
    Modified,
    /// Deleted.
    Deleted,
    /// Renamed.
    Renamed,
}
