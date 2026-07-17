//! ControlPlane facade — sole SQLite writer + runtime supervision.
//!
//! Components (logical):
//! - RuntimeSupervisor: adapter lifecycle (W1-D only)
//! - SessionCoordinator: Tracer session + prompt lifecycle
//! - EventIngestor: continuous drain of adapter events
//! - PersistenceCoordinator: sole DB writer via tracer-storage
//! - ApprovalCoordinator / CancellationCoordinator
//! - PresentationProjector: typed snapshots for shell
//! - RecoveryCoordinator: restart reconcile

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::json;
use tracer_domain::{AuthenticationState, ErrorClass, SessionStatus, EVENT_PROTOCOL_VERSION};
use tracer_runtime_adapter::{
    fake_acp_spawn_config, ApprovalDecisionRequest, PromptRequest, RuntimeAdapter,
    SessionCreateParams, ShutdownOptions,
};
use tracer_storage::{
    now_rfc3339, open_database, open_in_memory, OpenOptions, ProjectId, ProjectRecord,
    ProjectStatus, SessionId, SessionRecord, SessionStatusStorageExt, SqliteStorage,
};
use tracing::info;

use crate::convert::{error_payload, runtime_observation};
use crate::error::{ControlPlaneError, ControlPlaneResult};
use crate::heli_bridge::probe_heli;
use crate::session_runtime::{
    cancel_with_escalation, resolve_approval_blocking, submit_prompt_blocking, LiveSession,
};
use crate::types::{
    AppInfo, CancelResult, EventsListResult, PresentationEvent, PresentationSnapshot,
    ProjectSummary, RuntimeCreateOptions, RuntimeProcessView, SessionDetail, SessionSummary,
    SubmitPromptResult, SNAPSHOT_VERSION,
};

/// Configuration for a control plane instance.
#[derive(Debug, Clone)]
pub struct ControlPlaneConfig {
    /// Database path; `None` = in-memory (tests).
    pub database_path: Option<PathBuf>,
    /// Default fake runtime script path (repo-relative ok for tests).
    pub fake_js: Option<PathBuf>,
    /// Default node executable.
    pub node_bin: PathBuf,
    /// Heli probe start path.
    pub heli_probe_path: PathBuf,
    /// Whether cancel escalates to process stop on CapabilityUnsupported.
    pub escalate_cancel_to_process_stop: bool,
}

impl Default for ControlPlaneConfig {
    fn default() -> Self {
        Self {
            database_path: None,
            fake_js: None,
            node_bin: PathBuf::from("node"),
            heli_probe_path: PathBuf::from("."),
            escalate_cancel_to_process_stop: true,
        }
    }
}

/// Main control plane handle.
pub struct ControlPlane {
    storage: SqliteStorage,
    config: ControlPlaneConfig,
    /// Live sessions keyed by Tracer session id string.
    sessions: Mutex<HashMap<String, Arc<LiveSession>>>,
    /// Presentation fan-out (optional).
    presentation_tx: Mutex<Option<Sender<PresentationEvent>>>,
    /// Cached presentation snapshot.
    snapshot: Mutex<PresentationSnapshot>,
}

impl ControlPlane {
    /// Open control plane with config (async storage init).
    ///
    /// Prefer a multi-thread Tokio runtime so the async persist pump can run
    /// while `submit_prompt` blocks on a worker thread.
    pub async fn open(config: ControlPlaneConfig) -> ControlPlaneResult<Self> {
        let pool = if let Some(path) = &config.database_path {
            open_database(path, OpenOptions::default()).await?
        } else {
            open_in_memory().await?
        };
        let storage = SqliteStorage::new(pool);

        // Recovery: mark stale live sessions disconnected (F-S04).
        let report = storage
            .reconcile_stale_live_sessions(SessionStatus::Disconnected)
            .await?;
        if !report.sessions_updated.is_empty() {
            info!(
                reconciled = report.sessions_updated.len(),
                examined = report.sessions_examined,
                "reconciled stale live sessions on open"
            );
        }

        let heli = probe_heli(&config.heli_probe_path);
        let snapshot = PresentationSnapshot {
            heli,
            ..PresentationSnapshot::default()
        };

        Ok(Self {
            storage,
            config,
            sessions: Mutex::new(HashMap::new()),
            presentation_tx: Mutex::new(None),
            snapshot: Mutex::new(snapshot),
        })
    }

    /// Storage handle (tests / inspection). Sole writer policy still applies.
    pub fn storage(&self) -> &SqliteStorage {
        &self.storage
    }

    /// Subscribe presentation fan-out (UI / tests).
    pub fn set_presentation_sender(&self, tx: Sender<PresentationEvent>) {
        *self.presentation_tx.lock().expect("tx") = Some(tx);
    }

    /// Current presentation snapshot (versioned; shell-restorable).
    pub fn snapshot(&self) -> PresentationSnapshot {
        self.snapshot.lock().expect("snap").clone()
    }

    /// Refresh Heli read-only status into snapshot (never crashes).
    pub fn refresh_heli(&self) -> crate::types::HeliStatusView {
        let heli = probe_heli(&self.config.heli_probe_path);
        let mut snap = self.snapshot.lock().expect("snap");
        snap.heli = heli.clone();
        heli
    }

    // -------------------------------------------------------------------------
    // App
    // -------------------------------------------------------------------------

    /// `tracer_app_info`
    pub fn app_info(&self) -> AppInfo {
        AppInfo {
            app_version: env!("CARGO_PKG_VERSION").into(),
            event_protocol_version: EVENT_PROTOCOL_VERSION,
            command_contract_version: "1.0.0".into(),
            platform: std::env::consts::OS.into(),
            module: "W1-F".into(),
        }
    }

    // -------------------------------------------------------------------------
    // Projects
    // -------------------------------------------------------------------------

    /// `tracer_project_register`
    pub async fn project_register(
        &self,
        root_path: impl AsRef<Path>,
        name: Option<String>,
    ) -> ControlPlaneResult<ProjectSummary> {
        let root = root_path.as_ref();
        if !root.exists() {
            return Err(ControlPlaneError::not_found(format!(
                "project path missing: {}",
                root.display()
            )));
        }
        if !root.is_dir() {
            return Err(ControlPlaneError::invalid_argument(
                "project rootPath must be a directory",
            ));
        }

        let canonical = root
            .canonicalize()
            .unwrap_or_else(|_| root.to_path_buf())
            .display()
            .to_string();

        if let Some(existing) = self.storage.get_by_root_path_opt(&canonical).await? {
            return Ok(project_summary(&existing));
        }

        let now = now_rfc3339();
        let display_name = name.unwrap_or_else(|| {
            root.file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "project".into())
        });
        let is_git = root.join(".git").exists();
        let record = ProjectRecord {
            project_id: ProjectId::new(),
            name: display_name,
            root_path: canonical,
            status: ProjectStatus::Ready,
            is_git,
            last_opened_at: Some(now.clone()),
            created_at: now.clone(),
            updated_at: now,
        };
        self.storage.insert_project(&record).await?;
        Ok(project_summary(&record))
    }

    /// `tracer_project_list`
    pub async fn project_list(&self) -> ControlPlaneResult<Vec<ProjectSummary>> {
        let rows = self.storage.list_projects().await?;
        Ok(rows.iter().map(project_summary).collect())
    }

    /// `tracer_project_get`
    pub async fn project_get(&self, project_id: &str) -> ControlPlaneResult<ProjectSummary> {
        let id = ProjectId::parse(project_id)
            .map_err(|_| ControlPlaneError::invalid_argument("invalid projectId"))?;
        let row = self.storage.get_project(&id).await?;
        Ok(project_summary(&row))
    }

    // -------------------------------------------------------------------------
    // Sessions
    // -------------------------------------------------------------------------

    /// `tracer_session_list`
    pub async fn session_list(
        &self,
        project_id: &str,
        limit: i64,
    ) -> ControlPlaneResult<Vec<SessionSummary>> {
        let id = ProjectId::parse(project_id)
            .map_err(|_| ControlPlaneError::invalid_argument("invalid projectId"))?;
        let rows = self.storage.list_sessions(&id, limit).await?;
        Ok(rows.iter().map(session_summary).collect())
    }

    /// `tracer_session_create` — spawn fake/stock runtime, initialize, create session.
    pub async fn session_create(
        &self,
        project_id: &str,
        title: Option<String>,
        runtime: RuntimeCreateOptions,
    ) -> ControlPlaneResult<SessionDetail> {
        let pid = ProjectId::parse(project_id)
            .map_err(|_| ControlPlaneError::invalid_argument("invalid projectId"))?;
        let project = self.storage.get_project(&pid).await?;

        let session_id = SessionId::new();
        let now = now_rfc3339();
        let record = SessionRecord {
            session_id,
            project_id: pid,
            title,
            status: SessionStatus::StartingRuntime,
            runtime_kind: Some(runtime.runtime_kind.clone()),
            runtime_session_id: None,
            capabilities: None,
            last_error: None,
            active_agent_run_id: None,
            next_sequence: 1,
            created_at: now.clone(),
            updated_at: now,
        };
        self.storage.insert_session(&record).await?;

        // Build spawn spec (prefer fake ACP for CI).
        let cwd = PathBuf::from(&project.root_path);
        let scenario = runtime
            .scenario_id
            .clone()
            .unwrap_or_else(|| "happy_prompt_stream".into());
        let fake_js = runtime
            .fake_js
            .as_ref()
            .map(PathBuf::from)
            .or_else(|| self.config.fake_js.clone())
            .ok_or_else(|| {
                ControlPlaneError::invalid_argument(
                    "fake_js path required for acp-stdio fake runtime (tests/CI)",
                )
            })?;
        let node = runtime
            .executable_override
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.config.node_bin.clone());

        // Storage ids are domain id re-exports — pass through directly.
        let domain_project = pid;
        let domain_session = session_id;

        let start_result = tokio::task::spawn_blocking({
            let fake_js = fake_js.clone();
            let node = node.clone();
            let scenario = scenario.clone();
            let cwd = cwd.clone();
            move || {
                let spec = fake_acp_spawn_config(node, fake_js, scenario.as_str(), cwd);
                RuntimeAdapter::start(spec, domain_project, domain_session)
            }
        })
        .await
        .map_err(|e| ControlPlaneError::internal(format!("spawn join: {e}")))?;

        let adapter = match start_result {
            Ok(a) => Arc::new(a),
            Err(e) => {
                let err = ControlPlaneError::from_adapter(&e);
                let _ = self
                    .storage
                    .update_session_status(&session_id, SessionStatus::Failed)
                    .await;
                let mut rec = self.storage.get_session(&session_id).await?;
                rec.last_error = Some(error_payload(e.error_class.as_str(), &e.message));
                rec.status = SessionStatus::Failed;
                let _ = self.storage.update_session(&rec).await;
                return Err(err);
            }
        };

        let live = Arc::new(LiveSession::new(session_id, pid, Arc::clone(&adapter)));
        {
            let mut st = live.state.lock().expect("state");
            st.status = SessionStatus::StartingRuntime;
        }

        // Start continuous ingest BEFORE initialize/session so no events are lost.
        let fanout = self.presentation_tx.lock().expect("tx").clone();
        live.start_ingestor(self.storage.clone(), fanout);

        // initialize
        let init = tokio::task::spawn_blocking({
            let adapter = Arc::clone(&adapter);
            move || adapter.initialize()
        })
        .await
        .map_err(|e| ControlPlaneError::internal(format!("init join: {e}")))?;

        let caps = match init {
            Ok(c) => c,
            Err(e) => {
                live.stop_ingestor();
                let _ = adapter.shutdown(ShutdownOptions::default());
                let _ = self
                    .storage
                    .update_session_status(&session_id, SessionStatus::Failed)
                    .await;
                return Err(ControlPlaneError::from_adapter(&e));
            }
        };

        let caps_json = serde_json::to_value(&caps).unwrap_or(json!({}));
        {
            let mut st = live.state.lock().expect("state");
            st.capabilities = Some(caps_json.clone());
            st.auth_state = adapter.auth_state();
        }

        // create_session (may fail auth)
        let create = tokio::task::spawn_blocking({
            let adapter = Arc::clone(&adapter);
            let cwd_s = cwd.display().to_string();
            move || {
                adapter.create_session(SessionCreateParams {
                    cwd: cwd_s,
                    model_hints: None,
                })
            }
        })
        .await
        .map_err(|e| ControlPlaneError::internal(format!("session join: {e}")))?;

        match create {
            Ok(runtime_sid) => {
                {
                    let mut st = live.state.lock().expect("state");
                    st.runtime_session_id = Some(runtime_sid.clone());
                    st.status = SessionStatus::Ready;
                    st.auth_state = adapter.auth_state();
                }

                let mut rec = self.storage.get_session(&session_id).await?;
                rec.status = SessionStatus::Ready;
                rec.runtime_session_id = Some(runtime_sid);
                rec.capabilities = Some(caps_json);
                // Ingest pump may already have advanced next_sequence.
                self.update_session_preserving_sequence(rec).await?;

                {
                    self.sessions
                        .lock()
                        .expect("sessions")
                        .insert(session_id.to_string(), Arc::clone(&live));
                }

                self.refresh_snapshot_for(&live);
                // Brief drain so session.ready is persisted for tests.
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok(self.session_detail_from_live(&live).await?)
            }
            Err(e) => {
                // Auth required / other session create failure: keep process, no session-ready.
                let status = {
                    let mut st = live.state.lock().expect("state");
                    st.auth_state = adapter.auth_state();
                    st.last_error = Some(error_payload(e.error_class.as_str(), &e.message));
                    if e.error_class == ErrorClass::AuthenticationRequired {
                        st.status = SessionStatus::StartingRuntime; // process ready, not session ready
                    } else {
                        // AuthenticationFailed and other create failures share Failed.
                        st.status = SessionStatus::Failed;
                    }
                    st.status
                };

                let mut rec = self.storage.get_session(&session_id).await?;
                rec.status = status;
                rec.last_error = Some(error_payload(e.error_class.as_str(), &e.message));
                rec.capabilities = Some(caps_json);
                self.update_session_preserving_sequence(rec).await?;

                // Keep live session for inspect/auth paths when process still useful.
                {
                    self.sessions
                        .lock()
                        .expect("sessions")
                        .insert(session_id.to_string(), Arc::clone(&live));
                }
                self.refresh_snapshot_for(&live);
                tokio::time::sleep(Duration::from_millis(100)).await;
                Err(ControlPlaneError::from_adapter(&e))
            }
        }
    }

    /// `tracer_session_get`
    pub async fn session_get(&self, session_id: &str) -> ControlPlaneResult<SessionDetail> {
        let sid = SessionId::parse(session_id)
            .map_err(|_| ControlPlaneError::invalid_argument("invalid sessionId"))?;
        // Drop the sessions mutex before any await (Send bound for Tauri).
        let live_opt = {
            self.sessions
                .lock()
                .expect("sessions")
                .get(session_id)
                .cloned()
        };
        if let Some(live) = live_opt {
            return self.session_detail_from_live(&live).await;
        }
        // History-only (restart path)
        let rec = self.storage.get_session(&sid).await?;
        Ok(SessionDetail {
            session_id: rec.session_id.to_string(),
            project_id: rec.project_id.to_string(),
            title: rec.title,
            status: rec.status,
            runtime_kind: rec.runtime_kind,
            runtime_session_id: rec.runtime_session_id,
            capabilities: rec.capabilities,
            last_error: rec.last_error,
            active_agent_run_id: rec.active_agent_run_id.map(|id| id.to_string()),
            created_at: rec.created_at,
            updated_at: rec.updated_at,
            auth_state: AuthenticationState::NotRequired,
            process_alive: false,
            protocol_ready: false,
            session_ready: false,
        })
    }

    /// `tracer_session_submit_prompt` — returns after prompt RPC completes.
    /// Event ingestion continues concurrently on the ingestor thread.
    pub async fn session_submit_prompt(
        &self,
        session_id: &str,
        text: &str,
    ) -> ControlPlaneResult<SubmitPromptResult> {
        if text.trim().is_empty() {
            return Err(ControlPlaneError::invalid_argument("text is required"));
        }
        let live = self.live(session_id)?;
        {
            let st = live.state.lock().expect("state");
            if st.status != SessionStatus::Ready {
                return Err(ControlPlaneError::invalid_state(format!(
                    "cannot submit prompt while status={}",
                    st.status.as_str()
                )));
            }
            if !st.auth_state.allows_prompt() {
                return Err(ControlPlaneError::from_class(
                    ErrorClass::AuthenticationRequired,
                    "authentication required before prompt",
                ));
            }
            if !live.adapter.is_session_ready() {
                return Err(ControlPlaneError::from_class(
                    ErrorClass::RuntimeNotReady,
                    "session not ready for prompts",
                ));
            }
        }

        let prompt_id = uuid::Uuid::new_v4().to_string();
        let agent_run_id = uuid::Uuid::new_v4().to_string();
        {
            let mut st = live.state.lock().expect("state");
            st.status = SessionStatus::Running;
            st.prompt_in_flight = true;
            st.last_prompt_id = Some(prompt_id.clone());
            st.last_agent_run_id = Some(agent_run_id.clone());
        }
        let _ = self
            .storage
            .update_session_status(
                &SessionId::parse(session_id).unwrap(),
                SessionStatus::Running,
            )
            .await;

        let adapter = Arc::clone(&live.adapter);
        let prompt = PromptRequest {
            prompt_id: Some(prompt_id.clone()),
            text: text.to_string(),
        };

        // Blocking submit on OS thread — cancel/approve/ingest continue.
        // Prefer std::thread over spawn_blocking so the Tokio pool is not saturated
        // while the drain thread uses Handle::block_on for storage.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("cp-submit-prompt".into())
            .spawn(move || {
                let _ = tx.send(submit_prompt_blocking(&adapter, prompt));
            })
            .map_err(|e| ControlPlaneError::internal(format!("prompt spawn: {e}")))?;

        let result = tokio::task::spawn_blocking(move || {
            rx.recv_timeout(Duration::from_secs(90))
                .map_err(|_| {
                    tracer_runtime_adapter::AdapterError::new(
                        ErrorClass::Timeout,
                        "prompt worker channel timeout",
                    )
                })
                .and_then(|r| r)
        })
        .await
        .map_err(|e| ControlPlaneError::internal(format!("prompt join: {e}")))?;

        // Allow ingestor to flush terminal events.
        tokio::time::sleep(Duration::from_millis(250)).await;

        match result {
            Ok(()) => {
                // Drain a moment for terminal events to persist.
                for _ in 0..10 {
                    tokio::time::sleep(Duration::from_millis(30)).await;
                    let done = {
                        let st = live.state.lock().expect("state");
                        !st.prompt_in_flight
                    };
                    if done {
                        break;
                    }
                }
                let persist_failed = {
                    let st = live.state.lock().expect("state");
                    st.persist_failed
                };
                if persist_failed {
                    return Err(ControlPlaneError::from_class(
                        ErrorClass::StorageError,
                        "prompt finished but persistence failed; not claiming complete",
                    ));
                }
                self.refresh_snapshot_for(&live);
                Ok(SubmitPromptResult {
                    prompt_id,
                    agent_run_id,
                    accepted: true,
                })
            }
            Err(e) => {
                {
                    let mut st = live.state.lock().expect("state");
                    st.prompt_in_flight = false;
                    st.last_error = Some(error_payload(e.error_class.as_str(), &e.message));
                    if st.status == SessionStatus::Running {
                        // Map process failures distinctly.
                        match e.error_class {
                            ErrorClass::RuntimeCrashed => st.status = SessionStatus::Failed,
                            ErrorClass::RuntimeDisconnected => {
                                st.status = SessionStatus::Disconnected
                            }
                            ErrorClass::ProtocolParseError | ErrorClass::ProtocolViolation => {
                                st.status = SessionStatus::Failed;
                            }
                            _ => {}
                        }
                    }
                }
                self.refresh_snapshot_for(&live);
                Err(ControlPlaneError::from_adapter(&e))
            }
        }
    }

    /// `tracer_session_cancel` — concurrent with blocking submit; time-bounded.
    pub async fn session_cancel(&self, session_id: &str) -> ControlPlaneResult<CancelResult> {
        let live = self.live(session_id)?;
        {
            let st = live.state.lock().expect("state");
            if matches!(
                st.status,
                SessionStatus::Completed
                    | SessionStatus::Failed
                    | SessionStatus::Disconnected
                    | SessionStatus::Stopped
            ) {
                return Ok(CancelResult {
                    accepted: true,
                    mode: "already_terminal".into(),
                });
            }
            if !matches!(
                st.status,
                SessionStatus::Running
                    | SessionStatus::AwaitingApproval
                    | SessionStatus::Cancelling
            ) {
                // Still attempt cancel if adapter thinks prompt active.
                if !st.prompt_in_flight {
                    return Err(ControlPlaneError::invalid_state(format!(
                        "cannot cancel while status={}",
                        st.status.as_str()
                    )));
                }
            }
        }
        {
            let mut st = live.state.lock().expect("state");
            st.status = SessionStatus::Cancelling;
            // Clear stale actionable approvals on cancel path.
            st.pending_approvals.clear();
        }

        let adapter = Arc::clone(&live.adapter);
        let escalate = self.config.escalate_cancel_to_process_stop;
        let budget =
            tracer_runtime_adapter::PERMISSION_CANCEL_DEADLOCK_BUDGET + Duration::from_secs(5);

        let started = Instant::now();
        let result = tokio::task::spawn_blocking(move || {
            // Time-bounded cancel path for VS-05.
            cancel_with_escalation(&adapter, escalate)
        })
        .await
        .map_err(|e| ControlPlaneError::internal(format!("cancel join: {e}")))?;

        if started.elapsed() > budget + Duration::from_secs(2) {
            // Soft signal — adapter should already be budgeted.
            tracing::warn!(
                elapsed_ms = started.elapsed().as_millis(),
                "cancel exceeded soft budget"
            );
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        match result {
            Ok(mode) => {
                {
                    let mut st = live.state.lock().expect("state");
                    st.prompt_in_flight = false;
                    if mode == "process_stop" || st.status == SessionStatus::Cancelling {
                        st.status = SessionStatus::Stopped;
                    }
                    st.pending_approvals.clear();
                }
                self.refresh_snapshot_for(&live);
                Ok(CancelResult {
                    accepted: true,
                    mode: mode.into(),
                })
            }
            Err(e) => Err(ControlPlaneError::from_adapter(&e)),
        }
    }

    /// `tracer_session_stop`
    pub async fn session_stop(
        &self,
        session_id: &str,
        force: bool,
    ) -> ControlPlaneResult<serde_json::Value> {
        let live = self.live(session_id)?;
        live.stop_ingestor();
        let adapter = Arc::clone(&live.adapter);
        let res = tokio::task::spawn_blocking(move || {
            if force {
                adapter.force_kill()
            } else {
                adapter.shutdown(ShutdownOptions::default())
            }
        })
        .await
        .map_err(|e| ControlPlaneError::internal(format!("stop join: {e}")))?;

        res.map_err(|e| ControlPlaneError::from_adapter(&e))?;

        {
            let mut st = live.state.lock().expect("state");
            st.status = SessionStatus::Stopped;
            st.prompt_in_flight = false;
            st.pending_approvals.clear();
        }
        if let Ok(sid) = SessionId::parse(session_id) {
            let _ = self
                .storage
                .update_session_status(&sid, SessionStatus::Stopped)
                .await;
        }
        self.sessions.lock().expect("sessions").remove(session_id);
        Ok(json!({ "stopped": true }))
    }

    // -------------------------------------------------------------------------
    // Approvals
    // -------------------------------------------------------------------------

    /// `tracer_approval_list_pending`
    pub fn approval_list_pending(
        &self,
        session_id: &str,
    ) -> ControlPlaneResult<Vec<crate::types::PendingApprovalView>> {
        let live = self.live(session_id)?;
        let st = live.state.lock().expect("state");
        Ok(st.pending_approvals.values().cloned().collect())
    }

    /// `tracer_approval_resolve` — never auto-approves; once only.
    pub async fn approval_resolve(
        &self,
        session_id: &str,
        approval_id: &str,
        decision: &str,
        reason: Option<String>,
    ) -> ControlPlaneResult<serde_json::Value> {
        let decision = decision.to_ascii_lowercase();
        if !matches!(decision.as_str(), "allow" | "deny" | "cancel") {
            return Err(ControlPlaneError::invalid_argument(
                "decision must be allow|deny|cancel",
            ));
        }
        let live = self.live(session_id)?;
        {
            let st = live.state.lock().expect("state");
            if !st.pending_approvals.contains_key(approval_id) {
                return Err(ControlPlaneError::from_class(
                    ErrorClass::ApprovalUnknown,
                    format!("unknown or already resolved approvalId={approval_id}"),
                ));
            }
        }

        let adapter = Arc::clone(&live.adapter);
        let aid = approval_id.to_string();
        let dec = decision.clone();
        let result = tokio::task::spawn_blocking(move || {
            resolve_approval_blocking(
                &adapter,
                ApprovalDecisionRequest {
                    approval_id: aid,
                    decision: dec,
                    option_id: None,
                    reason,
                },
            )
        })
        .await
        .map_err(|e| ControlPlaneError::internal(format!("approval join: {e}")))?;

        result.map_err(|e| ControlPlaneError::from_adapter(&e))?;

        // Remove pending (ingestor also handles approval.resolved).
        {
            let mut st = live.state.lock().expect("state");
            st.pending_approvals.remove(approval_id);
        }

        // Persist decision audit row (sole writer).
        if let Ok(sid) = SessionId::parse(session_id) {
            let dec = match decision.as_str() {
                "allow" => tracer_storage::ApprovalDecision::Allow,
                "deny" => tracer_storage::ApprovalDecision::Deny,
                _ => tracer_storage::ApprovalDecision::Deny, // cancel maps to deny audit
            };
            let rec = tracer_storage::ApprovalDecisionRecord {
                approval_id: tracer_storage::ApprovalId::new(),
                session_id: sid,
                event_id: None,
                decision: dec,
                decided_at: now_rfc3339(),
                details: Some(json!({ "wireDecision": decision, "approvalId": approval_id })),
            };
            let _ = self.storage.insert_approval(&rec).await;
        }

        tokio::time::sleep(Duration::from_millis(80)).await;
        self.refresh_snapshot_for(&live);
        Ok(json!({ "resolved": true }))
    }

    // -------------------------------------------------------------------------
    // Events
    // -------------------------------------------------------------------------

    /// `tracer_events_list`
    pub async fn events_list(
        &self,
        session_id: &str,
        after_sequence: i64,
        limit: i64,
    ) -> ControlPlaneResult<EventsListResult> {
        let sid = SessionId::parse(session_id)
            .map_err(|_| ControlPlaneError::invalid_argument("invalid sessionId"))?;
        let list = self
            .storage
            .list_events(&sid, after_sequence, limit)
            .await?;
        let events: Vec<serde_json::Value> =
            list.events.iter().map(|e| e.to_envelope_json()).collect();
        Ok(EventsListResult {
            events,
            latest_sequence: list.latest_sequence,
        })
    }

    // -------------------------------------------------------------------------
    // Runtime / inspect
    // -------------------------------------------------------------------------

    /// `tracer_runtime_status`
    pub fn runtime_status(
        &self,
        session_id: Option<&str>,
    ) -> ControlPlaneResult<Vec<RuntimeProcessView>> {
        let sessions = self.sessions.lock().expect("sessions");
        let mut out = Vec::new();
        for (sid, live) in sessions.iter() {
            if let Some(filter) = session_id {
                if sid != filter {
                    continue;
                }
            }
            let st = live.state.lock().expect("state");
            out.push(RuntimeProcessView {
                session_id: sid.clone(),
                state: st.status.as_str().into(),
                pid: None,
                runtime_kind: "acp-stdio".into(),
                capabilities: st.capabilities.clone(),
                process_alive: live.adapter.is_process_alive(),
                protocol_ready: live.adapter.is_protocol_ready(),
                session_ready: live.adapter.is_session_ready(),
                auth_state: st.auth_state.as_str().into(),
            });
        }
        Ok(out)
    }

    /// Optional authenticate pass-through (distinct AuthenticationFailed).
    pub async fn authenticate(
        &self,
        session_id: &str,
        method_id: Option<String>,
    ) -> ControlPlaneResult<()> {
        let live = self.live(session_id)?;
        let adapter = Arc::clone(&live.adapter);
        let result =
            tokio::task::spawn_blocking(move || adapter.authenticate(method_id.as_deref()))
                .await
                .map_err(|e| ControlPlaneError::internal(format!("auth join: {e}")))?;

        match result {
            Ok(()) => {
                let mut st = live.state.lock().expect("state");
                st.auth_state = AuthenticationState::Authenticated;
                Ok(())
            }
            Err(e) => {
                let mut st = live.state.lock().expect("state");
                st.auth_state = AuthenticationState::Failed;
                st.last_error = Some(error_payload(e.error_class.as_str(), &e.message));
                // Distinct class: AuthenticationFailed ≠ AuthenticationRequired
                Err(ControlPlaneError::from_adapter(&e))
            }
        }
    }

    /// Shutdown all live sessions (app exit).
    pub async fn shutdown_all(&self) -> ControlPlaneResult<()> {
        let keys: Vec<String> = self
            .sessions
            .lock()
            .expect("sessions")
            .keys()
            .cloned()
            .collect();
        for k in keys {
            let _ = self.session_stop(&k, false).await;
        }
        Ok(())
    }

    /// Ingest-path metrics for a live session (soak / diagnostics).
    ///
    /// Returns `None` when the session is not live (history-only / stopped).
    pub fn session_ingest_metrics(
        &self,
        session_id: &str,
    ) -> Option<crate::session_runtime::IngestMetricsSnapshot> {
        self.sessions
            .lock()
            .expect("sessions")
            .get(session_id)
            .map(|live| live.metrics.snapshot())
    }

    // -------------------------------------------------------------------------
    // Internals
    // -------------------------------------------------------------------------

    fn live(&self, session_id: &str) -> ControlPlaneResult<Arc<LiveSession>> {
        self.sessions
            .lock()
            .expect("sessions")
            .get(session_id)
            .cloned()
            .ok_or_else(|| {
                ControlPlaneError::from_class(
                    ErrorClass::SessionNotFound,
                    format!("no live session {session_id}"),
                )
            })
    }

    /// Persist a session row without decreasing `next_sequence`.
    ///
    /// The ingest pump advances `next_sequence` concurrently. A full-row
    /// `update_session` from a stale `get_session` snapshot must never rewind
    /// the sequence counter (that causes UNIQUE (session_id, sequence) failures
    /// and silent event loss under burst — VS1-H3 soak finding).
    async fn update_session_preserving_sequence(
        &self,
        mut rec: SessionRecord,
    ) -> ControlPlaneResult<()> {
        let sid = rec.session_id;
        for _ in 0..8 {
            let current = self.storage.get_session(&sid).await?;
            let events = self.storage.list_events(&sid, 0, 1).await?;
            // next must stay strictly ahead of the highest committed event.
            let min_next = (events.latest_sequence + 1)
                .max(current.next_sequence)
                .max(rec.next_sequence)
                .max(1);
            rec.next_sequence = min_next;
            rec.updated_at = now_rfc3339();
            self.storage.update_session(&rec).await?;

            let after = self.storage.get_session(&sid).await?;
            let events_after = self.storage.list_events(&sid, 0, 1).await?;
            if after.next_sequence > events_after.latest_sequence {
                return Ok(());
            }
            // Pump advanced during write; repair and retry.
            rec = after;
            rec.next_sequence = events_after.latest_sequence + 1;
        }
        Err(ControlPlaneError::internal(
            "failed to update session without clobbering next_sequence",
        ))
    }

    async fn session_detail_from_live(
        &self,
        live: &LiveSession,
    ) -> ControlPlaneResult<SessionDetail> {
        let rec = self.storage.get_session(&live.session_id).await?;
        let st = live.state.lock().expect("state");
        Ok(SessionDetail {
            session_id: live.session_id.to_string(),
            project_id: live.project_id.to_string(),
            title: rec.title,
            status: st.status,
            runtime_kind: rec.runtime_kind,
            runtime_session_id: st.runtime_session_id.clone().or(rec.runtime_session_id),
            capabilities: st.capabilities.clone().or(rec.capabilities),
            last_error: st.last_error.clone().or(rec.last_error),
            active_agent_run_id: st.last_agent_run_id.clone(),
            created_at: rec.created_at,
            updated_at: rec.updated_at,
            auth_state: st.auth_state,
            process_alive: live.adapter.is_process_alive(),
            protocol_ready: live.adapter.is_protocol_ready(),
            session_ready: live.adapter.is_session_ready(),
        })
    }

    fn refresh_snapshot_for(&self, live: &LiveSession) {
        let st = live.state.lock().expect("state");
        let mut snap = self.snapshot.lock().expect("snap");
        snap.version = SNAPSHOT_VERSION;
        snap.active_project_id = Some(live.project_id.to_string());
        snap.active_session_id = Some(live.session_id.to_string());
        snap.session_status = Some(st.status);
        snap.auth_state = st.auth_state;
        snap.pending_approvals = st.pending_approvals.values().cloned().collect();
        snap.last_error = st.last_error.clone();
        snap.capabilities = st.capabilities.clone();
        snap.latest_sequence = st.latest_sequence;
        snap.prompt_in_flight = st.prompt_in_flight;
        snap.runtime_observation = runtime_observation(
            live.adapter.is_process_alive(),
            live.adapter.is_protocol_ready(),
            live.adapter.is_session_ready(),
            st.status,
        );
    }
}

fn project_summary(r: &ProjectRecord) -> ProjectSummary {
    ProjectSummary {
        project_id: r.project_id.to_string(),
        name: r.name.clone(),
        root_path: r.root_path.clone(),
        status: r.status.as_str().into(),
        last_opened_at: r.last_opened_at.clone(),
    }
}

fn session_summary(r: &SessionRecord) -> SessionSummary {
    SessionSummary {
        session_id: r.session_id.to_string(),
        project_id: r.project_id.to_string(),
        title: r.title.clone(),
        status: r.status,
        created_at: r.created_at.clone(),
        updated_at: r.updated_at.clone(),
    }
}

// Helper: storage may not expose get_by_root_path on SqliteStorage inherent — use trait path.
trait ProjectPathExt {
    async fn get_by_root_path_opt(&self, root: &str) -> ControlPlaneResult<Option<ProjectRecord>>;
    async fn update_session(&self, rec: &SessionRecord) -> ControlPlaneResult<()>;
}

impl ProjectPathExt for SqliteStorage {
    async fn get_by_root_path_opt(&self, root: &str) -> ControlPlaneResult<Option<ProjectRecord>> {
        use tracer_storage::ProjectRepository;
        Ok(ProjectRepository::get_by_root_path(self, root).await?)
    }

    async fn update_session(&self, rec: &SessionRecord) -> ControlPlaneResult<()> {
        use tracer_storage::SessionRepository;
        SessionRepository::update(self, rec).await?;
        Ok(())
    }
}

// silence unused import if SessionStatusStorageExt only used in docs
#[allow(dead_code)]
fn _status_ext(s: SessionStatus) -> bool {
    s.implies_live_process()
}
