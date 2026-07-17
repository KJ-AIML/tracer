//! High-level ACP request helpers (id allocation, pending map, permission).
//!
//! The client tracks outstanding request ids and detects duplicate responses.
//! Transport I/O is left to the caller / runtime adapter (threaded read loop).

use std::collections::{HashMap, HashSet};

use serde_json::{json, Map, Value};

use crate::error::AcpError;
use crate::message::{JsonRpcId, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::state::SessionProtocolState;

/// Configuration for client identity offered during `initialize`.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Protocol version offered (ACP observed: 1).
    pub protocol_version: u32,
    /// Client type meta.
    pub client_type: String,
    /// Client identifier.
    pub client_identifier: String,
    /// Client version string.
    pub client_version: String,
    /// Advertise fs.readTextFile capability.
    pub fs_read: bool,
    /// Advertise fs.writeTextFile capability.
    pub fs_write: bool,
    /// Advertise terminal capability.
    pub terminal: bool,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            protocol_version: 1,
            client_type: "tracer".into(),
            client_identifier: "tracer".into(),
            client_version: env!("CARGO_PKG_VERSION").into(),
            fs_read: true,
            fs_write: true,
            terminal: true,
        }
    }
}

/// Result of a completed request (success value or structured error).
#[derive(Debug, Clone, PartialEq)]
pub enum RequestResult {
    /// Success result payload.
    Ok(Value),
    /// JSON-RPC error.
    Err(AcpError),
}

/// Pending reverse-request permission from the agent.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingPermission {
    /// JSON-RPC id of the reverse-request.
    pub request_id: JsonRpcId,
    /// Runtime session id.
    pub session_id: Option<String>,
    /// Raw params (toolCall, options, …).
    pub params: Value,
    /// Tracer-facing approval id (assigned by adapter).
    pub approval_id: Option<String>,
}

/// ACP client correlation state (no IO).
#[derive(Debug)]
pub struct AcpClient {
    /// Client config.
    pub config: ClientConfig,
    /// Protocol state machine.
    pub state: SessionProtocolState,
    next_id: i64,
    pending: HashMap<String, String>,
    seen_response_ids: HashSet<String>,
    /// Open permission reverse-requests keyed by id string.
    pending_permissions: HashMap<String, PendingPermission>,
    /// Last initialize result (capabilities raw).
    pub last_initialize_result: Option<Value>,
}

impl Default for AcpClient {
    fn default() -> Self {
        Self::new(ClientConfig::default())
    }
}

impl AcpClient {
    /// Create a client.
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            state: SessionProtocolState::new(),
            next_id: 1,
            pending: HashMap::new(),
            seen_response_ids: HashSet::new(),
            pending_permissions: HashMap::new(),
            last_initialize_result: None,
        }
    }

    /// Allocate next numeric request id.
    pub fn alloc_id(&mut self) -> JsonRpcId {
        let id = self.next_id;
        self.next_id += 1;
        JsonRpcId::Number(id)
    }

    /// Build `initialize` request.
    pub fn build_initialize(&mut self) -> JsonRpcRequest {
        let id = self.alloc_id();
        self.pending.insert(id.as_key(), "initialize".into());
        let params = json!({
            "protocolVersion": self.config.protocol_version,
            "clientCapabilities": {
                "fs": {
                    "readTextFile": self.config.fs_read,
                    "writeTextFile": self.config.fs_write
                },
                "terminal": self.config.terminal
            },
            "_meta": {
                "clientType": self.config.client_type,
                "clientIdentifier": self.config.client_identifier,
                "clientVersion": self.config.client_version,
                "startupHints": {
                    "nonInteractive": true,
                    "skipGitStatus": true,
                    "skipProjectLayout": true
                }
            }
        });
        JsonRpcRequest::new(id, "initialize", Some(params))
    }

    /// Build `authenticate` request.
    pub fn build_authenticate(&mut self, method_id: Option<&str>) -> JsonRpcRequest {
        let id = self.alloc_id();
        self.pending.insert(id.as_key(), "authenticate".into());
        let mut params = Map::new();
        if let Some(m) = method_id {
            params.insert("methodId".into(), json!(m));
        }
        JsonRpcRequest::new(id, "authenticate", Some(Value::Object(params)))
    }

    /// Build `session/new` request.
    pub fn build_session_new(&mut self, cwd: &str) -> JsonRpcRequest {
        let id = self.alloc_id();
        self.pending.insert(id.as_key(), "session/new".into());
        let params = json!({
            "cwd": cwd,
            "mcpServers": []
        });
        JsonRpcRequest::new(id, "session/new", Some(params))
    }

    /// Build `session/prompt` request.
    pub fn build_session_prompt(
        &mut self,
        session_id: &str,
        text: &str,
        prompt_id: Option<&str>,
    ) -> JsonRpcRequest {
        let id = self.alloc_id();
        self.pending.insert(id.as_key(), "session/prompt".into());
        let mut params = json!({
            "sessionId": session_id,
            "prompt": [
                { "type": "text", "text": text }
            ]
        });
        if let Some(pid) = prompt_id {
            params
                .as_object_mut()
                .unwrap()
                .insert("_meta".into(), json!({ "promptId": pid }));
        }
        JsonRpcRequest::new(id, "session/prompt", Some(params))
    }

    /// Build `session/cancel` notification (ACP cancel is typically a notification).
    pub fn build_session_cancel(&self, session_id: &str) -> JsonRpcNotification {
        JsonRpcNotification::new("session/cancel", Some(json!({ "sessionId": session_id })))
    }

    /// Build permission decision response for a reverse-request.
    pub fn build_permission_response(
        &self,
        request_id: JsonRpcId,
        allow: bool,
        option_id: Option<&str>,
    ) -> JsonRpcResponse {
        if allow {
            let oid = option_id.unwrap_or("allow-once");
            JsonRpcResponse::result(
                request_id,
                json!({
                    "outcome": {
                        "outcome": "selected",
                        "optionId": oid
                    }
                }),
            )
        } else {
            let oid = option_id.unwrap_or("reject-once");
            JsonRpcResponse::result(
                request_id,
                json!({
                    "outcome": {
                        "outcome": "selected",
                        "optionId": oid
                    }
                }),
            )
        }
    }

    /// Build cancelled permission response.
    pub fn build_permission_cancelled(&self, request_id: JsonRpcId) -> JsonRpcResponse {
        JsonRpcResponse::result(
            request_id,
            json!({
                "outcome": { "outcome": "cancelled" }
            }),
        )
    }

    /// Handle an inbound response: correlate, detect duplicates.
    pub fn handle_response(
        &mut self,
        resp: &JsonRpcResponse,
    ) -> Result<(String, RequestResult), AcpError> {
        let key = resp.id.as_key();
        if !self.seen_response_ids.insert(key.clone()) {
            return Err(AcpError::duplicate_id(&key));
        }
        let method = self
            .pending
            .remove(&key)
            .unwrap_or_else(|| "unknown".into());

        if let Some(err) = &resp.error {
            let acp_err = AcpError::rpc_error(err.code, err.message.clone(), err.data.clone());
            return Ok((method, RequestResult::Err(acp_err)));
        }

        let result = resp.result.clone().unwrap_or(Value::Null);
        Ok((method, RequestResult::Ok(result)))
    }

    /// Register an inbound reverse-request (permission).
    pub fn handle_server_request(
        &mut self,
        req: &JsonRpcRequest,
        approval_id: Option<String>,
    ) -> Option<PendingPermission> {
        if req.method == "session/request_permission" {
            let session_id = req
                .params
                .as_ref()
                .and_then(|p| p.get("sessionId"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let pending = PendingPermission {
                request_id: req.id.clone(),
                session_id,
                params: req.params.clone().unwrap_or(Value::Null),
                approval_id,
            };
            self.pending_permissions
                .insert(req.id.as_key(), pending.clone());
            return Some(pending);
        }
        None
    }

    /// Take a pending permission by id key.
    pub fn take_permission(&mut self, id_key: &str) -> Option<PendingPermission> {
        self.pending_permissions.remove(id_key)
    }

    /// First pending permission (if any).
    pub fn first_pending_permission(&self) -> Option<&PendingPermission> {
        self.pending_permissions.values().next()
    }

    /// Whether any permission is open.
    pub fn has_pending_permission(&self) -> bool {
        !self.pending_permissions.is_empty()
    }

    /// Number of outstanding client requests.
    pub fn pending_request_count(&self) -> usize {
        self.pending.len()
    }

    /// Clear correlation state for restart.
    pub fn reset(&mut self) {
        self.state.reset_for_restart();
        self.pending.clear();
        self.seen_response_ids.clear();
        self.pending_permissions.clear();
        self.last_initialize_result = None;
        self.next_id = 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_response_detected() {
        let mut c = AcpClient::default();
        let req = c.build_initialize();
        let id = req.id.clone();
        let resp = JsonRpcResponse::result(id.clone(), json!({"protocolVersion": 1}));
        let (method, result) = c.handle_response(&resp).unwrap();
        assert_eq!(method, "initialize");
        assert!(matches!(result, RequestResult::Ok(_)));
        let err = c.handle_response(&resp).unwrap_err();
        assert_eq!(err.kind, crate::error::AcpErrorKind::DuplicateResponseId);
    }
}
