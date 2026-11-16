//! Session persistence and multi-turn transcript storage for Antares IDE.
//!
//! Stores conversation history, tool executions, and token metrics atomically to disk
//! under `.antares/sessions/` or the designated platform storage directory.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ui_state::ChatMessage;

/// Fully serializable multi-turn session archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersistentSession {
    pub session_id: String,
    pub workspace_root: PathBuf,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub active_role: String,
    pub model_id: String,
    pub total_input_tokens: usize,
    pub total_output_tokens: usize,
    pub messages: Vec<ChatMessage>,
}

impl PersistentSession {
    pub fn new(
        session_id: impl Into<String>,
        workspace_root: impl Into<PathBuf>,
        active_role: impl Into<String>,
        model_id: impl Into<String>,
        created_at_ms: u64,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            workspace_root: workspace_root.into(),
            created_at_ms,
            updated_at_ms: created_at_ms,
            active_role: active_role.into(),
            model_id: model_id.into(),
            total_input_tokens: 0,
            total_output_tokens: 0,
            messages: Vec::new(),
        }
    }

    pub fn append_message(&mut self, message: ChatMessage, timestamp_ms: u64) {
        self.messages.push(message);
        self.updated_at_ms = timestamp_ms;
    }

    pub fn add_token_usage(&mut self, input: usize, output: usize) {
        self.total_input_tokens += input;
        self.total_output_tokens += output;
    }
}

/// Errors originating from session persistence.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PersistenceError {
    #[error("Session not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("IO error: {0}")]
    Io(String),
}

/// Storage manager for persisting and loading sessions.
pub struct SessionStorage {
    storage_dir: PathBuf,
}

impl SessionStorage {
    /// Initialize session storage under a designated directory.
    pub fn new(storage_dir: impl AsRef<Path>) -> Result<Self, PersistenceError> {
        let dir = storage_dir.as_ref().to_path_buf();
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| {
                PersistenceError::Io(format!("Cannot create session dir: {}", e))
            })?;
        }
        Ok(Self { storage_dir: dir })
    }

    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }

    fn session_file_path(&self, session_id: &str) -> PathBuf {
        let sanitized = session_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();
        self.storage_dir.join(format!("{}.json", sanitized))
    }

    /// Save a persistent session atomically to disk.
    pub fn save_session(
        &self,
        session: &PersistentSession,
    ) -> Result<PathBuf, PersistenceError> {
        let file_path = self.session_file_path(&session.session_id);
        let temp_path = self
            .storage_dir
            .join(format!(".{}.tmp", session.session_id));

        let json_text = serde_json::to_string_pretty(session)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        fs::write(&temp_path, json_text).map_err(|e| {
            PersistenceError::Io(format!("Failed to write temp session: {}", e))
        })?;

        if let Err(e) = fs::rename(&temp_path, &file_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(PersistenceError::Io(format!(
                "Failed to atomic rename: {}",
                e
            )));
        }

        Ok(file_path)
    }

    /// Load a persistent session by ID.
    pub fn load_session(
        &self,
        session_id: &str,
    ) -> Result<PersistentSession, PersistenceError> {
        let file_path = self.session_file_path(session_id);
        if !file_path.exists() {
            return Err(PersistenceError::NotFound(session_id.to_string()));
        }

        let bytes = fs::read(&file_path).map_err(|e| {
            PersistenceError::Io(format!("Failed to read session file: {}", e))
        })?;

        let session: PersistentSession = serde_json::from_slice(&bytes)
            .map_err(|e| PersistenceError::Serialization(e.to_string()))?;

        Ok(session)
    }

    /// Delete a persistent session from disk.
    pub fn delete_session(
        &self,
        session_id: &str,
    ) -> Result<bool, PersistenceError> {
        let file_path = self.session_file_path(session_id);
        if file_path.exists() {
            fs::remove_file(&file_path).map_err(|e| {
                PersistenceError::Io(format!("Failed to delete session: {}", e))
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// List all stored session IDs.
    pub fn list_session_ids(&self) -> Result<Vec<String>, PersistenceError> {
        let mut ids = Vec::new();
        let entries = fs::read_dir(&self.storage_dir)
            .map_err(|e| PersistenceError::Io(e.to_string()))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|e| e.to_str()) == Some("json")
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                && !stem.starts_with('.')
            {
                ids.push(stem.to_string());
            }
        }

        ids.sort();
        Ok(ids)
    }
}
