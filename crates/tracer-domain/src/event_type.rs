//! Event type catalog (stable dotted strings).

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Known event types from `TRACER_EVENT_PROTOCOL_V1.md` §3.
///
/// Unknown types are preserved as [`EventType::Unknown`] so consumers never drop them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    // Runtime process lifecycle
    /// Child process spawned.
    RuntimeProcessStarted,
    /// Initialize + capability negotiation complete.
    RuntimeProcessReady,
    /// Non-empty stderr chunk.
    RuntimeProcessStderr,
    /// Process exit observed.
    RuntimeProcessExited,
    /// Spawn/start failure or unrecoverable process error.
    RuntimeProcessFailed,
    // Session lifecycle
    /// Tracer session record created.
    SessionCreated,
    /// Session may accept prompts.
    SessionReady,
    /// User/control-plane accepted a prompt.
    SessionPromptSubmitted,
    /// High-level status transition.
    SessionStatusChanged,
    /// Agent run finished successfully.
    SessionCompleted,
    /// Terminal failure for session/run.
    SessionFailed,
    /// Cancellation completed or forced.
    SessionCancelled,
    // Agent messaging / planning
    /// Streaming text fragment.
    AgentMessageDelta,
    /// Final message boundary.
    AgentMessageCompleted,
    /// Progress text or percentage.
    AgentProgressDelta,
    /// Structured plan snapshot or patch.
    AgentPlanUpdated,
    // Tools
    /// Tool call started.
    ToolStarted,
    /// Tool call partial update.
    ToolUpdated,
    /// Tool call completed.
    ToolCompleted,
    /// Tool call failed.
    ToolFailed,
    // Approvals
    /// Approval requested (fail closed).
    ApprovalRequested,
    /// Approval resolved.
    ApprovalResolved,
    // Files / terminal / storage / adapter
    /// File changed notification.
    FileChanged,
    /// Diff available.
    FileDiffAvailable,
    /// Terminal output chunk.
    TerminalOutput,
    /// Terminal exited.
    TerminalExited,
    /// Persistence failure.
    StorageError,
    /// Protocol/parse/negotiation error.
    AdapterProtocolError,
    /// Unmapped but accepted runtime notification.
    AdapterProtocolUnknown,
    /// Forward-compatible unknown type string.
    Unknown(String),
}

/// All known (non-Unknown) type wire strings.
pub const KNOWN_EVENT_TYPES: &[&str] = &[
    "runtime.process.started",
    "runtime.process.ready",
    "runtime.process.stderr",
    "runtime.process.exited",
    "runtime.process.failed",
    "session.created",
    "session.ready",
    "session.prompt.submitted",
    "session.status.changed",
    "session.completed",
    "session.failed",
    "session.cancelled",
    "agent.message.delta",
    "agent.message.completed",
    "agent.progress.delta",
    "agent.plan.updated",
    "tool.started",
    "tool.updated",
    "tool.completed",
    "tool.failed",
    "approval.requested",
    "approval.resolved",
    "file.changed",
    "file.diff.available",
    "terminal.output",
    "terminal.exited",
    "storage.error",
    "adapter.protocol.error",
    "adapter.protocol.unknown",
];

impl EventType {
    /// Wire string form.
    pub fn as_str(&self) -> &str {
        match self {
            Self::RuntimeProcessStarted => "runtime.process.started",
            Self::RuntimeProcessReady => "runtime.process.ready",
            Self::RuntimeProcessStderr => "runtime.process.stderr",
            Self::RuntimeProcessExited => "runtime.process.exited",
            Self::RuntimeProcessFailed => "runtime.process.failed",
            Self::SessionCreated => "session.created",
            Self::SessionReady => "session.ready",
            Self::SessionPromptSubmitted => "session.prompt.submitted",
            Self::SessionStatusChanged => "session.status.changed",
            Self::SessionCompleted => "session.completed",
            Self::SessionFailed => "session.failed",
            Self::SessionCancelled => "session.cancelled",
            Self::AgentMessageDelta => "agent.message.delta",
            Self::AgentMessageCompleted => "agent.message.completed",
            Self::AgentProgressDelta => "agent.progress.delta",
            Self::AgentPlanUpdated => "agent.plan.updated",
            Self::ToolStarted => "tool.started",
            Self::ToolUpdated => "tool.updated",
            Self::ToolCompleted => "tool.completed",
            Self::ToolFailed => "tool.failed",
            Self::ApprovalRequested => "approval.requested",
            Self::ApprovalResolved => "approval.resolved",
            Self::FileChanged => "file.changed",
            Self::FileDiffAvailable => "file.diff.available",
            Self::TerminalOutput => "terminal.output",
            Self::TerminalExited => "terminal.exited",
            Self::StorageError => "storage.error",
            Self::AdapterProtocolError => "adapter.protocol.error",
            Self::AdapterProtocolUnknown => "adapter.protocol.unknown",
            Self::Unknown(s) => s.as_str(),
        }
    }

    /// Parse a type string; unknown values become [`EventType::Unknown`].
    pub fn parse(s: &str) -> Self {
        match s {
            "runtime.process.started" => Self::RuntimeProcessStarted,
            "runtime.process.ready" => Self::RuntimeProcessReady,
            "runtime.process.stderr" => Self::RuntimeProcessStderr,
            "runtime.process.exited" => Self::RuntimeProcessExited,
            "runtime.process.failed" => Self::RuntimeProcessFailed,
            "session.created" => Self::SessionCreated,
            "session.ready" => Self::SessionReady,
            "session.prompt.submitted" => Self::SessionPromptSubmitted,
            "session.status.changed" => Self::SessionStatusChanged,
            "session.completed" => Self::SessionCompleted,
            "session.failed" => Self::SessionFailed,
            "session.cancelled" => Self::SessionCancelled,
            "agent.message.delta" => Self::AgentMessageDelta,
            "agent.message.completed" => Self::AgentMessageCompleted,
            "agent.progress.delta" => Self::AgentProgressDelta,
            "agent.plan.updated" => Self::AgentPlanUpdated,
            "tool.started" => Self::ToolStarted,
            "tool.updated" => Self::ToolUpdated,
            "tool.completed" => Self::ToolCompleted,
            "tool.failed" => Self::ToolFailed,
            "approval.requested" => Self::ApprovalRequested,
            "approval.resolved" => Self::ApprovalResolved,
            "file.changed" => Self::FileChanged,
            "file.diff.available" => Self::FileDiffAvailable,
            "terminal.output" => Self::TerminalOutput,
            "terminal.exited" => Self::TerminalExited,
            "storage.error" => Self::StorageError,
            "adapter.protocol.error" => Self::AdapterProtocolError,
            "adapter.protocol.unknown" => Self::AdapterProtocolUnknown,
            other => Self::Unknown(other.to_string()),
        }
    }

    /// Whether this is a catalog-known type.
    pub fn is_known(&self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for EventType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EventType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::parse(&s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_round_trip() {
        for wire in KNOWN_EVENT_TYPES {
            let t = EventType::parse(wire);
            assert!(t.is_known(), "{wire}");
            assert_eq!(t.as_str(), *wire);
            let json = serde_json::to_string(&t).unwrap();
            assert_eq!(json, format!("\"{wire}\""));
            let back: EventType = serde_json::from_str(&json).unwrap();
            assert_eq!(back, t);
        }
    }

    #[test]
    fn unknown_preserved() {
        let t = EventType::parse("vendor.x.ai.custom");
        assert!(!t.is_known());
        assert_eq!(t.as_str(), "vendor.x.ai.custom");
        let json = serde_json::to_string(&t).unwrap();
        let back: EventType = serde_json::from_str(&json).unwrap();
        assert_eq!(back.as_str(), "vendor.x.ai.custom");
    }
}
