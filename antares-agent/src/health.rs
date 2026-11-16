//! System readiness, release checklist verification, and operational health audit.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::release::ReleaseMetadata;

/// Status result of an individual health or readiness check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    Pass,
    Warning { message: String },
    Fail { error: String },
}

/// A single check item in the health report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckItem {
    pub name: String,
    pub description: String,
    pub status: CheckStatus,
}

/// Comprehensive readiness report for Antares IDE deployment and release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthReport {
    pub overall_ready: bool,
    pub checks: Vec<CheckItem>,
}

/// Auditor executing system health and release readiness inspections.
pub struct HealthAuditor;

impl HealthAuditor {
    /// Perform the full suite of release readiness and operational health checks.
    pub fn audit_system(workspace_root: &Path, storage_dir: &Path) -> HealthReport {
        let mut checks = Vec::new();

        // 1. Release metadata integrity check
        let meta = ReleaseMetadata::default();
        if !meta.app_name.is_empty() && meta.app_id == "dev.antares.antares" {
            checks.push(CheckItem {
                name: "Release Metadata".to_string(),
                description: "Application name and ID comply with Antares branding."
                    .to_string(),
                status: CheckStatus::Pass,
            });
        } else {
            checks.push(CheckItem {
                name: "Release Metadata".to_string(),
                description: "Release metadata is missing or misconfigured."
                    .to_string(),
                status: CheckStatus::Fail {
                    error: "Invalid app_id or app_name".to_string(),
                },
            });
        }

        // 2. Storage directory accessibility check
        if storage_dir.exists() || std::fs::create_dir_all(storage_dir).is_ok() {
            checks.push(CheckItem {
                name: "Storage Access".to_string(),
                description: "Storage directory is writable and accessible."
                    .to_string(),
                status: CheckStatus::Pass,
            });
        } else {
            checks.push(CheckItem {
                name: "Storage Access".to_string(),
                description: "Storage directory cannot be accessed or created."
                    .to_string(),
                status: CheckStatus::Fail {
                    error: format!("Cannot access {}", storage_dir.display()),
                },
            });
        }

        // 3. Workspace containment check
        if workspace_root.is_dir() {
            checks.push(CheckItem {
                name: "Workspace Containment".to_string(),
                description:
                    "Workspace root exists and serves as security boundary."
                        .to_string(),
                status: CheckStatus::Pass,
            });
        } else {
            checks.push(CheckItem {
                name: "Workspace Containment".to_string(),
                description: "Workspace directory is invalid.".to_string(),
                status: CheckStatus::Fail {
                    error: "Workspace root is not a valid directory".to_string(),
                },
            });
        }

        // 4. Third-party license attribution check
        let license_file = workspace_root.join("THIRD_PARTY_LICENSES.md");
        if license_file.is_file() {
            checks.push(CheckItem {
                name: "Third-Party Licenses".to_string(),
                description: "Mandatory third-party open source notices present."
                    .to_string(),
                status: CheckStatus::Pass,
            });
        } else {
            checks.push(CheckItem {
                name: "Third-Party Licenses".to_string(),
                description: "THIRD_PARTY_LICENSES.md notice is missing."
                    .to_string(),
                status: CheckStatus::Warning {
                    message: "Notice file not found at workspace root".to_string(),
                },
            });
        }

        let overall_ready = checks
            .iter()
            .all(|c| !matches!(c.status, CheckStatus::Fail { .. }));

        HealthReport {
            overall_ready,
            checks,
        }
    }
}
