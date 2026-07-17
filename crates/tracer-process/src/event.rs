//! Process-manager lifecycle signals.
//!
//! These are **local** signals for the control plane / adapter to compose into
//! Tracer Event Protocol envelopes. This crate does **not** assign `sequence`,
//! `eventId`, or emit `runtime.process.ready` — that event requires ACP
//! initialize + capability negotiation (adapter-owned).

use crate::error::ProcessErrorClass;
use crate::ids::ProcessId;

/// Exit information observed by the process manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitInfo {
    /// OS exit code when available.
    pub exit_code: Option<i32>,
    /// Signal name when terminated by signal (Unix); typically `None` on Windows.
    pub signal: Option<String>,
    /// `true` when the control plane requested stop/cancel; `false` for crashes.
    pub expected: bool,
}

impl ExitInfo {
    /// Build an exit record.
    pub fn new(exit_code: Option<i32>, signal: Option<String>, expected: bool) -> Self {
        Self {
            exit_code,
            signal,
            expected,
        }
    }

    /// Convenience for normal exit codes.
    pub fn code(code: i32, expected: bool) -> Self {
        Self {
            exit_code: Some(code),
            signal: None,
            expected,
        }
    }
}

/// Lifecycle / IO events from a managed process.
///
/// # Readiness boundary
///
/// - [`ProcessEvent::Started`] means the OS child is alive and pipes are open.
/// - It is **not** `runtime.process.ready` (adapter initialize + caps).
/// - It is **not** authenticated or session-ready (control plane / adapter).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessEvent {
    /// Child process spawned successfully.
    Started {
        /// Process manager id.
        process_id: ProcessId,
        /// OS pid when known.
        pid: u32,
        /// Executable path/name as configured.
        executable: String,
        /// Arguments (may be sanitized by caller before UI display).
        args: Vec<String>,
        /// Working directory.
        cwd: String,
    },
    /// Non-empty stderr chunk (ACP must not use stderr for JSON-RPC).
    StderrChunk {
        /// Process manager id.
        process_id: ProcessId,
        /// Lossy UTF-8 text.
        chunk: String,
        /// Whether `chunk` was truncated to the configured limit.
        truncated: bool,
    },
    /// Process exit observed.
    Exited {
        /// Process manager id.
        process_id: ProcessId,
        /// Exit details.
        info: ExitInfo,
    },
    /// Spawn/start failure or unrecoverable process error before/without a clean exit.
    Failed {
        /// Process manager id when one was allocated.
        process_id: ProcessId,
        /// Stable error class wire string.
        error_class: ProcessErrorClass,
        /// Human message.
        message: String,
        /// Retry hint.
        retryable: bool,
    },
}

impl ProcessEvent {
    /// Event type name aligned with Tracer Event Protocol process family where applicable.
    ///
    /// Note: there is no `runtime.process.ready` here by design.
    pub fn protocol_type_hint(&self) -> &'static str {
        match self {
            Self::Started { .. } => "runtime.process.started",
            Self::StderrChunk { .. } => "runtime.process.stderr",
            Self::Exited { .. } => "runtime.process.exited",
            Self::Failed { .. } => "runtime.process.failed",
        }
    }
}
