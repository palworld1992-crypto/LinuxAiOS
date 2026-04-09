use std::process::Command;
use thiserror::Error;
use tracing::warn;

#[derive(Error, Debug)]
pub enum WaydroidError {
    #[error("Waydroid command failed: {0}")]
    CommandFailed(String),
    #[error("Waydroid not available")]
    NotAvailable,
}

pub struct AndroidWaydroidExecutor;

impl AndroidWaydroidExecutor {
    pub fn new() -> Result<Self, WaydroidError> {
        if !Self::is_available() {
            return Err(WaydroidError::NotAvailable);
        }
        Ok(Self)
    }

    pub fn is_available() -> bool {
        match Command::new("waydroid").arg("status").output() {
            Ok(o) => o.status.success(),
            Err(e) => {
                warn!("Failed to check waydroid availability: {}", e);
                false
            }
        }
    }

    pub fn start_session(&self) -> Result<(), WaydroidError> {
        let output = Command::new("waydroid")
            .arg("session")
            .arg("start")
            .output()
            .map_err(|e| WaydroidError::CommandFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(WaydroidError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    pub fn stop_session(&self) -> Result<(), WaydroidError> {
        let output = Command::new("waydroid")
            .arg("session")
            .arg("stop")
            .output()
            .map_err(|e| WaydroidError::CommandFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(WaydroidError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }

    pub fn install_apk(&self, apk_path: &str) -> Result<(), WaydroidError> {
        let output = Command::new("waydroid")
            .arg("app")
            .arg("install")
            .arg(apk_path)
            .output()
            .map_err(|e| WaydroidError::CommandFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(WaydroidError::CommandFailed(
                String::from_utf8_lossy(&output.stderr).to_string(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waydroid_availability_check() {
        let _ = AndroidWaydroidExecutor::is_available();
    }
}
