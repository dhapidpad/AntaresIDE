use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Pinned ACP Protocol Version string.
pub const ACP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Client identification metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

impl Default for ClientInfo {
    fn default() -> Self {
        Self {
            name: "Antares IDE".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Agent identification metadata returned by the sidecar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgentInfo {
    pub name: Option<String>,
    pub version: Option<String>,
}

/// Capabilities advertised during initialize.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,
}

/// Capabilities returned by Octomind.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AgentCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sessions: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,
}

/// Parameters for `initialize` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    pub protocol_version: String,
    pub client_info: ClientInfo,
    pub capabilities: ClientCapabilities,
}

impl Default for InitializeParams {
    fn default() -> Self {
        Self {
            protocol_version: ACP_PROTOCOL_VERSION.to_string(),
            client_info: ClientInfo::default(),
            capabilities: ClientCapabilities::default(),
        }
    }
}

/// Result returned from `initialize` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    pub protocol_version: String,
    #[serde(default)]
    pub agent_info: AgentInfo,
    #[serde(default)]
    pub capabilities: AgentCapabilities,
}

/// Parameters for `session/new` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionNewParams {
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<Value>>,
}

/// Parameters for `session/load` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLoadParams {
    pub session_id: String,
}

/// Result for `session/new` and `session/load`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResult {
    pub session_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// A block of content sent in `session/prompt`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "resource")]
    Resource {
        uri: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
}

impl ContentBlock {
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            text: content.into(),
        }
    }

    pub fn resource(uri: impl Into<String>, text: Option<String>) -> Self {
        Self::Resource {
            uri: uri.into(),
            mime_type: None,
            text,
        }
    }
}

/// Parameters for `session/prompt` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPromptParams {
    pub session_id: String,
    pub prompt: Vec<ContentBlock>,
}

/// Parameters for `session/cancel` request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionCancelParams {
    pub session_id: String,
}

/// Parameters delivered in `session/update` notifications.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionUpdateParams {
    pub session_id: String,
    pub update: Value,
}

/// High-level typed update events decoded from `session/update`.
#[derive(Debug, Clone, PartialEq)]
pub enum SessionEvent {
    /// Incremental token stream.
    TextDelta(String),
    /// Agent is invoking an MCP or local tool.
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
    /// Result from a tool call.
    ToolResult {
        id: String,
        result: Value,
    },
    /// Overall turn status update (e.g. "thinking", "executing").
    Status(String),
    /// Turn completed.
    Completed,
    /// Fallback for unknown extension notifications.
    Extension(Value),
}

impl SessionUpdateParams {
    /// Safely parse the update object into a strongly typed `SessionEvent`.
    pub fn parse_event(&self) -> SessionEvent {
        if let Some(delta) = self.update.get("textDelta").and_then(Value::as_str) {
            return SessionEvent::TextDelta(delta.to_string());
        }

        if let Some(delta) = self.update.get("delta").and_then(Value::as_str) {
            return SessionEvent::TextDelta(delta.to_string());
        }

        if let Some(tool_call) = self.update.get("toolCall").and_then(Value::as_object) {
            let id = tool_call.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let name = tool_call.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            let arguments = tool_call.get("arguments").cloned().unwrap_or(Value::Null);
            return SessionEvent::ToolCall { id, name, arguments };
        }

        if let Some(tool_result) = self.update.get("toolResult").and_then(Value::as_object) {
            let id = tool_result.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            let result = tool_result.get("result").cloned().unwrap_or(Value::Null);
            return SessionEvent::ToolResult { id, result };
        }

        if let Some(status) = self.update.get("status").and_then(Value::as_str) {
            return SessionEvent::Status(status.to_string());
        }

        if let Some(done) = self.update.get("completed").and_then(Value::as_bool) {
            if done {
                return SessionEvent::Completed;
            }
        }

        SessionEvent::Extension(self.update.clone())
    }
}
