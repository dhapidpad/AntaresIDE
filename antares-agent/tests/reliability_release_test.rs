//! Integration tests for Phase 8: Reliability, crash recovery, privacy filtering,
//! audit logging, and release readiness health verification.

use std::fs;

use antares_agent::{
    AuditEventKind, AuditLog, CheckStatus, HealthAuditor, PrivacyFilter, RecoveryManager,
    TurnPhase,
};
use tempfile::tempdir;

#[test]
fn test_crash_recovery_wal_and_interrupted_turn_restoration() {
    let dir = tempdir().unwrap();
    let recovery = RecoveryManager::new(dir.path());

    // 1. Initial state has no interrupted turn
    assert!(recovery.check_recovery().unwrap().is_none());

    // 2. Start a prompt turn before dispatching to child process
    recovery
        .record_turn_start("sess-alpha", "Please refactor auth.rs", 1000)
        .unwrap();
    assert!(recovery.wal_path().exists());

    // 3. Update turn progress
    recovery
        .update_phase(TurnPhase::StreamingTurn { chunks_received: 42 }, 1050)
        .unwrap();

    // 4. Simulate process crash by creating a new RecoveryManager pointing to same WAL
    let recovery_reboot = RecoveryManager::new(dir.path());
    let interrupted = recovery_reboot.check_recovery().unwrap();
    assert!(interrupted.is_some());

    let turn = interrupted.unwrap();
    assert_eq!(turn.session_id, "sess-alpha");
    assert_eq!(turn.prompt, "Please refactor auth.rs");
    assert_eq!(turn.phase, TurnPhase::StreamingTurn { chunks_received: 42 });

    // 5. Recover the turn: extracts data, clears WAL, generates user notice
    let (recovered_turn, notice) = recovery_reboot
        .recover_interrupted_turn()
        .unwrap()
        .expect("turn should exist");

    assert_eq!(recovered_turn.session_id, "sess-alpha");
    assert!(notice.contains("Previous turn was interrupted"));
    assert!(!recovery_reboot.wal_path().exists());
    assert!(recovery_reboot.check_recovery().unwrap().is_none());
}

#[test]
fn test_privacy_scrubbing_and_audit_logging() {
    let dir = tempdir().unwrap();

    // 1. PrivacyFilter credential redaction
    let raw_log = concat!(
        "Connecting with key AIzaSyD1234567890123456789012345678901 and ",
        "token Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.secret and ",
        "sk-proj-abc12345678901234567890 and ghp_123456789012345678901234567890123456"
    );
    let scrubbed = PrivacyFilter::scrub(raw_log);

    assert!(!scrubbed.contains("AIzaSyD1234567890123456789012345678901"));
    assert!(!scrubbed.contains("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"));
    assert!(!scrubbed.contains("sk-proj-abc12345678901234567890"));
    assert!(!scrubbed.contains("ghp_123456789012345678901234567890123456"));
    assert!(scrubbed.contains("[REDACTED_GEMINI_KEY]"));
    assert!(scrubbed.contains("[REDACTED_API_KEY]"));
    assert!(scrubbed.contains("Bearer [REDACTED_TOKEN]"));
    assert!(scrubbed.contains("[REDACTED_GITHUB_TOKEN]"));

    // 2. Append-only audit logger
    let audit = AuditLog::new(dir.path());
    audit
        .record(
            AuditEventKind::SessionStarted {
                session_id: "sess-1".to_string(),
                role: "coder".to_string(),
            },
            true,
            1000,
        )
        .unwrap();

    audit
        .record(
            AuditEventKind::PatchApproved {
                file: "src/main.rs".to_string(),
                hunks: 2,
            },
            true,
            1050,
        )
        .unwrap();

    audit
        .record(
            AuditEventKind::BudgetBreached {
                metric: "typing_latency".to_string(),
                value: 45,
            },
            false,
            1100,
        )
        .unwrap();

    let entries = audit.read_entries().unwrap();
    assert_eq!(entries.len(), 3);
    assert!(matches!(
        entries[0].kind,
        AuditEventKind::SessionStarted { .. }
    ));
    assert!(matches!(
        entries[1].kind,
        AuditEventKind::PatchApproved { .. }
    ));
    assert!(matches!(
        entries[2].kind,
        AuditEventKind::BudgetBreached { .. }
    ));
}

#[test]
fn test_release_readiness_health_checks() {
    let dir = tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    let storage = dir.path().join("storage");
    fs::create_dir_all(&workspace).unwrap();
    fs::create_dir_all(&storage).unwrap();

    // Create required third-party notices
    fs::write(
        workspace.join("THIRD_PARTY_LICENSES.md"),
        b"# Third-Party Notices\nLicensed under Apache-2.0 and MIT.",
    )
    .unwrap();

    let report = HealthAuditor::audit_system(&workspace, &storage);
    assert!(report.overall_ready);
    assert!(report.checks.iter().all(|c| c.status == CheckStatus::Pass));
}
