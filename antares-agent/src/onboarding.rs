//! First-run setup wizard, agent runtime detection, and onboarding state machine.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::SidecarConfig;
use crate::error::AgentError;
use crate::locate_binary;

/// Current status of the Antares first-run setup flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirstRunStatus {
    /// Agent runtime is not yet installed or configured.
    NeedsSetup { recommended_runtime: String },
    /// Agent runtime is detected and ready for immediate use.
    Ready {
        binary_path: PathBuf,
        version: String,
    },
    /// Agent runtime is explicitly disabled by the user or policy.
    Disabled { reason: String },
}

/// Persistent onboarding configuration stored in the workspace or user config directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingConfig {
    pub completed: bool,
    pub selected_provider: String,
    pub custom_binary_path: Option<PathBuf>,
    pub telemetry_enabled: bool,
    pub setup_timestamp_ms: u64,
}

impl Default for OnboardingConfig {
    fn default() -> Self {
        Self {
            completed: false,
            selected_provider: "octomind".to_string(),
            custom_binary_path: None,
            telemetry_enabled: false,
            setup_timestamp_ms: 0,
        }
    }
}

/// Errors occurring during the onboarding process.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OnboardingError {
    #[error("IO error during onboarding: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid agent binary: {0}")]
    InvalidBinary(String),
}

/// Service managing the first-run experience and runtime readiness detection.
pub struct OnboardingService {
    config_path: PathBuf,
}

impl OnboardingService {
    pub fn new(storage_dir: impl AsRef<Path>) -> Self {
        let path = storage_dir.as_ref().join("onboarding.json");
        Self { config_path: path }
    }

    /// Load stored onboarding configuration from disk if present.
    pub fn load_config(&self) -> Result<OnboardingConfig, OnboardingError> {
        if !self.config_path.exists() {
            return Ok(OnboardingConfig::default());
        }

        let bytes = fs::read(&self.config_path).map_err(|e| {
            OnboardingError::Io(format!("Failed to read config: {}", e))
        })?;
        serde_json::from_slice(&bytes)
            .map_err(|e| OnboardingError::Serialization(e.to_string()))
    }

    /// Save onboarding configuration atomically to disk.
    pub fn save_config(
        &self,
        config: &OnboardingConfig,
    ) -> Result<(), OnboardingError> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                OnboardingError::Io(format!("Failed to create parent dir: {}", e))
            })?;
        }

        let temp_path = self.config_path.with_extension("tmp");
        let json = serde_json::to_string_pretty(config)
            .map_err(|e| OnboardingError::Serialization(e.to_string()))?;

        fs::write(&temp_path, json).map_err(|e| {
            OnboardingError::Io(format!("Failed to write temp config: {}", e))
        })?;

        fs::rename(&temp_path, &self.config_path).map_err(|e| {
            OnboardingError::Io(format!("Failed to rename config: {}", e))
        })?;

        Ok(())
    }

    /// Determine the current first-run status based on configuration and binary availability.
    pub fn evaluate_status(&self, sidecar: &SidecarConfig) -> FirstRunStatus {
        let config = self.load_config().unwrap_or_default();

        if !config.completed {
            return FirstRunStatus::NeedsSetup {
                recommended_runtime: "octomind-v0.72.0".to_string(),
            };
        }

        let lookup_config = if let Some(ref path) = config.custom_binary_path {
            SidecarConfig {
                binary: Some(path.clone()),
            }
        } else {
            sidecar.clone()
        };

        match locate_binary(&lookup_config) {
            Ok(binary_path) => FirstRunStatus::Ready {
                binary_path,
                version: "0.72.0".to_string(),
            },
            Err(AgentError::BinaryNotFound)
            | Err(AgentError::InvalidBinaryPath(_)) => FirstRunStatus::NeedsSetup {
                recommended_runtime: "octomind-v0.72.0".to_string(),
            },
            Err(e) => FirstRunStatus::Disabled {
                reason: format!("Agent detection error: {}", e),
            },
        }
    }

    /// Mark first-run setup as completed.
    pub fn complete_setup(
        &self,
        provider: &str,
        custom_binary: Option<PathBuf>,
        timestamp_ms: u64,
    ) -> Result<(), OnboardingError> {
        let config = OnboardingConfig {
            completed: true,
            selected_provider: provider.to_string(),
            custom_binary_path: custom_binary,
            telemetry_enabled: false,
            setup_timestamp_ms: timestamp_ms,
        };
        self.save_config(&config)
    }

    /// Reset onboarding state back to initial first-run defaults.
    pub fn reset_setup(&self) -> Result<(), OnboardingError> {
        if self.config_path.exists() {
            fs::remove_file(&self.config_path).map_err(|e| {
                OnboardingError::Io(format!("Failed to delete config: {}", e))
            })?;
        }
        Ok(())
    }
}
