//! NDJSON JSON-RPC codec: serialize outbound, deserialize inbound.
//!
//! - Preserves unknown fields via raw [`serde_json::Value`] where needed
//! - Deterministic structural reject for non-JSON-RPC frames
//! - Protocol parse errors ≠ process/auth errors

use serde_json::Value;

use crate::error::AcpError;
use crate::message::{JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

/// Encode any serializable value as a single NDJSON line (with trailing `\n`).
pub fn encode_line<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, AcpError> {
    let mut buf =
        serde_json::to_vec(value).map_err(|e| AcpError::parse(format!("serialize failed: {e}")))?;
    buf.push(b'\n');
    Ok(buf)
}

/// Encode a full JSON-RPC message.
pub fn encode_message(msg: &JsonRpcMessage) -> Result<Vec<u8>, AcpError> {
    encode_line(msg)
}

/// Encode a request.
pub fn encode_request(req: &JsonRpcRequest) -> Result<Vec<u8>, AcpError> {
    encode_line(req)
}

/// Encode a notification.
pub fn encode_notification(n: &JsonRpcNotification) -> Result<Vec<u8>, AcpError> {
    encode_line(n)
}

/// Encode a response (e.g. permission decision).
pub fn encode_response(r: &JsonRpcResponse) -> Result<Vec<u8>, AcpError> {
    encode_line(r)
}

/// Decode one trimmed line into a [`JsonRpcMessage`].
///
/// Empty lines return `Ok(None)`.
pub fn decode_line(line: &str) -> Result<Option<JsonRpcMessage>, AcpError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(trimmed)
        .map_err(|e| AcpError::parse(format!("malformed JSON frame: {e}")))?;
    let msg = JsonRpcMessage::from_value(value).map_err(AcpError::violation)?;
    Ok(Some(msg))
}

/// Incremental NDJSON frame decoder (handles partial reads + multi-message reads).
#[derive(Debug, Default)]
pub struct FrameDecoder {
    buffer: Vec<u8>,
}

impl FrameDecoder {
    /// Create an empty decoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Push raw bytes from a read; return complete frames (without trailing newlines).
    pub fn push(&mut self, data: &[u8]) -> Vec<String> {
        self.buffer.extend_from_slice(data);
        let mut frames = Vec::new();
        while let Some(pos) = self.buffer.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = self.buffer.drain(..=pos).collect();
            // strip trailing \n and optional \r
            let mut end = line_bytes.len().saturating_sub(1);
            if end > 0 && line_bytes[end - 1] == b'\r' {
                end -= 1;
            }
            let line = String::from_utf8_lossy(&line_bytes[..end]).into_owned();
            if !line.trim().is_empty() {
                frames.push(line);
            }
        }
        frames
    }

    /// Bytes currently buffered without a complete newline.
    pub fn pending_len(&self) -> usize {
        self.buffer.len()
    }

    /// Drain remaining buffer as an incomplete trailing frame (if any).
    pub fn take_remainder(&mut self) -> Option<String> {
        if self.buffer.is_empty() {
            return None;
        }
        let rem = std::mem::take(&mut self.buffer);
        let s = String::from_utf8_lossy(&rem).into_owned();
        if s.trim().is_empty() {
            None
        } else {
            Some(s)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::JsonRpcId;
    use serde_json::json;

    #[test]
    fn multi_message_and_partial() {
        let mut dec = FrameDecoder::new();
        let chunk1 = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}";
        assert!(dec.push(chunk1).is_empty());
        assert!(dec.pending_len() > 0);
        let frames =
            dec.push(b"\n{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{}}\n");
        assert_eq!(frames.len(), 2);
        let m0 = decode_line(&frames[0]).unwrap().unwrap();
        assert!(matches!(m0, JsonRpcMessage::Response(_)));
        let m1 = decode_line(&frames[1]).unwrap().unwrap();
        assert!(matches!(m1, JsonRpcMessage::Notification(_)));
    }

    #[test]
    fn malformed_line() {
        let err = decode_line("{not json").unwrap_err();
        assert_eq!(err.kind, crate::error::AcpErrorKind::ParseError);
    }

    #[test]
    fn encode_request_round_trip() {
        let req = JsonRpcRequest::new(JsonRpcId::Number(1), "initialize", Some(json!({})));
        let bytes = encode_request(&req).unwrap();
        assert!(bytes.ends_with(b"\n"));
        let line = std::str::from_utf8(&bytes).unwrap().trim();
        let back = decode_line(line).unwrap().unwrap();
        assert_eq!(back.method(), Some("initialize"));
    }
}
