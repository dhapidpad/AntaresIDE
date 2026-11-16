//! Integration tests for Phase 4: Controlled changes, patch proposals,
//! and host-owned `antares-workspace` MCP bridge.
//!
//! Verifies:
//! - Direct write/exec tools are rejected in default profile (fail-closed).
//! - Workspace read tools respect path security, secret blocking, and secret redaction.
//! - Patch proposals are validated against base revision hash before apply.
//! - Stale base revisions are detected and rejected without side effects.
//! - Applying is atomic and reversible (undo support).
//! - Command proposals require explicit user approval.

use std::fs;

use antares_agent::{
    CommandStatus, ContextBudget, ContextError, ContextService, McpBridge, McpToolError,
    PatchError, PatchHunk, PatchPipeline,
};
use serde_json::json;
use tempfile::tempdir;

#[test]
fn test_direct_agent_writes_and_commands_rejected() {
    let dir = tempdir().unwrap();
    let service = ContextService::new(dir.path(), ContextBudget::default()).unwrap();
    let mut pipeline = PatchPipeline::new();
    let bridge = McpBridge::new();

    // 1. Direct write_file tool
    let err = bridge
        .execute_tool(
            "write_file",
            &json!({"path": "file.rs", "content": "malicious"}),
            &service,
            &mut pipeline,
            100,
        )
        .unwrap_err();
    assert!(matches!(err, McpToolError::PermissionDenied(_)));

    // 2. Direct exec / run_command tool
    let err2 = bridge
        .execute_tool(
            "run_command",
            &json!({"command": "rm -rf /"}),
            &service,
            &mut pipeline,
            101,
        )
        .unwrap_err();
    assert!(matches!(err2, McpToolError::PermissionDenied(_)));
}

#[test]
fn test_read_tool_enforces_context_boundaries() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create a regular file with a secret
    let file = root.join("app.rs");
    fs::write(&file, "let key = \"AKIAIOSFODNN7EXAMPLE\";\n").unwrap();

    // Create a sensitive file
    let env_file = root.join(".env");
    fs::write(&env_file, "SECRET=123").unwrap();

    let service = ContextService::new(root, ContextBudget::default()).unwrap();
    let mut pipeline = PatchPipeline::new();
    let bridge = McpBridge::new();

    // Reading regular file succeeds and redacts credentials
    let res = bridge
        .execute_tool(
            "workspace_read_file",
            &json!({"path": "app.rs"}),
            &service,
            &mut pipeline,
            200,
        )
        .unwrap();
    let content = res["content"].as_str().unwrap();
    assert!(content.contains("[REDACTED: AWS_KEY]"));
    assert_eq!(res["redactions_count"], 1);

    // Reading .env file fails closed
    let err = bridge
        .execute_tool(
            "workspace_read_file",
            &json!({"path": ".env"}),
            &service,
            &mut pipeline,
            201,
        )
        .unwrap_err();
    assert!(matches!(
        err,
        McpToolError::Context(ContextError::ProtectedFileBlocked(_))
    ));

    // Traversal escape fails closed
    let err2 = bridge
        .execute_tool(
            "workspace_read_file",
            &json!({"path": "../outside.txt"}),
            &service,
            &mut pipeline,
            202,
        )
        .unwrap_err();
    assert!(matches!(
        err2,
        McpToolError::Context(ContextError::PathTraversal(_))
    ));
}

#[test]
fn test_patch_proposal_lifecycle_and_undo() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let file_path = root.join("lib.rs");
    let original_source =
        "pub fn greet(name: &str) -> String {\n    format!(\"Hi, {}\", name)\n}\n";
    fs::write(&file_path, original_source).unwrap();

    let service = ContextService::new(root, ContextBudget::default()).unwrap();
    let mut pipeline = PatchPipeline::new();
    let bridge = McpBridge::new();

    let base_hash = PatchPipeline::compute_hash(original_source);

    // Agent submits patch proposal via MCP tool
    let proposal_res = bridge
        .execute_tool(
            "workspace_propose_patch",
            &json!({
                "path": "lib.rs",
                "base_hash": base_hash,
                "summary": "Change greeting prefix",
                "hunks": [{
                    "old_start": 2,
                    "old_lines": 1,
                    "new_start": 2,
                    "new_lines": 1,
                    "old_text": "    format!(\"Hi, {}\", name)",
                    "new_text": "    format!(\"Hello, {}\", name)"
                }]
            }),
            &service,
            &mut pipeline,
            300,
        )
        .unwrap();

    let proposal_id = proposal_res["proposal_id"].as_str().unwrap();
    assert_eq!(proposal_res["status"], "Pending");

    // Attempting to apply before approval fails
    let err = pipeline.apply_proposal(proposal_id, &service, 301).unwrap_err();
    assert!(matches!(err, PatchError::NotApproved(_)));

    // User approves
    pipeline.approve_proposal(proposal_id).unwrap();

    // Apply patch
    let result = pipeline.apply_proposal(proposal_id, &service, 302).unwrap();
    assert_eq!(
        result.applied_content,
        "pub fn greet(name: &str) -> String {\n    format!(\"Hello, {}\", name)\n}\n"
    );

    // Verify on disk
    assert_eq!(
        fs::read_to_string(&file_path).unwrap(),
        result.applied_content
    );

    // Undo / Revert
    pipeline.revert_patch(&result, &service, 303).unwrap();
    assert_eq!(fs::read_to_string(&file_path).unwrap(), original_source);
}

#[test]
fn test_stale_base_revision_blocks_apply() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let file_path = root.join("counter.rs");
    fs::write(&file_path, "let count = 1;\n").unwrap();

    let service = ContextService::new(root, ContextBudget::default()).unwrap();
    let mut pipeline = PatchPipeline::new();

    let base_hash = PatchPipeline::compute_hash("let count = 1;\n");
    let hunk = PatchHunk::new(1, 1, 1, 1, "count = 1", "count = 2");

    let proposal = pipeline
        .submit_proposal("p-stale", "counter.rs", base_hash, vec![hunk], "Increment", 400)
        .unwrap();

    pipeline.approve_proposal("p-stale").unwrap();

    // Simulate concurrent edit on disk
    fs::write(&file_path, "let count = 100; // modified by user\n").unwrap();

    // Applying must fail due to stale base hash
    let err = pipeline.apply_proposal(&proposal.id, &service, 401).unwrap_err();
    assert!(matches!(err, PatchError::StaleBaseRevision { .. }));

    // File on disk must remain untouched
    assert_eq!(
        fs::read_to_string(&file_path).unwrap(),
        "let count = 100; // modified by user\n"
    );
}

#[test]
fn test_command_proposal_lifecycle() {
    let dir = tempdir().unwrap();
    let service = ContextService::new(dir.path(), ContextBudget::default()).unwrap();
    let mut pipeline = PatchPipeline::new();
    let bridge = McpBridge::new();

    let res = bridge
        .execute_tool(
            "workspace_request_command",
            &json!({"command": "cargo", "args": ["check"]}),
            &service,
            &mut pipeline,
            500,
        )
        .unwrap();

    let cmd_id = res["command_id"].as_str().unwrap();
    assert_eq!(res["status"], "PendingApproval");

    // User approves
    pipeline.approve_command(cmd_id).unwrap();
    assert_eq!(
        pipeline.get_command(cmd_id).unwrap().status,
        CommandStatus::Approved
    );
}
