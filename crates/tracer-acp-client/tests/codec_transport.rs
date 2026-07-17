//! Contract-style tests for NDJSON framing edge cases.

use serde_json::json;
use tracer_acp_client::{
    decode_line, encode_request, FrameDecoder, JsonRpcId, JsonRpcMessage, JsonRpcRequest,
    SessionProtocolState,
};

#[test]
fn partial_frame_then_complete() {
    let mut dec = FrameDecoder::new();
    assert!(dec.push(b"{\"jsonrpc\":\"2.0\"").is_empty());
    assert!(dec.push(b",\"id\":1,\"result\":{}}").is_empty());
    let frames = dec.push(b"\n");
    assert_eq!(frames.len(), 1);
    let msg = decode_line(&frames[0]).unwrap().unwrap();
    assert!(matches!(msg, JsonRpcMessage::Response(_)));
}

#[test]
fn multi_msg_single_read() {
    let mut dec = FrameDecoder::new();
    let chunk = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{}}\n";
    let frames = dec.push(chunk);
    assert_eq!(frames.len(), 2);
}

#[test]
fn malformed_deterministic_reject() {
    let err = decode_line("{nope").unwrap_err();
    assert_eq!(err.kind, tracer_acp_client::AcpErrorKind::ParseError);
    let te = err.to_tracer_error();
    assert_eq!(
        te.error_class,
        tracer_domain::ErrorClass::ProtocolParseError
    );
    assert_eq!(te.category().as_str(), "protocol");
}

#[test]
fn encode_initialize_shape() {
    let req = JsonRpcRequest::new(
        JsonRpcId::Number(1),
        "initialize",
        Some(json!({ "protocolVersion": 1 })),
    );
    let bytes = encode_request(&req).unwrap();
    assert!(bytes.ends_with(b"\n"));
    let line = std::str::from_utf8(&bytes).unwrap().trim();
    let msg = decode_line(line).unwrap().unwrap();
    assert_eq!(msg.method(), Some("initialize"));
}

#[test]
fn readiness_gates_proven() {
    let mut s = SessionProtocolState::new();
    s.on_process_alive().unwrap();
    s.on_initialize_start().unwrap();
    s.on_initialize_ok().unwrap();
    assert!(s.protocol_ready());
    assert!(!s.authenticated());
    assert!(!s.session_ready());
    assert!(!s.may_accept_prompt());
    s.on_session_create_start().unwrap();
    s.on_session_ready("rt").unwrap();
    assert!(s.session_ready());
    assert!(s.may_accept_prompt());
    s.on_prompt_start().unwrap();
    assert!(!s.may_accept_prompt());
    assert!(s.phase().is_prompt_active());
}
