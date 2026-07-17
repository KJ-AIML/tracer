//! Process-manager errors mapped to adapter-contract `errorClass` wire strings.
//!
//! Local copies of the stable classes so this crate does not hard-depend on
//! `tracer-domain` while Wave 1 modules land in parallel. Control plane can map
//! these into domain `ErrorClass` later.

use std::fmt;

use thiserror::Error;

/// Stable `errorClass` strings used in process events and spawn failures.
///
/// Values match `docs/contracts/RUNTIME_ADAPTER_CONTRACT_V1.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessErrorClass {
    /// Configured binary missing / not found on PATH.
    RuntimeExecutableNotFound,
    /// OS spawn failure (permissions, invalid cwd, etc.).
    RuntimeSpawnFailed,
    /// Unexpected non-zero / signal exit.
    RuntimeCrashed,
    /// Pipes closed / process already gone when an op needs it.
    RuntimeDisconnected,
    /// Operation exceeded a process-manager deadline.
    Timeout,
    /// Cancel/stop cooperative path failed within budget (caller may force-kill).
    CancellationFailed,
    /// Caller contract breach.
    InvalidArgument,
    /// Internal process-manager bug.
    InternalError,
}

impl ProcessErrorClass {
    /// Wire string (PascalCase).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeExecutableNotFound => "RuntimeExecutableNotFound",
            Self::RuntimeSpawnFailed => "RuntimeSpawnFailed",
            Self::RuntimeCrashed => "RuntimeCrashed",
            Self::RuntimeDisconnected => "RuntimeDisconnected",
            Self::Timeout => "Timeout",
            Self::CancellationFailed => "CancellationFailed",
            Self::InvalidArgument => "InvalidArgument",
            Self::InternalError => "InternalAdapterError",
        }
    }

    /// Typical retryability default.
    pub fn typically_retryable(self) -> bool {
        matches!(self, Self::RuntimeSpawnFailed | Self::Timeout)
    }
}

impl fmt::Display for ProcessErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured process-manager error.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{class}: {message}")]
pub struct ProcessError {
    /// Stable class.
    pub class: ProcessErrorClass,
    /// Human-readable message (no secrets).
    pub message: String,
    /// Whether a retry may help after env/config fix.
    pub retryable: bool,
}

impl ProcessError {
    /// Build with typical retryability for the class.
    pub fn new(class: ProcessErrorClass, message: impl Into<String>) -> Self {
        Self {
            class,
            message: message.into(),
            retryable: class.typically_retryable(),
        }
    }

    /// Missing executable.
    pub fn executable_not_found(path: impl fmt::Display) -> Self {
        Self::new(
            ProcessErrorClass::RuntimeExecutableNotFound,
            format!("runtime executable not found: {path}"),
        )
    }

    /// OS spawn failure.
    pub fn spawn_failed(message: impl Into<String>) -> Self {
        Self::new(ProcessErrorClass::RuntimeSpawnFailed, message)
    }

    /// Invalid caller argument.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(ProcessErrorClass::InvalidArgument, message)
    }

    /// Process already gone / disconnected.
    pub fn disconnected(message: impl Into<String>) -> Self {
        Self::new(ProcessErrorClass::RuntimeDisconnected, message)
    }

    /// Deadline exceeded.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(ProcessErrorClass::Timeout, message)
    }

    /// Internal failure.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ProcessErrorClass::InternalError, message)
    }
}
