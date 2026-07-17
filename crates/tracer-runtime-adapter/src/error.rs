//! Adapter-facing errors mapped onto domain [`TracerError`].

use thiserror::Error;
use tracer_acp_client::AcpError;
use tracer_domain::{ErrorClass, TracerError};
use tracer_process::ProcessError;

/// Runtime adapter error.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{error_class}: {message}")]
pub struct AdapterError {
    /// Stable class.
    pub error_class: ErrorClass,
    /// Human message (no secrets).
    pub message: String,
    /// Retry hint.
    pub retryable: bool,
    /// Optional details.
    pub details: serde_json::Map<String, serde_json::Value>,
}

impl AdapterError {
    /// Build from class + message.
    pub fn new(error_class: ErrorClass, message: impl Into<String>) -> Self {
        Self {
            error_class,
            message: message.into(),
            retryable: error_class.typically_retryable(),
            details: serde_json::Map::new(),
        }
    }

    /// Not ready for the requested op.
    pub fn not_ready(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::RuntimeNotReady, message)
    }

    /// Invalid state.
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::InvalidState, message)
    }

    /// Capability unsupported.
    pub fn capability_unsupported(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::CapabilityUnsupported, message)
    }

    /// Auth required.
    pub fn auth_required(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::AuthenticationRequired, message)
    }

    /// Auth failed.
    pub fn auth_failed(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::AuthenticationFailed, message)
    }

    /// Disconnected.
    pub fn disconnected(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::RuntimeDisconnected, message)
    }

    /// Crashed.
    pub fn crashed(message: impl Into<String>) -> Self {
        Self::new(ErrorClass::RuntimeCrashed, message)
    }

    /// Convert to domain error.
    pub fn to_tracer_error(&self) -> TracerError {
        let mut e = TracerError::new(self.error_class, self.message.clone());
        e.retryable = self.retryable;
        e.details = self.details.clone();
        e
    }

    /// From process manager error.
    pub fn from_process(err: &ProcessError) -> Self {
        let class = match err.class {
            tracer_process::ProcessErrorClass::RuntimeExecutableNotFound => {
                ErrorClass::RuntimeExecutableNotFound
            }
            tracer_process::ProcessErrorClass::RuntimeSpawnFailed => ErrorClass::RuntimeSpawnFailed,
            tracer_process::ProcessErrorClass::RuntimeCrashed => ErrorClass::RuntimeCrashed,
            tracer_process::ProcessErrorClass::RuntimeDisconnected => {
                ErrorClass::RuntimeDisconnected
            }
            tracer_process::ProcessErrorClass::Timeout => ErrorClass::Timeout,
            tracer_process::ProcessErrorClass::CancellationFailed => ErrorClass::CancellationFailed,
            tracer_process::ProcessErrorClass::InvalidArgument => ErrorClass::InvalidArgument,
            tracer_process::ProcessErrorClass::InternalError => ErrorClass::InternalAdapterError,
        };
        Self::new(class, err.message.clone())
    }

    /// From ACP client error (with auth taxonomy refinement).
    pub fn from_acp(err: &AcpError) -> Self {
        if err.is_authentication_required() {
            return Self::auth_required(err.message.clone());
        }
        let te = err.to_tracer_error();
        let mut a = Self::new(te.error_class, te.message);
        a.retryable = te.retryable;
        a.details = te.details;
        a
    }
}
