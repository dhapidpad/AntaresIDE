//! Crash recovery, write-ahead logging (WAL), and interrupted-turn state restoration
//! for Antares IDE agent sessions.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// High-level phase of an in-flight prompt turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnPhase {
    PendingDispatch,
    StreamingTurn { chunks_received: usize },
    ToolInvoked { tool_id: String },
    Committed,
}

/// A serialized write-ahead record of an in-flight prompt turn.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterruptedTurn {
    pub session_id: String,
    pub prompt: String,
    pub phase: TurnPhase,
    pub started_at_ms: u64,
    pub last_active_at_ms: u64,
}

/// Errors originating from the crash recovery manager.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RecoveryError {
    #[error("Recovery IO error: {0}")]
    Io(String),

    #[error("Recovery serialization error: {0}")]
    Serialization(String),
}

/// Manager handling turn journaling and startup crash recovery.
pub struct RecoveryManager {
    wal_path: PathBuf,
}

impl RecoveryManager {
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        Self {
            wal_path: storage_dir.as_ref().join("recovery.wal"),
        }
    }

    pub fn wal_path(&self) -> &Path {
        &self.wal_path
    }

    /// Record the start of an agent prompt turn before dispatching to ACP process.
    pub fn record_turn_start(
        &self,
        session_id: impl Into<String>,
        prompt: impl Into<String>,
        timestamp_ms: u64,
    ) -> Result<(), RecoveryError> {
        if let Some(parent) = self.wal_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                RecoveryError::Io(format!("Failed to create WAL dir: {}", e))
            })?;
        }

        let turn = InterruptedTurn {
            session_id: session_id.into(),
            prompt: prompt.into(),
            phase: TurnPhase::PendingDispatch,
            started_at_ms: timestamp_ms,
            last_active_at_ms: timestamp_ms,
        };

        let json = serde_json::to_string(&turn)
            .map_err(|e| RecoveryError::Serialization(e.to_string()))?;
        fs::write(&self.wal_path, json)
            .map_err(|e| RecoveryError::Io(format!("Failed to write WAL: {}", e)))?;

        Ok(())
    }

    /// Update the progress of an in-flight turn.
    pub fn update_phase(
        &self,
        phase: TurnPhase,
        timestamp_ms: u64,
    ) -> Result<bool, RecoveryError> {
        if !self.wal_path.exists() {
            return Ok(false);
        }

        let bytes = fs::read(&self.wal_path)
            .map_err(|e| RecoveryError::Io(format!("Failed to read WAL: {}", e)))?;
        let mut turn: InterruptedTurn = serde_json::from_slice(&bytes)
            .map_err(|e| RecoveryError::Serialization(e.to_string()))?;

        turn.phase = phase;
        turn.last_active_at_ms = timestamp_ms;

        let json = serde_json::to_string(&turn)
            .map_err(|e| RecoveryError::Serialization(e.to_string()))?;
        fs::write(&self.wal_path, json).map_err(|e| {
            RecoveryError::Io(format!("Failed to update WAL: {}", e))
        })?;

        Ok(true)
    }

    /// Clear the write-ahead journal after graceful turn completion.
    pub fn record_turn_finish(&self) -> Result<bool, RecoveryError> {
        if self.wal_path.exists() {
            fs::remove_file(&self.wal_path).map_err(|e| {
                RecoveryError::Io(format!("Failed to clear WAL: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Inspect whether an uncommitted turn exists from a prior crash or termination.
    pub fn check_recovery(&self) -> Result<Option<InterruptedTurn>, RecoveryError> {
        if !self.wal_path.exists() {
            return Ok(None);
        }

        let bytes = fs::read(&self.wal_path)
            .map_err(|e| RecoveryError::Io(format!("Failed to read WAL: {}", e)))?;
        let turn: InterruptedTurn = serde_json::from_slice(&bytes)
            .map_err(|e| RecoveryError::Serialization(e.to_string()))?;

        Ok(Some(turn))
    }

    /// Recover an interrupted turn, remove the WAL, and generate notice message.
    pub fn recover_interrupted_turn(
        &self,
    ) -> Result<Option<(InterruptedTurn, String)>, RecoveryError> {
        if let Some(turn) = self.check_recovery()? {
            self.record_turn_finish()?;
            let notice = format!(
                concat!(
                    "[System: Previous turn was interrupted ",
                    "during {:?}. Prompt restored.]"
                ),
                turn.phase
            );
            Ok(Some((turn, notice)))
        } else {
            Ok(None)
        }
    }
}
