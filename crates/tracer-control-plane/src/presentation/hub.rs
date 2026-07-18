//! Watch-style presentation hub with coalescing notifications.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{
    sync_channel, Receiver as StdReceiver, RecvTimeoutError, Sender as StdSender, SyncSender,
    TrySendError,
};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tokio::sync::watch;
use tracer_domain::SessionStatus;
use tracing::debug;

use crate::convert::runtime_observation;
use crate::types::{PresentationEvent, PresentationNotify, PresentationSnapshot, SNAPSHOT_VERSION};

/// Default capacity for per-consumer notification channels (coalescing).
pub const DEFAULT_NOTIFY_CAPACITY: usize = 1;

/// Inputs needed to project a live session into the canonical snapshot.
#[derive(Debug, Clone)]
pub struct SessionProjectionInput {
    pub session_id: String,
    pub project_id: String,
    pub status: SessionStatus,
    pub auth_state: tracer_domain::AuthenticationState,
    pub pending_approvals: Vec<crate::types::PendingApprovalView>,
    pub last_error: Option<serde_json::Value>,
    pub capabilities: Option<serde_json::Value>,
    pub latest_sequence: i64,
    pub prompt_in_flight: bool,
    pub process_alive: bool,
    pub protocol_ready: bool,
    pub session_ready: bool,
}

/// Observability counters for presentation delivery (do not affect control flow).
#[derive(Debug, Default)]
pub struct PresentationMetrics {
    /// Successful projection publishes (revision bumps).
    pub publishes: AtomicU64,
    /// Notification try_send dropped because the consumer channel was full (coalesced).
    pub notify_coalesced: AtomicU64,
    /// Notification try_send failures (disconnected consumer).
    pub notify_send_failures: AtomicU64,
    /// Successful notification delivers to a consumer slot.
    pub notify_sends: AtomicU64,
    /// Legacy event-sink try_send drops (full).
    pub event_drops: AtomicU64,
    /// Legacy event-sink successful try_sends.
    pub event_sends: AtomicU64,
    /// Consumers registered (subscribe / legacy).
    pub consumers_registered: AtomicU64,
    /// Consumers removed (drop / disconnect / shutdown).
    pub consumers_removed: AtomicU64,
}

impl PresentationMetrics {
    /// Snapshot counters for tests / soak.
    pub fn snapshot(&self) -> PresentationMetricsSnapshot {
        PresentationMetricsSnapshot {
            publishes: self.publishes.load(Ordering::Relaxed),
            notify_coalesced: self.notify_coalesced.load(Ordering::Relaxed),
            notify_send_failures: self.notify_send_failures.load(Ordering::Relaxed),
            notify_sends: self.notify_sends.load(Ordering::Relaxed),
            event_drops: self.event_drops.load(Ordering::Relaxed),
            event_sends: self.event_sends.load(Ordering::Relaxed),
            consumers_registered: self.consumers_registered.load(Ordering::Relaxed),
            consumers_removed: self.consumers_removed.load(Ordering::Relaxed),
        }
    }
}

/// Owned snapshot of [`PresentationMetrics`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PresentationMetricsSnapshot {
    pub publishes: u64,
    pub notify_coalesced: u64,
    pub notify_send_failures: u64,
    pub notify_sends: u64,
    pub event_drops: u64,
    pub event_sends: u64,
    pub consumers_registered: u64,
    pub consumers_removed: u64,
}

struct HubState {
    snapshot: PresentationSnapshot,
    revision: u64,
    /// Sticky terminal flag: once a terminal status is published for the active
    /// session, remains true until a non-terminal publish for a (possibly new)
    /// active session clears it. Ensures terminal cannot be permanently missed.
    terminal_sticky: bool,
}

struct NotifySink {
    tx: SyncSender<PresentationNotify>,
}

struct EventSink {
    tx: SyncSender<PresentationEvent>,
}

struct LegacyForwarder {
    join: JoinHandle<()>,
}

struct HubInner {
    state: RwLock<HubState>,
    /// Coalescing multi-consumer revision watch.
    revision_tx: watch::Sender<u64>,
    notify_sinks: Mutex<HashMap<u64, NotifySink>>,
    event_sinks: Mutex<HashMap<u64, EventSink>>,
    next_id: AtomicU64,
    shutdown: AtomicBool,
    legacy_forwarders: Mutex<Vec<LegacyForwarder>>,
    metrics: PresentationMetrics,
}

/// Canonical presentation projection + bounded delivery.
///
/// Clone is cheap (`Arc`); all clones share delivery state.
#[derive(Clone)]
pub struct PresentationHub {
    inner: Arc<HubInner>,
}

impl PresentationHub {
    /// Create a hub with an initial snapshot (revision 0 until first publish).
    pub fn new(initial: PresentationSnapshot) -> Self {
        let revision = initial.revision;
        let (revision_tx, _) = watch::channel(revision);
        Self {
            inner: Arc::new(HubInner {
                state: RwLock::new(HubState {
                    snapshot: initial,
                    revision,
                    terminal_sticky: false,
                }),
                revision_tx,
                notify_sinks: Mutex::new(HashMap::new()),
                event_sinks: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(1),
                shutdown: AtomicBool::new(false),
                legacy_forwarders: Mutex::new(Vec::new()),
                metrics: PresentationMetrics::default(),
            }),
        }
    }

    /// Delivery metrics (best-effort).
    pub fn metrics(&self) -> PresentationMetricsSnapshot {
        self.inner.metrics.snapshot()
    }

    /// Whether [`Self::shutdown`] has been called.
    pub fn is_shutdown(&self) -> bool {
        self.inner.shutdown.load(Ordering::SeqCst)
    }

    /// Current monotonic revision.
    pub fn revision(&self) -> u64 {
        self.inner.state.read().expect("hub state").revision
    }

    /// Latest canonical snapshot (always recoverable; independent of consumers).
    pub fn snapshot(&self) -> PresentationSnapshot {
        self.inner.state.read().expect("hub state").snapshot.clone()
    }

    /// Sticky terminal flag for the active projection.
    pub fn terminal_sticky(&self) -> bool {
        self.inner.state.read().expect("hub state").terminal_sticky
    }

    /// Replace the full snapshot (command paths / heli refresh) and notify.
    ///
    /// Preserves and increments revision. Schema `version` is forced to
    /// [`SNAPSHOT_VERSION`].
    pub fn publish_snapshot(&self, mut snapshot: PresentationSnapshot) -> u64 {
        if self.is_shutdown() {
            return self.revision();
        }
        snapshot.version = SNAPSHOT_VERSION;
        let terminal = snapshot
            .session_status
            .map(is_terminal_status)
            .unwrap_or(false);
        self.commit(snapshot, terminal)
    }

    /// Project a live session into the hub when it is the active session
    /// (or when no active session is set yet).
    ///
    /// Always safe to call after persist; no-ops (without revision bump) when
    /// the update is for a non-active session. Returns the current revision.
    pub fn publish_session_update(&self, input: SessionProjectionInput) -> u64 {
        if self.is_shutdown() {
            return self.revision();
        }

        let mut base = self.snapshot();
        let active = base.active_session_id.as_deref();
        let applies = match active {
            None => true,
            Some(id) => id == input.session_id,
        };
        if !applies {
            return base.revision;
        }

        base.active_project_id = Some(input.project_id);
        base.active_session_id = Some(input.session_id);
        base.session_status = Some(input.status);
        base.auth_state = input.auth_state;
        base.pending_approvals = input.pending_approvals;
        base.last_error = input.last_error;
        base.capabilities = input.capabilities;
        base.latest_sequence = input.latest_sequence;
        base.prompt_in_flight = input.prompt_in_flight;
        base.runtime_observation = runtime_observation(
            input.process_alive,
            input.protocol_ready,
            input.session_ready,
            input.status,
        );
        let terminal = is_terminal_status(input.status);
        self.commit(base, terminal)
    }

    /// Subscribe to coalescing revision notifications (async watch).
    pub fn subscribe(&self) -> PresentationSubscription {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let revision_rx = self.inner.revision_tx.subscribe();
        // Also register a capacity-1 notify channel for std/tests.
        let (tx, rx) = sync_channel::<PresentationNotify>(DEFAULT_NOTIFY_CAPACITY);
        {
            let mut sinks = self.inner.notify_sinks.lock().expect("notify sinks");
            sinks.insert(id, NotifySink { tx });
        }
        self.inner
            .metrics
            .consumers_registered
            .fetch_add(1, Ordering::Relaxed);
        PresentationSubscription {
            hub: Arc::clone(&self.inner),
            id,
            revision_rx,
            notify_rx: Some(rx),
            detached: false,
        }
    }

    /// Subscribe with only a bounded std notify receiver (no watch handle).
    ///
    /// Capacity is [`DEFAULT_NOTIFY_CAPACITY`] (1) — full channel coalesces.
    pub fn subscribe_notify(&self) -> (u64, StdReceiver<PresentationNotify>) {
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = sync_channel::<PresentationNotify>(DEFAULT_NOTIFY_CAPACITY);
        {
            let mut sinks = self.inner.notify_sinks.lock().expect("notify sinks");
            sinks.insert(id, NotifySink { tx });
        }
        self.inner
            .metrics
            .consumers_registered
            .fetch_add(1, Ordering::Relaxed);
        (id, rx)
    }

    /// Drop a notify-only subscription by id.
    pub fn unsubscribe(&self, id: u64) {
        let removed = {
            let mut sinks = self.inner.notify_sinks.lock().expect("notify sinks");
            sinks.remove(&id).is_some()
        };
        if removed {
            self.inner
                .metrics
                .consumers_removed
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Legacy SOAK / UI API: attach an unbounded `Sender` **without** queuing
    /// every event into it.
    ///
    /// Internally uses a capacity-1 bridge so the persist path never blocks and
    /// never retains unbounded intermediate notifications. A dedicated forwarder
    /// thread may block on the user sender if the consumer is slow; that cannot
    /// back-pressure persistence. At most one pending event sits on the bridge.
    pub fn attach_legacy_sender(&self, user_tx: StdSender<PresentationEvent>) {
        if self.is_shutdown() {
            return;
        }
        let (bridge_tx, bridge_rx) = sync_channel::<PresentationEvent>(DEFAULT_NOTIFY_CAPACITY);
        let id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        {
            let mut sinks = self.inner.event_sinks.lock().expect("event sinks");
            sinks.insert(id, EventSink { tx: bridge_tx });
        }
        self.inner
            .metrics
            .consumers_registered
            .fetch_add(1, Ordering::Relaxed);

        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_flag = Arc::clone(&shutdown);
        let join = thread::Builder::new()
            .name(format!("cp-pres-legacy-{id}"))
            .spawn(move || {
                legacy_forward_loop(bridge_rx, user_tx, shutdown_flag);
            })
            .expect("spawn presentation legacy forwarder");

        self.inner
            .legacy_forwarders
            .lock()
            .expect("legacy")
            .push(LegacyForwarder { join });
        // Store shutdown in a way we can signal — re-use hub shutdown via drop of bridge.
        // Forwarder exits when bridge is disconnected (event sink removed on hub shutdown).
        let _ = shutdown; // forwarder also checks hub via channel disconnect
        debug!(consumer_id = id, "legacy presentation sender attached");
    }

    /// Clear all consumers and mark hub shut down. Idempotent.
    ///
    /// Does not block on legacy forwarder threads indefinitely; drops bridges
    /// so forwarders exit, then joins with a short best-effort wait.
    pub fn shutdown(&self) {
        if self.inner.shutdown.swap(true, Ordering::SeqCst) {
            return;
        }
        {
            let mut sinks = self.inner.notify_sinks.lock().expect("notify sinks");
            let n = sinks.len() as u64;
            sinks.clear();
            if n > 0 {
                self.inner
                    .metrics
                    .consumers_removed
                    .fetch_add(n, Ordering::Relaxed);
            }
        }
        {
            let mut sinks = self.inner.event_sinks.lock().expect("event sinks");
            let n = sinks.len() as u64;
            sinks.clear();
            if n > 0 {
                self.inner
                    .metrics
                    .consumers_removed
                    .fetch_add(n, Ordering::Relaxed);
            }
        }
        // Dropping SyncSenders disconnects forwarders.
        let forwarders = std::mem::take(&mut *self.inner.legacy_forwarders.lock().expect("legacy"));
        for f in forwarders {
            // Best-effort join; forwarder should exit quickly after bridge drop.
            let _ = f.join.join();
        }
        debug!("presentation hub shutdown complete");
    }

    fn commit(&self, mut snapshot: PresentationSnapshot, terminal: bool) -> u64 {
        let (revision, notify) = {
            let mut st = self.inner.state.write().expect("hub state");
            st.revision = st.revision.saturating_add(1);
            snapshot.revision = st.revision;
            snapshot.version = SNAPSHOT_VERSION;
            if terminal {
                st.terminal_sticky = true;
            } else if st.snapshot.active_session_id.as_deref()
                != snapshot.active_session_id.as_deref()
            {
                // New active session clears sticky terminal from prior session.
                st.terminal_sticky = false;
            } else {
                // Non-terminal update on same session clears sticky.
                st.terminal_sticky = false;
            }
            st.snapshot = snapshot.clone();
            let notify = PresentationNotify {
                revision: st.revision,
                active_session_id: snapshot.active_session_id.clone(),
                latest_sequence: snapshot.latest_sequence,
                session_status: snapshot.session_status,
                terminal: terminal || st.terminal_sticky,
            };
            (st.revision, notify)
        };

        self.inner.metrics.publishes.fetch_add(1, Ordering::Relaxed);
        let _ = self.inner.revision_tx.send(revision);
        self.fanout_notify(&notify);
        self.fanout_event(&PresentationEvent {
            batch: true,
            events: Vec::new(),
        });
        revision
    }

    fn fanout_notify(&self, notify: &PresentationNotify) {
        let mut dead = Vec::new();
        {
            let sinks = self.inner.notify_sinks.lock().expect("notify sinks");
            for (id, sink) in sinks.iter() {
                match sink.tx.try_send(notify.clone()) {
                    Ok(()) => {
                        self.inner
                            .metrics
                            .notify_sends
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    Err(TrySendError::Full(_)) => {
                        // Capacity-1 coalescing: pending slot already has a notify;
                        // drop this intermediate. Latest state remains in snapshot.
                        self.inner
                            .metrics
                            .notify_coalesced
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        self.inner
                            .metrics
                            .notify_send_failures
                            .fetch_add(1, Ordering::Relaxed);
                        dead.push(*id);
                    }
                }
            }
        }
        if !dead.is_empty() {
            let mut sinks = self.inner.notify_sinks.lock().expect("notify sinks");
            for id in dead {
                if sinks.remove(&id).is_some() {
                    self.inner
                        .metrics
                        .consumers_removed
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    fn fanout_event(&self, event: &PresentationEvent) {
        let mut dead = Vec::new();
        {
            let sinks = self.inner.event_sinks.lock().expect("event sinks");
            for (id, sink) in sinks.iter() {
                match sink.tx.try_send(event.clone()) {
                    Ok(()) => {
                        self.inner
                            .metrics
                            .event_sends
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    Err(TrySendError::Full(_)) => {
                        self.inner
                            .metrics
                            .event_drops
                            .fetch_add(1, Ordering::Relaxed);
                        self.inner
                            .metrics
                            .notify_coalesced
                            .fetch_add(1, Ordering::Relaxed);
                    }
                    Err(TrySendError::Disconnected(_)) => {
                        dead.push(*id);
                    }
                }
            }
        }
        if !dead.is_empty() {
            let mut sinks = self.inner.event_sinks.lock().expect("event sinks");
            for id in dead {
                if sinks.remove(&id).is_some() {
                    self.inner
                        .metrics
                        .consumers_removed
                        .fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }
}

impl Drop for PresentationHub {
    fn drop(&mut self) {
        // Last Arc clone triggers cleanup.
        if Arc::strong_count(&self.inner) == 1 {
            self.shutdown();
        }
    }
}

/// Consumer handle: watch revision + optional bounded notify receiver.
pub struct PresentationSubscription {
    hub: Arc<HubInner>,
    id: u64,
    revision_rx: watch::Receiver<u64>,
    notify_rx: Option<StdReceiver<PresentationNotify>>,
    detached: bool,
}

impl PresentationSubscription {
    /// Subscription id (for diagnostics).
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Wait until revision changes (async).
    pub async fn changed(&mut self) -> Result<(), watch::error::RecvError> {
        self.revision_rx.changed().await
    }

    /// Borrowed current revision from the watch channel.
    pub fn watch_revision(&self) -> u64 {
        *self.revision_rx.borrow()
    }

    /// Latest snapshot from the hub (not the watch value alone).
    pub fn snapshot(&self) -> PresentationSnapshot {
        self.hub.state.read().expect("hub state").snapshot.clone()
    }

    /// Whether the consumer's view of `known_revision` is stale vs hub.
    pub fn is_stale(&self, known_revision: u64) -> bool {
        let current = self.hub.state.read().expect("hub state").revision;
        known_revision < current
    }

    /// Try to receive a coalesced notify (non-blocking).
    pub fn try_recv_notify(&self) -> Option<PresentationNotify> {
        self.notify_rx.as_ref()?.try_recv().ok()
    }

    /// Blocking recv with timeout on the notify channel.
    pub fn recv_notify_timeout(&self, timeout: Duration) -> Option<PresentationNotify> {
        match self.notify_rx.as_ref()?.recv_timeout(timeout) {
            Ok(n) => Some(n),
            Err(_) => None,
        }
    }

    /// Explicitly detach (remove delivery state). Also runs on drop.
    pub fn detach(&mut self) {
        if self.detached {
            return;
        }
        self.detached = true;
        let mut sinks = self.hub.notify_sinks.lock().expect("notify sinks");
        if sinks.remove(&self.id).is_some() {
            self.hub
                .metrics
                .consumers_removed
                .fetch_add(1, Ordering::Relaxed);
        }
        self.notify_rx.take();
    }
}

impl Drop for PresentationSubscription {
    fn drop(&mut self) {
        self.detach();
    }
}

fn is_terminal_status(status: SessionStatus) -> bool {
    matches!(
        status,
        SessionStatus::Failed
            | SessionStatus::Disconnected
            | SessionStatus::Stopped
            | SessionStatus::Completed
    )
}

fn legacy_forward_loop(
    rx: StdReceiver<PresentationEvent>,
    user_tx: StdSender<PresentationEvent>,
    _shutdown: Arc<AtomicBool>,
) {
    loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(ev) => {
                if user_tx.send(ev).is_err() {
                    break;
                }
            }
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        }
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::types::HeliStatusView;

    fn empty_snap() -> PresentationSnapshot {
        PresentationSnapshot {
            heli: HeliStatusView::unavailable("test"),
            ..PresentationSnapshot::default()
        }
    }

    #[test]
    fn revision_monotonic_and_snapshot_recoverable() {
        let hub = PresentationHub::new(empty_snap());
        assert_eq!(hub.revision(), 0);
        let mut s = empty_snap();
        s.active_session_id = Some("s1".into());
        s.session_status = Some(SessionStatus::Ready);
        let r1 = hub.publish_snapshot(s.clone());
        assert_eq!(r1, 1);
        s.latest_sequence = 3;
        let r2 = hub.publish_snapshot(s);
        assert_eq!(r2, 2);
        assert_eq!(hub.snapshot().latest_sequence, 3);
        assert_eq!(hub.snapshot().revision, 2);
    }

    #[test]
    fn slow_consumer_coalesces_without_unbounded_growth() {
        let hub = PresentationHub::new(empty_snap());
        let (_id, rx) = hub.subscribe_notify();
        // Do not read from rx — capacity 1.
        for i in 0..1000 {
            let mut s = empty_snap();
            s.latest_sequence = i;
            hub.publish_snapshot(s);
        }
        // At most one pending notify.
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert!(count <= 1, "coalesced pending={count}");
        assert_eq!(hub.snapshot().latest_sequence, 999);
        assert!(hub.metrics().notify_coalesced > 0 || hub.metrics().publishes == 1000);
    }

    #[test]
    fn disconnect_removes_consumer() {
        let hub = PresentationHub::new(empty_snap());
        let sub = hub.subscribe();
        let id = sub.id();
        drop(sub);
        assert!(hub.inner.notify_sinks.lock().unwrap().get(&id).is_none());
        assert_eq!(hub.metrics().consumers_removed, 1);
    }

    #[test]
    fn terminal_sticky_visible_on_snapshot() {
        let hub = PresentationHub::new(empty_snap());
        let mut s = empty_snap();
        s.active_session_id = Some("s1".into());
        s.session_status = Some(SessionStatus::Failed);
        hub.publish_snapshot(s);
        assert!(hub.terminal_sticky());
        assert_eq!(hub.snapshot().session_status, Some(SessionStatus::Failed));
    }
}
