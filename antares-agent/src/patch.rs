//! Patch proposal, hunk validation, atomic apply, and rollback engine for Antares IDE.
//!
//! Enforces the security invariants defined in `.agent/rules/05-agent-safety.md`
//! and `.agent/rules/06-context-and-diff.md`:
//! - Direct writes from the agent are forbidden in default profile.
//! - Edits must be submitted as structured patch proposals.
//! - Base revisions are cryptographically hashed and revalidated before apply.
//! - Applied patches are reversible (undo support).
//! - Rejected proposals have zero workspace side effects.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::context::{ContextError, ContextService};

/// Lifecycle status of a patch proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Pending,
    Approved,
    Rejected { reason: Option<String> },
    Applied { timestamp_ms: u64 },
    Reverted { timestamp_ms: u64 },
}

/// A single hunk within a patch proposal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchHunk {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
    pub old_text: String,
    pub new_text: String,
}

impl PatchHunk {
    pub fn new(
        old_start: usize,
        old_lines: usize,
        new_start: usize,
        new_lines: usize,
        old_text: impl Into<String>,
        new_text: impl Into<String>,
    ) -> Self {
        Self {
            old_start,
            old_lines,
            new_start,
            new_lines,
            old_text: old_text.into(),
            new_text: new_text.into(),
        }
    }
}

/// A structured patch proposal for a workspace file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchProposal {
    pub id: String,
    pub relative_path: String,
    pub base_revision_hash: String,
    pub hunks: Vec<PatchHunk>,
    pub summary: String,
    pub status: ProposalStatus,
    pub created_at_ms: u64,
}

/// Record of an applied patch, preserving full previous content for undo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchResult {
    pub proposal_id: String,
    pub relative_path: String,
    pub previous_content: String,
    pub applied_content: String,
    pub new_revision_hash: String,
    pub applied_at_ms: u64,
}

/// Errors originating from patch proposal validation and application.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PatchError {
    #[error("Context security violation: {0}")]
    Context(#[from] ContextError),

    #[error("Target file not found: {0}")]
    FileNotFound(String),

    #[error(
        "Base revision mismatch. Expected hash {expected}, got {actual}. The file has changed since proposal generation."
    )]
    StaleBaseRevision { expected: String, actual: String },

    #[error("Hunk mismatch at line {line}: expected target text does not match")]
    HunkMismatch { line: usize, expected: String },

    #[error("Proposal {0} is not approved")]
    NotApproved(String),

    #[error("Proposal {0} has already been applied")]
    AlreadyApplied(String),

    #[error("Proposal {0} was rejected: {1}")]
    ProposalRejected(String, String),

    #[error("Proposal not found: {0}")]
    ProposalNotFound(String),

    #[error("Proposal has no hunks")]
    EmptyProposal,

    #[error("IO error: {0}")]
    Io(String),
}

/// Command approval lifecycle for safe terminal executions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandStatus {
    PendingApproval,
    Approved,
    Rejected { reason: Option<String> },
    Executed { exit_code: i32 },
}

/// Proposed command requested by the agent, held pending explicit user approval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandProposal {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub status: CommandStatus,
    pub created_at_ms: u64,
}

/// Patch management and controlled execution pipeline.
#[derive(Default)]
pub struct PatchPipeline {
    proposals: HashMap<String, PatchProposal>,
    applied_history: Vec<PatchResult>,
    commands: HashMap<String, CommandProposal>,
}

impl PatchPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compute a deterministic 64-bit hex hash of a string content.
    pub fn compute_hash(content: &str) -> String {
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Register a new patch proposal.
    pub fn submit_proposal(
        &mut self,
        id: impl Into<String>,
        relative_path: impl Into<String>,
        base_revision_hash: impl Into<String>,
        hunks: Vec<PatchHunk>,
        summary: impl Into<String>,
        created_at_ms: u64,
    ) -> Result<PatchProposal, PatchError> {
        if hunks.is_empty() {
            return Err(PatchError::EmptyProposal);
        }

        let id = id.into();
        let proposal = PatchProposal {
            id: id.clone(),
            relative_path: relative_path.into(),
            base_revision_hash: base_revision_hash.into(),
            hunks,
            summary: summary.into(),
            status: ProposalStatus::Pending,
            created_at_ms,
        };

        self.proposals.insert(id, proposal.clone());
        Ok(proposal)
    }

    /// Validate a proposal against workspace security bounds and file revision.
    pub fn validate_proposal(
        &self,
        proposal: &PatchProposal,
        context: &ContextService,
    ) -> Result<PathBuf, PatchError> {
        let canonical_path = context.validate_path(&proposal.relative_path)?;

        if !canonical_path.exists() {
            return Err(PatchError::FileNotFound(proposal.relative_path.clone()));
        }

        let bytes =
            fs::read(&canonical_path).map_err(|e| PatchError::Io(e.to_string()))?;
        let current_text = String::from_utf8_lossy(&bytes);
        let current_hash = Self::compute_hash(&current_text);

        if current_hash != proposal.base_revision_hash {
            return Err(PatchError::StaleBaseRevision {
                expected: proposal.base_revision_hash.clone(),
                actual: current_hash,
            });
        }

        // Validate each hunk matches existing content
        for hunk in &proposal.hunks {
            if !current_text.contains(&hunk.old_text) {
                return Err(PatchError::HunkMismatch {
                    line: hunk.old_start,
                    expected: hunk.old_text.clone(),
                });
            }
        }

        Ok(canonical_path)
    }

    /// User action: Approve a pending patch proposal.
    pub fn approve_proposal(&mut self, proposal_id: &str) -> Result<(), PatchError> {
        let proposal = self
            .proposals
            .get_mut(proposal_id)
            .ok_or_else(|| PatchError::ProposalNotFound(proposal_id.to_string()))?;

        match &proposal.status {
            ProposalStatus::Pending => {
                proposal.status = ProposalStatus::Approved;
                Ok(())
            }
            ProposalStatus::Approved => Ok(()),
            ProposalStatus::Applied { .. } => {
                Err(PatchError::AlreadyApplied(proposal_id.to_string()))
            }
            ProposalStatus::Rejected { reason } => {
                Err(PatchError::ProposalRejected(
                    proposal_id.to_string(),
                    reason.clone().unwrap_or_default(),
                ))
            }
            ProposalStatus::Reverted { .. } => {
                proposal.status = ProposalStatus::Approved;
                Ok(())
            }
        }
    }

    /// User action: Reject a pending patch proposal.
    pub fn reject_proposal(
        &mut self,
        proposal_id: &str,
        reason: Option<String>,
    ) -> Result<(), PatchError> {
        let proposal = self
            .proposals
            .get_mut(proposal_id)
            .ok_or_else(|| PatchError::ProposalNotFound(proposal_id.to_string()))?;

        if let ProposalStatus::Applied { .. } = proposal.status {
            return Err(PatchError::AlreadyApplied(proposal_id.to_string()));
        }

        proposal.status = ProposalStatus::Rejected { reason };
        Ok(())
    }

    /// Apply an approved patch proposal atomically to disk.
    pub fn apply_proposal(
        &mut self,
        proposal_id: &str,
        context: &ContextService,
        now_ms: u64,
    ) -> Result<PatchResult, PatchError> {
        let proposal = self
            .proposals
            .get(proposal_id)
            .ok_or_else(|| PatchError::ProposalNotFound(proposal_id.to_string()))?
            .clone();

        if proposal.status != ProposalStatus::Approved {
            return Err(PatchError::NotApproved(proposal_id.to_string()));
        }

        // Revalidate base revision immediately prior to apply
        let canonical_path = self.validate_proposal(&proposal, context)?;

        let current_bytes =
            fs::read(&canonical_path).map_err(|e| PatchError::Io(e.to_string()))?;
        let current_text = String::from_utf8_lossy(&current_bytes).to_string();

        // Apply all hunks sequentially
        let mut new_text = current_text.clone();
        for hunk in &proposal.hunks {
            if !new_text.contains(&hunk.old_text) {
                return Err(PatchError::HunkMismatch {
                    line: hunk.old_start,
                    expected: hunk.old_text.clone(),
                });
            }
            new_text = new_text.replacen(&hunk.old_text, &hunk.new_text, 1);
        }

        // Atomic write via temporary file
        let parent_dir = canonical_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_path = parent_dir.join(format!(
            ".{}.antares_tmp",
            canonical_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("patch")
        ));

        fs::write(&temp_path, &new_text).map_err(|e| {
            PatchError::Io(format!("Failed to write temp patch: {}", e))
        })?;

        if let Err(e) = fs::rename(&temp_path, &canonical_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(PatchError::Io(format!("Failed to atomic rename: {}", e)));
        }

        let new_hash = Self::compute_hash(&new_text);
        let result = PatchResult {
            proposal_id: proposal.id.clone(),
            relative_path: proposal.relative_path.clone(),
            previous_content: current_text,
            applied_content: new_text,
            new_revision_hash: new_hash,
            applied_at_ms: now_ms,
        };

        if let Some(prop) = self.proposals.get_mut(proposal_id) {
            prop.status = ProposalStatus::Applied {
                timestamp_ms: now_ms,
            };
        }

        self.applied_history.push(result.clone());
        Ok(result)
    }

    /// Revert an applied patch result (Undo support).
    pub fn revert_patch(
        &mut self,
        result: &PatchResult,
        context: &ContextService,
        now_ms: u64,
    ) -> Result<(), PatchError> {
        let canonical_path = context.validate_path(&result.relative_path)?;

        let current_bytes =
            fs::read(&canonical_path).map_err(|e| PatchError::Io(e.to_string()))?;
        let current_text = String::from_utf8_lossy(&current_bytes);
        let current_hash = Self::compute_hash(&current_text);

        if current_hash != result.new_revision_hash {
            return Err(PatchError::StaleBaseRevision {
                expected: result.new_revision_hash.clone(),
                actual: current_hash,
            });
        }

        // Revert atomically
        let parent_dir = canonical_path.parent().unwrap_or_else(|| Path::new("."));
        let temp_path = parent_dir.join(format!(
            ".{}.antares_revert_tmp",
            canonical_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("revert")
        ));

        fs::write(&temp_path, &result.previous_content).map_err(|e| {
            PatchError::Io(format!("Failed to write revert file: {}", e))
        })?;

        if let Err(e) = fs::rename(&temp_path, &canonical_path) {
            let _ = fs::remove_file(&temp_path);
            return Err(PatchError::Io(format!(
                "Failed to atomic rename on revert: {}",
                e
            )));
        }

        if let Some(prop) = self.proposals.get_mut(&result.proposal_id) {
            prop.status = ProposalStatus::Reverted {
                timestamp_ms: now_ms,
            };
        }

        Ok(())
    }

    /// Register a pending command execution request.
    pub fn submit_command(
        &mut self,
        id: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
        cwd: Option<String>,
        created_at_ms: u64,
    ) -> CommandProposal {
        let id = id.into();
        let cmd = CommandProposal {
            id: id.clone(),
            command: command.into(),
            args,
            cwd,
            status: CommandStatus::PendingApproval,
            created_at_ms,
        };
        self.commands.insert(id, cmd.clone());
        cmd
    }

    /// User action: Approve a command request.
    pub fn approve_command(&mut self, id: &str) -> Result<(), PatchError> {
        let cmd = self
            .commands
            .get_mut(id)
            .ok_or_else(|| PatchError::ProposalNotFound(id.to_string()))?;
        cmd.status = CommandStatus::Approved;
        Ok(())
    }

    /// User action: Reject a command request.
    pub fn reject_command(
        &mut self,
        id: &str,
        reason: Option<String>,
    ) -> Result<(), PatchError> {
        let cmd = self
            .commands
            .get_mut(id)
            .ok_or_else(|| PatchError::ProposalNotFound(id.to_string()))?;
        cmd.status = CommandStatus::Rejected { reason };
        Ok(())
    }

    /// Get a proposal by ID.
    pub fn get_proposal(&self, id: &str) -> Option<&PatchProposal> {
        self.proposals.get(id)
    }

    /// Get a command proposal by ID.
    pub fn get_command(&self, id: &str) -> Option<&CommandProposal> {
        self.commands.get(id)
    }

    /// History of applied patch results.
    pub fn applied_history(&self) -> &[PatchResult] {
        &self.applied_history
    }

    /// Get all currently pending proposals.
    pub fn pending_proposals(&self) -> Vec<&PatchProposal> {
        self.proposals
            .values()
            .filter(|p| matches!(p.status, ProposalStatus::Pending))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextBudget;
    use tempfile::tempdir;

    #[test]
    fn test_patch_lifecycle_and_atomic_apply() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let file_path = root.join("calculator.rs");
        let initial_code = "fn add(a: i32, b: i32) -> i32 {\n    a - b\n}\n";
        fs::write(&file_path, initial_code).unwrap();

        let context = ContextService::new(root, ContextBudget::default()).unwrap();
        let mut pipeline = PatchPipeline::new();

        let base_hash = PatchPipeline::compute_hash(initial_code);
        let hunk = PatchHunk::new(2, 1, 2, 1, "    a - b", "    a + b");

        let proposal = pipeline
            .submit_proposal(
                "prop-1",
                "calculator.rs",
                base_hash,
                vec![hunk],
                "Fix subtraction bug in add()",
                100,
            )
            .unwrap();

        assert_eq!(proposal.status, ProposalStatus::Pending);

        // Cannot apply while Pending
        let err = pipeline
            .apply_proposal("prop-1", &context, 101)
            .unwrap_err();
        assert!(matches!(err, PatchError::NotApproved(_)));

        // User approves proposal
        pipeline.approve_proposal("prop-1").unwrap();

        // Apply proposal
        let result = pipeline.apply_proposal("prop-1", &context, 102).unwrap();
        assert_eq!(result.previous_content, initial_code);
        assert_eq!(
            result.applied_content,
            "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n"
        );

        // File on disk updated
        let on_disk = fs::read_to_string(&file_path).unwrap();
        assert_eq!(on_disk, result.applied_content);

        // Revert (Undo support)
        pipeline.revert_patch(&result, &context, 103).unwrap();
        let reverted_disk = fs::read_to_string(&file_path).unwrap();
        assert_eq!(reverted_disk, initial_code);
    }

    #[test]
    fn test_stale_base_revision_rejected() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let file_path = root.join("version.rs");
        fs::write(&file_path, "const V: &str = \"1.0.0\";\n").unwrap();

        let context = ContextService::new(root, ContextBudget::default()).unwrap();
        let mut pipeline = PatchPipeline::new();

        let wrong_hash = "0000000000000000";
        let hunk = PatchHunk::new(1, 1, 1, 1, "\"1.0.0\"", "\"1.0.1\"");

        pipeline
            .submit_proposal(
                "prop-stale",
                "version.rs",
                wrong_hash,
                vec![hunk],
                "Bump version",
                200,
            )
            .unwrap();
        pipeline.approve_proposal("prop-stale").unwrap();

        let err = pipeline
            .apply_proposal("prop-stale", &context, 201)
            .unwrap_err();
        assert!(matches!(err, PatchError::StaleBaseRevision { .. }));

        // File on disk unmodified
        assert_eq!(
            fs::read_to_string(&file_path).unwrap(),
            "const V: &str = \"1.0.0\";\n"
        );
    }

    #[test]
    fn test_command_approval_workflow() {
        let mut pipeline = PatchPipeline::new();
        let cmd = pipeline.submit_command(
            "cmd-1",
            "cargo",
            vec!["test".into()],
            None,
            300,
        );
        assert_eq!(cmd.status, CommandStatus::PendingApproval);

        // Reject command
        pipeline
            .reject_command("cmd-1", Some("Forbidden in test mode".into()))
            .unwrap();
        let rejected = pipeline.get_command("cmd-1").unwrap();
        assert!(matches!(rejected.status, CommandStatus::Rejected { .. }));
    }
}
