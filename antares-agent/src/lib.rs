//! Antares agent integration boundary.
//!
//! Provides child process supervision, JSON-RPC 2.0 stdio transport,
//! session lifecycle, and contract fixtures for the Octomind 0.72.0 ACP integration.

pub mod capabilities;
pub mod codec;
pub mod context;
pub mod error;
pub mod mcp;
pub mod metrics;
pub mod patch;
pub mod persistence;
pub mod protocol;
pub mod session;
pub mod supervisor;
pub mod ui_state;

use std::path::PathBuf;

pub use capabilities::{
    AgentRole, BackgroundTask, ProviderConfig, ProviderKind, TaskManager,
    TaskStatus, TokenTracker,
};
pub use context::{
    ContextBudget, ContextError, ContextService, ContextSnapshot, DiagnosticItem,
    DiagnosticSeverity, EditorRange, FileContext, SelectionContext, redact_secrets,
};
pub use error::AgentError;
pub use mcp::{McpBridge, McpToolError};
pub use metrics::{BudgetViolation, LatencyStats, PerformanceBudgets, PerformanceMetrics};
pub use patch::{
    CommandProposal, CommandStatus, PatchError, PatchHunk, PatchPipeline,
    PatchProposal, PatchResult, ProposalStatus,
};
pub use persistence::{PersistenceError, PersistentSession, SessionStorage};
pub use protocol::{ContentBlock, InitializeResult, SessionEvent, SessionResult};
pub use session::{AgentSession, SessionState};
pub use supervisor::{ProcessLaunchOptions, ProcessSupervisor, SupervisorEvent};
pub use ui_state::{
    ChatMessage, ChatRole, FocusTarget, PanelAction, PanelLifecycle,
    PanelSideEffect, PanelState, SessionSummary, ToolActivity, ToolStatus,
};

/// Process configuration owned by Antares.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SidecarConfig {
    /// An optional explicit executable path. If absent, search `PATH`.
    pub binary: Option<PathBuf>,
}

/// Locate the Octomind executable without starting a process.
pub fn locate_binary(config: &SidecarConfig) -> Result<PathBuf, AgentError> {
    if let Some(path) = config.binary.as_ref() {
        return if path.is_file() {
            Ok(path.clone())
        } else {
            Err(AgentError::InvalidBinaryPath(path.clone()))
        };
    }

    let executable = if cfg!(windows) {
        "octomind.exe"
    } else {
        "octomind"
    };
    let paths = std::env::var_os("PATH").ok_or(AgentError::BinaryNotFound)?;

    std::env::split_paths(&paths)
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
        .ok_or(AgentError::BinaryNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_missing_binary_is_reported() {
        let missing = PathBuf::from("/antares-test/nonexistent/octomind");
        let config = SidecarConfig {
            binary: Some(missing.clone()),
        };

        assert_eq!(
            locate_binary(&config),
            Err(AgentError::InvalidBinaryPath(missing))
        );
    }

    #[test]
    fn explicit_existing_file_is_returned() {
        let file =
            std::env::current_exe().expect("test executable path should exist");
        let config = SidecarConfig {
            binary: Some(file.clone()),
        };

        assert_eq!(locate_binary(&config), Ok(file));
    }
}
