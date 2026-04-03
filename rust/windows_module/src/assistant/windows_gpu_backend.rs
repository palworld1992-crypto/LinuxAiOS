//! GPU Backend for Windows Assistant – detects and uses GPU for inference

use parking_lot::RwLock;
use std::process::Command;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum GpuError {
    #[error("GPU detection error: {0}")]
    DetectionError(String),
    #[error("GPU initialization error: {0}")]
    InitError(String),
    #[error("No GPU available")]
    NoGpu,
}

#[derive(Clone, Debug)]
pub enum GpuBackend {
    Wgpu,
    Cpu,
}

pub struct WindowsGpuBackend {
    backend: RwLock<GpuBackend>,
    device_name: RwLock<Option<String>>,
    vulkan_available: RwLock<bool>,
    cuda_available: RwLock<bool>,
}

impl WindowsGpuBackend {
    pub fn new() -> Self {
        let (vulkan, cuda) = Self::detect_gpu_apis();
        let backend = if vulkan || cuda {
            GpuBackend::Wgpu
        } else {
            GpuBackend::Cpu
        };

        Self {
            backend: RwLock::new(backend),
            device_name: RwLock::new(Self::detect_device_name()),
            vulkan_available: RwLock::new(vulkan),
            cuda_available: RwLock::new(cuda),
        }
    }

    fn detect_gpu_apis() -> (bool, bool) {
        let vulkan = Self::check_vulkan();
        let cuda = Self::check_cuda();

        (vulkan, cuda)
    }

    fn check_vulkan() -> bool {
        if let Ok(output) = Command::new("vulkaninfo").output() {
            return output.status.success();
        }
        false
    }

    fn check_cuda() -> bool {
        if let Ok(output) = Command::new("nvidia-smi").output() {
            if output.status.success() {
                return true;
            }
        }
        false
    }

    fn detect_device_name() -> Option<String> {
        if let Ok(output) = Command::new("nvidia-smi").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("GeForce") || line.contains("RTX") || line.contains("GTX") {
                        return Some(line.trim().to_string());
                    }
                }
            }
        }

        if let Ok(output) = Command::new("vulkaninfo").output() {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if line.contains("GPU") || line.contains("Radeon") {
                        return Some(line.trim().to_string());
                    }
                }
            }
        }

        None
    }

    pub fn init(&mut self) -> Result<(), GpuError> {
        let backend = self.backend.read().clone();

        match backend {
            GpuBackend::Wgpu => {
                if *self.vulkan_available.read() || *self.cuda_available.read() {
                    info!("Initializing GPU backend with WGPU");
                    return Ok(());
                }
            }
            GpuBackend::Cpu => {
                info!("Using CPU backend for inference");
            }
        }

        Ok(())
    }

    pub fn get_backend(&self) -> GpuBackend {
        self.backend.read().clone()
    }

    pub fn get_device_name(&self) -> Option<String> {
        self.device_name.read().clone()
    }

    pub fn has_vulkan(&self) -> bool {
        *self.vulkan_available.read()
    }

    pub fn has_cuda(&self) -> bool {
        *self.cuda_available.read()
    }

    pub fn get_compute_units(&self) -> usize {
        if let Some(ref name) = *self.device_name.read() {
            if name.contains("RTX") || name.contains("GTX") {
                return 3072;
            }
        }
        0
    }

    pub fn get_vram_gb(&self) -> u32 {
        if let Ok(output) = Command::new("nvidia-smi")
            .args(["--query-gpu=memory.total", "--format=csv,noheader,nounits"])
            .output()
        {
            if output.status.success() {
                let mem = String::from_utf8_lossy(&output.stdout);
                if let Some(mb) = mem.lines().next() {
                    if let Ok(mb) = mb.trim().parse::<u32>() {
                        return mb / 1024;
                    }
                }
            }
        }
        0
    }

    pub fn is_gpu_recommended(&self) -> bool {
        let vram = self.get_vram_gb();
        let compute_units = self.get_compute_units();

        vram >= 4 || compute_units >= 1024
    }
}

impl Default for WindowsGpuBackend {
    fn default() -> Self {
        Self::new()
    }
}
