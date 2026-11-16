//! Antares agent integration boundary.
//!
//! P0 contains only binary discovery and typed lifecycle configuration. ACP transport,
//! process supervision and session handling are added in P1 after contract tests are prepared
//! from the pinned Octomind documentation.

use std::path::PathBuf;

/// Errors returned by the Antares agent layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentError {
    /// An explicitly configured binary path does not exist or is not a regular file.
    InvalidBinaryPath(PathBuf),
    /// No Octomind executable was found in the configured path or process PATH.
    BinaryNotFound,
}

/// Process configuration owned by Antares.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SidecarConfig {
    /// An optional explicit executable path. If absent, search `PATH`.
    pub binary: Option<PathBuf>,
}

/// Locate the Octomind executable without starting a process.
pub fn locate_binary(config: &SidecarConfig) -> Result<PathBuf, AgentError> {
    if let Some(path) = config.binary.as_ref() {
        return if path.is_file() {
            Ok(path.clone())
        } else {
            Err(AgentError::InvalidBinaryPath(path.clone()))
        };
    }

    let executable = if cfg!(windows) {
        "octomind.exe"
    } else {
        "octomind"
    };
    let paths = std::env::var_os("PATH").ok_or(AgentError::BinaryNotFound)?;

    std::env::split_paths(&paths)
        .map(|directory| directory.join(executable))
        .find(|candidate| candidate.is_file())
        .ok_or(AgentError::BinaryNotFound)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_missing_binary_is_reported() {
        let missing = PathBuf::from("/antares-test/nonexistent/octomind");
        let config = SidecarConfig {
            binary: Some(missing.clone()),
        };

        assert_eq!(locate_binary(&config), Err(AgentError::InvalidBinaryPath(missing)));
    }

    #[test]
    fn explicit_existing_file_is_returned() {
        let file = std::env::current_exe().expect("test executable path should exist");
        let config = SidecarConfig {
            binary: Some(file.clone()),
        };

        assert_eq!(locate_binary(&config), Ok(file));
    }
}
