use std::process::Command;
use thiserror::Error;
use tracing::warn;

#[derive(Error, Debug)]
pub enum SystemdError {
    #[error("Command failed: {0}")]
    CommandFailed(String),
    #[error("systemd-nspawn not found")]
    NotAvailable,
}

pub struct AndroidSystemdBindings;

impl Default for AndroidSystemdBindings {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidSystemdBindings {
    pub fn new() -> Self {
        Self
    }

    pub fn is_available() -> bool {
        match Command::new("systemd-nspawn").arg("--version").output() {
            Ok(o) => o.status.success(),
            Err(e) => {
                warn!("Failed to check systemd-nspawn availability: {}", e);
                false
            }
        }
    }

    pub fn create_container(name: &str, directory: &str) -> Result<(), SystemdError> {
        let output = Command::new("systemd-nspawn")
            .arg("-D")
            .arg(directory)
            .arg("--register=no")
            .arg("--machine")
            .arg(name)
            .arg("/bin/true")
            .output()
            .map_err(|e| SystemdError::CommandFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(SystemdError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemd_availability() {
        let _ = AndroidSystemdBindings::is_available();
    }
}
