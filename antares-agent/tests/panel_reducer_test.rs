//! Integration test suite for Phase 2: AI panel domain state machine & reducer.
//!
//! Confirms test plan requirements:
//! - Every state transition is tested without a native window.
//! - Empty, loading, ready, streaming, cancelling, auth-required, disconnected and error states render honestly.
//! - Cancel and workspace switch cancel stale work.
//! - Long transcript updates do not rebuild all existing messages.

use antares_agent::{
    ChatRole, PanelAction, PanelLifecycle, PanelSideEffect, PanelState, ToolStatus,
};

#[test]
fn test_all_lifecycle_state_transitions() {
    let mut state = PanelState::new();

    // 1. Empty / Idle state
    assert_eq!(state.lifecycle, PanelLifecycle::Idle);
    assert_eq!(state.status_text(), "Idle");
    assert!(!state.can_send());
    assert!(!state.can_cancel());

    // 2. Loading / Starting state
    let effects = state.reduce(PanelAction::StartProcess);
    assert_eq!(state.lifecycle, PanelLifecycle::Starting);
    assert_eq!(state.status_text(), "Starting...");
    assert_eq!(effects, vec![PanelSideEffect::LaunchProcess]);

    // 3. Ready state
    let effects = state.reduce(PanelAction::ProcessStarted {
        agent_name: Some("octomind-0.72.0".to_string()),
    });
    assert_eq!(state.lifecycle, PanelLifecycle::Ready);
    assert_eq!(state.status_text(), "Ready");
    assert!(effects.is_empty());

    // 4. Session created
    state.reduce(PanelAction::SessionCreated {
        session_id: "session-p2-001".to_string(),
        title: "P2 Session".to_string(),
    });
    assert_eq!(state.active_session_id.as_deref(), Some("session-p2-001"));

    // 5. Streaming state
    state.reduce(PanelAction::SetInputDraft("Explain Rust Edition 2024".to_string()));
    assert!(state.can_send());
    let effects = state.reduce(PanelAction::SubmitPrompt);
    assert_eq!(state.lifecycle, PanelLifecycle::Streaming);
    assert_eq!(state.status_text(), "Thinking...");
    assert!(state.can_cancel());
    assert_eq!(
        effects,
        vec![PanelSideEffect::SendPrompt {
            session_id: "session-p2-001".to_string(),
            text: "Explain Rust Edition 2024".to_string()
        }]
    );

    // 6. Cancelling state
    let effects = state.reduce(PanelAction::RequestCancel);
    assert_eq!(state.lifecycle, PanelLifecycle::Cancelling);
    assert_eq!(state.status_text(), "Cancelling...");
    assert_eq!(
        effects,
        vec![PanelSideEffect::SendCancel {
            session_id: "session-p2-001".to_string()
        }]
    );

    // Cancel acknowledged -> Back to Ready
    state.reduce(PanelAction::PromptCancelled);
    assert_eq!(state.lifecycle, PanelLifecycle::Ready);
    assert_eq!(state.status_text(), "Ready");

    // 7. Auth-required state
    state.reduce(PanelAction::RequireAuth("Missing ANTHROPIC_API_KEY".to_string()));
    assert_eq!(
        state.lifecycle,
        PanelLifecycle::AuthRequired {
            reason: "Missing ANTHROPIC_API_KEY".to_string()
        }
    );
    assert_eq!(state.status_text(), "Auth Required");

    // 8. Error state
    let effects = state.reduce(PanelAction::ReportError {
        message: "Network gateway unreachable".to_string(),
        code: Some(503),
    });
    assert_eq!(
        state.lifecycle,
        PanelLifecycle::Error {
            message: "Network gateway unreachable".to_string(),
            code: Some(503),
        }
    );
    assert_eq!(state.status_text(), "Error");
    assert_eq!(
        effects,
        vec![PanelSideEffect::NotifyError(
            "Network gateway unreachable".to_string()
        )]
    );

    // 9. Disconnected state
    state.reduce(PanelAction::ProcessExited(Some(0)));
    assert_eq!(state.lifecycle, PanelLifecycle::Disconnected);
    assert_eq!(state.status_text(), "Disconnected");
}

#[test]
fn test_streaming_long_transcript_does_not_rebuild_messages() {
    let mut state = PanelState::new();
    state.lifecycle = PanelLifecycle::Ready;
    state.active_session_id = Some("session-perf".to_string());
    state.input_draft = "Stream 1000 tokens".to_string();

    state.reduce(PanelAction::SubmitPrompt);
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0].role, ChatRole::User);
    assert_eq!(state.messages[1].role, ChatRole::Assistant);
    assert!(state.messages[1].is_streaming);

    // Stream 1,000 incremental tokens
    for i in 0..1000 {
        state.reduce(PanelAction::AppendTextDelta(format!("w{} ", i)));
    }

    // Still exactly 2 messages in the transcript
    assert_eq!(state.messages.len(), 2);
    assert!(state.messages[1].content.starts_with("w0 w1 w2 "));
    assert!(state.messages[1].content.ends_with("w999 "));
    assert!(state.messages[1].is_streaming);

    // Finalize
    state.reduce(PanelAction::FinishStreaming);
    assert_eq!(state.lifecycle, PanelLifecycle::Ready);
    assert!(!state.messages[1].is_streaming);
}

#[test]
fn test_cancel_and_workspace_switch_cancels_stale_work() {
    let mut state = PanelState::new();
    state.lifecycle = PanelLifecycle::Ready;
    state.active_session_id = Some("workspace-a-session".to_string());
    state.input_draft = "In flight turn".to_string();

    // Start streaming
    state.reduce(PanelAction::SubmitPrompt);
    assert!(state.is_streaming());
    state.reduce(PanelAction::AppendTextDelta("Partial response...".to_string()));

    // User switches workspace
    let effects = state.reduce(PanelAction::SwitchWorkspace);

    // Must emit cancellation side effect
    assert_eq!(
        effects,
        vec![PanelSideEffect::SendCancel {
            session_id: "workspace-a-session".to_string()
        }]
    );

    // Must reset active session ID and clear stale transcript messages
    assert_eq!(state.active_session_id, None);
    assert!(state.messages.is_empty());
    assert_eq!(state.input_draft, "");
    assert_eq!(state.lifecycle, PanelLifecycle::Ready);
}

#[test]
fn test_tool_activity_tracking_in_transcript() {
    let mut state = PanelState::new();
    state.lifecycle = PanelLifecycle::Ready;
    state.active_session_id = Some("session-tools".to_string());
    state.input_draft = "Search files".to_string();

    state.reduce(PanelAction::SubmitPrompt);

    // Tool starts
    state.reduce(PanelAction::AddToolActivity {
        id: "call-search-1".to_string(),
        name: "search_workspace".to_string(),
        arguments: Some(serde_json::json!({ "query": "fn main" })),
    });

    let asst_msg = &state.messages[1];
    assert_eq!(asst_msg.tool_activities.len(), 1);
    assert_eq!(asst_msg.tool_activities[0].name, "search_workspace");
    assert_eq!(asst_msg.tool_activities[0].status, ToolStatus::Running);
    assert!(asst_msg.tool_activities[0].result.is_none());

    // Tool completes with result
    state.reduce(PanelAction::UpdateToolActivity {
        id: "call-search-1".to_string(),
        status: ToolStatus::Completed,
        result: Some(serde_json::json!({ "matches": ["src/main.rs:1"] })),
    });

    let asst_msg = &state.messages[1];
    assert_eq!(asst_msg.tool_activities[0].status, ToolStatus::Completed);
    assert_eq!(
        asst_msg.tool_activities[0].result,
        Some(serde_json::json!({ "matches": ["src/main.rs:1"] }))
    );

    // Assistant generates summary after tool
    state.reduce(PanelAction::AppendTextDelta("Found main function.".to_string()));
    state.reduce(PanelAction::FinishStreaming);

    assert_eq!(state.messages[1].content, "Found main function.");
    assert_eq!(state.lifecycle, PanelLifecycle::Ready);
}
