//! Integration and stress tests for Phase 6: Performance hardening, latency budgets,
//! keyboard navigation history, and long transcript streaming.

use std::time::Instant;

use antares_agent::{
    BudgetViolation, ChatRole, FocusTarget, PanelAction, PanelLifecycle, PanelState,
    PerformanceBudgets, PerformanceMetrics,
};

#[test]
fn test_performance_metrics_and_budget_enforcement() {
    let budgets = PerformanceBudgets {
        max_typing_latency_ms: 16.6,
        max_scroll_frame_ms: 16.6,
        max_cancellation_latency_ms: 500.0,
        max_event_queue_depth: 100,
    };
    let mut metrics = PerformanceMetrics::new(budgets);

    // Record healthy latencies
    metrics.record_typing_latency(8.0);
    metrics.record_typing_latency(12.0);
    metrics.record_typing_latency(14.0);
    metrics.record_scroll_frame(10.0);
    metrics.record_scroll_frame(15.0);
    metrics.record_context_build(5.0);
    metrics.record_cancellation_latency(80.0);
    metrics.set_queue_depth(42);

    let typing = metrics.typing_stats();
    assert_eq!(typing.count, 3);
    assert!(typing.mean_ms < 16.6);
    assert!(typing.p95_ms <= 14.0);

    let scroll = metrics.scroll_stats();
    assert_eq!(scroll.count, 2);
    assert!(scroll.p95_ms <= 15.0);

    let cancel = metrics.cancellation_stats();
    assert_eq!(cancel.count, 1);
    assert_eq!(cancel.max_ms, 80.0);

    // No violations in healthy state
    assert!(metrics.check_budgets().is_empty());

    // Inject budget violations
    metrics.record_typing_latency(35.0); // > 16.6ms
    metrics.set_queue_depth(150); // > 100

    let violations = metrics.check_budgets();
    assert!(violations.iter().any(|v| matches!(
        v,
        BudgetViolation::TypingLatencyExceeded { .. }
    )));
    assert!(violations.iter().any(|v| matches!(
        v,
        BudgetViolation::EventQueueOverflow { .. }
    )));
}

#[test]
fn test_keyboard_navigation_and_draft_history() {
    let mut state = PanelState::new();
    state.active_session_id = Some("sess-1".to_string());
    state.lifecycle = PanelLifecycle::Ready;

    // Send three prompts
    state.reduce(PanelAction::SetInputDraft("git status".to_string()));
    state.reduce(PanelAction::SubmitPrompt);
    state.lifecycle = PanelLifecycle::Ready;

    state.reduce(PanelAction::SetInputDraft("cargo check".to_string()));
    state.reduce(PanelAction::SubmitPrompt);
    state.lifecycle = PanelLifecycle::Ready;

    state.reduce(PanelAction::SetInputDraft("cargo test".to_string()));
    state.reduce(PanelAction::SubmitPrompt);
    state.lifecycle = PanelLifecycle::Ready;

    assert_eq!(state.draft_history.len(), 3);

    // Type a new draft that hasn't been sent yet
    state.reduce(PanelAction::SetInputDraft("my unfinished draft".to_string()));

    // Press Up arrow: navigate history backward
    state.reduce(PanelAction::NavigateHistoryPrev);
    assert_eq!(state.input_draft, "cargo test");

    state.reduce(PanelAction::NavigateHistoryPrev);
    assert_eq!(state.input_draft, "cargo check");

    state.reduce(PanelAction::NavigateHistoryPrev);
    assert_eq!(state.input_draft, "git status");

    // Pressing Up at the oldest entry stays at oldest
    state.reduce(PanelAction::NavigateHistoryPrev);
    assert_eq!(state.input_draft, "git status");

    // Press Down arrow: navigate history forward
    state.reduce(PanelAction::NavigateHistoryNext);
    assert_eq!(state.input_draft, "cargo check");

    state.reduce(PanelAction::NavigateHistoryNext);
    assert_eq!(state.input_draft, "cargo test");

    // Pressing Down at latest entry restores the unsent draft
    state.reduce(PanelAction::NavigateHistoryNext);
    assert_eq!(state.input_draft, "my unfinished draft");

    // Test explicit keyboard focus switching
    assert_eq!(state.focus_target, FocusTarget::Composer);
    state.reduce(PanelAction::SetFocusTarget(FocusTarget::Transcript));
    assert_eq!(state.focus_target, FocusTarget::Transcript);
}

#[test]
fn test_streaming_stress_10000_chunks_throughput_and_memory() {
    let mut state = PanelState::new();
    state.active_session_id = Some("sess-stress".to_string());
    state.lifecycle = PanelLifecycle::Ready;

    state.reduce(PanelAction::SetInputDraft("Generate large code".to_string()));
    state.reduce(PanelAction::SubmitPrompt);
    assert_eq!(state.lifecycle, PanelLifecycle::Streaming);

    let start = Instant::now();
    let num_chunks = 10_000;
    for i in 0..num_chunks {
        let chunk = format!("chunk-{} ", i);
        state.reduce(PanelAction::AppendTextDelta(chunk));
    }
    let elapsed = start.elapsed();

    // Verify 10,000 chunks processed efficiently (well under 500ms)
    assert!(
        elapsed.as_millis() < 500,
        "Streaming 10,000 chunks took {}ms (expected < 500ms)",
        elapsed.as_millis()
    );

    // Verify content integrity
    let assistant_msg = state.messages.last().unwrap();
    assert_eq!(assistant_msg.role, ChatRole::Assistant);
    assert!(assistant_msg.content.starts_with("chunk-0 "));
    assert!(assistant_msg.content.ends_with("chunk-9999 "));

    // Finish streaming
    state.reduce(PanelAction::FinishStreaming);
    assert_eq!(state.lifecycle, PanelLifecycle::Ready);
    assert!(!state.messages.last().unwrap().is_streaming);
}

#[test]
fn test_long_transcript_windowing_and_virtualization() {
    let mut state = PanelState::new();
    state.active_session_id = Some("sess-long".to_string());
    state.lifecycle = PanelLifecycle::Ready;

    // Simulate 200 message exchanges (400 messages total)
    for i in 0..200 {
        state.lifecycle = PanelLifecycle::Ready;
        state.reduce(PanelAction::SetInputDraft(format!("Question {}", i)));
        state.reduce(PanelAction::SubmitPrompt);
        state.reduce(PanelAction::AppendTextDelta(format!("Answer {}", i)));
        state.reduce(PanelAction::FinishStreaming);
    }
    assert_eq!(state.messages.len(), 400);

    // Trim transcript to retain only 50 most recent messages for UI performance
    state.reduce(PanelAction::TrimTranscript { max_messages: 50 });
    assert_eq!(state.messages.len(), 50);

    // Verify newest messages are preserved
    assert_eq!(state.messages.last().unwrap().content, "Answer 199");
}
