//! Control-plane and command-surface errors.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use thiserror::Error;
use tracer_domain::ErrorClass;
use tracer_runtime_adapter::AdapterError;
use tracer_storage::StorageError;

/// Structured command error (TAURI_COMMAND_CONTRACT_V1 §3.2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// Stable error class string.
    pub error_class: String,
    /// Human-readable message (no secrets).
    pub message: String,
    /// Whether a retry might succeed.
    pub retryable: bool,
    /// Optional structured details.
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub details: Map<String, Value>,
}

impl CommandError {
    /// Build from class + message.
    pub fn new(error_class: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error_class: error_class.into(),
            message: message.into(),
            retryable: false,
            details: Map::new(),
        }
    }

    /// Attach retryable flag.
    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    /// Attach a detail field.
    pub fn with_detail(mut self, key: impl Into<String>, value: Value) -> Self {
        self.details.insert(key.into(), value);
        self
    }
}

/// Control-plane operation error.
#[derive(Debug, Error)]
pub enum ControlPlaneError {
    #[error("{error_class}: {message}", error_class = .0.error_class, message = .0.message)]
    Command(CommandError),

    #[error("storage: {0}")]
    Storage(#[from] StorageError),
}

impl ControlPlaneError {
    /// Invalid argument.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::Command(CommandError::new("InvalidArgument", message))
    }

    /// Not found.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::Command(CommandError::new("NotFound", message))
    }

    /// Invalid state for the requested op.
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::Command(CommandError::new("InvalidState", message))
    }

    /// Already exists.
    pub fn already_exists(message: impl Into<String>) -> Self {
        Self::Command(CommandError::new("AlreadyExists", message))
    }

    /// Unsupported feature.
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::Command(CommandError::new("Unsupported", message))
    }

    /// Internal error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Command(CommandError::new("InternalError", message))
    }

    /// From domain error class.
    pub fn from_class(class: ErrorClass, message: impl Into<String>) -> Self {
        let mut cmd = CommandError::new(class.as_str(), message);
        cmd.retryable = class.typically_retryable();
        Self::Command(cmd)
    }

    /// From adapter error (preserves distinct classes).
    pub fn from_adapter(err: &AdapterError) -> Self {
        let mut cmd = CommandError::new(err.error_class.as_str(), err.message.clone());
        cmd.retryable = err.retryable;
        cmd.details = err.details.clone();
        Self::Command(cmd)
    }

    /// Convert to command error envelope.
    pub fn to_command_error(&self) -> CommandError {
        match self {
            Self::Command(c) => c.clone(),
            Self::Storage(e) => storage_to_command(e),
        }
    }
}

fn storage_to_command(e: &StorageError) -> CommandError {
    use tracer_storage::StorageErrorClass;
    let class = e.error_class();
    let mut cmd = CommandError::new(class.as_str(), e.to_string());
    cmd.retryable = class.retryable();
    // Migration failures must surface clearly for app start refusal.
    if matches!(class, StorageErrorClass::MigrationFailed) {
        cmd.details
            .insert("category".into(), Value::String("migration".into()));
    }
    cmd
}

impl From<AdapterError> for ControlPlaneError {
    fn from(value: AdapterError) -> Self {
        Self::from_adapter(&value)
    }
}

/// Result alias.
pub type ControlPlaneResult<T> = Result<T, ControlPlaneError>;
