//! Runtime adapter: process composition + ACP client + event normalization.
//!
//! # Threading model
//!
//! - Background **reader** drains process stdout into NDJSON frames.
//! - Background **writer** serializes stdin writes (process handle is mutexed).
//! - Public methods are synchronous and block on RPC correlation with timeouts.
//! - Normalized events go to an `mpsc` channel (`take_event_receiver` / `poll_event`).
//!
//! # Cancellation safety
//!
//! Shutdown closes stdin (graceful), then force-kills via W1-C. Drop stops the child.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::ChildStdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tracer_acp_client::{
    decode_line, AcpClient, ClientConfig, FrameDecoder, InboundFrame, JsonRpcId, JsonRpcMessage,
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, NdjsonWriter, PendingPermission,
    RequestResult,
};
use tracer_domain::payload::builders;
use tracer_domain::{
    AgentRunId, AuthenticationState, Capabilities, ErrorClass, EventEnvelope, EventType, ProjectId,
    SessionId, Severity,
};
use tracer_process::{ManagedProcess, ProcessEvent, ProcessManager, SpawnConfig, StopPolicy};

use crate::config::{RuntimeKind, RuntimeSpawnSpec};
use crate::error::AdapterError;
use crate::normalize::{
    approval_resolved, capabilities_from_initialize, normalize_notification,
    normalize_server_request, protocol_error_event, EnvelopeBuilder,
};
use crate::readiness::AdapterReadiness;

/// Default timeout for initialize / session RPC.
pub const DEFAULT_RPC_TIMEOUT: Duration = Duration::from_secs(20);
/// Default cancel drain timeout.
pub const DEFAULT_CANCEL_TIMEOUT: Duration = Duration::from_secs(10);
/// Permission-cancel deadlock budget (mandatory risk test).
pub const PERMISSION_CANCEL_DEADLOCK_BUDGET: Duration = Duration::from_secs(5);

/// Stream item for subscribers.
#[derive(Debug, Clone)]
pub enum AdapterEvent {
    /// Normalized Tracer event.
    Event(Box<EventEnvelope>),
    /// Adapter-level error (often also mirrored as an event).
    Error(AdapterError),
}

impl AdapterEvent {
    /// Wrap a normalized envelope.
    pub fn from_envelope(env: EventEnvelope) -> Self {
        Self::Event(Box::new(env))
    }
}

/// Prompt submission.
#[derive(Debug, Clone)]
pub struct PromptRequest {
    /// Tracer prompt id (optional correlation).
    pub prompt_id: Option<String>,
    /// User text.
    pub text: String,
}

/// Approval decision from control plane / user.
#[derive(Debug, Clone)]
pub struct ApprovalDecisionRequest {
    /// Tracer approval id from `approval.requested`.
    pub approval_id: String,
    /// `allow` | `deny` | `cancel`.
    pub decision: String,
    /// Optional option id override (`allow-once`, `reject-once`, …).
    pub option_id: Option<String>,
    /// Optional reason.
    pub reason: Option<String>,
}

/// Session creation parameters.
#[derive(Debug, Clone)]
pub struct SessionCreateParams {
    /// Project cwd for session/new.
    pub cwd: String,
    /// Optional model hints (reserved).
    pub model_hints: Option<Value>,
}

/// Shutdown options.
#[derive(Debug, Clone)]
pub struct ShutdownOptions {
    /// Prefer graceful stdin close.
    pub graceful: bool,
    /// Graceful wait.
    pub graceful_timeout: Duration,
    /// Force wait after kill.
    pub force_timeout: Duration,
}

impl Default for ShutdownOptions {
    fn default() -> Self {
        Self {
            graceful: true,
            graceful_timeout: Duration::from_secs(5),
            force_timeout: Duration::from_secs(3),
        }
    }
}

/// Snapshot of adapter state for inspection.
#[derive(Debug, Clone)]
pub struct RuntimeAdapterState {
    /// Readiness gates.
    pub readiness: AdapterReadiness,
    /// Negotiated capabilities (if protocol ready).
    pub capabilities: Option<Capabilities>,
    /// Runtime kind.
    pub runtime_kind: String,
    /// Runtime session id if any.
    pub runtime_session_id: Option<String>,
    /// Auth methods advertised at initialize.
    pub auth_methods: Vec<Value>,
    /// Whether shutdown was requested.
    pub shutdown_requested: bool,
}

struct SharedInner {
    client: AcpClient,
    envelopes: EnvelopeBuilder,
    capabilities: Option<Capabilities>,
    auth_state: AuthenticationState,
    auth_methods: Vec<Value>,
    approvals: HashMap<String, PendingPermission>,
    approval_by_id: HashMap<String, String>,
    event_tx: Sender<AdapterEvent>,
    rpc_waiters: HashMap<String, Sender<Result<RequestResult, AdapterError>>>,
    shutdown_requested: bool,
    expected_exit: bool,
}

/// Write adapter that forwards to a channel (writer thread owns process stdin).
struct ChannelWriter {
    tx: Sender<Vec<u8>>,
}

impl Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.tx
            .send(buf.to_vec())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::BrokenPipe, e))?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Live ACP runtime adapter.
///
/// Most methods take `&self` so cancel/approval can run concurrently with a
/// blocking `submit_prompt` (required for permission-cancel and mid-stream cancel).
pub struct RuntimeAdapter {
    process: Arc<Mutex<ManagedProcess>>,
    writer: Mutex<NdjsonWriter<ChannelWriter>>,
    /// Keep sender alive so writer channel stays open until drop.
    _out_tx: Sender<Vec<u8>>,
    shared: Arc<Mutex<SharedInner>>,
    stop_flag: Arc<AtomicBool>,
    reader_join: Option<JoinHandle<()>>,
    writer_join: Option<JoinHandle<()>>,
    event_rx: Mutex<Option<Receiver<AdapterEvent>>>,
    runtime_kind: RuntimeKind,
    rpc_timeout: Duration,
    cancel_timeout: Duration,
}

/// Type alias for handoff docs.
pub type AdapterHandle = RuntimeAdapter;

impl RuntimeAdapter {
    /// Spawn a runtime process and attach ACP pipes.
    ///
    /// Emits `runtime.process.started` (and process events). Does **not**
    /// initialize ACP — call [`initialize`](Self::initialize) next.
    pub fn start(
        spec: RuntimeSpawnSpec,
        project_id: ProjectId,
        session_id: SessionId,
    ) -> Result<Self, AdapterError> {
        Self::start_with_timeouts(
            spec,
            project_id,
            session_id,
            DEFAULT_RPC_TIMEOUT,
            DEFAULT_CANCEL_TIMEOUT,
        )
    }

    /// Start with custom RPC / cancel timeouts.
    pub fn start_with_timeouts(
        spec: RuntimeSpawnSpec,
        project_id: ProjectId,
        session_id: SessionId,
        rpc_timeout: Duration,
        cancel_timeout: Duration,
    ) -> Result<Self, AdapterError> {
        let runtime_kind = spec.kind;
        let cwd_display = spec.cwd.display().to_string();
        let spawn_cfg: SpawnConfig = spec.to_spawn_config();
        let mut process = ProcessManager::new()
            .spawn(spawn_cfg)
            .map_err(|e| AdapterError::from_process(&e))?;

        let stdout = process
            .take_stdout()
            .ok_or_else(|| AdapterError::new(ErrorClass::InternalAdapterError, "stdout missing"))?;

        let (event_tx, event_rx) = mpsc::channel();
        let mut client = AcpClient::new(ClientConfig::default());
        let _ = client.state.on_process_starting();
        let _ = client.state.on_process_alive();

        let mut envelopes = EnvelopeBuilder::new(project_id, session_id);
        for pev in process.drain_events() {
            if let Some(env) = map_process_event(&mut envelopes, &pev) {
                let _ = event_tx.send(AdapterEvent::from_envelope(env));
            }
        }
        let created = envelopes.emit_info(
            EventType::SessionCreated,
            json!({ "cwd": cwd_display })
                .as_object()
                .cloned()
                .unwrap_or_default(),
            None,
        );
        let _ = event_tx.send(AdapterEvent::from_envelope(created));

        let shared = Arc::new(Mutex::new(SharedInner {
            client,
            envelopes,
            capabilities: None,
            auth_state: AuthenticationState::NotRequired,
            auth_methods: Vec::new(),
            approvals: HashMap::new(),
            approval_by_id: HashMap::new(),
            event_tx: event_tx.clone(),
            rpc_waiters: HashMap::new(),
            shutdown_requested: false,
            expected_exit: false,
        }));

        let stop_flag = Arc::new(AtomicBool::new(false));
        let process = Arc::new(Mutex::new(process));

        // Writer thread
        let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>();
        let proc_w = Arc::clone(&process);
        let writer_stop = Arc::clone(&stop_flag);
        let writer_join = thread::spawn(move || {
            while !writer_stop.load(Ordering::SeqCst) {
                match out_rx.recv_timeout(Duration::from_millis(50)) {
                    Ok(bytes) => {
                        if let Ok(mut p) = proc_w.lock() {
                            let _ = p.write_stdin(&bytes);
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
            while let Ok(bytes) = out_rx.try_recv() {
                if let Ok(mut p) = proc_w.lock() {
                    let _ = p.write_stdin(&bytes);
                }
            }
        });

        // Reader thread
        let reader_shared = Arc::clone(&shared);
        let reader_stop = Arc::clone(&stop_flag);
        let reader_join = thread::spawn(move || {
            reader_loop(stdout, reader_shared, reader_stop);
        });

        let writer = NdjsonWriter::new(ChannelWriter { tx: out_tx.clone() });

        Ok(Self {
            process,
            writer: Mutex::new(writer),
            _out_tx: out_tx,
            shared,
            stop_flag,
            reader_join: Some(reader_join),
            writer_join: Some(writer_join),
            event_rx: Mutex::new(Some(event_rx)),
            runtime_kind,
            rpc_timeout,
            cancel_timeout,
        })
    }

    /// Take the event receiver (once). Prefer this for streaming consumers.
    pub fn take_event_receiver(&self) -> Option<Receiver<AdapterEvent>> {
        self.event_rx.lock().ok().and_then(|mut g| g.take())
    }

    /// Poll one event without blocking.
    pub fn try_recv_event(&self) -> Option<AdapterEvent> {
        let guard = self.event_rx.lock().ok()?;
        let rx = guard.as_ref()?;
        rx.try_recv().ok()
    }

    /// Drain currently queued events.
    pub fn drain_events(&self) -> Vec<AdapterEvent> {
        let mut out = Vec::new();
        if let Ok(guard) = self.event_rx.lock() {
            if let Some(rx) = guard.as_ref() {
                while let Ok(ev) = rx.try_recv() {
                    out.push(ev);
                }
            }
        }
        out
    }

    /// Wait for an event matching `pred` or timeout.
    pub fn wait_event(
        &self,
        timeout: Duration,
        mut pred: impl FnMut(&AdapterEvent) -> bool,
    ) -> Result<AdapterEvent, AdapterError> {
        let guard = self
            .event_rx
            .lock()
            .map_err(|_| AdapterError::invalid_state("event lock poisoned"))?;
        let rx = guard
            .as_ref()
            .ok_or_else(|| AdapterError::invalid_state("event receiver already taken"))?;
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(AdapterError::new(
                    ErrorClass::Timeout,
                    "timed out waiting for adapter event",
                ));
            }
            match rx.recv_timeout(remaining.min(Duration::from_millis(100))) {
                Ok(ev) => {
                    if pred(&ev) {
                        return Ok(ev);
                    }
                }
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(AdapterError::disconnected("event channel closed"));
                }
            }
        }
    }

    /// Inspect adapter state (readiness, caps, auth).
    pub fn inspect(&self) -> RuntimeAdapterState {
        let g = self.shared.lock().expect("shared lock");
        let phase = g.client.state.phase();
        let session_ready = g.client.state.session_ready();
        let process_alive = self
            .process
            .lock()
            .map(|p| p.is_process_alive())
            .unwrap_or(false);
        RuntimeAdapterState {
            readiness: AdapterReadiness::new(process_alive, phase, g.auth_state, session_ready),
            capabilities: g.capabilities.clone(),
            runtime_kind: self.runtime_kind.as_str().into(),
            runtime_session_id: g.client.state.runtime_session_id().map(|s| s.to_string()),
            auth_methods: g.auth_methods.clone(),
            shutdown_requested: g.shutdown_requested,
        }
    }

    /// Whether process is alive (OS). Polls `try_wait` first so crashes are observed.
    pub fn is_process_alive(&self) -> bool {
        if let Ok(mut p) = self.process.lock() {
            let _ = p.try_wait();
            p.is_process_alive()
        } else {
            false
        }
    }

    /// Protocol ready (initialize ok). **Not** session-ready.
    pub fn is_protocol_ready(&self) -> bool {
        self.shared
            .lock()
            .map(|g| g.client.state.protocol_ready())
            .unwrap_or(false)
    }

    /// Session ready for prompts.
    pub fn is_session_ready(&self) -> bool {
        self.shared
            .lock()
            .map(|g| g.client.state.session_ready())
            .unwrap_or(false)
    }

    /// Auth state.
    pub fn auth_state(&self) -> AuthenticationState {
        self.shared
            .lock()
            .map(|g| g.auth_state)
            .unwrap_or(AuthenticationState::NotRequired)
    }

    /// Initialize ACP + capability negotiation. Emits `runtime.process.ready` on success.
    pub fn initialize(&self) -> Result<Capabilities, AdapterError> {
        {
            let mut g = self.shared.lock().expect("shared");
            g.client
                .state
                .on_initialize_start()
                .map_err(|e| AdapterError::invalid_state(e.to_string()))?;
        }
        let req = {
            let mut g = self.shared.lock().expect("shared");
            g.client.build_initialize()
        };
        let result = self.rpc(&req)?;
        let caps = match result {
            RequestResult::Ok(value) => {
                let caps = capabilities_from_initialize(&value);
                let mut g = self.shared.lock().expect("shared");
                g.client.last_initialize_result = Some(value.clone());
                if let Some(methods) = value.get("authMethods").and_then(|v| v.as_array()) {
                    g.auth_methods = methods.clone();
                }
                g.capabilities = Some(caps.clone());
                g.client
                    .state
                    .on_initialize_ok()
                    .map_err(|e| AdapterError::invalid_state(e.to_string()))?;
                // Auth not yet proven — leave NotRequired for fake; Unauthenticated if methods present
                // Fake runtime always allows session/new without auth unless scenario forces it.
                // Product: process ready ≠ authenticated.
                let payload = builders::process_ready(&caps, "acp-1");
                let env = g.envelopes.emit_info(
                    EventType::RuntimeProcessReady,
                    payload,
                    Some("initialize"),
                );
                let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
                caps
            }
            RequestResult::Err(e) => {
                let mut g = self.shared.lock().expect("shared");
                g.client.state.on_failed(e.message.clone());
                let env = protocol_error_event(
                    &mut g.envelopes,
                    ErrorClass::ProtocolInitializeFailed,
                    &e.message,
                );
                let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
                return Err(AdapterError::new(
                    ErrorClass::ProtocolInitializeFailed,
                    e.message,
                ));
            }
        };
        Ok(caps)
    }

    /// Inspect whether auth is required based on last initialize + auth methods.
    ///
    /// Does not itself change readiness; session/new auth errors set
    /// [`AuthenticationState::Unauthenticated`].
    pub fn inspect_auth_requirement(&self) -> AuthenticationState {
        self.auth_state()
    }

    /// Optional authenticate (fake is no-op success).
    pub fn authenticate(&self, method_id: Option<&str>) -> Result<(), AdapterError> {
        {
            let mut g = self.shared.lock().expect("shared");
            g.auth_state = AuthenticationState::InProgress;
        }
        let req = {
            let mut g = self.shared.lock().expect("shared");
            g.client.build_authenticate(method_id)
        };
        match self.rpc(&req)? {
            RequestResult::Ok(_) => {
                let mut g = self.shared.lock().expect("shared");
                g.auth_state = AuthenticationState::Authenticated;
                g.client.state.on_authenticated();
                Ok(())
            }
            RequestResult::Err(e) => {
                let mut g = self.shared.lock().expect("shared");
                g.auth_state = AuthenticationState::Failed;
                let _ = g.client.state.on_auth_failed(e.message.clone());
                Err(AdapterError::auth_failed(e.message))
            }
        }
    }

    /// Create a runtime session (`session/new`). Emits `session.ready` on success.
    ///
    /// Auth-required errors do **not** emit session.ready (process may still be ready).
    pub fn create_session(&self, params: SessionCreateParams) -> Result<String, AdapterError> {
        if !self.is_protocol_ready() {
            return Err(AdapterError::not_ready(
                "cannot create session before initialize / protocol ready",
            ));
        }
        {
            let mut g = self.shared.lock().expect("shared");
            g.client
                .state
                .on_session_create_start()
                .map_err(|e| AdapterError::invalid_state(e.to_string()))?;
        }
        let req = {
            let mut g = self.shared.lock().expect("shared");
            g.client.build_session_new(&params.cwd)
        };
        match self.rpc(&req)? {
            RequestResult::Ok(value) => {
                let sid = value
                    .get("sessionId")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let mut g = self.shared.lock().expect("shared");
                g.client
                    .state
                    .on_session_ready(sid.clone())
                    .map_err(|e| AdapterError::invalid_state(e.to_string()))?;
                g.envelopes.set_runtime_session_id(Some(sid.clone()));
                if g.auth_state == AuthenticationState::NotRequired
                    || g.auth_state == AuthenticationState::InProgress
                {
                    g.auth_state = AuthenticationState::Authenticated;
                }
                let payload = json!({ "runtimeSessionId": sid })
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                let env =
                    g.envelopes
                        .emit_info(EventType::SessionReady, payload, Some("session/new"));
                let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
                Ok(sid)
            }
            RequestResult::Err(e) => {
                let mut g = self.shared.lock().expect("shared");
                if e.is_authentication_required() {
                    g.auth_state = AuthenticationState::Unauthenticated;
                    let _ = g.client.state.on_auth_required();
                    let payload = builders::error_payload(
                        ErrorClass::AuthenticationRequired,
                        &e.message,
                        false,
                    );
                    let env = g.envelopes.emit(
                        EventType::SessionFailed,
                        payload,
                        Some(Severity::Warn),
                        Some("session/new"),
                        None,
                    );
                    // Note: intentionally NOT session.ready
                    let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
                    return Err(AdapterError::auth_required(e.message));
                }
                g.client.state.on_failed(e.message.clone());
                Err(AdapterError::from_acp(&e))
            }
        }
    }

    /// Submit a prompt. Emits `session.prompt.submitted` then stream events.
    ///
    /// Blocks until the prompt RPC completes (end_turn / cancelled / error).
    /// Call [`cancel_prompt`](Self::cancel_prompt) or [`resolve_approval`](Self::resolve_approval)
    /// from another thread while this blocks.
    pub fn submit_prompt(&self, prompt: PromptRequest) -> Result<(), AdapterError> {
        let state = self.inspect();
        if !state.readiness.may_accept_prompt() {
            return Err(AdapterError::not_ready(format!(
                "cannot submit prompt (process_alive={}, protocol_ready={}, session_ready={}, prompt_active={}, auth={})",
                state.readiness.process_alive,
                state.readiness.protocol_ready,
                state.readiness.session_ready,
                state.readiness.prompt_active,
                state.readiness.auth_state.as_str(),
            )));
        }
        let run_id = AgentRunId::new();
        let session_id = {
            let mut g = self.shared.lock().expect("shared");
            g.envelopes.set_agent_run(Some(run_id));
            g.client
                .state
                .on_prompt_start()
                .map_err(|e| AdapterError::invalid_state(e.to_string()))?;
            g.client
                .state
                .runtime_session_id()
                .unwrap_or("")
                .to_string()
        };
        let req = {
            let mut g = self.shared.lock().expect("shared");
            g.client
                .build_session_prompt(&session_id, &prompt.text, prompt.prompt_id.as_deref())
        };
        {
            let mut g = self.shared.lock().expect("shared");
            let payload = json!({
                "promptId": prompt.prompt_id,
                "text": prompt.text,
                "attachments": 0
            })
            .as_object()
            .cloned()
            .unwrap_or_default();
            let env = g.envelopes.emit_info(
                EventType::SessionPromptSubmitted,
                payload,
                Some("session/prompt"),
            );
            let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
        }

        // Prompt is long-running: register waiter then write; caller may drain events
        // while we block for the final result.
        let result = self.rpc(&req)?;
        match result {
            RequestResult::Ok(value) => {
                let stop = value
                    .get("stopReason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("end_turn");
                let mut g = self.shared.lock().expect("shared");
                if stop == "cancelled" {
                    let payload = json!({ "reason": "cancelled", "partial": true })
                        .as_object()
                        .cloned()
                        .unwrap_or_default();
                    let env = g.envelopes.emit_info(
                        EventType::SessionCancelled,
                        payload,
                        Some("session/prompt"),
                    );
                    let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
                    let _ = g.client.state.on_cancelled();
                } else {
                    // Final assistant boundary
                    let payload = json!({
                        "role": "assistant",
                        "stopReason": stop
                    })
                    .as_object()
                    .cloned()
                    .unwrap_or_default();
                    let env = g.envelopes.emit_info(
                        EventType::AgentMessageCompleted,
                        payload,
                        Some("session/prompt"),
                    );
                    let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
                    let payload = json!({ "summary": stop })
                        .as_object()
                        .cloned()
                        .unwrap_or_default();
                    let env = g.envelopes.emit_info(
                        EventType::SessionCompleted,
                        payload,
                        Some("session/prompt"),
                    );
                    let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
                    let _ = g.client.state.on_prompt_completed();
                }
                g.envelopes.set_agent_run(None);
                Ok(())
            }
            RequestResult::Err(e) => {
                let mut g = self.shared.lock().expect("shared");
                let payload =
                    builders::error_payload(ErrorClass::PromptRejected, &e.message, false);
                let env = g.envelopes.emit(
                    EventType::SessionFailed,
                    payload,
                    Some(Severity::Error),
                    Some("session/prompt"),
                    None,
                );
                let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
                g.client.state.on_failed(e.message.clone());
                Err(AdapterError::from_acp(&e))
            }
        }
    }

    /// Respond to a pending approval (never auto-approved).
    pub fn resolve_approval(&self, decision: ApprovalDecisionRequest) -> Result<(), AdapterError> {
        let (req_id, option_id, allow) = {
            let g = self.shared.lock().expect("shared");
            let key = g
                .approval_by_id
                .get(&decision.approval_id)
                .cloned()
                .ok_or_else(|| {
                    AdapterError::new(
                        ErrorClass::ApprovalUnknown,
                        format!("unknown approval id {}", decision.approval_id),
                    )
                })?;
            let pending = g.approvals.get(&key).ok_or_else(|| {
                AdapterError::new(
                    ErrorClass::ApprovalUnknown,
                    format!("approval {} not pending", decision.approval_id),
                )
            })?;
            let allow = matches!(
                decision.decision.as_str(),
                "allow" | "allow_once" | "allow_always"
            );
            let cancel = decision.decision == "cancel";
            let option = decision.option_id.clone().or_else(|| {
                if cancel {
                    None
                } else if allow {
                    Some("allow-once".into())
                } else {
                    Some("reject-once".into())
                }
            });
            (
                pending.request_id.clone(),
                option,
                if cancel { None } else { Some(allow) },
            )
        };

        let response = {
            let g = self.shared.lock().expect("shared");
            match allow {
                None => g.client.build_permission_cancelled(req_id),
                Some(a) => g
                    .client
                    .build_permission_response(req_id, a, option_id.as_deref()),
            }
        };
        self.write_response(&response)?;

        let decision_wire = match decision.decision.as_str() {
            "allow" | "allow_once" | "allow_always" => "allow",
            "cancel" => "cancel",
            _ => "deny",
        };
        {
            let mut g = self.shared.lock().expect("shared");
            if let Some(key) = g.approval_by_id.remove(&decision.approval_id) {
                g.approvals.remove(&key);
            }
            let env = approval_resolved(
                &mut g.envelopes,
                &decision.approval_id,
                decision_wire,
                "user",
            );
            let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
            // Return to streaming/prompting if still active
            let _ = g.client.state.on_streaming();
        }
        Ok(())
    }

    /// Cancel the active prompt.
    ///
    /// If `capabilities.cancellation` is false, returns
    /// [`ErrorClass::CapabilityUnsupported`] — control plane should process-stop.
    pub fn cancel_prompt(&self) -> Result<(), AdapterError> {
        let (session_id, has_cancel, has_permission) = {
            let g = self.shared.lock().expect("shared");
            let caps = g.capabilities.clone().unwrap_or_default();
            let sid = g
                .client
                .state
                .runtime_session_id()
                .unwrap_or("")
                .to_string();
            let perm = g.client.has_pending_permission() || !g.approvals.is_empty();
            (sid, caps.cancellation, perm)
        };
        if !has_cancel {
            return Err(AdapterError::capability_unsupported(
                "runtime does not support cooperative cancellation; use process stop",
            ));
        }
        {
            let mut g = self.shared.lock().expect("shared");
            let _ = g.client.state.on_cancel_start();
        }
        let n = {
            let g = self.shared.lock().expect("shared");
            g.client.build_session_cancel(&session_id)
        };
        self.write_notification(&n)?;

        // Permission-cancel path: also resolve open permission as cancelled to
        // avoid deadlock (mandatory risk test — time-bounded).
        if has_permission {
            let pending_ids: Vec<(String, JsonRpcId)> = {
                let g = self.shared.lock().expect("shared");
                g.approvals
                    .iter()
                    .map(|(k, p)| (k.clone(), p.request_id.clone()))
                    .collect()
            };
            for (key, rid) in pending_ids {
                let resp = {
                    let g = self.shared.lock().expect("shared");
                    g.client.build_permission_cancelled(rid)
                };
                let _ = self.write_response(&resp);
                let mut g = self.shared.lock().expect("shared");
                if let Some(p) = g.approvals.remove(&key) {
                    if let Some(aid) = p.approval_id {
                        g.approval_by_id.remove(&aid);
                        let env = approval_resolved(&mut g.envelopes, &aid, "cancel", "system");
                        let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
                    }
                }
            }
        }

        // Wait briefly for prompt result / cancelled terminal (non-fatal if timeout —
        // process fallback is control-plane responsibility for slow_cancel_ack).
        let budget = self.cancel_timeout.min(PERMISSION_CANCEL_DEADLOCK_BUDGET);
        let start = Instant::now();
        while start.elapsed() < budget {
            if let Ok(g) = self.shared.lock() {
                if !g.client.state.phase().is_prompt_active() {
                    return Ok(());
                }
            }
            thread::sleep(Duration::from_millis(20));
        }
        // Not fully drained — still OK for cancel_while_permission; no deadlock.
        Ok(())
    }

    /// Graceful or forced shutdown. Guarantees process cleanup via W1-C.
    pub fn shutdown(&self, opts: ShutdownOptions) -> Result<(), AdapterError> {
        {
            let mut g = self.shared.lock().expect("shared");
            g.shutdown_requested = true;
            g.expected_exit = true;
        }
        self.stop_flag.store(true, Ordering::SeqCst);

        let policy = if opts.graceful {
            StopPolicy::GracefulThenForce {
                graceful: opts.graceful_timeout,
                force_wait: opts.force_timeout,
            }
        } else {
            StopPolicy::Force {
                force_wait: opts.force_timeout,
            }
        };

        let info = {
            let mut p = self.process.lock().expect("process");
            p.mark_expected_exit();
            p.stop(policy).map_err(|e| AdapterError::from_process(&e))?
        };

        // Emit process events still in queue + exit envelope if not already
        self.pump_process_events(Some(&info));
        Ok(())
    }

    /// Force-kill process tree (orphan cleanup path).
    pub fn force_kill(&self) -> Result<(), AdapterError> {
        {
            let mut g = self.shared.lock().expect("shared");
            g.expected_exit = true;
            g.shutdown_requested = true;
        }
        self.stop_flag.store(true, Ordering::SeqCst);
        let info = {
            let mut p = self.process.lock().expect("process");
            p.kill_force(Duration::from_secs(3))
                .map_err(|e| AdapterError::from_process(&e))?
        };
        self.pump_process_events(Some(&info));
        Ok(())
    }

    /// Poll process manager events into normalized stream.
    pub fn pump_process_events(&self, exit: Option<&tracer_process::ExitInfo>) {
        let events: Vec<ProcessEvent> = {
            let p = self.process.lock().expect("process");
            p.drain_events()
        };
        let mut g = self.shared.lock().expect("shared");
        let expected_exit = g.expected_exit;
        for pev in events {
            if let ProcessEvent::Exited { info, .. } = &pev {
                if !info.expected && !expected_exit {
                    g.client
                        .state
                        .on_crashed(format!("process exited code={:?}", info.exit_code));
                } else {
                    g.client
                        .state
                        .on_disconnected(info.expected || expected_exit);
                }
            }
            if let Some(env) = map_process_event(&mut g.envelopes, &pev) {
                let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
            }
        }
        if let Some(info) = exit {
            let expected = info.expected || expected_exit;
            let payload =
                builders::process_exited(info.exit_code, info.signal.as_deref(), expected, None);
            let env = g.envelopes.emit(
                EventType::RuntimeProcessExited,
                payload,
                Some(if expected {
                    Severity::Info
                } else {
                    Severity::Error
                }),
                None,
                None,
            );
            let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
        }
    }

    /// Collect event type strings for a while (quiet-exit).
    pub fn collect_event_types(&self, timeout: Duration) -> Vec<String> {
        let deadline = Instant::now() + timeout;
        let mut types = Vec::new();
        let guard = match self.event_rx.lock() {
            Ok(g) => g,
            Err(_) => return types,
        };
        let Some(rx) = guard.as_ref() else {
            return types;
        };
        while Instant::now() < deadline {
            match rx.recv_timeout(Duration::from_millis(50)) {
                Ok(AdapterEvent::Event(e)) => types.push(e.event_type.as_str().to_string()),
                Ok(AdapterEvent::Error(_)) => {}
                Err(RecvTimeoutError::Timeout) => {
                    if rx.try_recv().is_err() {
                        thread::sleep(Duration::from_millis(30));
                        if rx.try_recv().is_err() {
                            break;
                        }
                    }
                }
                Err(RecvTimeoutError::Disconnected) => break,
            }
        }
        types
    }

    fn write_request(&self, req: &JsonRpcRequest) -> Result<(), AdapterError> {
        let mut w = self
            .writer
            .lock()
            .map_err(|_| AdapterError::invalid_state("writer lock poisoned"))?;
        w.write_request(req).map_err(|e| AdapterError::from_acp(&e))
    }

    fn write_notification(&self, n: &JsonRpcNotification) -> Result<(), AdapterError> {
        let mut w = self
            .writer
            .lock()
            .map_err(|_| AdapterError::invalid_state("writer lock poisoned"))?;
        w.write_notification(n)
            .map_err(|e| AdapterError::from_acp(&e))
    }

    fn write_response(&self, r: &JsonRpcResponse) -> Result<(), AdapterError> {
        let mut w = self
            .writer
            .lock()
            .map_err(|_| AdapterError::invalid_state("writer lock poisoned"))?;
        w.write_response(r).map_err(|e| AdapterError::from_acp(&e))
    }

    fn rpc(&self, req: &JsonRpcRequest) -> Result<RequestResult, AdapterError> {
        let key = req.id.as_key();
        let (tx, rx) = mpsc::channel();
        {
            let mut g = self.shared.lock().expect("shared");
            g.rpc_waiters.insert(key.clone(), tx);
        }
        self.write_request(req)?;

        // Prompt RPCs can run longer than initialize — use extended budget for session/prompt
        let timeout = if req.method == "session/prompt" {
            self.rpc_timeout
                .max(Duration::from_secs(60))
                .max(self.cancel_timeout + Duration::from_secs(5))
        } else {
            self.rpc_timeout
        };

        match rx.recv_timeout(timeout) {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(e)) => Err(e),
            Err(RecvTimeoutError::Timeout) => {
                let mut g = self.shared.lock().expect("shared");
                g.rpc_waiters.remove(&key);
                Err(AdapterError::new(
                    ErrorClass::Timeout,
                    format!("RPC timeout waiting for response id={key}"),
                ))
            }
            Err(RecvTimeoutError::Disconnected) => Err(AdapterError::disconnected(
                "RPC waiter channel disconnected",
            )),
        }
    }
}

impl Drop for RuntimeAdapter {
    fn drop(&mut self) {
        self.stop_flag.store(true, Ordering::SeqCst);
        // Process kill_on_drop handles orphan cleanup
        if let Ok(mut p) = self.process.lock() {
            if p.is_alive() {
                let _ = p.kill_force(Duration::from_secs(2));
            }
        }
        if let Some(j) = self.reader_join.take() {
            let _ = j.join();
        }
        if let Some(j) = self.writer_join.take() {
            let _ = j.join();
        }
    }
}

fn map_process_event(builder: &mut EnvelopeBuilder, pev: &ProcessEvent) -> Option<EventEnvelope> {
    match pev {
        ProcessEvent::Started {
            pid,
            executable,
            args,
            cwd,
            ..
        } => {
            let payload = json!({
                "pid": pid,
                "executable": executable,
                "args": args,
                "cwd": cwd,
            })
            .as_object()
            .cloned()
            .unwrap_or_default();
            Some(builder.emit_info(EventType::RuntimeProcessStarted, payload, None))
        }
        ProcessEvent::StderrChunk {
            chunk, truncated, ..
        } => {
            let payload = json!({
                "chunk": chunk,
                "truncated": truncated,
            })
            .as_object()
            .cloned()
            .unwrap_or_default();
            Some(builder.emit_info(EventType::RuntimeProcessStderr, payload, None))
        }
        ProcessEvent::Exited { info, .. } => {
            let payload = builders::process_exited(
                info.exit_code,
                info.signal.as_deref(),
                info.expected,
                None,
            );
            let sev = if info.expected {
                Severity::Info
            } else {
                Severity::Error
            };
            Some(builder.emit(
                EventType::RuntimeProcessExited,
                payload,
                Some(sev),
                None,
                None,
            ))
        }
        ProcessEvent::Failed {
            error_class,
            message,
            retryable,
            ..
        } => {
            let class =
                ErrorClass::parse(error_class.as_str()).unwrap_or(ErrorClass::RuntimeSpawnFailed);
            let payload = builders::error_payload(class, message, *retryable);
            Some(builder.emit(
                EventType::RuntimeProcessFailed,
                payload,
                Some(Severity::Error),
                None,
                None,
            ))
        }
    }
}

fn reader_loop(mut stdout: ChildStdout, shared: Arc<Mutex<SharedInner>>, stop: Arc<AtomicBool>) {
    let mut decoder = FrameDecoder::new();
    let mut buf = [0u8; 8192];
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        let n = match stdout.read(&mut buf) {
            Ok(0) => {
                if let Ok(mut g) = shared.lock() {
                    handle_stdout_eof(&mut g);
                }
                break;
            }
            Ok(n) => n,
            Err(_) => {
                if let Ok(mut g) = shared.lock() {
                    handle_stdout_eof(&mut g);
                }
                break;
            }
        };
        let lines = decoder.push(&buf[..n]);
        for line in lines {
            let frame = match decode_line(&line) {
                Ok(Some(msg)) => InboundFrame::Message(msg),
                Ok(None) => continue,
                Err(e) => InboundFrame::Malformed {
                    raw: line.chars().take(512).collect(),
                    error: e,
                },
            };
            if let Ok(mut g) = shared.lock() {
                dispatch_frame(&mut g, frame);
            }
        }
    }
}

fn handle_stdout_eof(g: &mut SharedInner) {
    let expected = g.expected_exit || g.shutdown_requested;
    g.client.state.on_disconnected(expected);
    let waiters: Vec<_> = g.rpc_waiters.drain().collect();
    for (_, tx) in waiters {
        let _ = tx.send(Err(AdapterError::disconnected("runtime stdout EOF")));
    }
    let phase = g.client.state.phase();
    if !expected && phase.is_prompt_active() {
        let payload = builders::error_payload(
            ErrorClass::RuntimeDisconnected,
            "unexpected runtime stdout EOF mid-prompt",
            false,
        );
        let env = g.envelopes.emit(
            EventType::SessionFailed,
            payload,
            Some(Severity::Error),
            None,
            None,
        );
        let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
    }
}

fn dispatch_frame(g: &mut SharedInner, frame: InboundFrame) {
    match frame {
        InboundFrame::Malformed { error, .. } => {
            let env = protocol_error_event(
                &mut g.envelopes,
                ErrorClass::ProtocolParseError,
                &error.message,
            );
            let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
        }
        InboundFrame::Message(JsonRpcMessage::Response(resp)) => {
            handle_response_message(g, resp);
        }
        InboundFrame::Message(JsonRpcMessage::Notification(n)) => {
            handle_notification(g, n);
        }
        InboundFrame::Message(JsonRpcMessage::Request(req)) => {
            handle_server_request_msg(g, req);
        }
    }
}

fn handle_response_message(g: &mut SharedInner, resp: JsonRpcResponse) {
    let key = resp.id.as_key();
    match g.client.handle_response(&resp) {
        Ok((_method, result)) => {
            if let Some(tx) = g.rpc_waiters.remove(&key) {
                let _ = tx.send(Ok(result));
            }
        }
        Err(e) if e.kind == tracer_acp_client::AcpErrorKind::DuplicateResponseId => {
            let env =
                protocol_error_event(&mut g.envelopes, ErrorClass::ProtocolViolation, &e.message);
            let _ = g.event_tx.send(AdapterEvent::from_envelope(env));
        }
        Err(e) => {
            if let Some(tx) = g.rpc_waiters.remove(&key) {
                let _ = tx.send(Err(AdapterError::from_acp(&e)));
            }
        }
    }
}

fn handle_notification(g: &mut SharedInner, n: JsonRpcNotification) {
    let _ = g.client.state.on_streaming();
    for ev in normalize_notification(&mut g.envelopes, &n) {
        let _ = g.event_tx.send(AdapterEvent::from_envelope(ev));
    }
}

fn handle_server_request_msg(g: &mut SharedInner, req: JsonRpcRequest) {
    let approval_id = uuid::Uuid::new_v4().to_string();
    if let Some(pending) = g
        .client
        .handle_server_request(&req, Some(approval_id.clone()))
    {
        g.approvals.insert(req.id.as_key(), pending);
        g.approval_by_id
            .insert(approval_id.clone(), req.id.as_key());
        let _ = g.client.state.on_awaiting_approval();
        if let Some(ev) = normalize_server_request(&mut g.envelopes, &req, &approval_id) {
            let _ = g.event_tx.send(AdapterEvent::from_envelope(ev));
        }
    } else if let Some(ev) = normalize_server_request(&mut g.envelopes, &req, &approval_id) {
        let _ = g.event_tx.send(AdapterEvent::from_envelope(ev));
    }
}
