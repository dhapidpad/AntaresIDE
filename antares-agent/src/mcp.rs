//! Host-owned `antares-workspace` MCP tool bridge.
//!
//! Guarantees that agent tool execution runs under Antares supervision:
//! - Direct writes and command executions fail closed with `PermissionDenied`.
//! - Read operations pass through `ContextService` (enforcing path containment,
//!   blocking secrets and binaries, redacting credentials, and budgeting bytes).
//! - Mutations must enter via `workspace_propose_patch` or `workspace_request_command`.

use std::fs;

use serde_json::{json, Value};

use crate::context::{ContextError, ContextService};
use crate::patch::{PatchError, PatchHunk, PatchPipeline};

/// Error returned by host-managed MCP tools.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum McpToolError {
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Context error: {0}")]
    Context(#[from] ContextError),

    #[error("Patch error: {0}")]
    Patch(#[from] PatchError),

    #[error("Unknown tool: {0}")]
    UnknownTool(String),

    #[error("IO error: {0}")]
    Io(String),
}

/// Host-managed workspace MCP dispatcher.
pub struct McpBridge {
    server_name: String,
    server_version: String,
}

impl Default for McpBridge {
    fn default() -> Self {
        Self {
            server_name: "antares-workspace".to_string(),
            server_version: "0.4.6".to_string(),
        }
    }
}

impl McpBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    pub fn server_version(&self) -> &str {
        &self.server_version
    }

    /// List all tool declarations exposed by the `antares-workspace` bridge.
    pub fn list_tools(&self) -> Vec<Value> {
        vec![
            json!({
                "name": "workspace_read_file",
                "description": "Read a file from the workspace with security checks, secret redaction, and token budgeting.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Relative workspace path to read" }
                    },
                    "required": ["path"]
                }
            }),
            json!({
                "name": "workspace_list_directory",
                "description": "List directory entries strictly bounded inside the active workspace.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Relative directory path (or empty for root)" }
                    }
                }
            }),
            json!({
                "name": "workspace_propose_patch",
                "description": "Submit a structured patch proposal for user review and approval before apply.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" },
                        "base_hash": { "type": "string" },
                        "summary": { "type": "string" },
                        "hunks": { "type": "array" }
                    },
                    "required": ["path", "base_hash", "summary", "hunks"]
                }
            }),
            json!({
                "name": "workspace_request_command",
                "description": "Request terminal command execution pending explicit user confirmation.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" },
                        "args": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["command"]
                }
            }),
        ]
    }

    /// Dispatch tool calls through the security boundary.
    pub fn execute_tool(
        &self,
        tool_name: &str,
        args: &Value,
        context: &ContextService,
        pipeline: &mut PatchPipeline,
        now_ms: u64,
    ) -> Result<Value, McpToolError> {
        match tool_name {
            "workspace_read_file" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpToolError::InvalidArguments("Missing 'path'".to_string()))?;

                let file_ctx = context.read_file_context(path)?;
                Ok(json!({
                    "relative_path": file_ctx.relative_path,
                    "content": file_ctx.content,
                    "is_truncated": file_ctx.is_truncated,
                    "byte_count": file_ctx.byte_count,
                    "redactions_count": file_ctx.redactions_count,
                }))
            }

            "workspace_list_directory" => {
                let rel_path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let canonical = context.validate_path(rel_path)?;
                if !canonical.is_dir() {
                    return Err(McpToolError::InvalidArguments(format!(
                        "Path is not a directory: {}",
                        rel_path
                    )));
                }

                let mut entries = Vec::new();
                let dir_iter = fs::read_dir(&canonical)
                    .map_err(|e| McpToolError::Io(e.to_string()))?;

                for entry in dir_iter.flatten() {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                    entries.push(json!({
                        "name": file_name,
                        "is_directory": is_dir,
                    }));
                }

                Ok(json!({
                    "path": rel_path,
                    "entries": entries,
                }))
            }

            "workspace_propose_patch" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpToolError::InvalidArguments("Missing 'path'".to_string()))?;

                let base_hash = args
                    .get("base_hash")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpToolError::InvalidArguments("Missing 'base_hash'".to_string())
                    })?;

                let summary = args
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Agent proposed patch");

                let hunks_val = args
                    .get("hunks")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| {
                        McpToolError::InvalidArguments("Missing or invalid 'hunks'".to_string())
                    })?;

                let mut hunks = Vec::new();
                for h in hunks_val {
                    let old_start =
                        h.get("old_start").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                    let old_lines =
                        h.get("old_lines").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                    let new_start =
                        h.get("new_start").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                    let new_lines =
                        h.get("new_lines").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
                    let old_text = h
                        .get("old_text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let new_text = h
                        .get("new_text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    hunks.push(PatchHunk::new(
                        old_start, old_lines, new_start, new_lines, old_text, new_text,
                    ));
                }

                let proposal_id = format!("prop-{}", now_ms);
                let proposal = pipeline.submit_proposal(
                    &proposal_id,
                    path,
                    base_hash,
                    hunks,
                    summary,
                    now_ms,
                )?;

                // Pre-validate proposal to give agent immediate feedback if invalid
                let _ = pipeline.validate_proposal(&proposal, context)?;

                Ok(json!({
                    "proposal_id": proposal.id,
                    "status": "Pending",
                    "message": "Patch submitted for user review. It will not be applied until approved."
                }))
            }

            "workspace_request_command" => {
                let cmd = args
                    .get("command")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpToolError::InvalidArguments("Missing 'command'".to_string())
                    })?;

                let arg_list: Vec<String> = args
                    .get("args")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                let cmd_id = format!("cmd-{}", now_ms);
                let proposal = pipeline.submit_command(&cmd_id, cmd, arg_list, None, now_ms);

                Ok(json!({
                    "command_id": proposal.id,
                    "status": "PendingApproval",
                    "message": "Command submitted for user approval. It will not execute until explicitly confirmed."
                }))
            }

            // Direct write/exec tools are blocked by default profile
            "write_file" | "edit_file" | "modify_file" | "bash" | "exec" | "run_command" => {
                Err(McpToolError::PermissionDenied(format!(
                    "Direct mutation '{}' is disabled in Antares default profile. Submit a patch proposal via 'workspace_propose_patch' instead.",
                    tool_name
                )))
            }

            other => Err(McpToolError::UnknownTool(other.to_string())),
        }
    }
}
