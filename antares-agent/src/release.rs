//! Release metadata, package identifiers, update endpoints, and desktop distribution
//! definitions for Antares IDE across Linux, Windows, and macOS.

use serde::{Deserialize, Serialize};

/// Supported desktop target platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlatformTarget {
    WindowsX64,
    LinuxX64,
    MacOsArm64,
    MacOsX64,
}

impl PlatformTarget {
    pub fn current() -> Self {
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        {
            Self::WindowsX64
        }
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        {
            Self::LinuxX64
        }
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        {
            Self::MacOsArm64
        }
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        {
            Self::MacOsX64
        }
        #[cfg(not(any(
            all(target_os = "windows", target_arch = "x86_64"),
            all(target_os = "linux", target_arch = "x86_64"),
            all(target_os = "macos", target_arch = "aarch64"),
            all(target_os = "macos", target_arch = "x86_64"),
        )))]
        {
            Self::LinuxX64
        }
    }

    pub fn package_format(&self) -> &'static str {
        match self {
            Self::WindowsX64 => "msi",
            Self::LinuxX64 => "deb",
            Self::MacOsArm64 | Self::MacOsX64 => "dmg",
        }
    }

    pub fn binary_name(&self) -> &'static str {
        match self {
            Self::WindowsX64 => "antares.exe",
            _ => "antares",
        }
    }
}

/// Official release identity and distribution metadata for Antares IDE.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseMetadata {
    pub app_name: String,
    pub app_id: String,
    pub version: String,
    pub platform: PlatformTarget,
    pub updater_url: String,
    pub releases_url: String,
    pub repo_url: String,
    pub licenses_url: String,
    pub pinned_octomind_version: String,
}

impl Default for ReleaseMetadata {
    fn default() -> Self {
        Self {
            app_name: "Antares IDE".to_string(),
            app_id: "dev.antares.antares".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            platform: PlatformTarget::current(),
            updater_url: "https://api.github.com/repos/dhapidpad/AntaresIDE/releases/latest"
                .to_string(),
            releases_url: "https://github.com/dhapidpad/AntaresIDE/releases".to_string(),
            repo_url: "https://github.com/dhapidpad/AntaresIDE".to_string(),
            licenses_url: concat!(
                "https://raw.githubusercontent.com/dhapidpad/AntaresIDE/",
                "main/THIRD_PARTY_LICENSES.md"
            )
            .to_string(),
            pinned_octomind_version: "0.72.0".to_string(),
        }
    }
}

impl ReleaseMetadata {
    /// Generate user-facing about dialog details.
    pub fn format_about_info(&self) -> String {
        format!(
            "{} v{}\nPlatform: {:?} ({})\nOctomind Runtime: v{}\nRepository: {}",
            self.app_name,
            self.version,
            self.platform,
            self.platform.package_format(),
            self.pinned_octomind_version,
            self.repo_url,
        )
    }

    /// Check if an upstream release version is newer than current.
    pub fn is_update_available(&self, latest_version: &str) -> bool {
        let current = self.version.trim_start_matches('v');
        let latest = latest_version.trim_start_matches('v');
        latest != current && !latest.is_empty()
    }
}
