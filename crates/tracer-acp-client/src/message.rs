//! JSON-RPC 2.0 message types for ACP wire traffic.
//!
//! Unknown fields are preserved via `serde_json::Value` where structural
//! openness is required. Protocol errors are distinct from process/auth errors.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// JSON-RPC id (string or number; null not used for requests we generate).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcId {
    /// Numeric id.
    Number(i64),
    /// String id.
    String(String),
}

impl JsonRpcId {
    /// Display form for maps/logs.
    pub fn as_key(&self) -> String {
        match self {
            Self::Number(n) => n.to_string(),
            Self::String(s) => s.clone(),
        }
    }
}

impl From<i64> for JsonRpcId {
    fn from(value: i64) -> Self {
        Self::Number(value)
    }
}

impl From<u64> for JsonRpcId {
    fn from(value: u64) -> Self {
        Self::Number(value as i64)
    }
}

impl From<String> for JsonRpcId {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for JsonRpcId {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

/// JSON-RPC error object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i64,
    /// Error message.
    pub message: String,
    /// Optional data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Outbound or inbound request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Request id.
    pub id: JsonRpcId,
    /// Method name.
    pub method: String,
    /// Params object/array.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Build a JSON-RPC 2.0 request.
    pub fn new(id: impl Into<JsonRpcId>, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

/// Notification (no id).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Method name.
    pub method: String,
    /// Params.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    /// Build a notification.
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            method: method.into(),
            params,
        }
    }
}

/// Response (result or error).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Always `"2.0"`.
    pub jsonrpc: String,
    /// Correlated id.
    pub id: JsonRpcId,
    /// Success result.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// Error object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Successful response.
    pub fn result(id: impl Into<JsonRpcId>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    /// Error response.
    pub fn error(id: impl Into<JsonRpcId>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id: id.into(),
            result: None,
            error: Some(error),
        }
    }

    /// Whether this is a success.
    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }
}

/// Discriminated inbound/outbound message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    /// Request with method + id.
    Request(JsonRpcRequest),
    /// Response with id and result/error (no method).
    Response(JsonRpcResponse),
    /// Notification with method, no id.
    Notification(JsonRpcNotification),
}

impl JsonRpcMessage {
    /// Classify a raw JSON object into a message kind.
    ///
    /// Deterministic structural reject for frames that are not JSON-RPC shaped.
    pub fn from_value(value: Value) -> Result<Self, String> {
        let obj = value
            .as_object()
            .ok_or_else(|| "JSON-RPC message must be an object".to_string())?;

        let jsonrpc = obj.get("jsonrpc").and_then(|v| v.as_str());
        if jsonrpc != Some("2.0") {
            // Tolerate missing jsonrpc for some vendor noise but prefer 2.0.
            // Still require object with method or id.
        }

        let has_method = obj.contains_key("method");
        let has_id = obj.contains_key("id");
        let has_result = obj.contains_key("result");
        let has_error = obj.contains_key("error");

        if has_method && has_id {
            // Agent reverse-request (e.g. session/request_permission) or client request
            let req: JsonRpcRequest = serde_json::from_value(value)
                .map_err(|e| format!("invalid JSON-RPC request: {e}"))?;
            return Ok(Self::Request(req));
        }

        if has_method && !has_id {
            let n: JsonRpcNotification = serde_json::from_value(value)
                .map_err(|e| format!("invalid JSON-RPC notification: {e}"))?;
            return Ok(Self::Notification(n));
        }

        if has_id && (has_result || has_error) && !has_method {
            let r: JsonRpcResponse = serde_json::from_value(value)
                .map_err(|e| format!("invalid JSON-RPC response: {e}"))?;
            return Ok(Self::Response(r));
        }

        Err("not a JSON-RPC 2.0 request, response, or notification (structural reject)".into())
    }

    /// Serialize to JSON value.
    pub fn to_value(&self) -> Result<Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    /// Method name if request/notification.
    pub fn method(&self) -> Option<&str> {
        match self {
            Self::Request(r) => Some(r.method.as_str()),
            Self::Notification(n) => Some(n.method.as_str()),
            Self::Response(_) => None,
        }
    }

    /// Id if request/response.
    pub fn id(&self) -> Option<&JsonRpcId> {
        match self {
            Self::Request(r) => Some(&r.id),
            Self::Response(r) => Some(&r.id),
            Self::Notification(_) => None,
        }
    }
}

/// Helper: object params as map.
pub fn params_as_object(params: &Option<Value>) -> Option<&Map<String, Value>> {
    params.as_ref().and_then(|v| v.as_object())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classify_request_response_notification() {
        let req = JsonRpcMessage::from_value(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }))
        .unwrap();
        assert!(matches!(req, JsonRpcMessage::Request(_)));

        let resp = JsonRpcMessage::from_value(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": { "protocolVersion": 1 }
        }))
        .unwrap();
        assert!(matches!(resp, JsonRpcMessage::Response(_)));

        let n = JsonRpcMessage::from_value(json!({
            "jsonrpc": "2.0",
            "method": "session/update",
            "params": { "sessionId": "s" }
        }))
        .unwrap();
        assert!(matches!(n, JsonRpcMessage::Notification(_)));
    }

    #[test]
    fn structural_reject() {
        let err = JsonRpcMessage::from_value(json!({ "foo": 1 })).unwrap_err();
        assert!(err.contains("structural reject"));
    }
}
