//! Integration and security test suite for Phase 3: Editor Context & Bounded Assembly.
//!
//! Confirms test plan requirements:
//! - Valid workspace file accepted.
//! - Parent traversal and absolute path rejected.
//! - Symlink escape rejected.
//! - .env, keys, credential files and ignored paths excluded.
//! - Large file truncation is explicit and bounded.
//! - Secret redaction marks what was removed.
//! - Workspace switch invalidates previous context.

use std::fs;
use tempfile::tempdir;

use antares_agent::{
    ContextBudget, ContextError, ContextService, DiagnosticItem, DiagnosticSeverity,
    EditorRange, SelectionContext,
};

#[test]
fn test_valid_workspace_file_accepted() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let sub = root.join("src");
    fs::create_dir_all(&sub).unwrap();
    let file = sub.join("lib.rs");
    fs::write(&file, "pub fn compute() -> usize { 42 }").unwrap();

    let service = ContextService::new(root, ContextBudget::default()).unwrap();
    let ctx = service.read_file_context("src/lib.rs").unwrap();

    assert_eq!(ctx.relative_path, "src/lib.rs");
    assert!(ctx.content.contains("pub fn compute"));
    assert_eq!(ctx.line_count, 1);
    assert!(!ctx.is_truncated);
    assert_eq!(ctx.redactions_count, 0);
}

#[test]
fn test_parent_traversal_and_absolute_escape_rejected() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create outside file
    let outside = dir.path().parent().unwrap().join("escaped_file.txt");
    fs::write(&outside, "sensitive outside content").unwrap();

    let service = ContextService::new(root, ContextBudget::default()).unwrap();

    // Relative parent traversal
    let err = service.validate_path("../escaped_file.txt").unwrap_err();
    assert!(matches!(err, ContextError::PathTraversal(_)));

    // Absolute path outside workspace
    let err2 = service.validate_path(&outside).unwrap_err();
    assert!(
        matches!(
            err2,
            ContextError::SymlinkEscape(_) | ContextError::OutsideWorkspace(_)
        ),
        "Expected escape detection, got {:?}",
        err2
    );

    // Clean up
    let _ = fs::remove_file(outside);
}

#[cfg(unix)]
#[test]
fn test_symlink_escape_rejected() {
    use std::os::unix::fs::symlink;

    let dir = tempdir().unwrap();
    let root = dir.path();
    let outside_dir = tempdir().unwrap();
    let outside_file = outside_dir.path().join("secret.txt");
    fs::write(&outside_file, "external secret").unwrap();

    let link = root.join("link_to_secret.txt");
    symlink(&outside_file, &link).unwrap();

    let service = ContextService::new(root, ContextBudget::default()).unwrap();
    let err = service.validate_path("link_to_secret.txt").unwrap_err();
    assert!(matches!(err, ContextError::SymlinkEscape(_)));
}

#[test]
fn test_protected_credentials_and_keys_excluded() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let files = [
        ".env",
        ".env.local",
        "id_rsa",
        "id_ed25519",
        "credentials.json",
        "app.key",
        "cert.pem",
    ];

    for name in &files {
        fs::write(root.join(name), "dummy secret").unwrap();
    }

    let service = ContextService::new(root, ContextBudget::default()).unwrap();

    for name in &files {
        let err = service.validate_path(name).unwrap_err();
        assert!(
            matches!(err, ContextError::ProtectedFileBlocked(_)),
            "Expected ProtectedFileBlocked for {}, got {:?}",
            name,
            err
        );
    }
}

#[test]
fn test_binary_files_excluded() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // By extension
    let exe = root.join("binary.exe");
    fs::write(&exe, b"MZ\x90\x00").unwrap();

    let service = ContextService::new(root, ContextBudget::default()).unwrap();
    assert!(matches!(
        service.validate_path("binary.exe").unwrap_err(),
        ContextError::BinaryFileExcluded(_)
    ));

    // By content null byte
    let fake_txt = root.join("data.txt");
    fs::write(&fake_txt, b"regular text\0with null byte").unwrap();
    assert!(matches!(
        service.read_file_context("data.txt").unwrap_err(),
        ContextError::BinaryFileExcluded(_)
    ));
}

#[test]
fn test_large_file_truncation_is_explicit_and_bounded() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = root.join("huge.txt");

    // 5,000 lines
    let mut data = String::new();
    for i in 0..5000 {
        data.push_str(&format!("line {}: some repeated text content\n", i));
    }
    fs::write(&file, &data).unwrap();

    let budget = ContextBudget {
        max_file_bytes: 4096,
        max_total_bytes: 16384,
        max_lines_per_file: 50,
    };

    let service = ContextService::new(root, budget).unwrap();
    let ctx = service.read_file_context("huge.txt").unwrap();

    assert!(ctx.is_truncated);
    assert!(ctx.byte_count <= 4096 + 256);
    assert!(ctx.content.contains("[TRUNCATED"));
}

#[test]
fn test_secret_redaction_marks_what_was_removed() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let file = root.join("config.rs");

    let source = r#"
        pub const AWS: &str = "AKIAIOSFODNN7EXAMPLE";
        pub const GITHUB: &str = "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890";
        pub const ANTHROPIC: &str = "sk-ant-api03-abcdef1234567890_ghijkl";
        pub const GEMINI: &str = "AIzaSyD12345678901234567890123456789012";
        pub const SAFE: &str = "https://example.com/api";
    "#;
    fs::write(&file, source).unwrap();

    let service = ContextService::new(root, ContextBudget::default()).unwrap();
    let ctx = service.read_file_context("config.rs").unwrap();

    assert_eq!(ctx.redactions_count, 4);
    assert!(ctx.content.contains("[REDACTED: AWS_KEY]"));
    assert!(ctx.content.contains("[REDACTED: GITHUB_TOKEN]"));
    assert!(ctx.content.contains("[REDACTED: ANTHROPIC_KEY]"));
    assert!(ctx.content.contains("[REDACTED: GEMINI_KEY]"));
    assert!(ctx.content.contains("https://example.com/api"));
    assert!(!ctx.content.contains("AKIAIOSFODNN7EXAMPLE"));
}

#[test]
fn test_workspace_switch_invalidates_context() {
    let dir1 = tempdir().unwrap();
    let dir2 = tempdir().unwrap();

    let file1 = dir1.path().join("file1.txt");
    fs::write(&file1, "workspace 1 content").unwrap();

    let file2 = dir2.path().join("file2.txt");
    fs::write(&file2, "workspace 2 content").unwrap();

    let service1 =
        ContextService::new(dir1.path(), ContextBudget::default()).unwrap();
    let snap1 = service1
        .build_snapshot(Some(&file1), None, &[], vec![], 1)
        .unwrap();
    assert_eq!(snap1.files.len(), 1);
    assert_eq!(snap1.files[0].relative_path, "file1.txt");

    // Switching to service2
    let service2 =
        ContextService::new(dir2.path(), ContextBudget::default()).unwrap();

    // Trying to read file1 from workspace 1 under service2 must fail
    assert!(service2.validate_path(&file1).is_err());

    let snap2 = service2
        .build_snapshot(Some(&file2), None, &[], vec![], 2)
        .unwrap();
    assert_eq!(snap2.files.len(), 1);
    assert_eq!(snap2.files[0].relative_path, "file2.txt");
}
