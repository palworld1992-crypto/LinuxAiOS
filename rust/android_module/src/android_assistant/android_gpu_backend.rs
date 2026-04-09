use thiserror::Error;

#[derive(Error, Debug)]
pub enum GpuBackendError {
    #[error("GPU not available")]
    GpuNotAvailable,
    #[error("Inference failed: {0}")]
    InferenceFailed(String),
}

pub enum ComputeBackend {
    Vulkan,
    CpuSimd,
}

pub struct AndroidGpuBackend {
    backend: ComputeBackend,
}

impl AndroidGpuBackend {
    pub fn new() -> Result<Self, GpuBackendError> {
        let backend = Self::detect_backend();
        Ok(Self { backend })
    }

    fn detect_backend() -> ComputeBackend {
        ComputeBackend::CpuSimd
    }

    pub fn get_backend(&self) -> &ComputeBackend {
        &self.backend
    }

    pub fn is_gpu_available(&self) -> bool {
        matches!(self.backend, ComputeBackend::Vulkan)
    }

    pub fn run_inference(&self, prompt: &str) -> Result<String, GpuBackendError> {
        match self.backend {
            ComputeBackend::Vulkan => Ok(format!("[Vulkan] Response to: {}", prompt)),
            ComputeBackend::CpuSimd => Ok(format!("[CPU-SIMD] Response to: {}", prompt)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_backend_creation() {
        let backend = AndroidGpuBackend::new();
        assert!(backend.is_ok());
    }
}
