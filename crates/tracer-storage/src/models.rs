//! Durable record types for the Tracer storage layer.
//!
//! Shapes follow Gate 0 contracts:
//! - projects / sessions: `TAURI_COMMAND_CONTRACT_V1`
//! - events: `TRACER_EVENT_PROTOCOL_V1` envelope
//! - session status vocabulary: vertical slice §7 (canonical in `tracer-domain`)

use crate::ids::{
    AgentRunId, ApprovalId, ArtifactId, EventId, ProcessId, ProjectId, SessionId, TracerId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// Canonical session/severity vocabulary from W1-B domain crate.
pub use tracer_domain::{SessionStatus, Severity};

/// Project path health as observed by control plane / storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Ready,
    Missing,
    Invalid,
}

impl ProjectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Missing => "missing",
            Self::Invalid => "invalid",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "ready" => Some(Self::Ready),
            "missing" => Some(Self::Missing),
            "invalid" => Some(Self::Invalid),
            _ => None,
        }
    }
}

/// Storage-oriented helpers on the domain [`SessionStatus`].
pub trait SessionStatusStorageExt {
    /// Statuses that imply a live runtime process may exist.
    fn implies_live_process(self) -> bool;
    /// Terminal statuses (delegates to domain `is_terminal`).
    fn is_terminal_status(self) -> bool;
}

impl SessionStatusStorageExt for SessionStatus {
    fn implies_live_process(self) -> bool {
        matches!(
            self,
            Self::Creating
                | Self::StartingRuntime
                | Self::Ready
                | Self::Running
                | Self::AwaitingApproval
                | Self::Cancelling
        )
    }

    fn is_terminal_status(self) -> bool {
        tracer_domain::is_terminal(self)
    }
}

/// Parse helpers for domain [`Severity`] (DB row mapping).
pub trait SeverityStorageExt {
    /// Parse wire string from SQLite column.
    fn parse(s: &str) -> Option<Severity>;
}

impl SeverityStorageExt for Severity {
    fn parse(s: &str) -> Option<Severity> {
        match s {
            "info" => Some(Severity::Info),
            "warn" => Some(Severity::Warn),
            "error" => Some(Severity::Error),
            _ => None,
        }
    }
}

/// Durable project record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRecord {
    pub project_id: ProjectId,
    pub name: String,
    /// Absolute path on the local machine (runtime data only; never commit fixtures with real homes).
    pub root_path: String,
    pub status: ProjectStatus,
    pub is_git: bool,
    pub last_opened_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Durable session record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    pub session_id: SessionId,
    pub project_id: ProjectId,
    pub title: Option<String>,
    pub status: SessionStatus,
    pub runtime_kind: Option<String>,
    pub runtime_session_id: Option<String>,
    pub capabilities: Option<JsonValue>,
    pub last_error: Option<JsonValue>,
    pub active_agent_run_id: Option<AgentRunId>,
    /// Next sequence number that will be assigned (starts at 1).
    pub next_sequence: i64,
    pub created_at: String,
    pub updated_at: String,
}

/// Normalized event envelope as stored and returned to consumers.
///
/// Unknown `event_type` values and unknown payload fields are preserved via
/// `payload` / `envelope_json` / free-form `event_type` string.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventRecord {
    pub event_version: u32,
    pub event_id: EventId,
    pub sequence: i64,
    pub timestamp: String,
    pub project_id: ProjectId,
    pub session_id: SessionId,
    pub agent_run_id: Option<AgentRunId>,
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: JsonValue,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter: Option<JsonValue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
}

impl EventRecord {
    /// Build the full envelope JSON object (protocol shape).
    pub fn to_envelope_json(&self) -> JsonValue {
        let mut map = serde_json::Map::new();
        map.insert("eventVersion".into(), JsonValue::from(self.event_version));
        map.insert("eventId".into(), JsonValue::from(self.event_id.as_str()));
        map.insert("sequence".into(), JsonValue::from(self.sequence));
        map.insert("timestamp".into(), JsonValue::from(self.timestamp.clone()));
        map.insert(
            "projectId".into(),
            JsonValue::from(self.project_id.as_str()),
        );
        map.insert(
            "sessionId".into(),
            JsonValue::from(self.session_id.as_str()),
        );
        map.insert(
            "agentRunId".into(),
            match &self.agent_run_id {
                Some(id) => JsonValue::from(id.as_str()),
                None => JsonValue::Null,
            },
        );
        map.insert("type".into(), JsonValue::from(self.event_type.clone()));
        map.insert("payload".into(), self.payload.clone());
        if let Some(adapter) = &self.adapter {
            map.insert("adapter".into(), adapter.clone());
        }
        if let Some(sev) = self.severity {
            map.insert("severity".into(), JsonValue::from(sev.as_str()));
        }
        JsonValue::Object(map)
    }

    /// Parse from a protocol envelope JSON object.
    pub fn from_envelope_json(value: &JsonValue) -> Result<Self, String> {
        let obj = value
            .as_object()
            .ok_or_else(|| "envelope must be an object".to_string())?;

        let event_version = obj
            .get("eventVersion")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| "missing eventVersion".to_string())? as u32;

        let event_id = obj
            .get("eventId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing eventId".to_string())
            .and_then(|s| EventId::parse(s).map_err(|e| e.to_string()))?;

        let sequence = obj
            .get("sequence")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| "missing sequence".to_string())?;

        let timestamp = obj
            .get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing timestamp".to_string())?
            .to_string();

        let project_id = obj
            .get("projectId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing projectId".to_string())
            .and_then(|s| ProjectId::parse(s).map_err(|e| e.to_string()))?;

        let session_id = obj
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing sessionId".to_string())
            .and_then(|s| SessionId::parse(s).map_err(|e| e.to_string()))?;

        let agent_run_id = match obj.get("agentRunId") {
            None | Some(JsonValue::Null) => None,
            Some(v) => {
                let s = v
                    .as_str()
                    .ok_or_else(|| "agentRunId must be string or null".to_string())?;
                Some(AgentRunId::parse(s).map_err(|e| e.to_string())?)
            }
        };

        let event_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing type".to_string())?
            .to_string();

        let payload = obj
            .get("payload")
            .cloned()
            .unwrap_or_else(|| JsonValue::Object(Default::default()));

        let adapter = match obj.get("adapter") {
            None | Some(JsonValue::Null) => None,
            Some(v) => Some(v.clone()),
        };

        let severity = match obj.get("severity") {
            None | Some(JsonValue::Null) => None,
            Some(v) => {
                let s = v
                    .as_str()
                    .ok_or_else(|| "severity must be string".to_string())?;
                Some(Severity::parse(s).ok_or_else(|| format!("unknown severity `{s}`"))?)
            }
        };

        Ok(Self {
            event_version,
            event_id,
            sequence,
            timestamp,
            project_id,
            session_id,
            agent_run_id,
            event_type,
            payload,
            adapter,
            severity,
        })
    }
}

/// Runtime process summary for diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProcessRecord {
    pub process_id: ProcessId,
    pub session_id: SessionId,
    pub pid: Option<i64>,
    pub executable: Option<String>,
    pub args: Option<JsonValue>,
    pub cwd: Option<String>,
    pub status: RuntimeProcessStatus,
    pub exit_code: Option<i64>,
    pub exit_signal: Option<String>,
    pub started_at: String,
    pub ended_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProcessStatus {
    Starting,
    Running,
    Exited,
    Failed,
}

impl RuntimeProcessStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Exited => "exited",
            Self::Failed => "failed",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "starting" => Some(Self::Starting),
            "running" => Some(Self::Running),
            "exited" => Some(Self::Exited),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    pub fn is_live(self) -> bool {
        matches!(self, Self::Starting | Self::Running)
    }
}

/// Approval decision audit row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApprovalDecisionRecord {
    pub approval_id: ApprovalId,
    pub session_id: SessionId,
    pub event_id: Option<EventId>,
    pub decision: ApprovalDecision,
    pub decided_at: String,
    pub details: Option<JsonValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Allow,
    Deny,
    AllowAlways,
    DenyAlways,
}

impl ApprovalDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
            Self::AllowAlways => "allow_always",
            Self::DenyAlways => "deny_always",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "allow" => Some(Self::Allow),
            "deny" => Some(Self::Deny),
            "allow_always" => Some(Self::AllowAlways),
            "deny_always" => Some(Self::DenyAlways),
            _ => None,
        }
    }
}

/// Basic artifact record (file-change summary etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactRecord {
    pub artifact_id: ArtifactId,
    pub session_id: SessionId,
    pub project_id: ProjectId,
    pub kind: String,
    pub path: Option<String>,
    pub summary: Option<String>,
    pub metadata: Option<JsonValue>,
    pub created_at: String,
}

/// Result of listing events (`tracer_events_list` shape).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventList {
    pub events: Vec<EventRecord>,
    pub latest_sequence: i64,
}

/// Outcome of boot-time stale session reconciliation (F-S04 / VS-10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReconcileReport {
    pub sessions_examined: usize,
    pub sessions_updated: Vec<SessionId>,
    pub target_status: SessionStatus,
}
