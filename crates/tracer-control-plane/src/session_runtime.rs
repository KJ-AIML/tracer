//! Per-session runtime handle: adapter + event ingestor + local state.
//!
//! # Concurrency / drain strategy
//!
//! Adapter event channel is **unbounded** (W1-D). W1-F / W2-A mitigation path:
//!
//! ```text
//! adapter unbounded receiver
//!     -> OS drain thread (continuous)
//!     -> bounded tokio mpsc handoff (BRIDGE_CAPACITY)
//!     -> async persist pump -> SqliteStorage::append_event
//!     -> update canonical presentation projection (W2-A hub)
//!     -> bounded / coalescing notification (revision signal)
//! ```
//!
//! Bounded handoff applies backpressure when SQLite is slow so W1-F does not
//! add a second unbounded buffer. Drain uses `Sender::blocking_send` (Tokio)
//! so the async pump never blocks a worker on `std::mpsc::recv`. Presentation
//! is post-persist, never blocks the persist path on slow consumers, and does
//! not retain every intermediate notification indefinitely.
//!
//! This split avoids `Handle::block_on` deadlocks while keeping ingestion
//! alive during blocking `submit_prompt`, pending approval, and cancel. No
//! lock is held across long-running adapter RPCs.
//!
//! # Drain lifecycle (W2.2-C)
//!
//! Prompt/adapter return does **not** end ingestion. Authoritative completion:
//! terminal event persisted → state/presentation committed → late-event grace
//! → source closed or stop → drain joined → pump joined → runtime shutdown.
//! Expected channel close is **not** a `persist_error`. Real storage failures
//! remain counted and observable via `persist_errors` / `persist_failed`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver as StdReceiver};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
use crate::presentation::{PresentationHub, SessionProjectionInput};
use crate::session::lifecycle::{
    is_prompt_terminal_event, is_run_terminal_status, late_event_disposition, DrainLifecyclePhase,
    LateEventDisposition, LATE_DRAIN_JOIN_TIMEOUT,
};
use crate::types::PendingApprovalView;

/// Bounded internal handoff between OS drain and async persist pump.
/// Keeps adapter drain continuous while preventing unbounded->unbounded growth.
///
/// Exported for soak/stress tests that size bursts relative to bridge capacity.
pub const BRIDGE_CAPACITY: usize = 256;

/// Observability counters for the dual-stage ingest path (soak / diagnostics).
///
/// Counters are best-effort and do not affect control flow. Used by VS1-H3 soak
/// and W2.2-C drain lifecycle tests. `persist_errors` counts **real** storage
/// failures only — never expected channel close or normal late-drain exit.
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
    /// Persist failures (non-duplicate). Real storage errors only.
    pub persist_errors: AtomicU64,
    /// Presentation hub publish attempts after successful persist.
    pub presentation_sends: AtomicU64,
    /// Presentation publish skipped (hub shutdown / absent). Kept for metric shape.
    pub presentation_send_failures: AtomicU64,
    /// Adapter event source disconnected (expected lifecycle; not a persist error).
    pub channel_closes: AtomicU64,
    /// Prompt-cycle terminal events successfully persisted.
    pub terminal_persisted: AtomicU64,
    /// Events observed after a terminal was already persisted this cycle.
    pub late_events_observed: AtomicU64,
    /// Late events that were still applied (no status regression / upgrades).
    pub late_events_applied: AtomicU64,
    /// Late duplicate terminals counted without status churn.
    pub late_duplicate_terminals: AtomicU64,
    /// OS drain thread join completed.
    pub drain_joins: AtomicU64,
    /// Async persist pump exited cleanly (joined or finished before abort).
    pub pump_joins: AtomicU64,
    /// Pump was aborted after join timeout (should stay near zero).
    pub pump_aborts: AtomicU64,
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
            channel_closes: self.channel_closes.load(Ordering::Relaxed),
            terminal_persisted: self.terminal_persisted.load(Ordering::Relaxed),
            late_events_observed: self.late_events_observed.load(Ordering::Relaxed),
            late_events_applied: self.late_events_applied.load(Ordering::Relaxed),
            late_duplicate_terminals: self.late_duplicate_terminals.load(Ordering::Relaxed),
            drain_joins: self.drain_joins.load(Ordering::Relaxed),
            pump_joins: self.pump_joins.load(Ordering::Relaxed),
            pump_aborts: self.pump_aborts.load(Ordering::Relaxed),
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
    /// See [`IngestMetrics::channel_closes`].
    pub channel_closes: u64,
    /// See [`IngestMetrics::terminal_persisted`].
    pub terminal_persisted: u64,
    /// See [`IngestMetrics::late_events_observed`].
    pub late_events_observed: u64,
    /// See [`IngestMetrics::late_events_applied`].
    pub late_events_applied: u64,
    /// See [`IngestMetrics::late_duplicate_terminals`].
    pub late_duplicate_terminals: u64,
    /// See [`IngestMetrics::drain_joins`].
    pub drain_joins: u64,
    /// See [`IngestMetrics::pump_joins`].
    pub pump_joins: u64,
    /// See [`IngestMetrics::pump_aborts`].
    pub pump_aborts: u64,
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

/// Process-wide test inject for deterministic persist failures (W2.2-C).
/// Prefer this over env vars so suite tests cannot leak across cases.
static TEST_FORCE_PERSIST_ERROR: AtomicBool = AtomicBool::new(false);

/// Enable/disable forced persist failures (tests only). Production never calls this.
pub fn set_test_force_persist_error(on: bool) {
    TEST_FORCE_PERSIST_ERROR.store(on, Ordering::SeqCst);
}

fn test_force_persist_error() -> bool {
    if TEST_FORCE_PERSIST_ERROR.load(Ordering::SeqCst) {
        return true;
    }
    // Legacy env hook retained for soak scripts.
    std::env::var("TRACER_TEST_FORCE_PERSIST_ERROR")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
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
    /// Current drain lifecycle phase (W2.2-C).
    pub drain_phase: DrainLifecyclePhase,
    /// Last prompt-cycle terminal event type that was persisted (if any).
    pub last_terminal_event: Option<String>,
    /// True once a prompt-cycle terminal has been persisted for the current run.
    pub terminal_persisted: bool,
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
            drain_phase: DrainLifecyclePhase::RuntimeStarted,
            last_terminal_event: None,
            terminal_persisted: false,
        }
    }

    /// Mark a new prompt cycle (clears sticky terminal of prior run).
    pub fn begin_prompt_cycle(&mut self) {
        self.prompt_in_flight = true;
        self.terminal_persisted = false;
        self.last_terminal_event = None;
        self.drain_phase = self.drain_phase.advance_to(DrainLifecyclePhase::PromptActive);
    }

    /// Record that the adapter operation returned (ingestion may still be active).
    pub fn mark_adapter_operation_returned(&mut self) {
        self.drain_phase = self
            .drain_phase
            .advance_to(DrainLifecyclePhase::AdapterOperationReturned);
        if self.terminal_persisted {
            self.drain_phase = self
                .drain_phase
                .advance_to(DrainLifecyclePhase::LateEventGrace);
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
    /// Shared presentation hub (W2-A). Optional only before `start_ingestor`.
    pub presentation: Arc<Mutex<Option<PresentationHub>>>,
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
            presentation: Arc::new(Mutex::new(None)),
        }
    }

    /// Start continuous event drain into storage.
    ///
    /// `presentation` is the control-plane hub (shared). After each successful
    /// persist the pump updates the canonical projection and emits a coalesced
    /// notification; it never blocks on consumer drain.
    pub fn start_ingestor(&self, storage: SqliteStorage, presentation: Option<PresentationHub>) {
        if let Some(hub) = presentation {
            *self.presentation.lock().expect("presentation") = Some(hub);
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

        {
            let mut st = self.state.lock().expect("state");
            st.drain_phase = st
                .drain_phase
                .advance_to(DrainLifecyclePhase::EventDrainActive);
        }

        let drain = thread::Builder::new()
            .name(format!("cp-drain-{}", sid))
            .spawn(move || {
                drain_adapter(adapter_rx, bridge_tx, stop, sid, metrics_drain);
            })
            .expect("spawn drain");
        *self.drain_join.lock().expect("join") = Some(drain);

        let state = Arc::clone(&self.state);
        let presentation = Arc::clone(&self.presentation);
        let adapter = Arc::clone(&self.adapter);
        let stop2 = Arc::clone(&self.stop_ingest);
        let storage = storage.clone();
        let metrics_pump = Arc::clone(&self.metrics);
        let project_id = self.project_id;
        let pump = tokio::spawn(async move {
            async_persist_pump(
                bridge_rx,
                stop2,
                state,
                sid,
                project_id,
                storage,
                presentation,
                adapter,
                metrics_pump,
            )
            .await;
        });
        *self.pump_abort.lock().expect("pump") = Some(pump);
    }

    /// Signal stop, join OS drain (closes bridge), then join pump with timeout.
    ///
    /// Prefer this from async control-plane paths. Does **not** abort the pump
    /// until `LATE_DRAIN_JOIN_TIMEOUT` so in-flight terminal persists complete.
    pub async fn stop_ingestor_async(&self) {
        self.signal_stop_ingest();
        self.join_drain_thread();
        self.join_pump_async().await;
        let mut st = self.state.lock().expect("state");
        st.drain_phase = st
            .drain_phase
            .advance_to(DrainLifecyclePhase::RuntimeShutdown);
    }

    /// Sync stop path (Drop / legacy callers): join drain, spin-wait pump, abort if stuck.
    pub fn stop_ingestor(&self) {
        self.signal_stop_ingest();
        self.join_drain_thread();
        self.join_pump_sync();
        let mut st = self.state.lock().expect("state");
        st.drain_phase = st
            .drain_phase
            .advance_to(DrainLifecyclePhase::RuntimeShutdown);
    }

    fn signal_stop_ingest(&self) {
        self.stop_ingest.store(true, Ordering::SeqCst);
        let mut st = self.state.lock().expect("state");
        st.drain_phase = st
            .drain_phase
            .advance_to(DrainLifecyclePhase::SourceClosedOrBoundedDrainComplete);
    }

    fn join_drain_thread(&self) {
        if let Ok(mut g) = self.drain_join.lock() {
            if let Some(h) = g.take() {
                let _ = h.join();
                self.metrics.drain_joins.fetch_add(1, Ordering::Relaxed);
                let mut st = self.state.lock().expect("state");
                st.drain_phase = st
                    .drain_phase
                    .advance_to(DrainLifecyclePhase::DrainTaskJoined);
            }
        }
    }

    async fn join_pump_async(&self) {
        let mut handle = match self.pump_abort.lock().expect("pump").take() {
            Some(h) => h,
            None => return,
        };
        let timeout = LATE_DRAIN_JOIN_TIMEOUT;
        tokio::select! {
            res = &mut handle => {
                let _ = res;
                self.metrics.pump_joins.fetch_add(1, Ordering::Relaxed);
            }
            _ = tokio::time::sleep(timeout) => {
                warn!(
                    session = %self.session_id,
                    "persist pump join timed out; aborting residual pump"
                );
                handle.abort();
                let _ = handle.await;
                self.metrics.pump_aborts.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn join_pump_sync(&self) {
        let handle = match self.pump_abort.lock().expect("pump").take() {
            Some(h) => h,
            None => return,
        };
        let deadline = Instant::now() + LATE_DRAIN_JOIN_TIMEOUT;
        while !handle.is_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        if handle.is_finished() {
            // Drop finished handle (result discarded).
            drop(handle);
            self.metrics.pump_joins.fetch_add(1, Ordering::Relaxed);
        } else {
            handle.abort();
            // Brief wait after abort so Drop does not race OS threads.
            let abort_deadline = Instant::now() + Duration::from_millis(200);
            while !handle.is_finished() && Instant::now() < abort_deadline {
                thread::sleep(Duration::from_millis(5));
            }
            drop(handle);
            self.metrics.pump_aborts.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Current lifecycle phase snapshot.
    pub fn drain_phase(&self) -> DrainLifecyclePhase {
        self.state.lock().expect("state").drain_phase
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
        // Join drain fully; join/abort pump with timeout so Drop does not leak.
        if let Ok(mut g) = self.drain_join.lock() {
            if let Some(h) = g.take() {
                let _ = h.join();
                self.metrics.drain_joins.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.join_pump_sync();
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
                        // Bridge closed (pump dropped) — lifecycle, not persist_error.
                        metrics.bridge_send_failures.fetch_add(1, Ordering::Relaxed);
                        break;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Expected channel close when adapter shuts down / drops sender.
                // Must never increment persist_errors.
                metrics.channel_closes.fetch_add(1, Ordering::Relaxed);
                debug!(session = %session_id, "adapter event channel closed (expected lifecycle)");
                break;
            }
        }
    }
    debug!(session = %session_id, "adapter drain stopped");
    // Dropping tx closes bridge for async pump → pump drains remaining then exits.
}

async fn async_persist_pump(
    mut rx: TokioReceiver<AdapterEvent>,
    stop: Arc<AtomicBool>,
    state: Arc<Mutex<SessionRuntimeState>>,
    session_id: SessionId,
    project_id: tracer_storage::ProjectId,
    storage: SqliteStorage,
    presentation: Arc<Mutex<Option<PresentationHub>>>,
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
                        persist_one(
                            &ev,
                            &state,
                            session_id,
                            project_id,
                            &storage,
                            &presentation,
                            &adapter,
                            &metrics,
                        )
                        .await;
                        // Drain any already-queued events without yielding extra.
                        while let Ok(ev) = rx.try_recv() {
                            persist_one(
                                &ev,
                                &state,
                                session_id,
                                project_id,
                                &storage,
                                &presentation,
                                &adapter,
                                &metrics,
                            )
                            .await;
                        }
                    }
                    None => break,
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(50)), if stop.load(Ordering::SeqCst) => {
                // Stop requested: finish remaining then exit.
                while let Ok(ev) = rx.try_recv() {
                    persist_one(
                        &ev,
                        &state,
                        session_id,
                        project_id,
                        &storage,
                        &presentation,
                        &adapter,
                        &metrics,
                    )
                    .await;
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
    project_id: tracer_storage::ProjectId,
    storage: &SqliteStorage,
    presentation: &Arc<Mutex<Option<PresentationHub>>>,
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

            // Observe terminal before persist so phase reflects drain progress.
            if is_prompt_terminal_event(&event_type) {
                let mut st = state.lock().expect("state");
                st.drain_phase = st
                    .drain_phase
                    .advance_to(DrainLifecyclePhase::AdapterTerminalObserved);
            }

            // Late-event bookkeeping (policy applied after successful persist).
            let late = {
                let st = state.lock().expect("state");
                st.terminal_persisted
            };
            if late {
                metrics.late_events_observed.fetch_add(1, Ordering::Relaxed);
            }

            // Deterministic test hook: force storage failure without redesigning SQLite.
            if test_force_persist_error() {
                warn!(
                    session = %session_id,
                    event_type = %event_type,
                    "TRACER_TEST_FORCE_PERSIST_ERROR: counting real persist failure"
                );
                metrics.persist_errors.fetch_add(1, Ordering::Relaxed);
                let mut st = state.lock().expect("state");
                st.persist_failed = true;
                st.last_error = Some(serde_json::json!({
                    "errorClass": "StorageError",
                    "message": "injected persist failure (TRACER_TEST_FORCE_PERSIST_ERROR)",
                }));
                return;
            }

            // Soak-only artificial latency (TRACER_SOAK_PERSIST_DELAY_MS).
            let delay = soak_persist_delay();
            if !delay.is_zero() {
                tokio::time::sleep(delay).await;
            }

            // Bounded retries: multi-session WAL writers + sequence races must not
            // inflate persist_errors for recoverable contention. True durable
            // failures still count after the budget is exhausted.
            const PERSIST_ATTEMPTS: u32 = 8;
            let mut last_err: Option<tracer_storage::StorageError> = None;
            let mut stored_ok = false;
            for attempt in 0..PERSIST_ATTEMPTS {
                let record = envelope_to_event_record(env);
                match storage.append_event(record).await {
                    Ok(stored) => {
                        on_persist_success(
                            metrics,
                            state,
                            session_id,
                            project_id,
                            storage,
                            presentation,
                            adapter,
                            &event_type,
                            &payload,
                            &session_id.to_string(),
                            &ts,
                            stored.sequence,
                            late,
                        )
                        .await;
                        stored_ok = true;
                        break;
                    }
                    Err(tracer_storage::StorageError::AlreadyExists { entity, id }) => {
                        warn!(
                            session = %session_id,
                            entity = %entity,
                            id = %id,
                            attempt,
                            "persist unique conflict; retrying"
                        );
                        last_err = Some(tracer_storage::StorageError::AlreadyExists {
                            entity,
                            id,
                        });
                        // Brief yield so peer writers / next_sequence advance.
                        tokio::time::sleep(Duration::from_millis(5 * (1u64 << attempt.min(4)))).await;
                    }
                    Err(e) => {
                        // Transient database lock / IO: retry. Other errors also
                        // get a short retry budget before counting as real failure.
                        warn!(
                            session = %session_id,
                            attempt,
                            error = %e,
                            "persist attempt failed; may retry"
                        );
                        last_err = Some(e);
                        tokio::time::sleep(Duration::from_millis(15 * (1u64 << attempt.min(4)))).await;
                    }
                }
            }

            if !stored_ok {
                match last_err {
                    Some(tracer_storage::StorageError::AlreadyExists { entity, id }) => {
                        // Unrecoverable duplicate / sequence stall: apply state only.
                        // Not a false storage failure — count as duplicate, not persist_error.
                        debug!(
                            session = %session_id,
                            entity = %entity,
                            id = %id,
                            "duplicate event ignored after retries"
                        );
                        metrics.events_duplicate.fetch_add(1, Ordering::Relaxed);
                        let mut st = state.lock().expect("state");
                        apply_event_to_state(
                            &mut st,
                            &event_type,
                            &payload,
                            &session_id.to_string(),
                            &ts,
                            metrics,
                        );
                        st.auth_state = adapter.auth_state();
                    }
                    Some(e) => {
                        warn!(session = %session_id, error = %e, "persist failed after retries");
                        metrics.persist_errors.fetch_add(1, Ordering::Relaxed);
                        let mut st = state.lock().expect("state");
                        st.persist_failed = true;
                        st.last_error = Some(serde_json::json!({
                            "errorClass": "StorageError",
                            "message": e.to_string(),
                        }));
                    }
                    None => {
                        metrics.persist_errors.fetch_add(1, Ordering::Relaxed);
                        let mut st = state.lock().expect("state");
                        st.persist_failed = true;
                    }
                }
            }
        }
    }
}

/// Shared post-persist path: state, presentation (post-persist only), terminal phase.
async fn on_persist_success(
    metrics: &IngestMetrics,
    state: &Arc<Mutex<SessionRuntimeState>>,
    session_id: SessionId,
    project_id: tracer_storage::ProjectId,
    storage: &SqliteStorage,
    presentation: &Arc<Mutex<Option<PresentationHub>>>,
    adapter: &RuntimeAdapter,
    event_type: &str,
    payload: &Value,
    session_id_str: &str,
    ts: &str,
    seq: i64,
    was_late: bool,
) {
    metrics.events_persisted.fetch_add(1, Ordering::Relaxed);
    {
        let mut st = state.lock().expect("state");
        st.latest_sequence = seq;
        apply_event_to_state(&mut st, event_type, payload, session_id_str, ts, metrics);
        st.auth_state = adapter.auth_state();

        if is_prompt_terminal_event(event_type) {
            st.terminal_persisted = true;
            st.last_terminal_event = Some(event_type.to_string());
            st.drain_phase = st
                .drain_phase
                .advance_to(DrainLifecyclePhase::TerminalPersisted);
            metrics.terminal_persisted.fetch_add(1, Ordering::Relaxed);
        }
        if was_late {
            metrics.late_events_applied.fetch_add(1, Ordering::Relaxed);
        }
    }

    // Presentation only after successful persist (terminal or not).
    publish_presentation(
        presentation,
        state,
        session_id,
        project_id,
        adapter,
        metrics,
    );

    if is_prompt_terminal_event(event_type) {
        let mut st = state.lock().expect("state");
        st.drain_phase = st
            .drain_phase
            .advance_to(DrainLifecyclePhase::TerminalStateCommitted);
        if matches!(
            st.drain_phase,
            DrainLifecyclePhase::AdapterOperationReturned
                | DrainLifecyclePhase::TerminalStateCommitted
        ) {
            st.drain_phase = st
                .drain_phase
                .advance_to(DrainLifecyclePhase::LateEventGrace);
        }
    }

    let status_now = state.lock().expect("state").status;
    let _ = storage.update_session_status(&session_id, status_now).await;
}

/// Post-persist presentation publish: projection update + coalesced notify.
/// Never blocks on consumers; never queues full event history.
fn publish_presentation(
    presentation: &Arc<Mutex<Option<PresentationHub>>>,
    state: &Arc<Mutex<SessionRuntimeState>>,
    session_id: SessionId,
    project_id: tracer_storage::ProjectId,
    adapter: &RuntimeAdapter,
    metrics: &IngestMetrics,
) {
    let hub = match presentation.lock().expect("presentation").clone() {
        Some(h) => h,
        None => return,
    };
    if hub.is_shutdown() {
        metrics
            .presentation_send_failures
            .fetch_add(1, Ordering::Relaxed);
        return;
    }
    let input = {
        let st = state.lock().expect("state");
        SessionProjectionInput {
            session_id: session_id.to_string(),
            project_id: project_id.to_string(),
            status: st.status,
            auth_state: st.auth_state,
            pending_approvals: st.pending_approvals.values().cloned().collect(),
            last_error: st.last_error.clone(),
            capabilities: st.capabilities.clone(),
            latest_sequence: st.latest_sequence,
            prompt_in_flight: st.prompt_in_flight,
            process_alive: adapter.is_process_alive(),
            protocol_ready: adapter.is_protocol_ready(),
            session_ready: adapter.is_session_ready(),
        }
    };
    metrics.presentation_sends.fetch_add(1, Ordering::Relaxed);
    let _ = hub.publish_session_update(input);
}

fn apply_event_to_state(
    st: &mut SessionRuntimeState,
    event_type: &str,
    payload: &Value,
    session_id: &str,
    ts: &str,
    metrics: &IngestMetrics,
) {
    let hard_terminal = matches!(
        st.status,
        SessionStatus::Failed | SessionStatus::Disconnected | SessionStatus::Stopped
    );
    // Prompt-cycle terminal already persisted: apply late-event policy.
    let already_prompt_terminal = st.terminal_persisted || hard_terminal;
    let disposition = late_event_disposition(
        already_prompt_terminal,
        st.last_terminal_event.as_deref(),
        event_type,
    );

    match disposition {
        LateEventDisposition::ExpectedChannelClose => return,
        LateEventDisposition::DuplicateTerminal => {
            metrics
                .late_duplicate_terminals
                .fetch_add(1, Ordering::Relaxed);
            // Persist already happened; do not churn status or reopen the run.
            st.prompt_in_flight = false;
            return;
        }
        LateEventDisposition::PersistNoStatusRegression => {
            // May update non-status maps (e.g. clear approvals) but never reopen
            // a finished run or clear hard terminal status.
            match event_type {
                "approval.resolved" => {
                    if let Some(aid) = approval_id_from_payload(payload) {
                        st.pending_approvals.remove(&aid);
                    }
                }
                "approval.requested" => {
                    // Late approval after terminal: keep for audit map only if still useful;
                    // do not move status back to AwaitingApproval.
                    if let Some(p) = pending_from_payload(session_id, payload, ts) {
                        st.pending_approvals.insert(p.approval_id.clone(), p);
                    }
                }
                "adapter.protocol.error" => {
                    st.last_error = Some(payload.clone());
                }
                "adapter.protocol.unknown" => {}
                // Late deltas / ready / submitted: ignore status transitions.
                _ => {}
            }
            return;
        }
        LateEventDisposition::ApplyFully => {}
    }

    let terminal = hard_terminal
        || (st.terminal_persisted && is_run_terminal_status(st.status));

    match event_type {
        "session.completed" if !hard_terminal && st.status != SessionStatus::Cancelling => {
            if !st.persist_failed {
                // Successful prompt cycle returns to Ready (session remains usable).
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
