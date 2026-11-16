//! Privacy controls, credential scrubbing, and security audit event logging.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Categories of security-relevant audit events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventKind {
    SessionStarted { session_id: String, role: String },
    PatchApproved { file: String, hunks: usize },
    PatchRejected { file: String, reason: String },
    CommandApproved { command: String },
    CommandRejected { command: String, reason: String },
    BudgetBreached { metric: String, value: u64 },
    CrashRecovered { session_id: String },
}

/// A structured, timestamped audit log entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp_ms: u64,
    pub kind: AuditEventKind,
    pub user_action: bool,
}

/// Privacy filter utility for scrubbing credentials, auth headers, and secrets
/// from outgoing diagnostics and bug reports.
pub struct PrivacyFilter;

impl PrivacyFilter {
    /// Scrub sensitive API keys, tokens, and credentials from log strings.
    pub fn scrub(text: &str) -> String {
        let mut scrubbed = text.to_string();

        let patterns = [
            (r"AIza[A-Za-z0-9_-]{20,}", "[REDACTED_GEMINI_KEY]"),
            (r"sk-[A-Za-z0-9_-]{20,}", "[REDACTED_API_KEY]"),
            (r"octo_[A-Za-z0-9_-]{16,}", "[REDACTED_OCTOMIND_TOKEN]"),
            (r"Bearer\s+[A-Za-z0-9_\-.]+", "Bearer [REDACTED_TOKEN]"),
            (r"ghp_[A-Za-z0-9_-]{20,}", "[REDACTED_GITHUB_TOKEN]"),
        ];

        for (pattern, replacement) in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                scrubbed = re.replace_all(&scrubbed, replacement).to_string();
            }
        }

        scrubbed
    }
}

/// Append-only audit logger writing to disk.
pub struct AuditLog {
    log_path: PathBuf,
}

impl AuditLog {
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        Self {
            log_path: storage_dir.as_ref().join("audit.log"),
        }
    }

    pub fn log_path(&self) -> &Path {
        &self.log_path
    }

    /// Record a structured audit entry.
    pub fn record(
        &self,
        kind: AuditEventKind,
        user_action: bool,
        timestamp_ms: u64,
    ) -> Result<AuditEntry, std::io::Error> {
        if let Some(parent) = self.log_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let entry = AuditEntry {
            timestamp_ms,
            kind,
            user_action,
        };

        let line = serde_json::to_string(&entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)?;

        writeln!(file, "{}", line)?;

        Ok(entry)
    }

    /// Read all recorded audit entries.
    pub fn read_entries(&self) -> Result<Vec<AuditEntry>, std::io::Error> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&self.log_path)?;
        let mut entries = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if !line.is_empty()
                && let Ok(entry) = serde_json::from_str::<AuditEntry>(line)
            {
                entries.push(entry);
            }
        }

        Ok(entries)
    }
}
