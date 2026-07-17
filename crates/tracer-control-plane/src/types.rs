//! Command DTOs and presentation snapshots (no raw ACP).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracer_domain::{AuthenticationState, SessionStatus};

/// Versioned presentation snapshot for the shell.
///
/// Shell can restore from this if live events were missed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationSnapshot {
    /// Snapshot schema version.
    pub version: u32,
    /// Active project id if any.
    pub active_project_id: Option<String>,
    /// Active session id if any.
    pub active_session_id: Option<String>,
    /// Projected session status for the active session.
    pub session_status: Option<SessionStatus>,
    /// Runtime observation string for UI pills (mapped from gates).
    pub runtime_observation: String,
    /// Auth state for banners (orthogonal to session status).
    pub auth_state: AuthenticationState,
    /// Pending approvals (fail-closed; never auto-filled as allowed).
    pub pending_approvals: Vec<PendingApprovalView>,
    /// Heli status summary (read-only; missing is not fatal).
    pub heli: HeliStatusView,
    /// Last structured error for the active session, if any.
    pub last_error: Option<Value>,
    /// Capabilities from initialize (normalized JSON), if any.
    pub capabilities: Option<Value>,
    /// Highest persisted sequence for active session (0 if none).
    pub latest_sequence: i64,
    /// Whether a prompt is currently in flight (adapter blocking).
    pub prompt_in_flight: bool,
}

impl Default for PresentationSnapshot {
    fn default() -> Self {
        Self {
            version: SNAPSHOT_VERSION,
            active_project_id: None,
            active_session_id: None,
            session_status: None,
            runtime_observation: "unknown".into(),
            auth_state: AuthenticationState::NotRequired,
            pending_approvals: Vec::new(),
            heli: HeliStatusView::unavailable("not probed"),
            last_error: None,
            capabilities: None,
            latest_sequence: 0,
            prompt_in_flight: false,
        }
    }
}

/// Presentation snapshot schema version.
pub const SNAPSHOT_VERSION: u32 = 1;

/// Pending approval view (typed; no raw ACP).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingApprovalView {
    pub approval_id: String,
    pub session_id: String,
    pub action: String,
    pub description: String,
    pub risk: String,
    pub created_at: String,
}

/// Read-only Heli projection for the shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeliStatusView {
    /// Whether a Heli workspace was found above the probe path.
    pub available: bool,
    /// Workspace root when available.
    pub workspace_root: Option<String>,
    /// Mode string when available.
    pub mode: Option<String>,
    /// Human summary (never panics path).
    pub summary: String,
    /// Non-fatal warnings.
    pub warnings: Vec<String>,
}

impl HeliStatusView {
    /// Build an unavailable view (must not crash the app).
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            available: false,
            workspace_root: None,
            mode: None,
            summary: reason.into(),
            warnings: Vec::new(),
        }
    }
}

/// Runtime spawn options for session create (CI uses fake ACP).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCreateOptions {
    /// `acp-stdio` (default).
    #[serde(default = "default_runtime_kind")]
    pub runtime_kind: String,
    /// Scenario id for fake ACP (tests / vertical slice).
    pub scenario_id: Option<String>,
    /// Executable override (node / grok).
    pub executable_override: Option<String>,
    /// Extra args (ignored for fake scenario path when scenario_id set).
    #[serde(default)]
    pub extra_args: Vec<String>,
    /// Path to fake-acp-runtime.js (tests).
    pub fake_js: Option<String>,
}

fn default_runtime_kind() -> String {
    "acp-stdio".into()
}

impl Default for RuntimeCreateOptions {
    fn default() -> Self {
        Self {
            runtime_kind: default_runtime_kind(),
            scenario_id: None,
            executable_override: None,
            extra_args: Vec::new(),
            fake_js: None,
        }
    }
}

/// Project summary (command surface).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub project_id: String,
    pub name: String,
    pub root_path: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_opened_at: Option<String>,
}

/// Session summary (command surface).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub session_id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub status: SessionStatus,
    pub created_at: String,
    pub updated_at: String,
}

/// Session detail.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetail {
    pub session_id: String,
    pub project_id: String,
    pub title: Option<String>,
    pub status: SessionStatus,
    pub runtime_kind: Option<String>,
    pub runtime_session_id: Option<String>,
    pub capabilities: Option<Value>,
    pub last_error: Option<Value>,
    pub active_agent_run_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub auth_state: AuthenticationState,
    pub process_alive: bool,
    pub protocol_ready: bool,
    pub session_ready: bool,
}

/// App info result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub app_version: String,
    pub event_protocol_version: u32,
    pub command_contract_version: String,
    pub platform: String,
    pub module: String,
}

/// Submit prompt result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitPromptResult {
    pub prompt_id: String,
    pub agent_run_id: String,
    pub accepted: bool,
}

/// Cancel result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelResult {
    pub accepted: bool,
    /// `cooperative` | `process_stop` | `already_terminal`.
    pub mode: String,
}

/// Events list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventsListResult {
    pub events: Vec<Value>,
    pub latest_sequence: i64,
}

/// Runtime process status view.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProcessView {
    pub session_id: String,
    pub state: String,
    pub pid: Option<u32>,
    pub runtime_kind: String,
    pub capabilities: Option<Value>,
    pub process_alive: bool,
    pub protocol_ready: bool,
    pub session_ready: bool,
    pub auth_state: String,
}

/// Fan-out event for UI subscription (`tracer://events`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationEvent {
    /// Single envelope or batch flag.
    #[serde(default)]
    pub batch: bool,
    /// Events in storage sequence order.
    pub events: Vec<Value>,
}
