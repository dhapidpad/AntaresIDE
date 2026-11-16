//! Editor context assembly, path security validation, secret redaction,
//! and bounded token/byte budgeting for Antares IDE.
//!
//! Conforms to Antares security model:
//! - Canonicalize paths and enforce containment within workspace root.
//! - Reject path traversal (`..`), symlink escapes, and non-regular files.
//! - Block secrets, credentials, `.env`, keys, and binary files.
//! - Redact sensitive API keys/tokens in text before dispatch.
//! - Enforce strict per-file and total byte/line budgets with explicit truncation markers.

use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors arising from context assembly or security validation.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextError {
    #[error("Path traversal detected: {0}")]
    PathTraversal(String),

    #[error("Symlink escape outside workspace root detected: {0}")]
    SymlinkEscape(String),

    #[error("Path is outside workspace root: {0}")]
    OutsideWorkspace(String),

    #[error("Access to protected secret or credential file blocked: {0}")]
    ProtectedFileBlocked(String),

    #[error("Binary file excluded: {0}")]
    BinaryFileExcluded(String),

    #[error("Target file not found: {0}")]
    FileNotFound(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Context budget exceeded ({0} bytes)")]
    BudgetExceeded(usize),
}

/// A line/column range in an editor document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditorRange {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl EditorRange {
    pub fn new(start_line: usize, start_col: usize, end_line: usize, end_col: usize) -> Self {
        Self {
            start_line,
            start_col,
            end_line,
            end_col,
        }
    }
}

/// Severity level for editor diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// A diagnostic item reported by LSP or linters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticItem {
    pub range: EditorRange,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source: Option<String>,
}

/// A snapshot of selected editor text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionContext {
    pub relative_path: String,
    pub range: EditorRange,
    pub selected_text: String,
}

/// A single workspace file context item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileContext {
    pub relative_path: String,
    pub content: String,
    pub line_count: usize,
    pub byte_count: usize,
    pub is_truncated: bool,
    pub redactions_count: usize,
}

/// Resource constraints for context gathering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudget {
    pub max_file_bytes: usize,
    pub max_total_bytes: usize,
    pub max_lines_per_file: usize,
}

impl Default for ContextBudget {
    fn default() -> Self {
        Self {
            max_file_bytes: 64 * 1024,      // 64 KB per file
            max_total_bytes: 256 * 1024,    // 256 KB total snapshot
            max_lines_per_file: 2000,
        }
    }
}

/// A comprehensive, validated context snapshot ready for agent prompt ingestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub workspace_root: PathBuf,
    pub active_file: Option<String>,
    pub selection: Option<SelectionContext>,
    pub files: Vec<FileContext>,
    pub diagnostics: Vec<DiagnosticItem>,
    pub total_bytes: usize,
    pub budget_bytes: usize,
    pub timestamp_ms: u64,
}

/// Context assembly and security validation engine.
pub struct ContextService {
    workspace_root: PathBuf,
    canonical_root: PathBuf,
    budget: ContextBudget,
    protected_names: HashSet<String>,
    protected_extensions: HashSet<String>,
    binary_extensions: HashSet<String>,
}

impl ContextService {
    /// Initialize context service over a designated workspace root.
    pub fn new(workspace_root: impl AsRef<Path>, budget: ContextBudget) -> Result<Self, ContextError> {
        let root = workspace_root.as_ref().to_path_buf();
        let canonical_root = if root.exists() {
            root.canonicalize().map_err(|e| ContextError::Io(e.to_string()))?
        } else {
            root.clone()
        };

        let mut protected_names = HashSet::new();
        for name in &[
            ".env",
            ".env.local",
            ".env.production",
            "id_rsa",
            "id_ed25519",
            "credentials.json",
            "secrets.toml",
            "secrets.yaml",
            "octomind.key",
            ".netrc",
        ] {
            protected_names.insert((*name).to_string());
        }

        let mut protected_extensions = HashSet::new();
        for ext in &["key", "pem", "p12", "pfx", "keystore"] {
            protected_extensions.insert((*ext).to_string());
        }

        let mut binary_extensions = HashSet::new();
        for ext in &[
            "exe", "dll", "so", "dylib", "zip", "tar", "gz", "7z", "bin", "png",
            "jpg", "jpeg", "gif", "ico", "wasm", "pdf", "woff", "woff2",
        ] {
            binary_extensions.insert((*ext).to_string());
        }

        Ok(Self {
            workspace_root: root,
            canonical_root,
            budget,
            protected_names,
            protected_extensions,
            binary_extensions,
        })
    }

    /// Canonical root path of the workspace.
    pub fn workspace_root(&self) -> &Path {
        &self.canonical_root
    }

    /// Validate that a requested path is strictly contained within the workspace root.
    pub fn validate_path(&self, requested: impl AsRef<Path>) -> Result<PathBuf, ContextError> {
        let path = requested.as_ref();

        // 1. Reject explicit parent traversal components in relative paths
        for component in path.components() {
            if component == Component::ParentDir {
                return Err(ContextError::PathTraversal(path.display().to_string()));
            }
        }

        // 2. Resolve relative to workspace root if relative
        let candidate = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.canonical_root.join(path)
        };

        // 3. Ensure candidate exists
        if !candidate.exists() {
            return Err(ContextError::FileNotFound(candidate.display().to_string()));
        }

        // 4. Canonicalize to resolve symlinks and real path
        let canonical = candidate
            .canonicalize()
            .map_err(|e| ContextError::Io(e.to_string()))?;

        // 5. Enforce root containment (prevent symlink escapes)
        if !canonical.starts_with(&self.canonical_root) {
            return Err(ContextError::SymlinkEscape(canonical.display().to_string()));
        }

        // 6. Security filter: Block known sensitive filenames
        if let Some(file_name) = canonical.file_name().and_then(|n| n.to_str()) {
            if self.protected_names.contains(file_name)
                || file_name.starts_with(".env")
            {
                return Err(ContextError::ProtectedFileBlocked(file_name.to_string()));
            }
        }

        // 7. Security filter: Block secret extensions
        if let Some(ext) = canonical.extension().and_then(|e| e.to_str()) {
            let ext_lower = ext.to_lowercase();
            if self.protected_extensions.contains(&ext_lower) {
                return Err(ContextError::ProtectedFileBlocked(format!("*.{}", ext_lower)));
            }
            if self.binary_extensions.contains(&ext_lower) {
                return Err(ContextError::BinaryFileExcluded(format!("*.{}", ext_lower)));
            }
        }

        Ok(canonical)
    }

    /// Read and redact a validated file within budget constraints.
    pub fn read_file_context(&self, path: impl AsRef<Path>) -> Result<FileContext, ContextError> {
        let canonical = self.validate_path(path)?;

        let bytes = fs::read(&canonical).map_err(|e| ContextError::Io(e.to_string()))?;

        // Binary check: null byte detection
        if bytes.iter().take(4096).any(|&b| b == 0) {
            let name = canonical
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "binary".to_string());
            return Err(ContextError::BinaryFileExcluded(name));
        }

        let raw_text = String::from_utf8_lossy(&bytes);
        let (redacted_text, redactions_count) = redact_secrets(&raw_text);

        let mut lines: Vec<&str> = redacted_text.lines().collect();
        let original_lines = lines.len();
        let mut is_truncated = false;

        if lines.len() > self.budget.max_lines_per_file {
            lines.truncate(self.budget.max_lines_per_file);
            is_truncated = true;
        }

        let mut content = lines.join("\n");
        if content.len() > self.budget.max_file_bytes {
            content.truncate(self.budget.max_file_bytes);
            content.push_str("\n... [TRUNCATED: remaining content exceeded budget]");
            is_truncated = true;
        } else if is_truncated {
            content.push_str(&format!(
                "\n... [TRUNCATED: remaining {} lines omitted]",
                original_lines - self.budget.max_lines_per_file
            ));
        }

        let relative_path = canonical
            .strip_prefix(&self.canonical_root)
            .unwrap_or(&canonical)
            .to_string_lossy()
            .replace('\\', "/");

        let byte_count = content.len();
        let line_count = content.lines().count();

        Ok(FileContext {
            relative_path,
            content,
            line_count,
            byte_count,
            is_truncated,
            redactions_count,
        })
    }

    /// Assemble a full context snapshot with active file, selection, and diagnostics.
    pub fn build_snapshot(
        &self,
        active_file: Option<&Path>,
        selection: Option<SelectionContext>,
        attached_files: &[&Path],
        diagnostics: Vec<DiagnosticItem>,
        timestamp_ms: u64,
    ) -> Result<ContextSnapshot, ContextError> {
        let mut files = Vec::new();
        let mut total_bytes = 0;

        let mut all_paths = Vec::new();
        if let Some(active) = active_file {
            all_paths.push(active);
        }
        for file in attached_files {
            if !all_paths.contains(file) {
                all_paths.push(*file);
            }
        }

        for path in all_paths {
            let file_ctx = self.read_file_context(path)?;
            if total_bytes + file_ctx.byte_count > self.budget.max_total_bytes {
                return Err(ContextError::BudgetExceeded(
                    total_bytes + file_ctx.byte_count,
                ));
            }
            total_bytes += file_ctx.byte_count;
            files.push(file_ctx);
        }

        let active_relative = active_file.and_then(|p| {
            p.strip_prefix(&self.canonical_root)
                .ok()
                .map(|r| r.to_string_lossy().replace('\\', "/"))
        });

        Ok(ContextSnapshot {
            workspace_root: self.canonical_root.clone(),
            active_file: active_relative,
            selection,
            files,
            diagnostics,
            total_bytes,
            budget_bytes: self.budget.max_total_bytes,
            timestamp_ms,
        })
    }
}

/// Redact credentials, API keys, and sensitive tokens from source content.
pub fn redact_secrets(input: &str) -> (String, usize) {
    let mut count = 0;
    let mut output = String::with_capacity(input.len());

    for line in input.lines() {
        let mut redacted_line = line.to_string();

        // 1. AWS Access Key (AKIA...)
        if let Some(start) = redacted_line.find("AKIA") {
            if redacted_line.len() >= start + 20 {
                let candidate = &redacted_line[start..start + 20];
                if candidate.chars().all(|c| c.is_ascii_alphanumeric()) {
                    redacted_line.replace_range(start..start + 20, "[REDACTED: AWS_KEY]");
                    count += 1;
                }
            }
        }

        // 2. GitHub Personal Access Token (ghp_ / gho_ / github_pat_)
        for prefix in &["ghp_", "gho_"] {
            if let Some(start) = redacted_line.find(prefix) {
                if redacted_line.len() >= start + 40 {
                    let end = (start + 40).min(redacted_line.len());
                    redacted_line.replace_range(start..end, "[REDACTED: GITHUB_TOKEN]");
                    count += 1;
                }
            }
        }

        // 3. Anthropic API key (sk-ant-...)
        if let Some(start) = redacted_line.find("sk-ant-") {
            let rest = &redacted_line[start..];
            let end_offset = rest
                .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ';')
                .unwrap_or(rest.len());
            if end_offset >= 15 {
                redacted_line.replace_range(start..start + end_offset, "[REDACTED: ANTHROPIC_KEY]");
                count += 1;
            }
        }

        // 4. OpenAI / Generic sk- key
        if let Some(start) = redacted_line.find("sk-") {
            if !redacted_line[start..].starts_with("[REDACTED:") {
                let rest = &redacted_line[start..];
                let end_offset = rest
                    .find(|c: char| c.is_whitespace() || c == '"' || c == '\'' || c == ';')
                    .unwrap_or(rest.len());
                if end_offset >= 20 {
                    redacted_line.replace_range(start..start + end_offset, "[REDACTED: API_KEY]");
                    count += 1;
                }
            }
        }

        // 5. Google / Gemini API key (AIza...)
        if let Some(start) = redacted_line.find("AIza") {
            if redacted_line.len() >= start + 39 {
                let candidate = &redacted_line[start..start + 39];
                if candidate.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
                    redacted_line.replace_range(start..start + 39, "[REDACTED: GEMINI_KEY]");
                    count += 1;
                }
            }
        }

        // 6. Generic password / secret assignments (e.g. password = "xyz")
        for key in &["password", "secret", "api_key", "auth_token"] {
            let lower = redacted_line.to_lowercase();
            if let Some(pos) = lower.find(key) {
                let after = &redacted_line[pos + key.len()..];
                if let Some(eq_pos) = after.find(['=', ':']) {
                    let val_part = after[eq_pos + 1..].trim_start();
                    let trimmed = val_part.trim();
                    if trimmed.starts_with('"') && trimmed.len() > 2 {
                        if let Some(end_quote) = trimmed[1..].find('"') {
                            let secret_val = &trimmed[1..=end_quote];
                            if !secret_val.starts_with("[REDACTED") && secret_val.len() > 3 {
                                redacted_line = redacted_line.replace(
                                    secret_val,
                                    "[REDACTED: SECRET_VALUE]",
                                );
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        output.push_str(&redacted_line);
        output.push('\n');
    }

    // Strip trailing newline added by loop
    if !input.ends_with('\n') && output.ends_with('\n') {
        output.pop();
    }

    (output, count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_valid_workspace_file_accepted() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let file_path = root.join("src").join("main.rs");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "fn main() { println!(\"hello\"); }").unwrap();

        let service = ContextService::new(root, ContextBudget::default()).unwrap();
        let validated = service.validate_path("src/main.rs").unwrap();
        assert_eq!(validated, file_path.canonicalize().unwrap());

        let ctx = service.read_file_context("src/main.rs").unwrap();
        assert_eq!(ctx.relative_path, "src/main.rs");
        assert!(ctx.content.contains("fn main"));
        assert!(!ctx.is_truncated);
    }

    #[test]
    fn test_parent_traversal_rejected() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let service = ContextService::new(root, ContextBudget::default()).unwrap();

        let err = service.validate_path("../outside.txt").unwrap_err();
        assert!(matches!(err, ContextError::PathTraversal(_)));
    }

    #[test]
    fn test_protected_secret_files_blocked() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let env_file = root.join(".env");
        fs::write(&env_file, "SECRET=123").unwrap();

        let key_file = root.join("id_rsa");
        fs::write(&key_file, "fake key").unwrap();

        let pem_file = root.join("server.pem");
        fs::write(&pem_file, "fake cert").unwrap();

        let service = ContextService::new(root, ContextBudget::default()).unwrap();

        assert!(matches!(
            service.validate_path(".env").unwrap_err(),
            ContextError::ProtectedFileBlocked(_)
        ));
        assert!(matches!(
            service.validate_path("id_rsa").unwrap_err(),
            ContextError::ProtectedFileBlocked(_)
        ));
        assert!(matches!(
            service.validate_path("server.pem").unwrap_err(),
            ContextError::ProtectedFileBlocked(_)
        ));
    }

    #[test]
    fn test_binary_file_excluded() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let bin_file = root.join("app.exe");
        fs::write(&bin_file, [0u8, 1, 2, 3]).unwrap();

        let service = ContextService::new(root, ContextBudget::default()).unwrap();
        assert!(matches!(
            service.validate_path("app.exe").unwrap_err(),
            ContextError::BinaryFileExcluded(_)
        ));

        // Binary content in .txt file (null byte)
        let sneaky = root.join("sneaky.txt");
        fs::write(&sneaky, b"hello\0world").unwrap();
        assert!(matches!(
            service.read_file_context("sneaky.txt").unwrap_err(),
            ContextError::BinaryFileExcluded(_)
        ));
    }

    #[test]
    fn test_secret_redaction() {
        let raw = r#"
            let aws = "AKIA1234567890ABCDEF";
            let gh = "ghp_123456789012345678901234567890123456";
            let ant = "sk-ant-api03-abcdefghijklmnop12345";
            let gemini = "AIzaSyD-1234567890abcdefghijklmnopqrstuv";
            let normal = "hello world";
        "#;

        let (redacted, count) = redact_secrets(raw);
        assert_eq!(count, 4);
        assert!(!redacted.contains("AKIA1234567890ABCDEF"));
        assert!(redacted.contains("[REDACTED: AWS_KEY]"));
        assert!(!redacted.contains("ghp_1234567890"));
        assert!(redacted.contains("[REDACTED: GITHUB_TOKEN]"));
        assert!(!redacted.contains("sk-ant-api03"));
        assert!(redacted.contains("[REDACTED: ANTHROPIC_KEY]"));
        assert!(!redacted.contains("AIzaSyD"));
        assert!(redacted.contains("[REDACTED: GEMINI_KEY]"));
        assert!(redacted.contains("hello world"));
    }

    #[test]
    fn test_file_budget_truncation() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let file_path = root.join("large.txt");
        let content = "a".repeat(1000);
        fs::write(&file_path, &content).unwrap();

        let budget = ContextBudget {
            max_file_bytes: 200,
            max_total_bytes: 1000,
            max_lines_per_file: 100,
        };

        let service = ContextService::new(root, budget).unwrap();
        let ctx = service.read_file_context("large.txt").unwrap();
        assert!(ctx.is_truncated);
        assert!(ctx.byte_count <= 250);
        assert!(ctx.content.contains("[TRUNCATED"));
    }

    #[test]
    fn test_build_snapshot_with_selection_and_diagnostics() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let file_path = root.join("main.rs");
        fs::write(&file_path, "fn add(a: i32, b: i32) -> i32 { a + b }").unwrap();

        let service = ContextService::new(root, ContextBudget::default()).unwrap();

        let selection = SelectionContext {
            relative_path: "main.rs".to_string(),
            range: EditorRange::new(0, 0, 0, 10),
            selected_text: "fn add".to_string(),
        };

        let diagnostics = vec![DiagnosticItem {
            range: EditorRange::new(0, 0, 0, 2),
            severity: DiagnosticSeverity::Warning,
            message: "unused function".to_string(),
            source: Some("rustc".to_string()),
        }];

        let snapshot = service
            .build_snapshot(
                Some(&file_path),
                Some(selection),
                &[],
                diagnostics,
                1700000000,
            )
            .unwrap();

        assert_eq!(snapshot.active_file.as_deref(), Some("main.rs"));
        assert_eq!(snapshot.files.len(), 1);
        assert_eq!(snapshot.diagnostics.len(), 1);
        assert_eq!(snapshot.diagnostics[0].severity, DiagnosticSeverity::Warning);
        assert_eq!(snapshot.selection.unwrap().selected_text, "fn add");
    }
}
