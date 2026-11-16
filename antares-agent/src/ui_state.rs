//! Headless domain state machine and pure reducer for the Antares AI panel.
//!
//! Conforms to Antares UI-Floem rules: keeps domain state and reducer logic
//! completely independent from GUI frameworks so every state transition can be
//! tested headlessly without native window dependencies.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::SessionEvent;

/// High-level lifecycle state of the AI Agent panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PanelLifecycle {
    /// No agent process or active session.
    Idle,
    /// Starting the agent child process and establishing ACP handshake.
    Starting,
    /// Connected and ready to process prompt turns.
    Ready,
    /// Actively receiving streaming response tokens or tool executions.
    Streaming,
    /// Cancellation requested by user; waiting for agent turn to halt.
    Cancelling,
    /// Agent or backend requires API key or authentication.
    AuthRequired { reason: String },
    /// Agent child process disconnected or exited.
    Disconnected,
    /// Agent or protocol encountered an error.
    Error { message: String, code: Option<i64> },
}

/// Role of a message author in the conversation transcript.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

/// Execution status of an agent tool invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolStatus {
    Running,
    Completed,
    Failed,
}

/// Tracking model for an agent tool call and its output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolActivity {
    pub id: String,
    pub name: String,
    pub arguments: Option<Value>,
    pub status: ToolStatus,
    pub result: Option<Value>,
}

/// A single message entry in the panel's conversation history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: ChatRole,
    pub content: String,
    pub tool_activities: Vec<ToolActivity>,
    pub is_streaming: bool,
    pub timestamp_ms: u64,
}

/// Metadata summary of a selectable agent session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub updated_at_ms: u64,
}

/// Navigable focus targets within the Antares AI panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusTarget {
    Composer,
    Transcript,
    SessionList,
    ToolDetails,
}

/// Complete domain state of the Antares AI panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelState {
    pub lifecycle: PanelLifecycle,
    pub active_session_id: Option<String>,
    pub sessions: Vec<SessionSummary>,
    pub messages: Vec<ChatMessage>,
    pub input_draft: String,
    pub agent_name: Option<String>,
    pub status_message: Option<String>,
    pub focus_target: FocusTarget,
    pub draft_history: Vec<String>,
    pub history_cursor: Option<usize>,
    pub temporary_draft: Option<String>,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            lifecycle: PanelLifecycle::Idle,
            active_session_id: None,
            sessions: Vec::new(),
            messages: Vec::new(),
            input_draft: String::new(),
            agent_name: None,
            status_message: Some("Idle".to_string()),
            focus_target: FocusTarget::Composer,
            draft_history: Vec::new(),
            history_cursor: None,
            temporary_draft: None,
        }
    }
}

/// Actions dispatched to the pure panel reducer.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelAction {
    /// User or system requests launching the agent process.
    StartProcess,
    /// Agent process successfully spawned and initialized.
    ProcessStarted { agent_name: Option<String> },
    /// Agent process failed to start.
    ProcessFailed(String),
    /// Agent process terminated with an optional exit code.
    ProcessExited(Option<i32>),
    /// A new ACP session was created.
    SessionCreated { session_id: String, title: String },
    /// User switched to another session.
    SessionSelected(String),
    /// User edited the composer input draft.
    SetInputDraft(String),
    /// User submitted the composer input draft as a prompt.
    SubmitPrompt,
    /// Incremental text delta received from the streaming agent.
    AppendTextDelta(String),
    /// Agent began executing a tool call.
    AddToolActivity {
        id: String,
        name: String,
        arguments: Option<Value>,
    },
    /// Agent completed a tool call.
    UpdateToolActivity {
        id: String,
        status: ToolStatus,
        result: Option<Value>,
    },
    /// Agent completed the current turn.
    FinishStreaming,
    /// User clicked Cancel during active prompt generation.
    RequestCancel,
    /// Cancellation handshake acknowledged by the agent.
    PromptCancelled,
    /// Agent indicated authentication or API credential required.
    RequireAuth(String),
    /// Generic or session-level error reported.
    ReportError { message: String, code: Option<i64> },
    /// User switched workspace; cancels in-flight work and resets active context.
    SwitchWorkspace,
    /// Clear transcript messages for the current view.
    ClearTranscript,
    /// Navigate to previous entry in prompt draft history.
    NavigateHistoryPrev,
    /// Navigate to next entry in prompt draft history.
    NavigateHistoryNext,
    /// Set explicit keyboard focus target.
    SetFocusTarget(FocusTarget),
    /// Trim display transcript to bounded recent messages to prevent rendering lag.
    TrimTranscript { max_messages: usize },
}

impl PanelAction {
    /// Convert an incoming ACP session event into a corresponding panel action.
    pub fn from_session_event(event: SessionEvent) -> Option<Self> {
        match event {
            SessionEvent::TextDelta(delta) => {
                Some(PanelAction::AppendTextDelta(delta))
            }
            SessionEvent::ToolCall {
                id,
                name,
                arguments,
            } => Some(PanelAction::AddToolActivity {
                id,
                name,
                arguments: if arguments.is_null() {
                    None
                } else {
                    Some(arguments)
                },
            }),
            SessionEvent::ToolResult { id, result } => {
                Some(PanelAction::UpdateToolActivity {
                    id,
                    status: ToolStatus::Completed,
                    result: Some(result),
                })
            }
            SessionEvent::Completed => Some(PanelAction::FinishStreaming),
            SessionEvent::Status(_) | SessionEvent::Extension(_) => None,
        }
    }
}

/// Side effects emitted by the pure reducer for asynchronous execution.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelSideEffect {
    LaunchProcess,
    SendPrompt { session_id: String, text: String },
    SendCancel { session_id: String },
    NotifyError(String),
}

impl PanelState {
    /// Create a fresh panel state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the user can currently submit a prompt.
    pub fn can_send(&self) -> bool {
        self.lifecycle == PanelLifecycle::Ready
            && !self.input_draft.trim().is_empty()
            && self.active_session_id.is_some()
    }

    /// Whether the user can currently cancel an ongoing turn.
    pub fn can_cancel(&self) -> bool {
        matches!(
            self.lifecycle,
            PanelLifecycle::Streaming | PanelLifecycle::Cancelling
        )
    }

    /// Whether the panel is actively receiving streaming tokens.
    pub fn is_streaming(&self) -> bool {
        self.lifecycle == PanelLifecycle::Streaming
    }

    /// Concise label representing the current lifecycle status for UI badges.
    pub fn status_text(&self) -> &str {
        match &self.lifecycle {
            PanelLifecycle::Idle => "Idle",
            PanelLifecycle::Starting => "Starting...",
            PanelLifecycle::Ready => "Ready",
            PanelLifecycle::Streaming => "Thinking...",
            PanelLifecycle::Cancelling => "Cancelling...",
            PanelLifecycle::AuthRequired { .. } => "Auth Required",
            PanelLifecycle::Disconnected => "Disconnected",
            PanelLifecycle::Error { .. } => "Error",
        }
    }

    /// Pure reducer function: processes an action, mutates state, and returns side effects.
    pub fn reduce(&mut self, action: PanelAction) -> Vec<PanelSideEffect> {
        let mut effects = Vec::new();

        match action {
            PanelAction::StartProcess => {
                if matches!(
                    self.lifecycle,
                    PanelLifecycle::Idle
                        | PanelLifecycle::Disconnected
                        | PanelLifecycle::Error { .. }
                ) {
                    self.lifecycle = PanelLifecycle::Starting;
                    self.status_message =
                        Some("Starting Octomind ACP agent...".to_string());
                    effects.push(PanelSideEffect::LaunchProcess);
                }
            }
            PanelAction::ProcessStarted { agent_name } => {
                self.lifecycle = PanelLifecycle::Ready;
                self.agent_name = agent_name;
                self.status_message = Some("Agent connected".to_string());
            }
            PanelAction::ProcessFailed(err) => {
                self.lifecycle = PanelLifecycle::Error {
                    message: err.clone(),
                    code: None,
                };
                self.status_message = Some(err.clone());
                effects.push(PanelSideEffect::NotifyError(err));
            }
            PanelAction::ProcessExited(code) => {
                self.lifecycle = PanelLifecycle::Disconnected;
                self.status_message = Some(format!(
                    "Agent process exited with code {}",
                    code.map(|c| c.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ));
            }
            PanelAction::SessionCreated { session_id, title } => {
                self.active_session_id = Some(session_id.clone());
                if let Some(existing) =
                    self.sessions.iter_mut().find(|s| s.id == session_id)
                {
                    existing.title = title;
                } else {
                    self.sessions.push(SessionSummary {
                        id: session_id,
                        title,
                        updated_at_ms: 0,
                    });
                }
                if self.lifecycle == PanelLifecycle::Starting {
                    self.lifecycle = PanelLifecycle::Ready;
                }
                self.status_message = Some("Session ready".to_string());
            }
            PanelAction::SessionSelected(session_id) => {
                if self.active_session_id.as_deref() != Some(&session_id) {
                    if self.is_streaming()
                        && let Some(ref active_id) = self.active_session_id
                    {
                        effects.push(PanelSideEffect::SendCancel {
                            session_id: active_id.clone(),
                        });
                    }
                    self.active_session_id = Some(session_id);
                    self.messages.clear();
                    self.lifecycle = PanelLifecycle::Ready;
                    self.status_message = Some("Switched session".to_string());
                }
            }
            PanelAction::SetInputDraft(text) => {
                self.input_draft = text;
            }
            PanelAction::SubmitPrompt => {
                if self.can_send() {
                    let prompt_text = std::mem::take(&mut self.input_draft);
                    let session_id = self
                        .active_session_id
                        .clone()
                        .expect("active_session_id verified in can_send");

                    // Append user message
                    self.messages.push(ChatMessage {
                        id: format!("msg-user-{}", self.messages.len()),
                        role: ChatRole::User,
                        content: prompt_text.clone(),
                        tool_activities: Vec::new(),
                        is_streaming: false,
                        timestamp_ms: 0,
                    });

                    if !prompt_text.trim().is_empty() {
                        self.draft_history.push(prompt_text.clone());
                    }
                    self.history_cursor = None;
                    self.temporary_draft = None;

                    // Append streaming assistant message placeholder
                    self.messages.push(ChatMessage {
                        id: format!("msg-asst-{}", self.messages.len()),
                        role: ChatRole::Assistant,
                        content: String::new(),
                        tool_activities: Vec::new(),
                        is_streaming: true,
                        timestamp_ms: 0,
                    });

                    self.lifecycle = PanelLifecycle::Streaming;
                    self.status_message = Some("Thinking...".to_string());
                    effects.push(PanelSideEffect::SendPrompt {
                        session_id,
                        text: prompt_text,
                    });
                }
            }
            PanelAction::AppendTextDelta(delta) => {
                if self.lifecycle == PanelLifecycle::Streaming
                    && let Some(last_msg) = self.messages.last_mut()
                    && last_msg.role == ChatRole::Assistant
                    && last_msg.is_streaming
                {
                    last_msg.content.push_str(&delta);
                }
            }
            PanelAction::AddToolActivity {
                id,
                name,
                arguments,
            } => {
                if self.lifecycle == PanelLifecycle::Streaming
                    && let Some(last_msg) = self.messages.last_mut()
                    && last_msg.role == ChatRole::Assistant
                {
                    last_msg.tool_activities.push(ToolActivity {
                        id,
                        name,
                        arguments,
                        status: ToolStatus::Running,
                        result: None,
                    });
                }
            }
            PanelAction::UpdateToolActivity { id, status, result } => {
                if let Some(last_msg) = self.messages.last_mut()
                    && let Some(tool) =
                        last_msg.tool_activities.iter_mut().find(|t| t.id == id)
                {
                    tool.status = status;
                    if result.is_some() {
                        tool.result = result;
                    }
                }
            }
            PanelAction::FinishStreaming => {
                if self.lifecycle == PanelLifecycle::Streaming {
                    if let Some(last_msg) = self.messages.last_mut()
                        && last_msg.role == ChatRole::Assistant
                    {
                        last_msg.is_streaming = false;
                    }
                    self.lifecycle = PanelLifecycle::Ready;
                    self.status_message = Some("Ready".to_string());
                }
            }
            PanelAction::RequestCancel => {
                if self.lifecycle == PanelLifecycle::Streaming {
                    self.lifecycle = PanelLifecycle::Cancelling;
                    self.status_message = Some("Cancelling turn...".to_string());
                    if let Some(ref session_id) = self.active_session_id {
                        effects.push(PanelSideEffect::SendCancel {
                            session_id: session_id.clone(),
                        });
                    }
                }
            }
            PanelAction::PromptCancelled => {
                if matches!(
                    self.lifecycle,
                    PanelLifecycle::Cancelling | PanelLifecycle::Streaming
                ) {
                    if let Some(last_msg) = self.messages.last_mut()
                        && last_msg.role == ChatRole::Assistant
                    {
                        last_msg.is_streaming = false;
                        if last_msg.content.is_empty() {
                            last_msg.content = "[Cancelled by user]".to_string();
                        }
                    }
                    self.lifecycle = PanelLifecycle::Ready;
                    self.status_message = Some("Prompt cancelled".to_string());
                }
            }
            PanelAction::RequireAuth(reason) => {
                self.lifecycle = PanelLifecycle::AuthRequired {
                    reason: reason.clone(),
                };
                self.status_message = Some(reason);
            }
            PanelAction::ReportError { message, code } => {
                if let Some(last_msg) = self.messages.last_mut()
                    && last_msg.role == ChatRole::Assistant
                    && last_msg.is_streaming
                {
                    last_msg.is_streaming = false;
                }
                self.lifecycle = PanelLifecycle::Error {
                    message: message.clone(),
                    code,
                };
                self.status_message = Some(message.clone());
                effects.push(PanelSideEffect::NotifyError(message));
            }
            PanelAction::SwitchWorkspace => {
                // Cancel stale work if currently streaming
                if self.is_streaming()
                    && let Some(ref session_id) = self.active_session_id
                {
                    effects.push(PanelSideEffect::SendCancel {
                        session_id: session_id.clone(),
                    });
                }
                self.active_session_id = None;
                self.messages.clear();
                self.input_draft.clear();
                self.lifecycle = PanelLifecycle::Ready;
                self.status_message =
                    Some("Workspace switched. Session reset.".to_string());
            }
            PanelAction::ClearTranscript => {
                self.messages.clear();
            }
            PanelAction::NavigateHistoryPrev => {
                if !self.draft_history.is_empty() {
                    match self.history_cursor {
                        None => {
                            self.temporary_draft = Some(self.input_draft.clone());
                            let last_idx = self.draft_history.len() - 1;
                            self.history_cursor = Some(last_idx);
                            self.input_draft = self.draft_history[last_idx].clone();
                        }
                        Some(idx) => {
                            if idx > 0 {
                                let new_idx = idx - 1;
                                self.history_cursor = Some(new_idx);
                                self.input_draft =
                                    self.draft_history[new_idx].clone();
                            }
                        }
                    }
                }
            }
            PanelAction::NavigateHistoryNext => {
                if let Some(idx) = self.history_cursor {
                    if idx + 1 < self.draft_history.len() {
                        let new_idx = idx + 1;
                        self.history_cursor = Some(new_idx);
                        self.input_draft = self.draft_history[new_idx].clone();
                    } else {
                        self.history_cursor = None;
                        self.input_draft =
                            self.temporary_draft.take().unwrap_or_default();
                    }
                }
            }
            PanelAction::SetFocusTarget(target) => {
                self.focus_target = target;
            }
            PanelAction::TrimTranscript { max_messages } => {
                if self.messages.len() > max_messages {
                    let drain_count = self.messages.len() - max_messages;
                    self.messages.drain(0..drain_count);
                }
            }
        }

        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let state = PanelState::new();
        assert_eq!(state.lifecycle, PanelLifecycle::Idle);
        assert_eq!(state.status_text(), "Idle");
        assert!(!state.can_send());
        assert!(!state.can_cancel());
        assert!(!state.is_streaming());
        assert!(state.messages.is_empty());
    }

    #[test]
    fn test_startup_and_session_flow() {
        let mut state = PanelState::new();

        // 1. Request start process
        let effects = state.reduce(PanelAction::StartProcess);
        assert_eq!(state.lifecycle, PanelLifecycle::Starting);
        assert_eq!(state.status_text(), "Starting...");
        assert_eq!(effects, vec![PanelSideEffect::LaunchProcess]);

        // 2. Process started
        let effects = state.reduce(PanelAction::ProcessStarted {
            agent_name: Some("octomind-0.72.0".to_string()),
        });
        assert_eq!(state.lifecycle, PanelLifecycle::Ready);
        assert_eq!(state.agent_name.as_deref(), Some("octomind-0.72.0"));
        assert!(effects.is_empty());

        // 3. Create session
        let effects = state.reduce(PanelAction::SessionCreated {
            session_id: "sess-123".to_string(),
            title: "Test Session".to_string(),
        });
        assert_eq!(state.active_session_id.as_deref(), Some("sess-123"));
        assert_eq!(state.sessions.len(), 1);
        assert!(effects.is_empty());

        // Cannot send because draft is empty
        assert!(!state.can_send());

        // 4. Set input draft
        state.reduce(PanelAction::SetInputDraft("What is Lapce?".to_string()));
        assert!(state.can_send());
    }

    #[test]
    fn test_prompt_streaming_and_incremental_delta() {
        let mut state = PanelState::new();
        state.lifecycle = PanelLifecycle::Ready;
        state.active_session_id = Some("sess-456".to_string());
        state.input_draft = "Hello agent".to_string();

        // 1. Submit prompt
        let effects = state.reduce(PanelAction::SubmitPrompt);
        assert_eq!(state.lifecycle, PanelLifecycle::Streaming);
        assert_eq!(state.status_text(), "Thinking...");
        assert!(state.can_cancel());
        assert!(!state.can_send());
        assert_eq!(state.input_draft, "");
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].role, ChatRole::User);
        assert_eq!(state.messages[0].content, "Hello agent");
        assert_eq!(state.messages[1].role, ChatRole::Assistant);
        assert_eq!(state.messages[1].content, "");
        assert!(state.messages[1].is_streaming);
        assert_eq!(
            effects,
            vec![PanelSideEffect::SendPrompt {
                session_id: "sess-456".to_string(),
                text: "Hello agent".to_string(),
            }]
        );

        // 2. Stream incremental deltas without rebuilding message list
        for i in 0..10 {
            state.reduce(PanelAction::AppendTextDelta(format!("tok{} ", i)));
        }
        assert_eq!(
            state.messages[1].content,
            "tok0 tok1 tok2 tok3 tok4 tok5 tok6 tok7 tok8 tok9 "
        );

        // 3. Tool activity
        state.reduce(PanelAction::AddToolActivity {
            id: "call-1".to_string(),
            name: "read_file".to_string(),
            arguments: Some(serde_json::json!({ "path": "main.rs" })),
        });
        assert_eq!(state.messages[1].tool_activities.len(), 1);
        assert_eq!(
            state.messages[1].tool_activities[0].status,
            ToolStatus::Running
        );

        state.reduce(PanelAction::UpdateToolActivity {
            id: "call-1".to_string(),
            status: ToolStatus::Completed,
            result: Some(serde_json::json!({ "lines": 42 })),
        });
        assert_eq!(
            state.messages[1].tool_activities[0].status,
            ToolStatus::Completed
        );

        // 4. Finish streaming
        state.reduce(PanelAction::FinishStreaming);
        assert_eq!(state.lifecycle, PanelLifecycle::Ready);
        assert!(!state.messages[1].is_streaming);
        assert_eq!(state.status_text(), "Ready");
    }

    #[test]
    fn test_cancellation_flow() {
        let mut state = PanelState::new();
        state.lifecycle = PanelLifecycle::Ready;
        state.active_session_id = Some("sess-cancel".to_string());
        state.input_draft = "Long query".to_string();

        state.reduce(PanelAction::SubmitPrompt);
        assert_eq!(state.lifecycle, PanelLifecycle::Streaming);

        // Request cancel
        let effects = state.reduce(PanelAction::RequestCancel);
        assert_eq!(state.lifecycle, PanelLifecycle::Cancelling);
        assert_eq!(state.status_text(), "Cancelling...");
        assert_eq!(
            effects,
            vec![PanelSideEffect::SendCancel {
                session_id: "sess-cancel".to_string()
            }]
        );

        // Acknowledge cancel
        state.reduce(PanelAction::PromptCancelled);
        assert_eq!(state.lifecycle, PanelLifecycle::Ready);
        assert_eq!(state.status_text(), "Ready");
        assert!(!state.messages[1].is_streaming);
        assert_eq!(state.messages[1].content, "[Cancelled by user]");
    }

    #[test]
    fn test_workspace_switch_cancels_stale_work() {
        let mut state = PanelState::new();
        state.lifecycle = PanelLifecycle::Ready;
        state.active_session_id = Some("sess-old".to_string());
        state.input_draft = "In progress".to_string();

        state.reduce(PanelAction::SubmitPrompt);
        assert!(state.is_streaming());

        // Switch workspace during streaming
        let effects = state.reduce(PanelAction::SwitchWorkspace);
        assert_eq!(
            effects,
            vec![PanelSideEffect::SendCancel {
                session_id: "sess-old".to_string()
            }]
        );
        assert_eq!(state.lifecycle, PanelLifecycle::Ready);
        assert_eq!(state.active_session_id, None);
        assert!(state.messages.is_empty());
        assert_eq!(state.input_draft, "");
    }

    #[test]
    fn test_auth_required_and_error_handling() {
        let mut state = PanelState::new();

        // Auth required
        state.reduce(PanelAction::RequireAuth("Missing API Key".to_string()));
        assert_eq!(
            state.lifecycle,
            PanelLifecycle::AuthRequired {
                reason: "Missing API Key".to_string()
            }
        );
        assert_eq!(state.status_text(), "Auth Required");

        // Error report
        let effects = state.reduce(PanelAction::ReportError {
            message: "Connection timeout".to_string(),
            code: Some(-32000),
        });
        assert_eq!(
            state.lifecycle,
            PanelLifecycle::Error {
                message: "Connection timeout".to_string(),
                code: Some(-32000),
            }
        );
        assert_eq!(state.status_text(), "Error");
        assert_eq!(
            effects,
            vec![PanelSideEffect::NotifyError(
                "Connection timeout".to_string()
            )]
        );
    }

    #[test]
    fn test_abrupt_process_exit() {
        let mut state = PanelState::new();
        state.lifecycle = PanelLifecycle::Ready;

        state.reduce(PanelAction::ProcessExited(Some(137)));
        assert_eq!(state.lifecycle, PanelLifecycle::Disconnected);
        assert_eq!(state.status_text(), "Disconnected");
        assert!(state.status_message.unwrap().contains("137"));
    }

    #[test]
    fn test_from_session_event_mapping() {
        assert_eq!(
            PanelAction::from_session_event(SessionEvent::TextDelta(
                "hi".to_string()
            )),
            Some(PanelAction::AppendTextDelta("hi".to_string()))
        );
        assert_eq!(
            PanelAction::from_session_event(SessionEvent::Completed),
            Some(PanelAction::FinishStreaming)
        );
        assert_eq!(
            PanelAction::from_session_event(SessionEvent::ToolCall {
                id: "tc1".to_string(),
                name: "test".to_string(),
                arguments: Value::Null,
            }),
            Some(PanelAction::AddToolActivity {
                id: "tc1".to_string(),
                name: "test".to_string(),
                arguments: None,
            })
        );
    }
}
