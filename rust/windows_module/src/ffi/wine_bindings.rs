//! Wine Bindings – Safe wrapper for Wine loader and prefix management

use std::path::Path;
use std::process::Command;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum WineError {
    #[error("Wine not found: {0}")]
    NotFound(String),
    #[error("Failed to execute Wine: {0}")]
    ExecutionFailed(String),
    #[error("Prefix error: {0}")]
    PrefixError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
pub struct WineConfig {
    pub prefix: Option<String>,
    pub arch: WineArch,
    pub server_timeout: u32,
    pub debug_level: WineDebugLevel,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum WineArch {
    #[default]
    Win32,
    Win64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum WineDebugLevel {
    #[default]
    Error,
    Warn,
    Fixme,
    Trace,
}

impl Default for WineConfig {
    fn default() -> Self {
        Self {
            prefix: None,
            arch: WineArch::Win32,
            server_timeout: 60,
            debug_level: WineDebugLevel::Error,
        }
    }
}

pub struct WineBindings;

impl WineBindings {
    pub fn new() -> Self {
        Self
    }

    pub fn check_wine_available(&self) -> Result<bool, WineError> {
        let output = Command::new("wine")
            .arg("--version")
            .output()
            .map_err(|e| WineError::NotFound(e.to_string()))?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            info!("Wine available: {}", version.trim());
            Ok(true)
        } else {
            warn!("Wine not available");
            Ok(false)
        }
    }

    pub fn get_wine_version(&self) -> Result<String, WineError> {
        let output = Command::new("wine")
            .arg("--version")
            .output()
            .map_err(|e| WineError::NotFound(e.to_string()))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(WineError::ExecutionFailed(
                "Failed to get version".to_string(),
            ))
        }
    }

    pub fn init_prefix(&self, prefix_path: &Path) -> Result<(), WineError> {
        if prefix_path.exists() {
            debug!("Prefix already exists: {:?}", prefix_path);
            return Ok(());
        }

        let mut cmd = Command::new("wineboot");
        cmd.arg("-u").env("WINEPREFIX", prefix_path);

        match cmd.output() {
            Ok(output) => {
                if output.status.success() {
                    info!("Wine prefix created: {:?}", prefix_path);
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    Err(WineError::PrefixError(stderr.to_string()))
                }
            }
            Err(e) => Err(WineError::IoError(e)),
        }
    }

    pub fn run_program(
        &self,
        config: &WineConfig,
        program: &str,
        args: &[&str],
    ) -> Result<u32, WineError> {
        let mut cmd = Command::new("wine");

        if let Some(ref prefix) = config.prefix {
            cmd.env("WINEPREFIX", prefix);
        }

        match config.arch {
            WineArch::Win64 => {
                cmd.env("WINARCH", "win64");
            }
            WineArch::Win32 => {
                cmd.env("WINARCH", "win32");
            }
        }

        cmd.arg(program);
        cmd.args(args);

        let child = cmd
            .spawn()
            .map_err(|e| WineError::ExecutionFailed(e.to_string()))?;
        Ok(child.id())
    }

    pub fn kill_process(&self, pid: u32) -> Result<(), WineError> {
        let output = Command::new("wineserver")
            .arg("-k")
            .arg(pid.to_string())
            .output()
            .map_err(|e| WineError::ExecutionFailed(e.to_string()))?;

        if output.status.success() {
            debug!("Killed Wine process {}", pid);
            Ok(())
        } else {
            Err(WineError::ExecutionFailed(
                "Failed to kill process".to_string(),
            ))
        }
    }

    pub fn set_dll_overrides(
        &self,
        prefix: &Path,
        overrides: &[(&str, &str)],
    ) -> Result<(), WineError> {
        let config_path = prefix.join("user.reg");
        if !config_path.exists() {
            return Err(WineError::PrefixError("Prefix not initialized".to_string()));
        }

        for (dll, mode) in overrides {
            debug!("DLL override: {} -> {}", dll, mode);
        }

        Ok(())
    }

    pub fn get_running_processes(&self, prefix: &Path) -> Result<Vec<u32>, WineError> {
        let mut cmd = Command::new("wineserver");
        cmd.arg("-p");

        if let Some(ref prefix_str) = prefix.to_str() {
            cmd.env("WINEPREFIX", prefix_str);
        }

        let output = cmd
            .output()
            .map_err(|e| WineError::ExecutionFailed(e.to_string()))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let pids: Vec<u32> = stdout
                .lines()
                .filter_map(|line| line.trim().parse().ok())
                .collect();
            Ok(pids)
        } else {
            Ok(vec![])
        }
    }
}

impl Default for WineBindings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wine_config_default() {
        let config = WineConfig::default();
        assert_eq!(config.server_timeout, 60);
        assert_eq!(config.arch, WineArch::Win32);
    }

    #[test]
    fn test_wine_bindings_default() {
        let bindings = WineBindings::new();
        assert!(!bindings.check_wine_available().map_or(false, |v| v));
    }
}
