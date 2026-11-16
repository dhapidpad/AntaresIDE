//! Integration tests for Phase 5: Agent capabilities, multi-turn session persistence,
//! specialist role catalog, token tracking, and background tasks.

use antares_agent::{
    AgentRole, ChatMessage, ChatRole, PersistentSession, ProviderConfig,
    ProviderKind, SessionStorage, TaskManager, TaskStatus, TokenTracker,
};
use tempfile::tempdir;

#[test]
fn test_multi_turn_session_atomic_save_and_reload() {
    let dir = tempdir().unwrap();
    let storage = SessionStorage::new(dir.path()).unwrap();

    let mut session = PersistentSession::new(
        "sess-alpha",
        "/workspace/project",
        "coder",
        "octomind-v0.72",
        1000,
    );

    // Multi-turn messages
    session.append_message(
        ChatMessage {
            id: "msg-1".to_string(),
            role: ChatRole::User,
            content: "Please inspect calculator.rs".to_string(),
            tool_activities: vec![],
            is_streaming: false,
            timestamp_ms: 1001,
        },
        1001,
    );
    session.append_message(
        ChatMessage {
            id: "msg-2".to_string(),
            role: ChatRole::Assistant,
            content: "I inspected the file. It implements basic addition."
                .to_string(),
            tool_activities: vec![],
            is_streaming: false,
            timestamp_ms: 1002,
        },
        1002,
    );
    session.add_token_usage(150, 45);

    // Save atomically
    let path = storage.save_session(&session).unwrap();
    assert!(path.exists());

    // List sessions
    let ids = storage.list_session_ids().unwrap();
    assert_eq!(ids, vec!["sess-alpha"]);

    // Reload and verify integrity
    let loaded = storage.load_session("sess-alpha").unwrap();
    assert_eq!(loaded.session_id, "sess-alpha");
    assert_eq!(loaded.active_role, "coder");
    assert_eq!(loaded.model_id, "octomind-v0.72");
    assert_eq!(loaded.total_input_tokens, 150);
    assert_eq!(loaded.total_output_tokens, 45);
    assert_eq!(loaded.messages.len(), 2);
    assert_eq!(loaded.messages[0].content, "Please inspect calculator.rs");
    assert_eq!(
        loaded.messages[1].content,
        "I inspected the file. It implements basic addition."
    );

    // Delete session
    let deleted = storage.delete_session("sess-alpha").unwrap();
    assert!(deleted);
    assert_eq!(storage.list_session_ids().unwrap().len(), 0);
}

#[test]
fn test_specialist_roles_catalog_and_permissions() {
    let roles = AgentRole::builtin_roles();
    assert_eq!(roles.len(), 3);

    // 1. Coder has write/proposal tools
    let coder = roles.iter().find(|r| r.id == "coder").unwrap();
    assert!(
        coder
            .allowed_tools
            .contains(&"workspace_propose_patch".to_string())
    );
    assert!(
        coder
            .allowed_tools
            .contains(&"workspace_read_file".to_string())
    );

    // 2. Reviewer is strictly read-only
    let reviewer = roles.iter().find(|r| r.id == "reviewer").unwrap();
    assert!(
        reviewer
            .allowed_tools
            .contains(&"workspace_read_file".to_string())
    );
    assert!(
        !reviewer
            .allowed_tools
            .contains(&"workspace_propose_patch".to_string())
    );
    assert!(
        !reviewer
            .allowed_tools
            .contains(&"workspace_request_command".to_string())
    );

    // 3. Architect has read and patch proposal tools
    let architect = roles.iter().find(|r| r.id == "architect").unwrap();
    assert!(
        architect
            .allowed_tools
            .contains(&"workspace_propose_patch".to_string())
    );
}

#[test]
fn test_provider_and_token_tracker() {
    let config = ProviderConfig::octomind_default();
    assert_eq!(config.kind, ProviderKind::Octomind);
    assert!(config.active);

    let gemini = ProviderConfig::gemini_flash();
    assert_eq!(gemini.kind, ProviderKind::Gemini);
    assert!(!gemini.active);

    // Token tracking with 1,000 token budget limit
    let mut tracker = TokenTracker::new(Some(1000));
    assert_eq!(tracker.total_tokens(), 0);
    assert!(!tracker.is_budget_exceeded());

    // Record turn 1: 300 input, 100 output (price: $0.15 / 1M in, $0.60 / 1M out)
    tracker.record_usage(300, 100, (0.15, 0.60));
    assert_eq!(tracker.total_tokens(), 400);
    assert!(!tracker.is_budget_exceeded());
    assert!(tracker.estimated_cost_usd > 0.0);

    // Record turn 2: 500 input, 200 output -> total 1100 tokens (> 1000)
    tracker.record_usage(500, 200, (0.15, 0.60));
    assert_eq!(tracker.total_tokens(), 1100);
    assert!(tracker.is_budget_exceeded());
}

#[test]
fn test_background_task_lifecycle() {
    let mut manager = TaskManager::new();

    // 1. Register task
    let task = manager.register_task("t-1", "Indexing workspace symbols", 100);
    assert_eq!(task.status, TaskStatus::Queued);
    assert_eq!(manager.list_tasks().len(), 1);

    // 2. Start running
    manager.update_status("t-1", TaskStatus::Running, Some(50), 105);
    assert_eq!(manager.get_task("t-1").unwrap().status, TaskStatus::Running);
    assert_eq!(manager.get_task("t-1").unwrap().progress_percent, Some(50));

    // 3. Complete
    manager.update_status("t-1", TaskStatus::Completed, Some(100), 110);
    let completed = manager.get_task("t-1").unwrap();
    assert_eq!(completed.status, TaskStatus::Completed);
    assert_eq!(completed.finished_at_ms, Some(110));

    // 4. Cancel another task
    manager.register_task("t-2", "Background lint check", 120);
    manager.cancel_task("t-2", 125);
    let cancelled = manager.get_task("t-2").unwrap();
    assert_eq!(cancelled.status, TaskStatus::Cancelled);
    assert_eq!(cancelled.finished_at_ms, Some(125));
}
