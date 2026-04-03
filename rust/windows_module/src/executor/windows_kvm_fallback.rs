//! KVM Fallback for Windows Module – determines hardware capabilities

use parking_lot::RwLock;
use std::path::Path;
use sysinfo::{System, SystemExt, ProcessExt};
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
    capabilities: RwLock<Option<HardwareCapabilities>>,
    checked: RwLock<bool>,
}

impl KvmFallback {
    pub fn new() -> Self {
        Self {
            capabilities: RwLock::new(None),
            checked: RwLock::new(false),
        }
    }

    pub fn detect(&self) -> Result<HardwareCapabilities, KvmFallbackError> {
        if let Some(cap) = self.capabilities.read().clone() {
            return Ok(cap);
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

        *self.capabilities.write() = Some(capabilities.clone());
        *self.checked.write() = true;

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

        if !available {
            let dmar_path = Path::new("/dev/dmar");
            if dmar_path.exists() {
                return Ok(true);
            }
            warn!("IOMMU is not available");
            return Err(KvmFallbackError::NoIommu);
        }

        let groups = std::fs::read_dir(iommu_path)
            .map_err(|e| KvmFallbackError::DetectionError(e.to_string()))?;

        let count = groups.count();
        if count > 0 {
            info!("IOMMU available with {} groups", count);
            return Ok(true);
        }

        warn!("IOMMU groups directory is empty");
        Ok(false)
    }

    fn detect_gpu(&self) -> Result<(bool, Option<String>, Option<String>), KvmFallbackError> {
        let mut sys = System::new();
        sys.refresh_processes();

        for process in sys.processes().values() {
            let name = process.name();
            if name.contains("nvidia") || name.contains("amd") || name.contains("radeon") {
                return Ok((true, None, Some(name.to_string())));
            }
        }

        let pci_devices = self.scan_pci()?;
        for (addr, name) in &pci_devices {
            let lower = name.to_lowercase();
            if lower.contains("vga")
                || lower.contains("graphic")
                || lower.contains("nvidia")
                || lower.contains("amd")
                || lower.contains("radeon")
                || lower.contains("intel")
            {
                return Ok((true, Some(addr.clone()), Some(name.clone())));
            }
        }

        Ok((false, None, None))
    }

    fn scan_pci(&self) -> Result<Vec<(String, String)>, KvmFallbackError> {
        let sys_bus_pci = Path::new("/sys/bus/pci/devices");
        if !sys_bus_pci.exists() {
            return Ok(Vec::new());
        }

        let mut devices = Vec::new();

        if let Ok(entries) = std::fs::read_dir(sys_bus_pci) {
            for entry in entries.flatten() {
                let path = entry.path();
                let addr = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let vendor_path = path.join("vendor");
                let device_path = path.join("device");

                if vendor_path.exists() && device_path.exists() {
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();

                    devices.push((addr, name));
                }
            }
        }

        Ok(devices)
    }

    pub fn get_recommended_mode(&self) -> Result<VmMode, KvmFallbackError> {
        let caps = self.detect()?;
        Ok(caps.recommended_mode)
    }

    pub fn can_use_passthrough(&self) -> Result<bool, KvmFallbackError> {
        let caps = self.detect()?;
        Ok(matches!(caps.recommended_mode, VmMode::Passthrough))
    }

    pub fn get_gpu_info(&self) -> Option<(String, String)> {
        let caps = self.capabilities.read();
        caps.as_ref()
            .and_then(|c| c.gpu_pci_addr.clone().zip(c.gpu_driver.clone()))
    }
}

impl Default for KvmFallback {
    fn default() -> Self {
        Self::new()
    }
}
