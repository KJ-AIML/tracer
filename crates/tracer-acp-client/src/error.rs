//! ACP client error taxonomy (distinct from process / auth product errors).

use serde_json::Value;
use thiserror::Error;
use tracer_domain::{ErrorCategory, ErrorClass, TracerError};

/// Fine-grained ACP client failure kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AcpErrorKind {
    /// Malformed JSON or broken NDJSON framing.
    ParseError,
    /// Structurally invalid JSON-RPC (missing method/id rules).
    ProtocolViolation,
    /// Duplicate response id observed.
    DuplicateResponseId,
    /// Request timed out waiting for a response.
    Timeout,
    /// Write to transport failed.
    WriteFailed,
    /// Transport EOF while reading.
    UnexpectedEof,
    /// Clean EOF (stdin closed / peer finished).
    CleanEof,
    /// Invalid session protocol state transition or op.
    InvalidState,
    /// JSON-RPC error response from the agent.
    RpcError,
    /// Caller argument invalid.
    InvalidArgument,
    /// Internal client bug.
    Internal,
}

impl AcpErrorKind {
    /// Map to domain [`ErrorClass`].
    pub fn to_error_class(self) -> ErrorClass {
        match self {
            Self::ParseError => ErrorClass::ProtocolParseError,
            Self::ProtocolViolation | Self::DuplicateResponseId => ErrorClass::ProtocolViolation,
            Self::Timeout => ErrorClass::Timeout,
            Self::WriteFailed | Self::UnexpectedEof | Self::CleanEof => {
                ErrorClass::RuntimeDisconnected
            }
            Self::InvalidState => ErrorClass::InvalidState,
            Self::RpcError => ErrorClass::PromptRejected,
            Self::InvalidArgument => ErrorClass::InvalidArgument,
            Self::Internal => ErrorClass::InternalAdapterError,
        }
    }

    /// High-level category.
    pub fn category(self) -> ErrorCategory {
        self.to_error_class().category()
    }
}

/// Structured ACP client error.
#[derive(Debug, Clone, PartialEq, Error)]
#[error("{kind:?}: {message}")]
pub struct AcpError {
    /// Kind.
    pub kind: AcpErrorKind,
    /// Human message (no secrets).
    pub message: String,
    /// Optional JSON-RPC error code.
    pub rpc_code: Option<i64>,
    /// Optional safe details.
    pub details: Option<Value>,
}

impl AcpError {
    /// Construct a kinded error.
    pub fn new(kind: AcpErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            rpc_code: None,
            details: None,
        }
    }

    /// Parse / framing error.
    pub fn parse(message: impl Into<String>) -> Self {
        Self::new(AcpErrorKind::ParseError, message)
    }

    /// Protocol violation.
    pub fn violation(message: impl Into<String>) -> Self {
        Self::new(AcpErrorKind::ProtocolViolation, message)
    }

    /// Duplicate response id.
    pub fn duplicate_id(id: &str) -> Self {
        Self::new(
            AcpErrorKind::DuplicateResponseId,
            format!("duplicate JSON-RPC response id: {id}"),
        )
    }

    /// Timeout.
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(AcpErrorKind::Timeout, message)
    }

    /// Write failure.
    pub fn write_failed(message: impl Into<String>) -> Self {
        Self::new(AcpErrorKind::WriteFailed, message)
    }

    /// Unexpected EOF.
    pub fn unexpected_eof(message: impl Into<String>) -> Self {
        Self::new(AcpErrorKind::UnexpectedEof, message)
    }

    /// Clean EOF.
    pub fn clean_eof() -> Self {
        Self::new(AcpErrorKind::CleanEof, "peer closed stdout cleanly")
    }

    /// Invalid state for the requested operation.
    pub fn invalid_state(message: impl Into<String>) -> Self {
        Self::new(AcpErrorKind::InvalidState, message)
    }

    /// JSON-RPC error object from agent.
    pub fn rpc_error(code: i64, message: impl Into<String>, data: Option<Value>) -> Self {
        let mut e = Self::new(AcpErrorKind::RpcError, message);
        e.rpc_code = Some(code);
        e.details = data;
        e
    }

    /// Invalid argument.
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(AcpErrorKind::InvalidArgument, message)
    }

    /// Convert to domain [`TracerError`].
    pub fn to_tracer_error(&self) -> TracerError {
        let mut err = TracerError::new(self.kind.to_error_class(), self.message.clone());
        if let Some(code) = self.rpc_code {
            err.details.insert("rpcCode".into(), Value::from(code));
        }
        if let Some(details) = &self.details {
            err.details.insert("rpcData".into(), details.clone());
        }
        err.details.insert(
            "acpErrorKind".into(),
            Value::from(format!("{:?}", self.kind)),
        );
        err
    }

    /// Detect authentication-required JSON-RPC errors (live-scrubbed shape).
    pub fn is_authentication_required(&self) -> bool {
        if self.kind != AcpErrorKind::RpcError {
            return false;
        }
        let msg = self.message.to_ascii_lowercase();
        msg.contains("authentication required")
            || msg.contains("auth required")
            || self.rpc_code == Some(-32000) && msg.contains("authentication")
    }
}
