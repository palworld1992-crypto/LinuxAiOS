//! KSM Manager – Controls Kernel Same-page Merging for VM memory optimization

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use thiserror::Error;
use tracing::{debug, info};

#[derive(Error, Debug)]
pub enum KsmError {
    #[error("KSM not available: {0}")]
    NotAvailable(String),
    #[error("Failed to write to sysfs: {0}")]
    SysfsError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
pub struct KsmStats {
    pub pages_sharing: u64,
    pub pages_shared: u64,
    pub pages_volatile: u64,
    pub full_scans: u64,
    pub merge_across_nodes: u32,
}

pub struct KsmManager {
    enabled: AtomicBool,
    merge_across_nodes: AtomicBool,
    run_background: AtomicBool,
    pages_to_scan: AtomicU32,
    sleep_millis: AtomicU32,
}

impl KsmManager {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            merge_across_nodes: AtomicBool::new(true),
            run_background: AtomicBool::new(true),
            pages_to_scan: AtomicU32::new(1024),
            sleep_millis: AtomicU32::new(50),
        }
    }

    pub fn is_available(&self) -> bool {
        std::path::Path::new("/sys/kernel/mm/ksm").exists()
    }

    pub fn enable(&self) -> Result<(), KsmError> {
        if !self.is_available() {
            return Err(KsmError::NotAvailable("KSM not available".to_string()));
        }

        self.write_sysfs("run", "1")?;
        self.enabled.store(true, Ordering::Relaxed);
        info!("KSM enabled");
        Ok(())
    }

    pub fn disable(&self) -> Result<(), KsmError> {
        if !self.is_available() {
            return Err(KsmError::NotAvailable("KSM not available".to_string()));
        }

        self.write_sysfs("run", "0")?;
        self.enabled.store(false, Ordering::Relaxed);
        info!("KSM disabled");
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_merge_across_nodes(&self, enabled: bool) -> Result<(), KsmError> {
        if !self.is_available() {
            return Err(KsmError::NotAvailable("KSM not available".to_string()));
        }

        let value = if enabled { "1" } else { "0" };
        self.write_sysfs("merge_across_nodes", value)?;
        self.merge_across_nodes.store(enabled, Ordering::Relaxed);
        debug!("KSM merge_across_nodes set to {}", enabled);
        Ok(())
    }

    pub fn get_merge_across_nodes(&self) -> bool {
        self.merge_across_nodes.load(Ordering::Relaxed)
    }

    pub fn set_run_background(&self, enabled: bool) -> Result<(), KsmError> {
        let value = if enabled { "1" } else { "0" };
        self.write_sysfs("run", value)?;
        self.run_background.store(enabled, Ordering::Relaxed);
        Ok(())
    }

    pub fn set_pages_to_scan(&self, pages: u32) -> Result<(), KsmError> {
        if pages == 0 {
            return Err(KsmError::SysfsError(
                "pages_to_scan cannot be 0".to_string(),
            ));
        }

        self.write_sysfs("pages_to_scan", &pages.to_string())?;
        self.pages_to_scan.store(pages, Ordering::Relaxed);
        debug!("KSM pages_to_scan set to {}", pages);
        Ok(())
    }

    pub fn set_sleep_millis(&self, millis: u32) -> Result<(), KsmError> {
        if millis == 0 {
            return Err(KsmError::SysfsError("sleep_millis cannot be 0".to_string()));
        }

        if !self.is_writable("sleep_millis") {
            return Err(KsmError::SysfsError(
                "KSM sysfs not writable (not running as root or KSM not enabled)".to_string(),
            ));
        }

        self.write_sysfs("sleep_millis", &millis.to_string())?;
        self.sleep_millis.store(millis, Ordering::Relaxed);
        debug!("KSM sleep_millis set to {} ms", millis);
        Ok(())
    }

    pub fn get_stats(&self) -> Result<KsmStats, KsmError> {
        if !self.is_available() {
            return Err(KsmError::NotAvailable("KSM not available".to_string()));
        }

        let pages_sharing = match self.read_sysfs_u64("pages_sharing") {
            Ok(v) => v,
            Err(_) => 0,
        };
        let pages_shared = match self.read_sysfs_u64("pages_shared") {
            Ok(v) => v,
            Err(_) => 0,
        };
        let pages_volatile = match self.read_sysfs_u64("pages_volatile") {
            Ok(v) => v,
            Err(_) => 0,
        };
        let full_scans = match self.read_sysfs_u64("full_scans") {
            Ok(v) => v,
            Err(_) => 0,
        };
        let merge_across_nodes = match self.read_sysfs_u32("merge_across_nodes") {
            Ok(v) => v,
            Err(_) => 1,
        };

        Ok(KsmStats {
            pages_sharing,
            pages_shared,
            pages_volatile,
            full_scans,
            merge_across_nodes,
        })
    }

    pub fn get_sharing_ratio(&self) -> Option<f64> {
        let stats = self.get_stats().ok()?;

        if stats.pages_sharing == 0 {
            return Some(0.0);
        }

        let ratio = stats.pages_sharing as f64 / stats.pages_shared.max(1) as f64;
        Some(ratio)
    }

    fn write_sysfs(&self, file: &str, value: &str) -> Result<(), KsmError> {
        let path = format!("/sys/kernel/mm/ksm/{}", file);

        std::fs::write(&path, value)
            .map_err(|e| KsmError::SysfsError(format!("Failed to write {}: {}", path, e)))
    }

    fn is_writable(&self, file: &str) -> bool {
        let path = format!("/sys/kernel/mm/ksm/{}", file);
        std::path::Path::new(&path).exists()
            && match std::fs::metadata(&path) {
                Ok(m) => !m.permissions().readonly(),
                Err(_) => false,
            }
    }

    fn read_sysfs_u64(&self, file: &str) -> Result<u64, KsmError> {
        let path = format!("/sys/kernel/mm/ksm/{}", file);
        let content = std::fs::read_to_string(&path).map_err(KsmError::IoError)?;

        content
            .trim()
            .parse()
            .map_err(|e| KsmError::SysfsError(format!("Failed to parse {}: {}", file, e)))
    }

    fn read_sysfs_u32(&self, file: &str) -> Result<u32, KsmError> {
        let path = format!("/sys/kernel/mm/ksm/{}", file);
        let content = std::fs::read_to_string(&path).map_err(KsmError::IoError)?;

        content
            .trim()
            .parse()
            .map_err(|e| KsmError::SysfsError(format!("Failed to parse {}: {}", file, e)))
    }
}

impl Default for KsmManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ksm_manager_new() {
        let manager = KsmManager::new();
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_ksm_availability() {
        let manager = KsmManager::new();
        let _ = manager.is_available();
    }
}
