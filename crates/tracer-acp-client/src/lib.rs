//! ACP JSON-RPC client over NDJSON stdio (W1-D).
//!
//! # Layers
//!
//! ```text
//! transport (NDJSON frames)
//!   → codec (JSON-RPC serialize/deserialize)
//!   → session protocol state machine
//! ```
//!
//! This crate does **not** own OS processes (see `tracer-process`) and does
//! **not** emit Tracer Event Protocol envelopes (see `tracer-runtime-adapter`).
//!
//! # Transport rules (W0-B / adapter contract)
//!
//! - Framing: newline-delimited JSON-RPC 2.0
//! - Client writes requests/notifications on agent **stdin**
//! - Agent writes responses/notifications on **stdout**
//! - **stderr** is process diagnostics — never parsed as ACP

#![deny(missing_docs)]

pub mod client;
pub mod codec;
pub mod error;
pub mod message;
pub mod state;
pub mod transport;

pub use client::{AcpClient, ClientConfig, PendingPermission, RequestResult};
pub use codec::{
    decode_line, encode_message, encode_notification, encode_request, encode_response, FrameDecoder,
};
pub use error::{AcpError, AcpErrorKind};
pub use message::{
    JsonRpcError, JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
pub use state::{ProtocolPhase, SessionProtocolState, TransitionError};
pub use transport::{InboundFrame, NdjsonReader, NdjsonWriter};
