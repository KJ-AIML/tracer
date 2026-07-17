//! VS1-H3 soak helpers (shared by integration tests).
//!
//! Environment: fake ACP, file-backed SQLite, network: no, credentials: no.

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use tempfile::tempdir;
use tracer_control_plane::{
    ControlPlane, ControlPlaneConfig, RuntimeCreateOptions, BRIDGE_CAPACITY,
};

/// Hard invariants for the soak suite (defined before run).
pub mod thresholds {
    /// Maximum allowed silent event loss across soak scenarios.
    pub const MAX_EVENT_LOSS: u64 = 0;
    /// Maximum allowed duplicated *persisted* events (storage PK collisions).
    pub const MAX_DUPLICATED_PERSISTED: u64 = 0;
    /// Terminal events (session.completed / cancelled / failed) must not be lost.
    pub const MAX_TERMINAL_EVENTS_LOST: u64 = 0;
    /// Orphan processes after stop/shutdown.
    pub const MAX_ORPHAN_PROCESSES: u64 = 0;
    /// Stale actionable approvals after cancel / terminal.
    pub const MAX_STALE_ACTIONABLE_APPROVALS: u64 = 0;
    /// Unjoined owned tasks after shutdown (ingest drain must join).
    pub const MAX_UNJOINED_OWNED_TASKS: u64 = 0;
    /// Burst size must exceed bridge capacity.
    pub const BURST_CHUNK_COUNT: u64 = 600;
    /// Default wall-clock budget for a single soak scenario (seconds).
    pub const SCENARIO_BUDGET_SECS: u64 = 90;
    /// Repeated sequential sessions (stress).
    pub const REPEATED_SESSION_COUNT: usize = 12;
}

/// Serialize soak scenarios (node + SQLite contend under Windows).
pub async fn soak_lock() -> tokio::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
        .lock()
        .await
}

pub fn repo_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // tests
    p.pop(); // repo root
    p
}

pub fn stock_fake_js() -> PathBuf {
    repo_root().join("tools/fake-acp-runtime/bin/fake-acp-runtime.js")
}

pub fn burst_fake_js() -> PathBuf {
    repo_root().join("tools/soak-runner/burst-fake-acp.js")
}

pub fn assert_fakes_present() {
    assert!(
        stock_fake_js().is_file(),
        "missing stock fake at {}",
        stock_fake_js().display()
    );
    assert!(
        burst_fake_js().is_file(),
        "missing burst fake at {}",
        burst_fake_js().display()
    );
    assert_eq!(BRIDGE_CAPACITY, 256, "document BRIDGE_CAPACITY expectation");
    assert!(
        thresholds::BURST_CHUNK_COUNT as usize > BRIDGE_CAPACITY,
        "burst must exceed bridge capacity"
    );
}

pub async fn open_file_cp() -> (tempfile::TempDir, ControlPlane, PathBuf) {
    assert_fakes_present();
    let dir = tempdir().expect("tempdir");
    let db = dir.path().join(format!(
        "soak-{}.db",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let config = ControlPlaneConfig {
        database_path: Some(db.clone()),
        fake_js: Some(stock_fake_js()),
        node_bin: PathBuf::from("node"),
        heli_probe_path: repo_root(),
        escalate_cancel_to_process_stop: true,
    };
    let cp = ControlPlane::open(config)
        .await
        .expect("open control plane");
    (dir, cp, db)
}

pub async fn open_file_cp_at(db: PathBuf) -> ControlPlane {
    assert_fakes_present();
    let config = ControlPlaneConfig {
        database_path: Some(db),
        fake_js: Some(stock_fake_js()),
        node_bin: PathBuf::from("node"),
        heli_probe_path: repo_root(),
        escalate_cancel_to_process_stop: true,
    };
    ControlPlane::open(config)
        .await
        .expect("reopen control plane")
}

pub async fn register_project(cp: &ControlPlane) -> (tempfile::TempDir, String) {
    let dir = tempdir().unwrap();
    let proj = cp
        .project_register(dir.path(), Some("soak-project".into()))
        .await
        .expect("register");
    (dir, proj.project_id)
}

pub fn stock_opts(scenario: &str) -> RuntimeCreateOptions {
    RuntimeCreateOptions {
        runtime_kind: "acp-stdio".into(),
        scenario_id: Some(scenario.into()),
        executable_override: None,
        extra_args: vec![],
        fake_js: Some(stock_fake_js().display().to_string()),
    }
}

pub fn burst_opts() -> RuntimeCreateOptions {
    // Point at soak-only burst fake; scenario_id is still required for spawn env.
    RuntimeCreateOptions {
        runtime_kind: "acp-stdio".into(),
        scenario_id: Some("happy_burst".into()),
        executable_override: None,
        extra_args: vec![],
        fake_js: Some(burst_fake_js().display().to_string()),
    }
}

pub fn permission_burst_opts() -> RuntimeCreateOptions {
    RuntimeCreateOptions {
        runtime_kind: "acp-stdio".into(),
        scenario_id: Some("permission_hold".into()),
        executable_override: None,
        extra_args: vec![],
        fake_js: Some(burst_fake_js().display().to_string()),
    }
}

pub fn has_type(events: &[serde_json::Value], t: &str) -> bool {
    events
        .iter()
        .any(|e| e.get("type").and_then(|x| x.as_str()) == Some(t))
}

pub fn event_types(events: &[serde_json::Value]) -> Vec<String> {
    events
        .iter()
        .filter_map(|e| {
            e.get("type")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
        })
        .collect()
}

pub fn sequences_monotonic(events: &[serde_json::Value]) -> bool {
    let mut last = 0i64;
    for e in events {
        let seq = e.get("sequence").and_then(|s| s.as_i64()).unwrap_or(0);
        if seq <= last && last != 0 {
            return false;
        }
        if seq > 0 {
            last = seq;
        }
    }
    true
}

pub fn sequences_unique(events: &[serde_json::Value]) -> bool {
    let mut seen = std::collections::HashSet::new();
    for e in events {
        if let Some(seq) = e.get("sequence").and_then(|s| s.as_i64()) {
            if !seen.insert(seq) {
                return false;
            }
        }
    }
    true
}

pub async fn wait_for_type(
    cp: &ControlPlane,
    session_id: &str,
    event_type: &str,
    budget: Duration,
) -> Vec<serde_json::Value> {
    let deadline = Instant::now() + budget;
    let mut events = cp
        .events_list(session_id, 0, 10_000)
        .await
        .expect("events")
        .events;
    while Instant::now() < deadline && !has_type(&events, event_type) {
        tokio::time::sleep(Duration::from_millis(40)).await;
        events = cp
            .events_list(session_id, 0, 10_000)
            .await
            .expect("events poll")
            .events;
    }
    events
}

pub async fn wait_events_stable(
    cp: &ControlPlane,
    session_id: &str,
    budget: Duration,
) -> Vec<serde_json::Value> {
    let deadline = Instant::now() + budget;
    let mut last_len = 0usize;
    let mut stable_ticks = 0u32;
    let mut events = Vec::new();
    while Instant::now() < deadline {
        events = cp
            .events_list(session_id, 0, 50_000)
            .await
            .expect("events")
            .events;
        if events.len() == last_len {
            stable_ticks += 1;
            if stable_ticks >= 5 {
                break;
            }
        } else {
            stable_ticks = 0;
            last_len = events.len();
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    events
}

/// Process-RSS style memory proxy: count live process handle if available is hard;
/// we record event count + metrics as the primary growth signal for soak docs.
#[derive(Debug, Clone)]
pub struct ScenarioReport {
    pub name: &'static str,
    pub passed: bool,
    pub duration_ms: u128,
    pub events_persisted_storage: usize,
    pub metrics_persisted: Option<u64>,
    pub bridge_accepted: Option<u64>,
    pub persist_errors: Option<u64>,
    pub notes: String,
}

impl ScenarioReport {
    pub fn log_line(&self) -> String {
        format!(
            "[{}] pass={} duration_ms={} storage_events={} metrics_persisted={:?} bridge_accepted={:?} persist_errors={:?} notes={}",
            self.name,
            self.passed,
            self.duration_ms,
            self.events_persisted_storage,
            self.metrics_persisted,
            self.bridge_accepted,
            self.persist_errors,
            self.notes
        )
    }
}
