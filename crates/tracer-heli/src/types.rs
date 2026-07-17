//! Serde representations of HeliHarness concurrency state files.
//!
//! These mirror on-disk JSON from heli-harness ≥0.5.24 concurrent mode.
//! Fields unknown to Tracer are preserved via optional / defaulted members
//! so minor harness schema growth does not break the adapter.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Workspace concurrency mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMode {
    /// Single shared current-task.md style.
    #[default]
    Legacy,
    /// Multi-task concurrent leases and sessions.
    Concurrent,
}

impl WorkspaceMode {
    /// Parse harness schema `mode` string.
    pub fn parse(raw: &str) -> Self {
        if raw.eq_ignore_ascii_case("concurrent") {
            Self::Concurrent
        } else {
            Self::Legacy
        }
    }
}

/// `.heli-harness/workspace/schema.json`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSchemaFile {
    /// Schema version number.
    #[serde(default = "one")]
    pub schema_version: u32,
    /// `legacy` or `concurrent`.
    #[serde(default)]
    pub mode: String,
    /// Last update timestamp (ISO-8601), if present.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Repo entry in `workspace/index.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepoEntry {
    /// Logical repository id (e.g. `tracer`).
    pub name: String,
    /// Path relative to workspace root.
    pub path: String,
    /// Git root relative path.
    #[serde(default)]
    pub git_root: Option<String>,
    /// Profile name if any.
    #[serde(default)]
    pub profile: Option<String>,
    /// Whether this is the default target.
    #[serde(default)]
    pub default_target: bool,
    /// Free-form notes.
    #[serde(default)]
    pub notes: Option<String>,
}

/// `.heli-harness/workspace/index.json`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceIndexFile {
    /// Schema version.
    #[serde(default = "one")]
    pub schema_version: u32,
    /// Workspace root marker (often `.`).
    #[serde(default)]
    pub workspace_root: Option<String>,
    /// Registered repositories.
    #[serde(default)]
    pub repos: Vec<RepoEntry>,
}

/// `.heli-harness/workspace/target.json` (advisory in concurrent mode).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetFile {
    /// Schema version.
    #[serde(default = "one")]
    pub schema_version: u32,
    /// Selected repository id.
    #[serde(default)]
    pub target_repo: Option<String>,
    /// Git root relative path.
    #[serde(default)]
    pub target_git_root: Option<String>,
    /// Path under which writes are allowed.
    #[serde(default)]
    pub writes_allowed_under: Option<String>,
    /// Active profile name.
    #[serde(default)]
    pub active_profile: Option<String>,
    /// Selection timestamp.
    #[serde(default)]
    pub selected_at: Option<String>,
    /// Who selected the target.
    #[serde(default)]
    pub selected_by: Option<String>,
    /// Free-form reason.
    #[serde(default)]
    pub reason: Option<String>,
    /// Note (e.g. concurrent mode caveat).
    #[serde(default)]
    pub note: Option<String>,
}

/// Path claim sets on a task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PathClaims {
    /// Paths this task exclusively owns.
    #[serde(default)]
    pub owns: Vec<String>,
    /// Paths this task may read.
    #[serde(default)]
    pub reads: Vec<String>,
    /// Paths intentionally shared with other tasks.
    #[serde(default)]
    pub shared: Vec<String>,
    /// Paths this task must not touch.
    #[serde(default)]
    pub forbidden: Vec<String>,
}

/// Task source metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskSource {
    /// Plan path if any.
    #[serde(default)]
    pub plan_path: Option<String>,
    /// Work item key (e.g. `W1-H`).
    #[serde(default)]
    pub work_item_key: Option<String>,
    /// Fingerprint of the work item.
    #[serde(default)]
    pub fingerprint: Option<String>,
}

/// Task target binding metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TaskTarget {
    /// Repository id.
    #[serde(default)]
    pub repository_id: Option<String>,
    /// Repository path.
    #[serde(default)]
    pub repository_path: Option<String>,
    /// Worktree path recorded on the task (may lag live lease).
    #[serde(default)]
    pub worktree_path: Option<String>,
    /// Branch name.
    #[serde(default)]
    pub branch: Option<String>,
    /// Base SHA.
    #[serde(default)]
    pub base_sha: Option<String>,
    /// Head SHA.
    #[serde(default)]
    pub head_sha: Option<String>,
}

/// YOLO flag block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct YoloFlag {
    /// Whether yolo is enabled.
    #[serde(default)]
    pub enabled: bool,
}

/// `tasks/<id>/task.json`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskRecord {
    /// Schema version.
    #[serde(default = "one")]
    pub schema_version: u32,
    /// Task id.
    pub task_id: String,
    /// Human title.
    #[serde(default)]
    pub title: Option<String>,
    /// Task status string (`active`, `complete`, …).
    #[serde(default)]
    pub status: Option<String>,
    /// Program id if any.
    #[serde(default)]
    pub program_id: Option<String>,
    /// Parent task id if any.
    #[serde(default)]
    pub parent_task_id: Option<String>,
    /// Source metadata.
    #[serde(default)]
    pub source: TaskSource,
    /// Target metadata.
    #[serde(default)]
    pub target: TaskTarget,
    /// Task mode (`strict`, …).
    #[serde(default)]
    pub mode: Option<String>,
    /// Revision counter.
    #[serde(default)]
    pub revision: Option<u32>,
    /// Path claims.
    #[serde(default)]
    pub path_claims: PathClaims,
    /// YOLO flags.
    #[serde(default)]
    pub yolo: YoloFlag,
    /// Created at ISO timestamp.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Updated at ISO timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,
}

impl TaskRecord {
    /// True when the task is still considered active by harness conventions.
    pub fn is_active(&self) -> bool {
        match self
            .status
            .as_deref()
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            None | Some("") => false,
            Some("complete") | Some("closed") | Some("abandoned") | Some("cancelled") => false,
            Some(_) => true,
        }
    }
}

/// Session mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionMode {
    /// Write lease holder.
    Write,
    /// Reviewer.
    Review,
    /// Observer.
    Observe,
}

impl SessionMode {
    /// Parse a mode string; unknown values map to observe.
    pub fn parse(raw: &str) -> Self {
        match raw.to_ascii_lowercase().as_str() {
            "write" => Self::Write,
            "review" => Self::Review,
            _ => Self::Observe,
        }
    }
}

/// `sessions/<id>.json`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRecord {
    /// Schema version.
    #[serde(default = "one")]
    pub schema_version: u32,
    /// Session id.
    pub session_id: String,
    /// External host session id if any.
    #[serde(default)]
    pub external_host_session_id: Option<String>,
    /// Host agent name (`grok-build`, `grok`, …).
    #[serde(default)]
    pub host: Option<String>,
    /// Bound task id.
    #[serde(default)]
    pub task_id: Option<String>,
    /// Session mode string.
    #[serde(default)]
    pub mode: Option<String>,
    /// Worktree path for this session.
    #[serde(default)]
    pub worktree_path: Option<String>,
    /// Session status (`active`, `closed`, …).
    #[serde(default)]
    pub status: Option<String>,
    /// YOLO flags.
    #[serde(default)]
    pub yolo: YoloFlag,
    /// Created at.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Last seen at.
    #[serde(default)]
    pub last_seen_at: Option<String>,
    /// Closed at.
    #[serde(default)]
    pub closed_at: Option<String>,
}

impl SessionRecord {
    /// True when status is active.
    pub fn is_active(&self) -> bool {
        self.status.as_deref() == Some("active")
    }

    /// Parsed session mode.
    pub fn session_mode(&self) -> SessionMode {
        SessionMode::parse(self.mode.as_deref().unwrap_or("observe"))
    }
}

/// Write lease (`locks/tasks/<id>.write.lock/lease.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaseRecord {
    /// Schema version.
    #[serde(default = "one")]
    pub schema_version: u32,
    /// Lease id.
    pub lease_id: String,
    /// Task id.
    pub task_id: String,
    /// Owning session id.
    pub session_id: String,
    /// Lease mode (normally `write`).
    #[serde(default)]
    pub mode: Option<String>,
    /// Worktree bound to this lease.
    #[serde(default)]
    pub worktree_path: Option<String>,
    /// Acquired at.
    #[serde(default)]
    pub acquired_at: Option<String>,
    /// Last activity at.
    #[serde(default)]
    pub last_activity_at: Option<String>,
    /// Expires at (required for validity).
    pub expires_at: String,
    /// TTL seconds.
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
    /// Revision.
    #[serde(default)]
    pub revision: Option<u32>,
}

/// Host binding entry inside a worktree binding file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostBinding {
    /// Session id for this host.
    pub session_id: String,
    /// Mode if known.
    #[serde(default)]
    pub mode: Option<String>,
    /// Updated at.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Task-to-worktree binding (`bindings/worktrees/<hash>.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeBinding {
    /// Schema version.
    #[serde(default = "one")]
    pub schema_version: u32,
    /// Canonical worktree path.
    pub canonical_worktree_path: String,
    /// Host → session bindings.
    #[serde(default)]
    pub host_bindings: BTreeMap<String, HostBinding>,
    /// Task id bound to this worktree.
    #[serde(default)]
    pub task_id: Option<String>,
    /// Default session id.
    #[serde(default)]
    pub default_session_id: Option<String>,
    /// Updated at.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Source used to resolve a task's live worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorktreeSource {
    /// Active write lease.
    WriteLease,
    /// Writer session record.
    WriterSession,
    /// Any active session for the task.
    ActiveSession,
    /// Stale (expired) lease.
    StaleLease,
    /// Worktree binding file.
    Binding,
    /// Task metadata worktreePath.
    TaskMetadata,
    /// Unknown / unresolved.
    Unknown,
}

impl WorktreeSource {
    /// Stable string for reports (matches heli CLI wording).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WriteLease => "write-lease",
            Self::WriterSession => "writer-session",
            Self::ActiveSession => "active-session",
            Self::StaleLease => "stale-lease",
            Self::Binding => "binding",
            Self::TaskMetadata => "task-metadata",
            Self::Unknown => "unknown",
        }
    }
}

/// Lease lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaseState {
    /// No lease file.
    None,
    /// Active unexpired lease.
    Active,
    /// Expired lease still on disk.
    Stale,
    /// Unreadable / missing required fields.
    Malformed,
}

/// Manifest subset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ManifestInfo {
    /// Harness version string.
    #[serde(default)]
    pub version: Option<String>,
    /// Package name.
    #[serde(default)]
    pub name: Option<String>,
}

fn one() -> u32 {
    1
}
