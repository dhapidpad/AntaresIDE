use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AgentError;

/// JSON-RPC 2.0 specification version constant.
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC Request ID, which can be an integer or a string per spec.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
}

/// Outgoing or incoming JSON-RPC Request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    pub fn new(id: RequestId, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// Outgoing or incoming JSON-RPC Notification (no ID).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcNotification {
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
        }
    }
}

/// Standard JSON-RPC Error Object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC Successful Response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcSuccessResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: Value,
}

/// JSON-RPC Error Response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    pub error: JsonRpcErrorObject,
}

/// Generic parsed JSON-RPC message.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
    Success(JsonRpcSuccessResponse),
    Error(JsonRpcErrorResponse),
}

/// Encode a message to a single line of JSON with a trailing newline.
pub fn encode_line(value: &impl Serialize) -> Result<String, AgentError> {
    let mut json = serde_json::to_string(value)
        .map_err(|e| AgentError::SerializationError(e.to_string()))?;
    json.push('\n');
    Ok(json)
}

/// Decode a single line of JSON-RPC text.
pub fn decode_line(line: &str) -> Result<JsonRpcMessage, AgentError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Err(AgentError::ProtocolError("empty JSON-RPC line".to_string()));
    }

    let val: Value = serde_json::from_str(trimmed)
        .map_err(|e| AgentError::SerializationError(e.to_string()))?;

    let obj = val.as_object().ok_or_else(|| {
        AgentError::ProtocolError("JSON-RPC message must be a JSON object".to_string())
    })?;

    let version = obj
        .get("jsonrpc")
        .and_then(Value::as_str)
        .ok_or_else(|| AgentError::ProtocolError("missing or non-string 'jsonrpc' field".to_string()))?;

    if version != JSONRPC_VERSION {
        return Err(AgentError::ProtocolError(format!(
            "unsupported jsonrpc version: {version}"
        )));
    }

    let has_id = obj.contains_key("id");
    let has_method = obj.contains_key("method");
    let has_result = obj.contains_key("result");
    let has_error = obj.contains_key("error");

    if has_method && has_id {
        let req: JsonRpcRequest = serde_json::from_value(val)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;
        Ok(JsonRpcMessage::Request(req))
    } else if has_method && !has_id {
        let notif: JsonRpcNotification = serde_json::from_value(val)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;
        Ok(JsonRpcMessage::Notification(notif))
    } else if has_id && has_result {
        let succ: JsonRpcSuccessResponse = serde_json::from_value(val)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;
        Ok(JsonRpcMessage::Success(succ))
    } else if has_id && has_error {
        let err: JsonRpcErrorResponse = serde_json::from_value(val)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;
        Ok(JsonRpcMessage::Error(err))
    } else {
        Err(AgentError::ProtocolError(format!(
            "unrecognized JSON-RPC message structure: {trimmed}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_and_decode_request() {
        let req = JsonRpcRequest::new(
            RequestId::Number(1),
            "initialize",
            Some(serde_json::json!({"client": "antares"})),
        );
        let encoded = encode_line(&req).unwrap();
        assert!(encoded.ends_with('\n'));

        let decoded = decode_line(&encoded).unwrap();
        match decoded {
            JsonRpcMessage::Request(r) => {
                assert_eq!(r.id, RequestId::Number(1));
                assert_eq!(r.method, "initialize");
                assert_eq!(r.params, Some(serde_json::json!({"client": "antares"})));
            }
            other => panic!("expected Request, got {other:?}"),
        }
    }

    #[test]
    fn test_encode_and_decode_notification() {
        let notif = JsonRpcNotification::new(
            "session/update",
            Some(serde_json::json!({"text": "streaming"})),
        );
        let encoded = encode_line(&notif).unwrap();
        let decoded = decode_line(&encoded).unwrap();
        match decoded {
            JsonRpcMessage::Notification(n) => {
                assert_eq!(n.method, "session/update");
                assert_eq!(n.params, Some(serde_json::json!({"text": "streaming"})));
            }
            other => panic!("expected Notification, got {other:?}"),
        }
    }

    #[test]
    fn test_decode_success_response() {
        let raw = r#"{"jsonrpc": "2.0", "id": 42, "result": {"sessionId": "s-123"}}"#;
        let decoded = decode_line(raw).unwrap();
        match decoded {
            JsonRpcMessage::Success(res) => {
                assert_eq!(res.id, RequestId::Number(42));
                assert_eq!(res.result, serde_json::json!({"sessionId": "s-123"}));
            }
            other => panic!("expected Success, got {other:?}"),
        }
    }

    #[test]
    fn test_decode_error_response() {
        let raw = r#"{"jsonrpc": "2.0", "id": "req-1", "error": {"code": -32601, "message": "Method not found"}}"#;
        let decoded = decode_line(raw).unwrap();
        match decoded {
            JsonRpcMessage::Error(res) => {
                assert_eq!(res.id, RequestId::String("req-1".to_string()));
                assert_eq!(res.error.code, -32601);
                assert_eq!(res.error.message, "Method not found");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn test_decode_invalid_json() {
        assert!(decode_line("not json").is_err());
        assert!(decode_line(r#"{"missing": "jsonrpc"}"#).is_err());
        assert!(decode_line(r#"{"jsonrpc": "1.0", "id": 1, "result": true}"#).is_err());
    }
}
