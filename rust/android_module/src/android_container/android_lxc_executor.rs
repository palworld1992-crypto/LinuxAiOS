use std::process::Command;
use thiserror::Error;
use tracing::warn;

#[derive(Error, Debug)]
pub enum LxcError {
    #[error("LXC command failed: {0}")]
    CommandFailed(String),
    #[error("LXC not available")]
    NotAvailable,
}

pub struct AndroidLxcExecutor;

impl AndroidLxcExecutor {
    pub fn new() -> Result<Self, LxcError> {
        if !Self::is_available() {
            return Err(LxcError::NotAvailable);
        }
        Ok(Self)
    }

    pub fn is_available() -> bool {
        match Command::new("lxc-info").arg("--version").output() {
            Ok(o) => o.status.success(),
            Err(e) => {
                warn!("Failed to check lxc availability: {}", e);
                false
            }
        }
    }

    pub fn create_container(&self, name: &str, _template: &str) -> Result<(), LxcError> {
        let output = Command::new("lxc-create")
            .arg("-n")
            .arg(name)
            .arg("-t")
            .arg("download")
            .output()
            .map_err(|e| LxcError::CommandFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(LxcError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    pub fn start_container(&self, name: &str) -> Result<(), LxcError> {
        let output = Command::new("lxc-start")
            .arg("-n")
            .arg(name)
            .arg("-d")
            .output()
            .map_err(|e| LxcError::CommandFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(LxcError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    pub fn stop_container(&self, name: &str) -> Result<(), LxcError> {
        let output = Command::new("lxc-stop")
            .arg("-n")
            .arg(name)
            .output()
            .map_err(|e| LxcError::CommandFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(LxcError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lxc_availability_check() {
        let _ = AndroidLxcExecutor::is_available();
    }
}
