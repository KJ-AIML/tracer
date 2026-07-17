//! Runtime spawn descriptors (path-portable; no machine-absolute Grok paths).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracer_process::{SpawnConfig, StopPolicy};

/// Logical runtime kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RuntimeKind {
    /// ACP over stdio (stock Grok or fake).
    AcpStdio,
}

impl RuntimeKind {
    /// Wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcpStdio => "acp-stdio",
        }
    }
}

/// Installation / spawn specification for the adapter.
#[derive(Debug, Clone)]
pub struct RuntimeSpawnSpec {
    /// Runtime kind.
    pub kind: RuntimeKind,
    /// Display name.
    pub display_name: String,
    /// Executable (PATH name or relative/absolute path supplied by caller).
    pub executable: PathBuf,
    /// Arguments.
    pub args: Vec<String>,
    /// Working directory (project root).
    pub cwd: PathBuf,
    /// Extra env.
    pub env: HashMap<String, String>,
    /// Clear inherited env (hermetic tests).
    pub clear_env: bool,
    /// Stop policy.
    pub stop_policy: StopPolicy,
    /// Isolate process tree (orphan prevention via W1-C).
    pub isolate_process_tree: bool,
    /// Kill on drop.
    pub kill_on_drop: bool,
}

impl RuntimeSpawnSpec {
    /// Convert to process-manager spawn config.
    pub fn to_spawn_config(&self) -> SpawnConfig {
        let mut cfg = SpawnConfig::new(self.executable.clone(), self.cwd.clone());
        cfg.args = self.args.clone();
        cfg.env = self.env.clone();
        cfg.clear_env = self.clear_env;
        cfg.stop_policy = self.stop_policy;
        cfg.isolate_process_tree = self.isolate_process_tree;
        cfg.kill_on_drop = self.kill_on_drop;
        cfg
    }
}

/// Stock Grok ACP argv (path-portable). Discovery of `grok` on PATH is caller's job.
///
/// Documented invocation: `grok agent --no-leader stdio`
pub fn grok_stdio_args() -> Vec<String> {
    vec!["agent".into(), "--no-leader".into(), "stdio".into()]
}

/// Build a spawn spec for stock Grok Build ACP mode.
///
/// Does **not** assume live Windows authenticated session creation is proven.
pub fn grok_stdio_spawn_config(
    grok_executable: impl Into<PathBuf>,
    project_cwd: impl Into<PathBuf>,
) -> RuntimeSpawnSpec {
    RuntimeSpawnSpec {
        kind: RuntimeKind::AcpStdio,
        display_name: "Grok Build ACP stdio".into(),
        executable: grok_executable.into(),
        args: grok_stdio_args(),
        cwd: project_cwd.into(),
        env: HashMap::new(),
        clear_env: false,
        stop_policy: StopPolicy::default(),
        isolate_process_tree: true,
        kill_on_drop: true,
    }
}

/// Build a spawn spec for the W1-G fake ACP runtime (primary CI target).
///
/// `node_executable` is typically `"node"`; `fake_runtime_js` is the path to
/// `tools/fake-acp-runtime/bin/fake-acp-runtime.js`.
pub fn fake_acp_spawn_config(
    node_executable: impl Into<PathBuf>,
    fake_runtime_js: impl AsRef<Path>,
    scenario_id: &str,
    project_cwd: impl Into<PathBuf>,
) -> RuntimeSpawnSpec {
    let mut env = HashMap::new();
    env.insert("TRACER_FAKE_ACP_SCENARIO".into(), scenario_id.into());
    RuntimeSpawnSpec {
        kind: RuntimeKind::AcpStdio,
        display_name: format!("Fake ACP ({scenario_id})"),
        executable: node_executable.into(),
        args: vec![
            fake_runtime_js.as_ref().display().to_string(),
            "--scenario".into(),
            scenario_id.into(),
        ],
        cwd: project_cwd.into(),
        env,
        clear_env: false,
        stop_policy: StopPolicy::default(),
        isolate_process_tree: true,
        kill_on_drop: true,
    }
}
