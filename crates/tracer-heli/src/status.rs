//! High-level workspace / task status projection (read-only).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::conflict::{detect_path_claim_conflicts, PathClaimConflict};
use crate::error::HeliError;
use crate::paths::{canonicalize_path, path_to_forward_slash, find_workspace_root, HeliPaths};
use crate::types::{
    LeaseRecord, LeaseState, ManifestInfo, SessionRecord, TargetFile, TaskRecord, WorktreeBinding,
    WorktreeSource, WorkspaceIndexFile, WorkspaceMode, WorkspaceSchemaFile,
};

/// Result of probing for a workspace without failing hard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceProbe {
    /// Workspace found at the given root.
    Found {
        /// Canonical workspace root.
        root: PathBuf,
    },
    /// No `.heli-harness/HARNESS.md` above start.
    Missing {
        /// Search start path.
        start: PathBuf,
    },
}

impl WorkspaceProbe {
    /// Probe upward from `start`.
    pub fn probe(start: impl AsRef<Path>) -> Self {
        let start_buf = start.as_ref().to_path_buf();
        match find_workspace_root(&start_buf) {
            Some(root) => Self::Found { root },
            None => Self::Missing { start: start_buf },
        }
    }

    /// True when a workspace was found.
    pub fn is_found(&self) -> bool {
        matches!(self, Self::Found { .. })
    }
}

/// Aggregated read-only snapshot of a HeliHarness workspace.
#[derive(Debug, Clone)]
pub struct WorkspaceStatus {
    /// Workspace root path.
    pub workspace_root: PathBuf,
    /// Concurrent vs legacy mode.
    pub mode: WorkspaceMode,
    /// Harness version from manifest when present.
    pub harness_version: Option<String>,
    /// Workspace index (repos), if readable.
    pub index: Option<WorkspaceIndexFile>,
    /// Active target file (advisory in concurrent mode).
    pub target: Option<TargetFile>,
    /// All task records that parsed successfully.
    pub tasks: Vec<TaskRecord>,
    /// All session records that parsed successfully.
    pub sessions: Vec<SessionRecord>,
    /// All worktree bindings that parsed successfully.
    pub bindings: Vec<WorktreeBinding>,
    /// Per-task status projections.
    pub task_views: Vec<TaskStatusView>,
    /// Path-claim conflicts among active tasks.
    pub conflicts: Vec<PathClaimConflict>,
    /// Non-fatal parse/read warnings.
    pub warnings: Vec<String>,
}

/// Projected status for one task (mirrors heli status fields).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskStatusView {
    /// Task id.
    pub task_id: String,
    /// Task status string.
    pub status: String,
    /// Task mode.
    pub mode: String,
    /// Writer session id or `"none"`.
    pub writer: String,
    /// Resolved worktree path or `"unknown"`.
    pub worktree: String,
    /// How the worktree was resolved.
    pub worktree_source: WorktreeSource,
    /// Repository id.
    pub repo: String,
    /// Branch if known.
    pub branch: String,
    /// Lease lifecycle state.
    pub lease_state: LeaseState,
    /// Lease expiry ISO timestamp when present.
    pub lease_expires_at: Option<String>,
    /// Active lease record when usable.
    pub lease: Option<LeaseRecord>,
    /// Reviewer session count.
    pub reviewer_count: usize,
    /// Observer session count.
    pub observer_count: usize,
    /// Projection warnings (e.g. metadata/lease worktree mismatch).
    pub warnings: Vec<String>,
}

/// Load a full workspace status snapshot starting from any path under the workspace.
pub fn load_workspace_status(start: impl AsRef<Path>) -> Result<WorkspaceStatus, HeliError> {
    let paths = HeliPaths::discover(start)?;
    load_workspace_status_from_paths(&paths)
}

/// Safe variant: returns `Ok(None)` when no workspace is found instead of erroring.
pub fn try_load_workspace_status(
    start: impl AsRef<Path>,
) -> Result<Option<WorkspaceStatus>, HeliError> {
    match load_workspace_status(start) {
        Ok(s) => Ok(Some(s)),
        Err(e) if e.is_workspace_not_found() => Ok(None),
        Err(e) => Err(e),
    }
}

fn load_workspace_status_from_paths(paths: &HeliPaths) -> Result<WorkspaceStatus, HeliError> {
    let mut warnings = Vec::new();

    let mode = match read_json_optional::<WorkspaceSchemaFile>(&paths.schema_path())? {
        Some(s) => WorkspaceMode::parse(&s.mode),
        None => WorkspaceMode::Legacy,
    };

    let manifest = read_json_optional::<ManifestInfo>(&paths.manifest_path())?;
    let harness_version = manifest.and_then(|m| m.version);

    let index = match read_json_optional::<WorkspaceIndexFile>(&paths.index_path())? {
        Some(i) => Some(i),
        None => {
            if paths.index_path().exists() {
                warnings.push(format!(
                    "failed to interpret workspace index at {}",
                    paths.index_path().display()
                ));
            }
            None
        }
    };

    let target = read_json_optional::<TargetFile>(&paths.target_path())?;

    let tasks = read_all_tasks(paths, &mut warnings)?;
    let sessions = read_all_sessions(paths, &mut warnings)?;
    let bindings = read_all_bindings(paths, &mut warnings)?;

    let now_ms = system_time_ms();
    let mut task_views = Vec::with_capacity(tasks.len());
    for task in &tasks {
        let lease = read_lease(paths, &task.task_id, &mut warnings)?;
        task_views.push(project_task(
            task,
            lease,
            &sessions,
            &bindings,
            now_ms,
        ));
    }

    let conflicts = detect_path_claim_conflicts(&tasks, None);

    Ok(WorkspaceStatus {
        workspace_root: paths.workspace_root.clone(),
        mode,
        harness_version,
        index,
        target,
        tasks,
        sessions,
        bindings,
        task_views,
        conflicts,
        warnings,
    })
}

fn read_all_tasks(
    paths: &HeliPaths,
    warnings: &mut Vec<String>,
) -> Result<Vec<TaskRecord>, HeliError> {
    let dir = paths.tasks_dir();
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|source| HeliError::Io {
        path: dir.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| HeliError::Io {
            path: dir.clone(),
            source,
        })?;
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let task_json = entry.path().join("task.json");
        if !task_json.is_file() {
            continue;
        }
        match read_json::<TaskRecord>(&task_json) {
            Ok(t) => out.push(t),
            Err(e) => warnings.push(format!("skip task {}: {e}", task_json.display())),
        }
    }
    out.sort_by(|a, b| a.task_id.cmp(&b.task_id));
    Ok(out)
}

fn read_all_sessions(
    paths: &HeliPaths,
    warnings: &mut Vec<String>,
) -> Result<Vec<SessionRecord>, HeliError> {
    let dir = paths.sessions_dir();
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|source| HeliError::Io {
        path: dir.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| HeliError::Io {
            path: dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match read_json::<SessionRecord>(&path) {
            Ok(s) => out.push(s),
            Err(e) => warnings.push(format!("skip session {}: {e}", path.display())),
        }
    }
    out.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    Ok(out)
}

fn read_all_bindings(
    paths: &HeliPaths,
    warnings: &mut Vec<String>,
) -> Result<Vec<WorktreeBinding>, HeliError> {
    let dir = paths.bindings_dir();
    if !dir.is_dir() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).map_err(|source| HeliError::Io {
        path: dir.clone(),
        source,
    })? {
        let entry = entry.map_err(|source| HeliError::Io {
            path: dir.clone(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        match read_json::<WorktreeBinding>(&path) {
            Ok(b) => out.push(b),
            Err(e) => warnings.push(format!("skip binding {}: {e}", path.display())),
        }
    }
    out.sort_by(|a, b| a.canonical_worktree_path.cmp(&b.canonical_worktree_path));
    Ok(out)
}

fn read_lease(
    paths: &HeliPaths,
    task_id: &str,
    warnings: &mut Vec<String>,
) -> Result<Option<LeaseOutcome>, HeliError> {
    let path = paths.lease_json(task_id);
    if !path.is_file() {
        return Ok(None);
    }
    let value: serde_json::Value = match read_json(&path) {
        Ok(v) => v,
        Err(e) => {
            warnings.push(format!("malformed lease for {task_id}: {e}"));
            return Ok(Some(LeaseOutcome::Malformed {
                reason: e.to_string(),
            }));
        }
    };
    let session_id = value.get("sessionId").and_then(|v| v.as_str());
    let lease_id = value.get("leaseId").and_then(|v| v.as_str());
    let tid = value.get("taskId").and_then(|v| v.as_str());
    let expires_at = value.get("expiresAt").and_then(|v| v.as_str());
    if session_id.is_none() || lease_id.is_none() || tid.is_none() || expires_at.is_none() {
        let reason =
            "lease.json missing required fields (sessionId, leaseId, taskId, expiresAt)".into();
        warnings.push(format!("malformed lease for {task_id}: {reason}"));
        return Ok(Some(LeaseOutcome::Malformed { reason }));
    }
    match serde_json::from_value::<LeaseRecord>(value) {
        Ok(lease) => Ok(Some(LeaseOutcome::Ok(lease))),
        Err(e) => {
            let reason = e.to_string();
            warnings.push(format!("malformed lease for {task_id}: {reason}"));
            Ok(Some(LeaseOutcome::Malformed { reason }))
        }
    }
}

enum LeaseOutcome {
    Ok(LeaseRecord),
    Malformed {
        #[allow(dead_code)]
        reason: String,
    },
}

fn project_task(
    task: &TaskRecord,
    lease_outcome: Option<LeaseOutcome>,
    sessions: &[SessionRecord],
    bindings: &[WorktreeBinding],
    now_ms: u128,
) -> TaskStatusView {
    let mut warnings = Vec::new();
    let active_sessions: Vec<&SessionRecord> = sessions
        .iter()
        .filter(|s| s.is_active() && s.task_id.as_deref() == Some(task.task_id.as_str()))
        .collect();

    let mut worktree = String::new();
    let mut source = WorktreeSource::Unknown;
    let mut writer = "none".to_string();
    let mut lease_state = LeaseState::None;
    let mut lease_expires_at = None;
    let mut lease_record = None;

    match lease_outcome {
        Some(LeaseOutcome::Malformed { reason: _ }) => {
            lease_state = LeaseState::Malformed;
            warnings.push(format!("malformed lease for {}", task.task_id));
        }
        Some(LeaseOutcome::Ok(lease)) => {
            let expired = is_lease_expired(&lease.expires_at, now_ms);
            if expired {
                lease_state = LeaseState::Stale;
                writer = lease.session_id.clone();
                lease_expires_at = Some(lease.expires_at.clone());
                if let Some(ref wt) = lease.worktree_path {
                    if !wt.is_empty() {
                        worktree = normalize_worktree(wt);
                        source = WorktreeSource::StaleLease;
                        warnings.push("lease is stale — worktree shown from expired lease".into());
                    }
                }
                lease_record = Some(lease);
            } else {
                lease_state = LeaseState::Active;
                writer = lease.session_id.clone();
                lease_expires_at = Some(lease.expires_at.clone());
                if let Some(ref wt) = lease.worktree_path {
                    if !wt.is_empty() {
                        worktree = normalize_worktree(wt);
                        source = WorktreeSource::WriteLease;
                    }
                }
                // Cross-check writer session worktree
                if let Some(ws) = sessions.iter().find(|s| s.session_id == lease.session_id) {
                    if let Some(ref swt) = ws.worktree_path {
                        let sess_wt = normalize_worktree(swt);
                        if !worktree.is_empty() && !sess_wt.is_empty() && worktree != sess_wt {
                            warnings.push(format!(
                                "writer session worktree ({sess_wt}) differs from lease worktree ({worktree})"
                            ));
                        }
                        if worktree.is_empty() && !sess_wt.is_empty() {
                            worktree = sess_wt;
                            source = WorktreeSource::WriterSession;
                        }
                    }
                }
                lease_record = Some(lease);
            }
        }
        None => {}
    }

    let reviewers = active_sessions
        .iter()
        .filter(|s| matches!(s.session_mode(), crate::types::SessionMode::Review))
        .count();
    let observers = active_sessions
        .iter()
        .filter(|s| matches!(s.session_mode(), crate::types::SessionMode::Observe))
        .count();
    let writer_sessions: Vec<&&SessionRecord> = active_sessions
        .iter()
        .filter(|s| matches!(s.session_mode(), crate::types::SessionMode::Write))
        .collect();

    if worktree.is_empty() {
        let fallback: Option<&SessionRecord> = writer_sessions
            .first()
            .map(|s| **s)
            .or_else(|| {
                active_sessions
                    .iter()
                    .find(|s| s.session_id == writer)
                    .copied()
            });
        if let Some(ws) = fallback {
            if let Some(ref wt) = ws.worktree_path {
                if !wt.is_empty() {
                    worktree = normalize_worktree(wt);
                    if source == WorktreeSource::Unknown {
                        source = WorktreeSource::ActiveSession;
                    }
                }
            }
        }
    }

    if worktree.is_empty() {
        if let Some(b) = bindings.iter().find(|b| b.task_id.as_deref() == Some(&task.task_id)) {
            if !b.canonical_worktree_path.is_empty() {
                worktree = normalize_worktree(&b.canonical_worktree_path);
                source = WorktreeSource::Binding;
            }
        }
    }

    let meta_wt = task
        .target
        .worktree_path
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(normalize_worktree);

    if worktree.is_empty() {
        if let Some(ref m) = meta_wt {
            worktree = m.clone();
            source = WorktreeSource::TaskMetadata;
        }
    } else if let Some(ref m) = meta_wt {
        if worktree != *m && lease_state == LeaseState::Active {
            warnings.push(format!(
                "task metadata worktree ({m}) differs from live {} ({worktree})",
                source.as_str()
            ));
        }
    }

    TaskStatusView {
        task_id: task.task_id.clone(),
        status: task.status.clone().unwrap_or_default(),
        mode: task.mode.clone().unwrap_or_else(|| "strict".into()),
        writer,
        worktree: if worktree.is_empty() {
            "unknown".into()
        } else {
            worktree
        },
        worktree_source: source,
        repo: task.target.repository_id.clone().unwrap_or_default(),
        branch: task.target.branch.clone().unwrap_or_default(),
        lease_state,
        lease_expires_at,
        lease: lease_record,
        reviewer_count: reviewers,
        observer_count: observers,
        warnings,
    }
}

fn normalize_worktree(wt: &str) -> String {
    // Do not require the path to exist; still normalize separators / case.
    let p = PathBuf::from(wt);
    if p.exists() {
        path_to_forward_slash(canonicalize_path(&p))
    } else {
        let mut s = wt.replace('\\', "/");
        if cfg!(windows) {
            s = s.to_lowercase();
        }
        if s.len() > 3 && s.ends_with('/') {
            s.pop();
        }
        s
    }
}

fn is_lease_expired(expires_at: &str, now_ms: u128) -> bool {
    parse_iso_ms(expires_at)
        .map(|exp| now_ms > exp)
        .unwrap_or(false)
}

fn parse_iso_ms(s: &str) -> Option<u128> {
    // Accept RFC3339 / ISO-8601 with Z.
    // Prefer `time` crate parsing.
    let odt = time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339).ok()?;
    let millis = odt.unix_timestamp_nanos() / 1_000_000;
    if millis < 0 {
        return None;
    }
    Some(millis as u128)
}

fn system_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, HeliError> {
    let text = std::fs::read_to_string(path).map_err(|source| HeliError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&text).map_err(|source| HeliError::InvalidJson {
        path: path.to_path_buf(),
        source,
    })
}

fn read_json_optional<T: serde::de::DeserializeOwned>(
    path: &Path,
) -> Result<Option<T>, HeliError> {
    if !path.is_file() {
        return Ok(None);
    }
    match read_json::<T>(path) {
        Ok(v) => Ok(Some(v)),
        Err(HeliError::InvalidJson { .. }) => Ok(None),
        Err(e) => Err(e),
    }
}

impl WorkspaceStatus {
    /// Active tasks only.
    pub fn active_tasks(&self) -> impl Iterator<Item = &TaskRecord> {
        self.tasks.iter().filter(|t| t.is_active())
    }

    /// Find a task view by id.
    pub fn task_view(&self, task_id: &str) -> Option<&TaskStatusView> {
        self.task_views.iter().find(|t| t.task_id == task_id)
    }

    /// Binding for a task id, if any.
    pub fn binding_for_task(&self, task_id: &str) -> Option<&WorktreeBinding> {
        self.bindings
            .iter()
            .find(|b| b.task_id.as_deref() == Some(task_id))
    }

    /// Default target repo name from index or target file.
    pub fn default_target_repo(&self) -> Option<&str> {
        if let Some(t) = &self.target {
            if let Some(r) = t.target_repo.as_deref() {
                if !r.is_empty() {
                    return Some(r);
                }
            }
        }
        self.index.as_ref().and_then(|idx| {
            idx.repos
                .iter()
                .find(|r| r.default_target)
                .map(|r| r.name.as_str())
                .or_else(|| idx.repos.first().map(|r| r.name.as_str()))
        })
    }
}
