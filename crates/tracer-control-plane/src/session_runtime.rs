//! Per-session runtime handle: adapter + event ingestor + local state.
//!
//! # Concurrency / drain strategy
//!
//! Adapter event channel is **unbounded** (W1-D). W1-F mitigation path:
//!
//! ```text
//! adapter unbounded receiver
//!     -> OS drain thread (continuous)
//!     -> bounded sync_channel handoff (BRIDGE_CAPACITY)
//!     -> async persist pump -> SqliteStorage::append_event
//!     -> optional presentation fan-out (after persist)
//! ```
//!
//! Bounded handoff applies backpressure when SQLite is slow so W1-F does not
//! add a second unbounded buffer. Drain uses `send_timeout` so stop remains
//! responsive if the bridge is full. Presentation is post-persist and must not
//! block the adapter channel indefinitely.
//!
//! This split avoids `Handle::block_on` / `spawn_blocking` deadlocks while
//! keeping ingestion alive during blocking `submit_prompt`, pending approval,
//! and cancel. No lock is held across long-running adapter RPCs.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{
    self, Receiver as StdReceiver, RecvTimeoutError, Sender as StdSender, SyncSender,
    SendTimeoutError,
};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Bounded internal handoff between OS drain and async persist pump.
/// Keeps adapter drain continuous while preventing unbounded->unbounded growth.
const BRIDGE_CAPACITY: usize = 256;
/// Max wait for bridge space before re-checking stop flag.
const BRIDGE_SEND_TIMEOUT: Duration = Duration::from_millis(100);

use serde_json::Value;
use tracer_domain::{AuthenticationState, SessionStatus};
use tracer_runtime_adapter::{
    AdapterEvent, ApprovalDecisionRequest, PromptRequest, RuntimeAdapter, ShutdownOptions,
};
use tracer_storage::{SessionId, SqliteStorage};
use tracing::{debug, warn};

use crate::convert::{
    approval_id_from_payload, envelope_to_event_record, pending_from_payload,
    status_hint_from_event_type,
};
use crate::types::{PendingApprovalView, PresentationEvent};

/// Shared mutable session runtime state (not held across adapter RPCs for cancel).
#[derive(Debug)]
pub struct SessionRuntimeState {
    pub status: SessionStatus,
    pub auth_state: AuthenticationState,
    pub capabilities: Option<Value>,
    pub runtime_session_id: Option<String>,
    pub last_error: Option<Value>,
    pub pending_approvals: HashMap<String, PendingApprovalView>,
    pub latest_sequence: i64,
    pub prompt_in_flight: bool,
    pub last_prompt_id: Option<String>,
    pub last_agent_run_id: Option<String>,
    pub persist_failed: bool,
}

impl SessionRuntimeState {
    pub fn new() -> Self {
        Self {
            status: SessionStatus::Creating,
            auth_state: AuthenticationState::NotRequired,
            capabilities: None,
            runtime_session_id: None,
            last_error: None,
            pending_approvals: HashMap::new(),
            latest_sequence: 0,
            prompt_in_flight: false,
            last_prompt_id: None,
            last_agent_run_id: None,
            persist_failed: false,
        }
    }
}

impl Default for SessionRuntimeState {
    fn default() -> Self {
        Self::new()
    }
}

/// Live session with adapter + dual-stage ingestor.
pub struct LiveSession {
    pub session_id: SessionId,
    pub project_id: tracer_storage::ProjectId,
    pub adapter: Arc<RuntimeAdapter>,
    pub state: Arc<Mutex<SessionRuntimeState>>,
    stop_ingest: Arc<AtomicBool>,
    drain_join: Mutex<Option<JoinHandle<()>>>,
    pump_abort: Mutex<Option<tokio::task::JoinHandle<()>>>,
    pub event_fanout: Arc<Mutex<Option<StdSender<PresentationEvent>>>>,
}

impl LiveSession {
    pub fn new(
        session_id: SessionId,
        project_id: tracer_storage::ProjectId,
        adapter: Arc<RuntimeAdapter>,
    ) -> Self {
        Self {
            session_id,
            project_id,
            adapter,
            state: Arc::new(Mutex::new(SessionRuntimeState::new())),
            stop_ingest: Arc::new(AtomicBool::new(false)),
            drain_join: Mutex::new(None),
            pump_abort: Mutex::new(None),
            event_fanout: Arc::new(Mutex::new(None)),
        }
    }

    /// Start continuous event drain into storage.
    pub fn start_ingestor(
        &self,
        storage: SqliteStorage,
        presentation_tx: Option<StdSender<PresentationEvent>>,
    ) {
        if let Some(tx) = presentation_tx {
            *self.event_fanout.lock().expect("fanout") = Some(tx);
        }

        let adapter_rx = match self.adapter.take_event_receiver() {
            Some(rx) => rx,
            None => {
                warn!(session = %self.session_id, "event receiver already taken");
                return;
            }
        };

        // Bounded bridge: adapter unbounded -> sync_channel -> async pump.
        let (bridge_tx, bridge_rx) = mpsc::sync_channel::<AdapterEvent>(BRIDGE_CAPACITY);
        let stop = Arc::clone(&self.stop_ingest);
        let sid = self.session_id;

        let drain = thread::Builder::new()
            .name(format!("cp-drain-{}", sid))
            .spawn(move || {
                drain_adapter(adapter_rx, bridge_tx, stop, sid);
            })
            .expect("spawn drain");
        *self.drain_join.lock().expect("join") = Some(drain);

        let state = Arc::clone(&self.state);
        let fanout = Arc::clone(&self.event_fanout);
        let adapter = Arc::clone(&self.adapter);
        let stop2 = Arc::clone(&self.stop_ingest);
        let storage = storage.clone();
        let pump = tokio::spawn(async move {
            async_persist_pump(bridge_rx, stop2, state, sid, storage, fanout, adapter).await;
        });
        *self.pump_abort.lock().expect("pump") = Some(pump);
    }

    pub fn stop_ingestor(&self) {
        self.stop_ingest.store(true, Ordering::SeqCst);
        // Join drain first (exits via stop + try_send / send_timeout); then abort pump.
        if let Ok(mut g) = self.drain_join.lock() {
            if let Some(h) = g.take() {
                let _ = h.join();
            }
        }
        if let Ok(mut g) = self.pump_abort.lock() {
            if let Some(h) = g.take() {
                // Pump exits when bridge sender drops after drain join; abort is fallback.
                h.abort();
            }
        }
    }
}

impl Drop for LiveSession {
    fn drop(&mut self) {
        self.stop_ingest.store(true, Ordering::SeqCst);
        let _ = self.adapter.shutdown(ShutdownOptions {
            graceful: true,
            graceful_timeout: Duration::from_secs(2),
            force_timeout: Duration::from_secs(2),
        });
        if let Ok(mut g) = self.drain_join.lock() {
            if let Some(h) = g.take() {
                let _ = h.join();
            }
        }
        if let Ok(mut g) = self.pump_abort.lock() {
            if let Some(h) = g.take() {
                h.abort();
            }
        }
    }
}

fn drain_adapter(
    rx: StdReceiver<AdapterEvent>,
    tx: SyncSender<AdapterEvent>,
    stop: Arc<AtomicBool>,
    session_id: SessionId,
) {
    debug!(session = %session_id, "adapter drain started");
    loop {
        if stop.load(Ordering::SeqCst) {
            // Best-effort flush without blocking forever on a full bridge.
            while let Ok(ev) = rx.try_recv() {
                if tx.try_send(ev).is_err() {
                    break;
                }
            }
            break;
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(ev) => {
                // Apply backpressure with timeout; re-check stop when full.
                let mut pending = Some(ev);
                while let Some(item) = pending.take() {
                    match tx.send_timeout(item, BRIDGE_SEND_TIMEOUT) {
                        Ok(()) => {}
                        Err(SendTimeoutError::Timeout(held)) => {
                            if stop.load(Ordering::SeqCst) {
                                let _ = tx.try_send(held);
                                debug!(session = %session_id, "adapter drain stopped (stop while full)");
                                return;
                            }
                            pending = Some(held);
                        }
                        Err(SendTimeoutError::Disconnected(_)) => {
                            debug!(session = %session_id, "adapter drain stopped (bridge closed)");
                            return;
                        }
                    }
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
    debug!(session = %session_id, "adapter drain stopped");
    // Dropping tx closes bridge for async pump.
}

async fn async_persist_pump(
    rx: StdReceiver<AdapterEvent>,
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<SessionRuntimeState>>,
    session_id: SessionId,
    storage: SqliteStorage,
    fanout: Arc<Mutex<Option<StdSender<PresentationEvent>>>>,
    adapter: Arc<RuntimeAdapter>,
) {
    debug!(session = %session_id, "async persist pump started");
    loop {
        // Non-blocking batch drain of the bridge.
        let mut batch = Vec::new();
        while let Ok(ev) = rx.try_recv() {
            batch.push(ev);
        }

        if batch.is_empty() {
            if stop.load(Ordering::SeqCst) {
                // Final try
                while let Ok(ev) = rx.try_recv() {
                    batch.push(ev);
                }
                if batch.is_empty() {
                    break;
                }
            } else {
                tokio::time::sleep(Duration::from_millis(10)).await;
                // Detect closed bridge: try_recv empty + disconnect.
                // std mpsc doesn't expose is_disconnected without recv; use timeout recv.
                match rx.recv_timeout(Duration::from_millis(5)) {
                    Ok(ev) => batch.push(ev),
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => {
                        while let Ok(ev) = rx.try_recv() {
                            batch.push(ev);
                        }
                        for ev in batch {
                            persist_one(&ev, &state, session_id, &storage, &fanout, &adapter).await;
                        }
                        break;
                    }
                }
            }
        }

        for ev in batch {
            persist_one(&ev, &state, session_id, &storage, &fanout, &adapter).await;
        }
    }
    debug!(session = %session_id, "async persist pump stopped");
}

async fn persist_one(
    ev: &AdapterEvent,
    state: &Arc<Mutex<SessionRuntimeState>>,
    session_id: SessionId,
    storage: &SqliteStorage,
    fanout: &Arc<Mutex<Option<StdSender<PresentationEvent>>>>,
    adapter: &RuntimeAdapter,
) {
    match ev {
        AdapterEvent::Error(err) => {
            let mut st = state.lock().expect("state");
            st.last_error = Some(serde_json::json!({
                "errorClass": err.error_class.as_str(),
                "message": err.message,
            }));
        }
        AdapterEvent::Event(env) => {
            let event_type = env.event_type.as_str().to_string();
            let payload = Value::Object(env.payload.clone());
            let ts = env
                .timestamp
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default();
            let record = envelope_to_event_record(env);

            match storage.append_event(record).await {
                Ok(stored) => {
                    let envelope_json = stored.to_envelope_json();
                    let seq = stored.sequence;
                    {
                        let mut st = state.lock().expect("state");
                        st.latest_sequence = seq;
                        apply_event_to_state(
                            &mut st,
                            &event_type,
                            &payload,
                            &session_id.to_string(),
                            &ts,
                        );
                        st.auth_state = adapter.auth_state();
                    }
                    // Presentation is post-persist; never blocks ingestion indefinitely
                    // (unbounded fanout or disconnected receiver drops).
                    if let Ok(g) = fanout.lock() {
                        if let Some(tx) = g.as_ref() {
                            let _ = tx.send(PresentationEvent {
                                batch: false,
                                events: vec![envelope_json],
                            });
                        }
                    }
                    let status_now = state.lock().expect("state").status;
                    let _ = storage.update_session_status(&session_id, status_now).await;
                }
                Err(tracer_storage::StorageError::AlreadyExists { .. }) => {
                    debug!(session = %session_id, "duplicate event ignored");
                    let mut st = state.lock().expect("state");
                    apply_event_to_state(
                        &mut st,
                        &event_type,
                        &payload,
                        &session_id.to_string(),
                        &ts,
                    );
                    st.auth_state = adapter.auth_state();
                }
                Err(e) => {
                    warn!(session = %session_id, error = %e, "persist failed");
                    let mut st = state.lock().expect("state");
                    st.persist_failed = true;
                    st.last_error = Some(serde_json::json!({
                        "errorClass": "StorageError",
                        "message": e.to_string(),
                    }));
                }
            }
        }
    }
}

fn apply_event_to_state(
    st: &mut SessionRuntimeState,
    event_type: &str,
    payload: &Value,
    session_id: &str,
    ts: &str,
) {
    let terminal = matches!(
        st.status,
        SessionStatus::Failed | SessionStatus::Disconnected | SessionStatus::Stopped
    );

    match event_type {
        "session.completed" if !terminal && st.status != SessionStatus::Cancelling => {
            if !st.persist_failed {
                st.status = SessionStatus::Ready;
            }
            st.prompt_in_flight = false;
        }
        "session.cancelled" => {
            st.status = SessionStatus::Stopped;
            st.prompt_in_flight = false;
            st.pending_approvals.clear();
        }
        "session.failed" => {
            st.status = SessionStatus::Failed;
            st.prompt_in_flight = false;
            st.last_error = Some(payload.clone());
        }
        "approval.requested" => {
            if !terminal {
                st.status = SessionStatus::AwaitingApproval;
            }
            if let Some(p) = pending_from_payload(session_id, payload, ts) {
                st.pending_approvals.insert(p.approval_id.clone(), p);
            }
        }
        "approval.resolved" => {
            if let Some(aid) = approval_id_from_payload(payload) {
                st.pending_approvals.remove(&aid);
            }
            if st.pending_approvals.is_empty() && st.status == SessionStatus::AwaitingApproval {
                st.status = if st.prompt_in_flight {
                    SessionStatus::Running
                } else {
                    SessionStatus::Ready
                };
            }
        }
        "session.ready" if !terminal => {
            st.status = SessionStatus::Ready;
        }
        "session.prompt.submitted" if !terminal => {
            st.status = SessionStatus::Running;
            st.prompt_in_flight = true;
        }
        "runtime.process.exited" => {
            let expected = payload
                .get("expected")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let code = payload.get("exitCode").and_then(|v| v.as_i64());
            if !expected {
                let is_crash = code.is_some_and(|c| c != 0);
                if is_crash {
                    st.status = SessionStatus::Failed;
                    st.prompt_in_flight = false;
                    st.last_error = Some(serde_json::json!({
                        "errorClass": "RuntimeCrashed",
                        "message": "runtime process crashed",
                        "exitCode": code,
                    }));
                } else if st.prompt_in_flight
                    || matches!(
                        st.status,
                        SessionStatus::Running | SessionStatus::AwaitingApproval
                    )
                {
                    st.status = SessionStatus::Disconnected;
                    st.prompt_in_flight = false;
                    st.last_error = Some(serde_json::json!({
                        "errorClass": "RuntimeDisconnected",
                        "message": "runtime EOF / unexpected exit",
                        "exitCode": code,
                    }));
                }
            }
        }
        "runtime.process.failed" => {
            st.status = SessionStatus::Failed;
            st.prompt_in_flight = false;
            st.last_error = Some(payload.clone());
        }
        "adapter.protocol.error" => {
            st.last_error = Some(payload.clone());
        }
        "adapter.protocol.unknown" => {}
        other => {
            if !terminal {
                if let Some(hint) = status_hint_from_event_type(other) {
                    if !matches!(
                        st.status,
                        SessionStatus::Running
                            | SessionStatus::AwaitingApproval
                            | SessionStatus::Cancelling
                    ) {
                        st.status = hint;
                    }
                }
            }
        }
    }
}

/// Submit prompt on a worker while ingestion continues.
pub fn submit_prompt_blocking(
    adapter: &RuntimeAdapter,
    prompt: PromptRequest,
) -> Result<(), tracer_runtime_adapter::AdapterError> {
    adapter.submit_prompt(prompt)
}

/// Resolve approval without holding session registry locks across the call.
pub fn resolve_approval_blocking(
    adapter: &RuntimeAdapter,
    decision: ApprovalDecisionRequest,
) -> Result<(), tracer_runtime_adapter::AdapterError> {
    adapter.resolve_approval(decision)
}

/// Cancel prompt; escalate to force_kill only after graceful/unsupported failure.
pub fn cancel_with_escalation(
    adapter: &RuntimeAdapter,
    force_on_unsupported: bool,
) -> Result<&'static str, tracer_runtime_adapter::AdapterError> {
    match adapter.cancel_prompt() {
        Ok(()) => Ok("cooperative"),
        Err(e)
            if e.error_class == tracer_domain::ErrorClass::CapabilityUnsupported
                && force_on_unsupported =>
        {
            adapter.force_kill()?;
            Ok("process_stop")
        }
        Err(e) => Err(e),
    }
}