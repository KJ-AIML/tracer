//! Per-session runtime handle: adapter + event ingestor + local state.
//!
//! # Concurrency / drain strategy
//!
//! Adapter event channel is **unbounded** (W1-D). W1-F mitigation path:
//!
//! ```text
//! adapter unbounded receiver
//!     -> OS drain thread (continuous)
//!     -> bounded tokio mpsc handoff (BRIDGE_CAPACITY)
//!     -> async persist pump -> SqliteStorage::append_event
//!     -> optional presentation fan-out (after persist)
//! ```
//!
//! Bounded handoff applies backpressure when SQLite is slow so W1-F does not
//! add a second unbounded buffer. Drain uses `Sender::blocking_send` (Tokio)
//! so the async pump never blocks a worker on `std::mpsc::recv`. Presentation
//! is post-persist and must not block the adapter channel indefinitely.
//!
//! This split avoids `Handle::block_on` deadlocks while keeping ingestion
//! alive during blocking `submit_prompt`, pending approval, and cancel. No
//! lock is held across long-running adapter RPCs.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver as StdReceiver, Sender as StdSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc::{
    channel as tokio_channel, Receiver as TokioReceiver, Sender as TokioSender,
};
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

/// Bounded internal handoff between OS drain and async persist pump.
/// Keeps adapter drain continuous while preventing unbounded->unbounded growth.
///
/// Exported for soak/stress tests that size bursts relative to bridge capacity.
pub const BRIDGE_CAPACITY: usize = 256;

/// Observability counters for the dual-stage ingest path (soak / diagnostics).
///
/// Counters are best-effort and do not affect control flow. Used by VS1-H3 soak
/// to measure drain throughput, persist success, and presentation fan-out without
/// redesigning the control plane.
#[derive(Debug, Default)]
pub struct IngestMetrics {
    /// Adapter events accepted into the bounded bridge (`blocking_send` ok).
    pub bridge_accepted: AtomicU64,
    /// `blocking_send` failures (bridge closed / receiver dropped).
    pub bridge_send_failures: AtomicU64,
    /// Events successfully appended to SQLite.
    pub events_persisted: AtomicU64,
    /// Duplicate event_id ignored by storage.
    pub events_duplicate: AtomicU64,
    /// Persist failures (non-duplicate).
    pub persist_errors: AtomicU64,
    /// Presentation fan-out send attempts after successful persist.
    pub presentation_sends: AtomicU64,
    /// Presentation fan-out send failures (disconnected consumer).
    pub presentation_send_failures: AtomicU64,
}

impl IngestMetrics {
    /// Snapshot for tests / soak reports.
    pub fn snapshot(&self) -> IngestMetricsSnapshot {
        IngestMetricsSnapshot {
            bridge_accepted: self.bridge_accepted.load(Ordering::Relaxed),
            bridge_send_failures: self.bridge_send_failures.load(Ordering::Relaxed),
            events_persisted: self.events_persisted.load(Ordering::Relaxed),
            events_duplicate: self.events_duplicate.load(Ordering::Relaxed),
            persist_errors: self.persist_errors.load(Ordering::Relaxed),
            presentation_sends: self.presentation_sends.load(Ordering::Relaxed),
            presentation_send_failures: self.presentation_send_failures.load(Ordering::Relaxed),
        }
    }
}

/// Owned snapshot of [`IngestMetrics`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IngestMetricsSnapshot {
    /// See [`IngestMetrics::bridge_accepted`].
    pub bridge_accepted: u64,
    /// See [`IngestMetrics::bridge_send_failures`].
    pub bridge_send_failures: u64,
    /// See [`IngestMetrics::events_persisted`].
    pub events_persisted: u64,
    /// See [`IngestMetrics::events_duplicate`].
    pub events_duplicate: u64,
    /// See [`IngestMetrics::persist_errors`].
    pub persist_errors: u64,
    /// See [`IngestMetrics::presentation_sends`].
    pub presentation_sends: u64,
    /// See [`IngestMetrics::presentation_send_failures`].
    pub presentation_send_failures: u64,
}

/// Optional artificial persist delay for soak "slow database" injection.
///
/// Set env `TRACER_SOAK_PERSIST_DELAY_MS` to a positive integer. Production
/// paths leave this unset (zero delay). Not a production SLA hook.
///
/// Read on each persist so soak tests can toggle mid-process (no OnceLock).
fn soak_persist_delay() -> Duration {
    std::env::var("TRACER_SOAK_PERSIST_DELAY_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|ms| *ms > 0)
        .map(Duration::from_millis)
        .unwrap_or(Duration::ZERO)
}

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
    /// Dual-stage ingest counters (soak / diagnostics).
    pub metrics: Arc<IngestMetrics>,
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
            metrics: Arc::new(IngestMetrics::default()),
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

        // Bounded bridge: adapter unbounded -> tokio mpsc(BRIDGE_CAPACITY) -> async pump.
        let (bridge_tx, bridge_rx) = tokio_channel::<AdapterEvent>(BRIDGE_CAPACITY);
        let stop = Arc::clone(&self.stop_ingest);
        let sid = self.session_id;
        let metrics_drain = Arc::clone(&self.metrics);

        let drain = thread::Builder::new()
            .name(format!("cp-drain-{}", sid))
            .spawn(move || {
                drain_adapter(adapter_rx, bridge_tx, stop, sid, metrics_drain);
            })
            .expect("spawn drain");
        *self.drain_join.lock().expect("join") = Some(drain);

        let state = Arc::clone(&self.state);
        let fanout = Arc::clone(&self.event_fanout);
        let adapter = Arc::clone(&self.adapter);
        let stop2 = Arc::clone(&self.stop_ingest);
        let storage = storage.clone();
        let metrics_pump = Arc::clone(&self.metrics);
        let pump = tokio::spawn(async move {
            async_persist_pump(
                bridge_rx,
                stop2,
                state,
                sid,
                storage,
                fanout,
                adapter,
                metrics_pump,
            )
            .await;
        });
        *self.pump_abort.lock().expect("pump") = Some(pump);
    }

    pub fn stop_ingestor(&self) {
        self.stop_ingest.store(true, Ordering::SeqCst);
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
    tx: TokioSender<AdapterEvent>,
    stop: Arc<AtomicBool>,
    session_id: SessionId,
    metrics: Arc<IngestMetrics>,
) {
    debug!(session = %session_id, "adapter drain started");
    loop {
        if stop.load(Ordering::SeqCst) {
            // Best-effort non-blocking flush on stop.
            while let Ok(ev) = rx.try_recv() {
                match tx.try_send(ev) {
                    Ok(()) => {
                        metrics.bridge_accepted.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        metrics.bridge_send_failures.fetch_add(1, Ordering::Relaxed);
                        break;
                    }
                }
            }
            break;
        }
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(ev) => {
                // blocking_send applies backpressure without Handle::block_on.
                // On stop, try_send to avoid hanging if the pump is full.
                if stop.load(Ordering::SeqCst) {
                    match tx.try_send(ev) {
                        Ok(()) => {
                            metrics.bridge_accepted.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(_) => {
                            metrics.bridge_send_failures.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    break;
                }
                match tx.blocking_send(ev) {
                    Ok(()) => {
                        metrics.bridge_accepted.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(_) => {
                        metrics.bridge_send_failures.fetch_add(1, Ordering::Relaxed);
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    debug!(session = %session_id, "adapter drain stopped");
    // Dropping tx closes bridge for async pump.
}

async fn async_persist_pump(
    mut rx: TokioReceiver<AdapterEvent>,
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<SessionRuntimeState>>,
    session_id: SessionId,
    storage: SqliteStorage,
    fanout: Arc<Mutex<Option<StdSender<PresentationEvent>>>>,
    adapter: Arc<RuntimeAdapter>,
    metrics: Arc<IngestMetrics>,
) {
    debug!(session = %session_id, "async persist pump started");
    loop {
        tokio::select! {
            biased;
            maybe = rx.recv() => {
                match maybe {
                    Some(ev) => {
                        persist_one(&ev, &state, session_id, &storage, &fanout, &adapter, &metrics).await;
                        // Drain any already-queued events without yielding extra.
                        while let Ok(ev) = rx.try_recv() {
                            persist_one(&ev, &state, session_id, &storage, &fanout, &adapter, &metrics).await;
                        }
                    }
                    None => break,
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)), if stop.load(Ordering::SeqCst) => {
                // Stop requested: finish remaining then exit.
                while let Ok(ev) = rx.try_recv() {
                    persist_one(&ev, &state, session_id, &storage, &fanout, &adapter, &metrics).await;
                }
                if rx.is_empty() {
                    break;
                }
            }
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
    metrics: &IngestMetrics,
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

            // Soak-only artificial latency (TRACER_SOAK_PERSIST_DELAY_MS).
            let delay = soak_persist_delay();
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }

            match storage.append_event(record).await {
                Ok(stored) => {
                    metrics.events_persisted.fetch_add(1, Ordering::Relaxed);
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
                    if let Ok(g) = fanout.lock() {
                        if let Some(tx) = g.as_ref() {
                            metrics.presentation_sends.fetch_add(1, Ordering::Relaxed);
                            if tx
                                .send(PresentationEvent {
                                    batch: false,
                                    events: vec![envelope_json],
                                })
                                .is_err()
                            {
                                metrics
                                    .presentation_send_failures
                                    .fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    let status_now = state.lock().expect("state").status;
                    let _ = storage.update_session_status(&session_id, status_now).await;
                }
                Err(tracer_storage::StorageError::AlreadyExists { entity, id }) => {
                    // Under burst load SQLite may surface UNIQUE on (session_id, sequence)
                    // when next_sequence is briefly contended. Retry once with a fresh
                    // storage event_id; append_event re-reads next_sequence each call.
                    warn!(
                        session = %session_id,
                        entity = %entity,
                        id = %id,
                        "persist unique conflict; retrying once"
                    );
                    let retry = envelope_to_event_record(env);
                    match storage.append_event(retry).await {
                        Ok(stored) => {
                            metrics.events_persisted.fetch_add(1, Ordering::Relaxed);
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
                            if let Ok(g) = fanout.lock() {
                                if let Some(tx) = g.as_ref() {
                                    metrics.presentation_sends.fetch_add(1, Ordering::Relaxed);
                                    if tx
                                        .send(PresentationEvent {
                                            batch: false,
                                            events: vec![envelope_json],
                                        })
                                        .is_err()
                                    {
                                        metrics
                                            .presentation_send_failures
                                            .fetch_add(1, Ordering::Relaxed);
                                    }
                                }
                            }
                            let status_now = state.lock().expect("state").status;
                            let _ = storage.update_session_status(&session_id, status_now).await;
                        }
                        Err(tracer_storage::StorageError::AlreadyExists { entity, id }) => {
                            // True duplicate (or unrecoverable sequence stall): apply state only.
                            debug!(
                                session = %session_id,
                                entity = %entity,
                                id = %id,
                                "duplicate event ignored after retry"
                            );
                            metrics.events_duplicate.fetch_add(1, Ordering::Relaxed);
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
                            warn!(session = %session_id, error = %e, "persist retry failed");
                            metrics.persist_errors.fetch_add(1, Ordering::Relaxed);
                            let mut st = state.lock().expect("state");
                            st.persist_failed = true;
                            st.last_error = Some(serde_json::json!({
                                "errorClass": "StorageError",
                                "message": e.to_string(),
                            }));
                        }
                    }
                }
                Err(e) => {
                    warn!(session = %session_id, error = %e, "persist failed");
                    metrics.persist_errors.fetch_add(1, Ordering::Relaxed);
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
