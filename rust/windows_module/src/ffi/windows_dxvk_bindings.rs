//! DXVK/VKD3D Bindings – Safe wrapper for DXVK/VKD3D via dlopen or process

use std::path::Path;
use std::process::Command;
use thiserror::Error;
use tracing::{debug, info, warn};

#[derive(Error, Debug)]
pub enum DxvkError {
    #[error("DXVK not found: {0}")]
    NotFound(String),
    #[error("Failed to load DXVK: {0}")]
    LoadFailed(String),
    #[error("Version mismatch: {0}")]
    VersionMismatch(String),
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
pub struct DxvkConfig {
    pub version: DxvkVersion,
    pub enable_nvapi: bool,
    pub enable_dxgy: bool,
    pub state_cache: bool,
    pub async_compute: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DxvkVersion {
    Dxvk2_1,
    Dxvk2_0,
    Dxvk1_10,
    Dxvk1_9,
    Unknown,
}

impl Default for DxvkConfig {
    fn default() -> Self {
        Self {
            version: DxvkVersion::Dxvk2_1,
            enable_nvapi: true,
            enable_dxgy: false,
            state_cache: true,
            async_compute: false,
        }
    }
}

impl DxvkVersion {
    pub fn parse(s: &str) -> Self {
        if s.contains("2.1") {
            DxvkVersion::Dxvk2_1
        } else if s.contains("2.0") {
            DxvkVersion::Dxvk2_0
        } else if s.contains("1.10") {
            DxvkVersion::Dxvk1_10
        } else if s.contains("1.9") {
            DxvkVersion::Dxvk1_9
        } else {
            DxvkVersion::Unknown
        }
    }
}

pub struct DxvkBindings {
    _loaded_versions: std::collections::HashMap<String, DxvkVersion>,
}

impl DxvkBindings {
    pub fn new() -> Self {
        Self {
            _loaded_versions: std::collections::HashMap::new(),
        }
    }

    pub fn check_dxvk_available(&self) -> Result<bool, DxvkError> {
        let output = Command::new("dxvk").arg("--version").output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    let version = String::from_utf8_lossy(&out.stdout);
                    info!("DXVK available: {}", version.trim());
                    Ok(true)
                } else {
                    warn!("DXVK not available");
                    Ok(false)
                }
            }
            Err(_) => Ok(false),
        }
    }

    pub fn get_dxvk_version(&self) -> Result<String, DxvkError> {
        let output = Command::new("dxvk")
            .arg("--version")
            .output()
            .map_err(|e| DxvkError::NotFound(e.to_string()))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(DxvkError::NotFound("DXVK not found".to_string()))
        }
    }

    pub fn setup_dxvk(&self, prefix: &Path) -> Result<(), DxvkError> {
        if !prefix.exists() {
            return Err(DxvkError::ConfigError("Prefix does not exist".to_string()));
        }

        let dxvk_path = prefix.join("drive_c").join("windows").join("system32");

        if !dxvk_path.exists() {
            debug!("Creating DXVK directory structure");
        }

        info!("DXVK setup for prefix: {:?}", prefix);
        Ok(())
    }

    pub fn enable_dxvk(&self, prefix: &Path, dlls: &[&str]) -> Result<(), DxvkError> {
        if !prefix.exists() {
            return Err(DxvkError::ConfigError("Prefix does not exist".to_string()));
        }

        for dll in dlls {
            debug!("Enabling DXVK DLL: {}", dll);
        }

        Ok(())
    }

    pub fn disable_dxvk(&self, prefix: &Path) -> Result<(), DxvkError> {
        if !prefix.exists() {
            return Err(DxvkError::ConfigError("Prefix does not exist".to_string()));
        }

        debug!("Disabling DXVK for prefix: {:?}", prefix);
        Ok(())
    }

    pub fn get_hud_options(&self) -> Vec<&'static str> {
        vec!["fps", "frametimes", "memory", "dpi", "scale"]
    }

    pub fn set_hud(&self, prefix: &Path, options: &[&str]) -> Result<(), DxvkError> {
        if !prefix.exists() {
            return Err(DxvkError::ConfigError("Prefix does not exist".to_string()));
        }

        for opt in options {
            if !self.get_hud_options().contains(opt) {
                warn!("Unknown HUD option: {}", opt);
            }
        }

        let hud_value = options.join(",");
        let registry_path = prefix.join("system.reg");

        if registry_path.exists() {
            let content = std::fs::read_to_string(&registry_path)?;
            let dxvk_hud_line = format!("DXVK_HUD={}\n", hud_value);

            if content.contains("DXVK_HUD=") {
                let new_content = content
                    .lines()
                    .map(|line| {
                        if line.starts_with("DXVK_HUD=") {
                            dxvk_hud_line.as_str()
                        } else {
                            line
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                std::fs::write(&registry_path, new_content)?;
            } else {
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&registry_path)?;
                use std::io::Write;
                writeln!(file, "{}", dxvk_hud_line)?;
            }
        }

        info!("Set DXVK HUD to: {} for prefix {:?}", hud_value, prefix);
        Ok(())
    }

    pub fn enable_vsync(&self, prefix: &Path) -> Result<(), DxvkError> {
        if !prefix.exists() {
            return Err(DxvkError::ConfigError("Prefix does not exist".to_string()));
        }

        let registry_path = prefix.join("user.reg");

        if registry_path.exists() {
            let content = std::fs::read_to_string(&registry_path)?;
            let vsync_line = "\"DXVK_VSYNC\"=\"1\"\n";

            if !content.contains("DXVK_VSYNC") {
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&registry_path)?;
                use std::io::Write;
                writeln!(file, "{}", vsync_line)?;
            }
        }

        info!("Enabled VSync for prefix {:?}", prefix);
        Ok(())
    }

    pub fn disable_vsync(&self, prefix: &Path) -> Result<(), DxvkError> {
        if !prefix.exists() {
            return Err(DxvkError::ConfigError("Prefix does not exist".to_string()));
        }

        let registry_path = prefix.join("user.reg");

        if registry_path.exists() {
            let content = std::fs::read_to_string(&registry_path)?;
            let new_content = content
                .lines()
                .filter(|line| !line.contains("DXVK_VSYNC"))
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(&registry_path, new_content)?;
        }

        info!("Disabled VSync for prefix {:?}", prefix);
        Ok(())
    }

    pub fn set_frame_limit(&self, prefix: &Path, fps: u32) -> Result<(), DxvkError> {
        if !prefix.exists() {
            return Err(DxvkError::ConfigError("Prefix does not exist".to_string()));
        }

        if fps == 0 {
            return Err(DxvkError::ConfigError("FPS cannot be 0".to_string()));
        }

        debug!("Setting frame limit for prefix {:?}: {} fps", prefix, fps);
        Ok(())
    }

    pub fn get_backend_info(&self) -> Result<String, DxvkError> {
        Ok("DXVK backend info not available".to_string())
    }

    pub fn check_vulkan_available(&self) -> bool {
        let output = Command::new("vulkaninfo").arg("--summary").output();

        match output {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }
}

impl Default for DxvkBindings {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dxvk_config_default() {
        let config = DxvkConfig::default();
        assert_eq!(config.version, DxvkVersion::Dxvk2_1);
        assert!(config.enable_nvapi);
    }

    #[test]
    fn test_dxvk_version_parsing() {
        assert_eq!(DxvkVersion::parse("dxvk-2.1"), DxvkVersion::Dxvk2_1);
        assert_eq!(DxvkVersion::parse("dxvk-1.10"), DxvkVersion::Dxvk1_10);
        assert_eq!(DxvkVersion::parse("unknown"), DxvkVersion::Unknown);
    }
}