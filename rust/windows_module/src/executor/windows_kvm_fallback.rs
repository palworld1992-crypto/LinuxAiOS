//! KVM Fallback for Windows Module – determines hardware capabilities

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use sysinfo::{System, SystemExt};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Error, Debug)]
pub enum KvmFallbackError {
    #[error("Hardware detection error: {0}")]
    DetectionError(String),
    #[error("IOMMU not available")]
    NoIommu,
}

#[derive(Clone, Debug)]
pub enum VmMode {
    Passthrough,
    VirtioGpu,
    SoftwareRendering,
}

#[derive(Clone, Debug)]
pub struct HardwareCapabilities {
    pub has_kvm: bool,
    pub has_iommu: bool,
    pub has_gpu: bool,
    pub gpu_pci_addr: Option<String>,
    pub gpu_driver: Option<String>,
    pub cpu_cores: usize,
    pub memory_gb: u64,
    pub recommended_mode: VmMode,
}

pub struct KvmFallback {
    capabilities: OnceLock<HardwareCapabilities>,
    checked: AtomicBool,
}

impl KvmFallback {
    pub fn new() -> Self {
        Self {
            capabilities: OnceLock::new(),
            checked: AtomicBool::new(false),
        }
    }

    pub fn detect(&self) -> Result<HardwareCapabilities, KvmFallbackError> {
        if let Some(cap) = self.capabilities.get() {
            return Ok(cap.clone());
        }

        let has_kvm = self.check_kvm()?;
        let has_iommu = self.check_iommu()?;
        let (has_gpu, gpu_pci_addr, gpu_driver) = self.detect_gpu()?;
        let mut sys = System::new();
        sys.refresh_cpu();
        sys.refresh_memory();
        let cpu_cores = sys.cpus().len();
        let memory_gb = sys.total_memory() / (1024 * 1024 * 1024);

        let recommended_mode = if has_kvm && has_iommu && has_gpu {
            VmMode::Passthrough
        } else if has_kvm {
            VmMode::VirtioGpu
        } else {
            VmMode::SoftwareRendering
        };

        let capabilities = HardwareCapabilities {
            has_kvm,
            has_iommu,
            has_gpu,
            gpu_pci_addr,
            gpu_driver,
            cpu_cores,
            memory_gb,
            recommended_mode,
        };

        let _ = self.capabilities.set(capabilities.clone());
        self.checked.store(true, Ordering::Relaxed);

        info!("Detected hardware capabilities: {:?}", capabilities);
        Ok(capabilities)
    }

    fn check_kvm(&self) -> Result<bool, KvmFallbackError> {
        let kvm_path = Path::new("/dev/kvm");
        let available = kvm_path.exists();

        if available {
            info!("KVM is available");
        } else {
            warn!("KVM is not available");
        }

        Ok(available)
    }

    fn check_iommu(&self) -> Result<bool, KvmFallbackError> {
        let iommu_path = Path::new("/sys/kernel/iommu_groups");
        let available = iommu_path.exists() && iommu_path.is_dir();

        if available {
            info!("IOMMU is available");
        } else {
            warn!("IOMMU is not available");
        }

        Ok(available)
    }

    fn detect_gpu(&self) -> Result<(bool, Option<String>, Option<String>), KvmFallbackError> {
        let lspci_path = Path::new("/usr/bin/lspci");
        if !lspci_path.exists() {
            return Ok((false, None, None));
        }

        let output = std::process::Command::new("lspci")
            .arg("-nn")
            .arg("-d::1002:")
            .output()
            .map_err(|e| KvmFallbackError::DetectionError(e.to_string()))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("VGA") || line.contains("GPU") {
                    let parts: Vec<&str> = line.splitn(2, " ").collect();
                    if parts.len() >= 2 {
                        let addr = parts[0].to_string();
                        let driver = self.get_gpu_driver(&addr);
                        return Ok((true, Some(addr), driver));
                    }
                }
            }
        }

        Ok((false, None, None))
    }

    fn get_gpu_driver(&self, pci_addr: &str) -> Option<String> {
        let path = format!("/sys/bus/pci/drivers/{}/bind", pci_addr);
        if Path::new(&path).exists() {
            Some("unknown".to_string())
        } else {
            None
        }
    }

    pub fn has_been_checked(&self) -> bool {
        self.checked.load(Ordering::Relaxed)
    }

    pub fn get_cached(&self) -> Option<HardwareCapabilities> {
        self.capabilities.get().cloned()
    }
}

impl Default for KvmFallback {
    fn default() -> Self {
        Self::new()
    }
}
