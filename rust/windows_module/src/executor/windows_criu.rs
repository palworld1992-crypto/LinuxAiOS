//! CRIU Wrapper – Checkpoint/Restore for Windows VM using CRIU via Zig

use std::path::Path;
use std::process::Command;
use thiserror::Error;
use tracing::{debug, error, info, warn};

#[derive(Error, Debug)]
pub enum CriuError {
    #[error("CRIU not found")]
    NotFound,
    #[error("Checkpoint failed: {0}")]
    CheckpointFailed(String),
    #[error("Restore failed: {0}")]
    RestoreFailed(String),
    #[error("Process not found: {0}")]
    ProcessNotFound(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
pub struct CriuConfig {
    pub images_dir: String,
    pub log_file: Option<String>,
    pub tcp_established: bool,
    pub shell_job: bool,
    pub leaf_only: bool,
    pub verbose: u32,
}

impl Default for CriuConfig {
    fn default() -> Self {
        Self {
            images_dir: "/var/lib/aios/windows_module/snapshots".to_string(),
            log_file: None,
            tcp_established: true,
            shell_job: false,
            leaf_only: false,
            verbose: 0,
        }
    }
}

pub struct CriuWrapper {
    config: CriuConfig,
}

impl CriuWrapper {
    pub fn new(config: CriuConfig) -> Self {
        Self { config }
    }

    pub fn check_available(&self) -> Result<bool, CriuError> {
        let output = Command::new("criu").arg("--version").output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    let version = String::from_utf8_lossy(&out.stdout);
                    info!("CRIU available: {}", version.trim());
                    Ok(true)
                } else {
                    warn!("CRIU not available");
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }

    pub fn get_version(&self) -> Result<String, CriuError> {
        let output = Command::new("criu")
            .arg("--version")
            .output()
            .map_err(|_| CriuError::NotFound)?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(CriuError::NotFound)
        }
    }

    pub fn dump_process(&self, pid: u32, vm_id: &str) -> Result<String, CriuError> {
        if !self.check_available()? {
            return Err(CriuError::NotFound);
        }

        let dump_dir = Path::new(&self.config.images_dir).join(format!("vm_{}", vm_id));

        std::fs::create_dir_all(&dump_dir)
            .map_err(|e| CriuError::CheckpointFailed(format!("Failed to create dir: {}", e)))?;

        let mut cmd = Command::new("criu");
        cmd.arg("dump")
            .arg("-t")
            .arg(pid.to_string())
            .arg("-D")
            .arg(&dump_dir)
            .arg("--tcp-established")
            .arg("--shell-job");

        if self.config.verbose > 0 {
            cmd.arg("-v").arg(self.config.verbose.to_string());
        }

        if let Some(ref log) = self.config.log_file {
            cmd.arg("-o").arg(log);
        }

        debug!("Running CRIU dump for PID {} to {:?}", pid, dump_dir);

        let output = cmd
            .output()
            .map_err(|e| CriuError::CheckpointFailed(format!("Failed to execute: {}", e)))?;

        if output.status.success() {
            info!("Checkpoint successful for VM {} (PID {})", vm_id, pid);
            Ok(dump_dir.to_string_lossy().to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("CRIU dump failed: {}", stderr);
            Err(CriuError::CheckpointFailed(stderr.to_string()))
        }
    }

    pub fn restore_process(&self, images_dir: &str, vm_id: &str) -> Result<u32, CriuError> {
        if !self.check_available()? {
            return Err(CriuError::NotFound);
        }

        let img_path = Path::new(images_dir);

        if !img_path.exists() {
            return Err(CriuError::RestoreFailed(format!(
                "Images directory not found: {}",
                images_dir
            )));
        }

        let mut cmd = Command::new("criu");
        cmd.arg("restore")
            .arg("-d")
            .arg("-D")
            .arg(img_path)
            .arg("--tcp-established")
            .arg("--shell-job");

        if self.config.verbose > 0 {
            cmd.arg("-v").arg(self.config.verbose.to_string());
        }

        debug!("Running CRIU restore from {:?}", img_path);

        let output = cmd
            .output()
            .map_err(|e| CriuError::RestoreFailed(format!("Failed to execute: {}", e)))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            info!("Restore successful for VM {}", vm_id);

            if let Some(pid_str) = stdout.lines().last() {
                if let Ok(pid) = pid_str.trim().parse() {
                    return Ok(pid);
                }
            }

            Ok(0)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("CRIU restore failed: {}", stderr);
            Err(CriuError::RestoreFailed(stderr.to_string()))
        }
    }

    pub fn dump_qemu(&self, qemu_pid: u32, vm_id: &str) -> Result<String, CriuError> {
        self.dump_process(qemu_pid, vm_id)
    }

    pub fn restore_qemu(&self, images_dir: &str, vm_id: &str) -> Result<u32, CriuError> {
        self.restore_process(images_dir, vm_id)
    }

    pub fn pre_dump(&self, pid: u32, images_dir: &str) -> Result<(), CriuError> {
        if !self.check_available()? {
            return Err(CriuError::NotFound);
        }

        let dump_dir = Path::new(images_dir).join("pre-dump");

        std::fs::create_dir_all(&dump_dir).ok();

        let output = Command::new("criu")
            .arg("dump")
            .arg("-t")
            .arg(pid.to_string())
            .arg("-D")
            .arg(&dump_dir)
            .arg("--pre-dump")
            .arg("--tcp-established")
            .output()
            .map_err(|e| CriuError::CheckpointFailed(e.to_string()))?;

        if output.status.success() {
            debug!("Pre-dump successful for PID {}", pid);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(CriuError::CheckpointFailed(stderr.to_string()))
        }
    }

    pub fn check_kernel_features(&self) -> Result<Vec<String>, CriuError> {
        let output = Command::new("criu")
            .arg("check")
            .arg("--verbose")
            .output()
            .map_err(|_| CriuError::NotFound)?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut features = Vec::new();

        for line in stdout.lines() {
            if line.contains("OK") || line.contains("enabled") {
                features.push(line.trim().to_string());
            }
        }

        Ok(features)
    }
}

impl Default for CriuWrapper {
    fn default() -> Self {
        Self::new(CriuConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_criu_config_default() {
        let config = CriuConfig::default();
        assert!(config.tcp_established);
        assert!(!config.shell_job);
    }

    #[test]
    fn test_criu_wrapper_new() {
        let wrapper = CriuWrapper::default();
        assert!(wrapper.check_available().is_ok());
    }
}
