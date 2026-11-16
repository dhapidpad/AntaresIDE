//! Integration tests for Phase 7: Antares branding, onboarding setup wizard,
//! runtime detection, and release metadata.

use std::fs;

use antares_agent::{
    FirstRunStatus, OnboardingService, PlatformTarget, ReleaseMetadata, SidecarConfig,
};
use tempfile::tempdir;

#[test]
fn test_first_run_detection_and_onboarding_lifecycle() {
    let dir = tempdir().unwrap();
    let service = OnboardingService::new(dir.path());

    // 1. Initial unconfigured state reports NeedsSetup
    let sidecar = SidecarConfig::default();
    let status = service.evaluate_status(&sidecar);
    assert!(matches!(status, FirstRunStatus::NeedsSetup { .. }));

    // 2. Create mock octomind executable
    let mock_bin = dir.path().join(if cfg!(windows) {
        "octomind.exe"
    } else {
        "octomind"
    });
    fs::write(&mock_bin, b"fake octomind executable").unwrap();

    // 3. Complete onboarding wizard with custom binary path
    service
        .complete_setup("octomind", Some(mock_bin.clone()), 1700000000)
        .unwrap();

    // 4. Evaluate status now reports Ready
    let status_after = service.evaluate_status(&sidecar);
    match status_after {
        FirstRunStatus::Ready {
            binary_path,
            version,
        } => {
            assert_eq!(binary_path, mock_bin);
            assert_eq!(version, "0.72.0");
        }
        other => panic!("Expected Ready, got {:?}", other),
    }

    // 5. Reset setup returns back to NeedsSetup
    service.reset_setup().unwrap();
    let reset_status = service.evaluate_status(&sidecar);
    assert!(matches!(reset_status, FirstRunStatus::NeedsSetup { .. }));
}

#[test]
fn test_release_metadata_and_update_checking() {
    let meta = ReleaseMetadata::default();
    assert_eq!(meta.app_name, "Antares IDE");
    assert_eq!(meta.app_id, "dev.antares.antares");
    assert_eq!(meta.pinned_octomind_version, "0.72.0");
    assert!(meta.repo_url.contains("AntaresIDE"));
    assert!(meta.licenses_url.contains("THIRD_PARTY_LICENSES.md"));

    let about_text = meta.format_about_info();
    assert!(about_text.contains("Antares IDE"));
    assert!(about_text.contains("Octomind Runtime: v0.72.0"));

    // Version update checks
    assert!(!meta.is_update_available(&meta.version));
    assert!(!meta.is_update_available(""));
    assert!(meta.is_update_available("9.9.9"));
}

#[test]
fn test_platform_packaging_format_and_binary_names() {
    assert_eq!(PlatformTarget::WindowsX64.package_format(), "msi");
    assert_eq!(PlatformTarget::WindowsX64.binary_name(), "antares.exe");

    assert_eq!(PlatformTarget::LinuxX64.package_format(), "deb");
    assert_eq!(PlatformTarget::LinuxX64.binary_name(), "antares");

    assert_eq!(PlatformTarget::MacOsArm64.package_format(), "dmg");
    assert_eq!(PlatformTarget::MacOsArm64.binary_name(), "antares");

    assert_eq!(PlatformTarget::MacOsX64.package_format(), "dmg");
    assert_eq!(PlatformTarget::MacOsX64.binary_name(), "antares");
}
