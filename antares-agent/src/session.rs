use std::{sync::Arc, time::Duration};

use parking_lot::Mutex;
use serde_json::Value;

use crate::{
    error::AgentError,
    protocol::{
        ContentBlock, InitializeParams, InitializeResult, SessionCancelParams,
        SessionEvent, SessionLoadParams, SessionNewParams, SessionPromptParams,
        SessionResult,
    },
    supervisor::{ProcessSupervisor, SupervisorEvent},
};

/// Lifecycle state of an Antares ACP Agent Session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    Uninitialized,
    Ready,
    ActivePrompt,
    Cancelling,
    Closed,
    Failed(AgentError),
}

/// High-level session manager orchestrating ACP turns with the child process.
#[derive(Clone)]
pub struct AgentSession {
    supervisor: Arc<ProcessSupervisor>,
    current_session_id: Arc<Mutex<Option<String>>>,
    state: Arc<Mutex<SessionState>>,
}

impl AgentSession {
    /// Create a session controller over a running process supervisor.
    pub fn new(supervisor: Arc<ProcessSupervisor>) -> Self {
        Self {
            supervisor,
            current_session_id: Arc::new(Mutex::new(None)),
            state: Arc::new(Mutex::new(SessionState::Uninitialized)),
        }
    }

    /// Current session lifecycle state.
    pub fn state(&self) -> SessionState {
        self.state.lock().clone()
    }

    /// Currently active session ID, if any.
    pub fn session_id(&self) -> Option<String> {
        self.current_session_id.lock().clone()
    }

    /// Perform ACP initialization handshake with the agent.
    pub fn initialize(
        &self,
        timeout: Duration,
    ) -> Result<InitializeResult, AgentError> {
        let params = InitializeParams::default();
        let val = serde_json::to_value(&params)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        let res = self
            .supervisor
            .send_request("initialize", Some(val), timeout)?;
        let init_result: InitializeResult = serde_json::from_value(res)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        *self.state.lock() = SessionState::Ready;
        Ok(init_result)
    }

    /// Create a new session in the specified workspace directory.
    pub fn new_session(
        &self,
        cwd: impl Into<String>,
        mcp_servers: Option<Vec<Value>>,
        timeout: Duration,
    ) -> Result<SessionResult, AgentError> {
        let params = SessionNewParams {
            cwd: cwd.into(),
            mcp_servers,
        };
        let val = serde_json::to_value(&params)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        let res = self
            .supervisor
            .send_request("session/new", Some(val), timeout)?;
        let session_result: SessionResult = serde_json::from_value(res)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        *self.current_session_id.lock() = Some(session_result.session_id.clone());
        *self.state.lock() = SessionState::Ready;
        Ok(session_result)
    }

    /// Resume an existing saved session by ID.
    pub fn load_session(
        &self,
        session_id: impl Into<String>,
        timeout: Duration,
    ) -> Result<SessionResult, AgentError> {
        let sid = session_id.into();
        let params = SessionLoadParams {
            session_id: sid.clone(),
        };
        let val = serde_json::to_value(&params)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        let res =
            self.supervisor
                .send_request("session/load", Some(val), timeout)?;
        let session_result: SessionResult = serde_json::from_value(res)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        *self.current_session_id.lock() = Some(session_result.session_id.clone());
        *self.state.lock() = SessionState::Ready;
        Ok(session_result)
    }

    /// Send a user prompt with text or context blocks.
    pub fn prompt(
        &self,
        blocks: Vec<ContentBlock>,
        timeout: Duration,
    ) -> Result<Value, AgentError> {
        let session_id = self.session_id().ok_or_else(|| {
            AgentError::SessionNotFound("no active session".to_string())
        })?;

        let params = SessionPromptParams {
            session_id,
            prompt: blocks,
        };
        let val = serde_json::to_value(&params)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        *self.state.lock() = SessionState::ActivePrompt;
        let res = self
            .supervisor
            .send_request("session/prompt", Some(val), timeout);

        match res {
            Ok(v) => {
                *self.state.lock() = SessionState::Ready;
                Ok(v)
            }
            Err(e) => {
                *self.state.lock() = SessionState::Failed(e.clone());
                Err(e)
            }
        }
    }

    /// Request cancellation of the currently active turn.
    pub fn cancel(&self) -> Result<(), AgentError> {
        let session_id = self.session_id().ok_or_else(|| {
            AgentError::SessionNotFound("no active session to cancel".to_string())
        })?;

        *self.state.lock() = SessionState::Cancelling;
        let params = SessionCancelParams { session_id };
        let val = serde_json::to_value(&params)
            .map_err(|e| AgentError::SerializationError(e.to_string()))?;

        // session/cancel is typically a notification or fast request
        let _ = self
            .supervisor
            .send_notification("session/cancel", Some(val));
        Ok(())
    }

    /// Wait for the next typed session event with timeout.
    pub fn next_event(
        &self,
        timeout: Duration,
    ) -> Result<Option<SessionEvent>, AgentError> {
        match self.supervisor.recv_event_timeout(timeout) {
            Ok(SupervisorEvent::SessionUpdate(update)) => {
                Ok(Some(update.parse_event()))
            }
            Ok(SupervisorEvent::ProcessExited { status }) => {
                *self.state.lock() =
                    SessionState::Failed(AgentError::ProcessExited(status));
                Err(AgentError::ProcessExited(status))
            }
            Ok(SupervisorEvent::Notification(_)) => Ok(None),
            Err(AgentError::Timeout(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Shutdown the underlying child process.
    pub fn shutdown(&self, timeout: Duration) -> Result<(), AgentError> {
        *self.state.lock() = SessionState::Closed;
        self.supervisor.shutdown(timeout)
    }
}
