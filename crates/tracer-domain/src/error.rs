//! Error classes and high-level error categories.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// High-level error category for routing / UX grouping.
///
/// Task W1-B requires: protocol, process, authentication, permission, storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    /// Framing, JSON-RPC, negotiation, catalog violations.
    Protocol,
    /// Spawn, pipes, crash, disconnect.
    Process,
    /// Sign-in required / failed / expired.
    Authentication,
    /// Approvals, policy denial, unknown approval.
    Permission,
    /// Persistence failures.
    Storage,
    /// Caller contract breach or internal adapter bug.
    Internal,
    /// Capability mismatch / unsupported operation.
    Capability,
    /// Timeouts and generic rejections not otherwise classified.
    Operation,
}

impl ErrorCategory {
    /// Wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Protocol => "protocol",
            Self::Process => "process",
            Self::Authentication => "authentication",
            Self::Permission => "permission",
            Self::Storage => "storage",
            Self::Internal => "internal",
            Self::Capability => "capability",
            Self::Operation => "operation",
        }
    }
}

/// Stable `errorClass` strings used across adapter, Tauri, and event payloads.
///
/// Includes Stage 0.1 additive auth classes recommended for Wave 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorClass {
    // Process
    /// Configured binary missing.
    RuntimeExecutableNotFound,
    /// OS spawn failure.
    RuntimeSpawnFailed,
    /// Prompt/session op before ready.
    RuntimeNotReady,
    /// Pipes closed / unexpected EOF.
    RuntimeDisconnected,
    /// Non-zero or signal exit unexpectedly.
    RuntimeCrashed,
    // Protocol
    /// Handshake failed.
    ProtocolInitializeFailed,
    /// Malformed JSON / framing.
    ProtocolParseError,
    /// Duplicate ids, invalid protocol state.
    ProtocolViolation,
    // Capability
    /// Negotiated caps insufficient.
    CapabilityMismatch,
    /// Op requires missing cap.
    CapabilityUnsupported,
    // Session / ops
    /// Unknown session handle.
    SessionNotFound,
    /// Runtime refused prompt.
    PromptRejected,
    /// Cancel not honored in time.
    CancellationFailed,
    /// Operation exceeded deadline.
    Timeout,
    /// Invalid session state for command.
    InvalidState,
    /// Caller contract breach.
    InvalidArgument,
    // Permission
    /// Unknown approval id.
    ApprovalUnknown,
    /// Policy denied action.
    PermissionDenied,
    // Authentication (additive W1)
    /// Auth required before session ready.
    AuthenticationRequired,
    /// Auth attempt failed.
    AuthenticationFailed,
    // Storage / internal
    /// Persistence failure.
    StorageError,
    /// Adapter bug / unexpected.
    InternalAdapterError,
}

impl ErrorClass {
    /// Wire string form (PascalCase stable identifiers).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RuntimeExecutableNotFound => "RuntimeExecutableNotFound",
            Self::RuntimeSpawnFailed => "RuntimeSpawnFailed",
            Self::RuntimeNotReady => "RuntimeNotReady",
            Self::RuntimeDisconnected => "RuntimeDisconnected",
            Self::RuntimeCrashed => "RuntimeCrashed",
            Self::ProtocolInitializeFailed => "ProtocolInitializeFailed",
            Self::ProtocolParseError => "ProtocolParseError",
            Self::ProtocolViolation => "ProtocolViolation",
            Self::CapabilityMismatch => "CapabilityMismatch",
            Self::CapabilityUnsupported => "CapabilityUnsupported",
            Self::SessionNotFound => "SessionNotFound",
            Self::PromptRejected => "PromptRejected",
            Self::CancellationFailed => "CancellationFailed",
            Self::Timeout => "Timeout",
            Self::InvalidState => "InvalidState",
            Self::InvalidArgument => "InvalidArgument",
            Self::ApprovalUnknown => "ApprovalUnknown",
            Self::PermissionDenied => "PermissionDenied",
            Self::AuthenticationRequired => "AuthenticationRequired",
            Self::AuthenticationFailed => "AuthenticationFailed",
            Self::StorageError => "StorageError",
            Self::InternalAdapterError => "InternalAdapterError",
        }
    }

    /// Map to a high-level category.
    pub fn category(self) -> ErrorCategory {
        match self {
            Self::RuntimeExecutableNotFound
            | Self::RuntimeSpawnFailed
            | Self::RuntimeNotReady
            | Self::RuntimeDisconnected
            | Self::RuntimeCrashed => ErrorCategory::Process,

            Self::ProtocolInitializeFailed
            | Self::ProtocolParseError
            | Self::ProtocolViolation => ErrorCategory::Protocol,

            Self::CapabilityMismatch | Self::CapabilityUnsupported => ErrorCategory::Capability,

            Self::AuthenticationRequired | Self::AuthenticationFailed => {
                ErrorCategory::Authentication
            }

            Self::ApprovalUnknown | Self::PermissionDenied => ErrorCategory::Permission,

            Self::StorageError => ErrorCategory::Storage,

            Self::InternalAdapterError | Self::InvalidArgument => ErrorCategory::Internal,

            Self::SessionNotFound
            | Self::PromptRejected
            | Self::CancellationFailed
            | Self::Timeout
            | Self::InvalidState => ErrorCategory::Operation,
        }
    }

    /// Typical retryability default from adapter contract.
    pub fn typically_retryable(self) -> bool {
        matches!(
            self,
            Self::RuntimeNotReady
                | Self::RuntimeSpawnFailed
                | Self::ProtocolParseError
                | Self::ProtocolViolation
                | Self::Timeout
                | Self::PromptRejected
                | Self::StorageError
        )
    }

    /// Parse a wire string; unknown classes return `None`.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "RuntimeExecutableNotFound" => Some(Self::RuntimeExecutableNotFound),
            "RuntimeSpawnFailed" => Some(Self::RuntimeSpawnFailed),
            "RuntimeNotReady" => Some(Self::RuntimeNotReady),
            "RuntimeDisconnected" => Some(Self::RuntimeDisconnected),
            "RuntimeCrashed" => Some(Self::RuntimeCrashed),
            "ProtocolInitializeFailed" => Some(Self::ProtocolInitializeFailed),
            "ProtocolParseError" => Some(Self::ProtocolParseError),
            "ProtocolViolation" => Some(Self::ProtocolViolation),
            "CapabilityMismatch" => Some(Self::CapabilityMismatch),
            "CapabilityUnsupported" => Some(Self::CapabilityUnsupported),
            "SessionNotFound" => Some(Self::SessionNotFound),
            "PromptRejected" => Some(Self::PromptRejected),
            "CancellationFailed" => Some(Self::CancellationFailed),
            "Timeout" => Some(Self::Timeout),
            "InvalidState" => Some(Self::InvalidState),
            "InvalidArgument" => Some(Self::InvalidArgument),
            "ApprovalUnknown" => Some(Self::ApprovalUnknown),
            "PermissionDenied" => Some(Self::PermissionDenied),
            "AuthenticationRequired" => Some(Self::AuthenticationRequired),
            "AuthenticationFailed" => Some(Self::AuthenticationFailed),
            "StorageError" => Some(Self::StorageError),
            "InternalAdapterError" => Some(Self::InternalAdapterError),
            _ => None,
        }
    }
}

impl std::fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Structured domain error for control-plane / adapter boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[error("{error_class}: {message}")]
pub struct TracerError {
    /// Stable class.
    pub error_class: ErrorClass,
    /// Human-readable message (no secrets).
    pub message: String,
    /// Whether a retry may help.
    pub retryable: bool,
    /// Optional JSON-safe details (no secrets).
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub details: serde_json::Map<String, serde_json::Value>,
}

impl TracerError {
    /// Build with typical retryability for the class.
    pub fn new(error_class: ErrorClass, message: impl Into<String>) -> Self {
        Self {
            error_class,
            message: message.into(),
            retryable: error_class.typically_retryable(),
            details: serde_json::Map::new(),
        }
    }

    /// High-level category.
    pub fn category(&self) -> ErrorCategory {
        self.error_class.category()
    }

    /// JSON form matching adapter contract field names.
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "errorClass": self.error_class.as_str(),
            "message": self.message,
            "retryable": self.retryable,
            "details": self.details,
            "category": self.category().as_str(),
        })
    }
}

// Custom serde for ErrorClass as string
mod error_class_serde {
    // We use derive with default rename — but ErrorClass variants are PascalCase
    // already matching wire. Serde default external tagging would be wrong for a
    // bare string. The derive on the enum without rename_all emits variant names
    // as unit variants in externally tagged form. We need string form.
}

// Actually serde on unit enums without rename produces "\"RuntimeNotReady\"" for
// unit variants when using #[serde(untagged)] or we need serialize_with.
// Default for unit enum: `"RuntimeNotReady"` as string — correct for externally
// tagged... wait, for a unit enum, serde serializes as a string of the variant
// name by default. Yes: `enum E { A, B }` => `"A"` / `"B"`. Good.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn categories_cover_required() {
        let required = [
            ErrorCategory::Protocol,
            ErrorCategory::Process,
            ErrorCategory::Authentication,
            ErrorCategory::Permission,
            ErrorCategory::Storage,
        ];
        for c in required {
            assert!(!c.as_str().is_empty());
        }
        assert_eq!(
            ErrorClass::ProtocolParseError.category(),
            ErrorCategory::Protocol
        );
        assert_eq!(
            ErrorClass::RuntimeCrashed.category(),
            ErrorCategory::Process
        );
        assert_eq!(
            ErrorClass::AuthenticationRequired.category(),
            ErrorCategory::Authentication
        );
        assert_eq!(
            ErrorClass::PermissionDenied.category(),
            ErrorCategory::Permission
        );
        assert_eq!(ErrorClass::StorageError.category(), ErrorCategory::Storage);
    }

    #[test]
    fn error_json_shape() {
        let err = TracerError::new(
            ErrorClass::AuthenticationRequired,
            "Authentication required before session/new",
        );
        let v = err.to_json_value();
        assert_eq!(v["errorClass"], "AuthenticationRequired");
        assert_eq!(v["category"], "authentication");
        assert_eq!(v["retryable"], false);
    }

    #[test]
    fn error_class_serde_string() {
        let c = ErrorClass::ProtocolViolation;
        let s = serde_json::to_string(&c).unwrap();
        assert_eq!(s, "\"ProtocolViolation\"");
        let back: ErrorClass = serde_json::from_str(&s).unwrap();
        assert_eq!(back, c);
    }
}