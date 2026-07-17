//! Spawn configuration for managed sidecar processes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::ProcessError;

/// Default graceful-stop wait after stdin close / cooperative signal.
pub const DEFAULT_GRACEFUL_TIMEOUT: Duration = Duration::from_secs(5);

/// Default force-kill wait after graceful budget (`T_term`).
pub const DEFAULT_FORCE_TIMEOUT: Duration = Duration::from_secs(3);

/// Default max bytes retained per stderr chunk event.
pub const DEFAULT_STDERR_CHUNK_LIMIT: usize = 16 * 1024;

/// How a managed process should be stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopPolicy {
    /// Close stdin (if open), wait up to `graceful`, then force-kill tree.
    GracefulThenForce {
        /// Wait for voluntary exit after stdin close.
        graceful: Duration,
        /// Extra wait after force kill before giving up on observing exit.
        force_wait: Duration,
    },
    /// Immediately kill the process tree (Job Object / process group).
    Force {
        /// Wait after kill for exit observation.
        force_wait: Duration,
    },
}

impl Default for StopPolicy {
    fn default() -> Self {
        Self::GracefulThenForce {
            graceful: DEFAULT_GRACEFUL_TIMEOUT,
            force_wait: DEFAULT_FORCE_TIMEOUT,
        }
    }
}

/// Configuration for spawning one managed runtime process.
///
/// This is the process-manager view of a runtime installation descriptor
/// (`docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md` §3.2). ACP argv selection
/// is the caller's responsibility — this crate does not hardcode Grok paths.
#[derive(Debug, Clone)]
pub struct SpawnConfig {
    /// Executable path or PATH command name. Never a hard-coded machine path in
    /// committed product config; tests may use `CARGO_BIN_EXE_*` or temp helpers.
    pub executable: PathBuf,
    /// Arguments passed to the executable.
    pub args: Vec<String>,
    /// Working directory for the child. Must exist.
    pub cwd: PathBuf,
    /// Extra environment variables (merged over the current process env).
    pub env: HashMap<String, String>,
    /// When true, clear inherited env before applying `env` (hermetic tests).
    pub clear_env: bool,
    /// Kill the process tree when `ManagedProcess` is dropped.
    pub kill_on_drop: bool,
    /// Max bytes per `ProcessEvent::StderrChunk` (longer data is truncated).
    pub stderr_chunk_limit: usize,
    /// Default stop policy used by `ManagedProcess::stop_default`.
    pub stop_policy: StopPolicy,
    /// Attach to a Windows Job Object / Unix process group for orphan prevention.
    /// Tests may disable only when validating the abstraction boundary.
    pub isolate_process_tree: bool,
}

impl SpawnConfig {
    /// Build a config with required fields and safe defaults.
    pub fn new(executable: impl Into<PathBuf>, cwd: impl Into<PathBuf>) -> Self {
        Self {
            executable: executable.into(),
            args: Vec::new(),
            cwd: cwd.into(),
            env: HashMap::new(),
            clear_env: false,
            kill_on_drop: true,
            stderr_chunk_limit: DEFAULT_STDERR_CHUNK_LIMIT,
            stop_policy: StopPolicy::default(),
            isolate_process_tree: true,
        }
    }

    /// Append a single argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Append many arguments.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Set one environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Validate paths that can be checked before spawn.
    pub fn validate(&self) -> Result<(), ProcessError> {
        if self.executable.as_os_str().is_empty() {
            return Err(ProcessError::invalid_argument(
                "executable path/name must not be empty",
            ));
        }
        if self.cwd.as_os_str().is_empty() {
            return Err(ProcessError::invalid_argument("cwd must not be empty"));
        }
        if !self.cwd.exists() {
            return Err(ProcessError::spawn_failed(format!(
                "working directory does not exist: {}",
                self.cwd.display()
            )));
        }
        if !self.cwd.is_dir() {
            return Err(ProcessError::spawn_failed(format!(
                "working directory is not a directory: {}",
                self.cwd.display()
            )));
        }
        Ok(())
    }

    /// Resolve executable for diagnostics (does not search PATH deeply).
    pub fn executable_display(&self) -> String {
        self.executable.display().to_string()
    }

    /// Borrow cwd.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }
}
