pub mod android_gpu_backend;
pub mod android_lnn_predictor;
pub mod android_model_manager;
pub mod android_rl_policy;

use sha2::Digest;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AssistantError {
    #[error("Model not loaded: {0}")]
    ModelNotLoaded(String),
    #[error("Inference failed: {0}")]
    InferenceError(String),
    #[error("GPU backend error: {0}")]
    GpuError(String),
    #[error("Tensor pool error: {0}")]
    TensorPoolError(String),
}

pub struct AndroidAssistant {
    model_loaded: bool,
    tensor_pool: Option<linux_module::tensor::TensorPool>,
    model_name: Option<String>,
    gpu_backend: Option<crate::android_assistant::android_gpu_backend::AndroidGpuBackend>,
}

impl AndroidAssistant {
    pub fn new() -> Result<Self, AssistantError> {
        let tensor_pool = Self::create_tensor_pool()?;
        let gpu_backend =
            crate::android_assistant::android_gpu_backend::AndroidGpuBackend::new().ok();
        Ok(Self {
            model_loaded: false,
            tensor_pool,
            model_name: None,
            gpu_backend,
        })
    }

    fn create_tensor_pool() -> Result<Option<linux_module::tensor::TensorPool>, AssistantError> {
        let pool = linux_module::tensor::TensorPool::new("android_assistant", 256 * 1024 * 1024)
            .map_err(|e| AssistantError::TensorPoolError(e.to_string()))?;
        Ok(Some(pool))
    }

    pub fn load_model(&mut self, model_path: &str, model_name: &str) -> Result<(), AssistantError> {
        let pool = self.tensor_pool.as_mut().ok_or_else(|| {
            AssistantError::TensorPoolError("Tensor pool not initialized".to_string())
        })?;

        let data = std::fs::read(model_path)
            .map_err(|e| AssistantError::ModelNotLoaded(format!("Failed to read model: {}", e)))?;

        let version = "1.0";
        let hash = sha2::Sha256::digest(&data).to_vec();

        pool.load_model(model_name, &data, version, hash)
            .map_err(|e| AssistantError::TensorPoolError(format!("Failed to load model: {}", e)))?;

        self.model_loaded = true;
        self.model_name = Some(model_name.to_string());

        Ok(())
    }

    pub fn get_model_data(&self, model_name: &str) -> Option<&[u8]> {
        self.tensor_pool.as_ref()?.get_model_data(model_name)
    }

    pub fn infer(&self, prompt: &str) -> Result<String, AssistantError> {
        if !self.model_loaded {
            return Err(AssistantError::ModelNotLoaded(
                "Model not loaded".to_string(),
            ));
        }

        if let Some(ref backend) = self.gpu_backend {
            backend
                .run_inference(prompt)
                .map_err(|e| AssistantError::GpuError(e.to_string()))
        } else {
            Ok(format!("Response to: {}", prompt))
        }
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model_loaded
    }

    pub fn get_tensor_pool(&self) -> Option<&linux_module::tensor::TensorPool> {
        self.tensor_pool.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assistant_creation() -> anyhow::Result<()> {
        std::env::set_var("AIOS_BASE_DIR", "/tmp/test_aios");
        let assistant = AndroidAssistant::new();
        std::env::remove_var("AIOS_BASE_DIR");
        assert!(assistant.is_ok());
        Ok(())
    }

    #[test]
    fn test_model_not_loaded() -> anyhow::Result<()> {
        std::env::set_var("AIOS_BASE_DIR", "/tmp/test_aios");
        let assistant = AndroidAssistant::new()?;
        std::env::remove_var("AIOS_BASE_DIR");
        let result = assistant.infer("test");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_invalid_model_path() -> anyhow::Result<()> {
        std::env::set_var("AIOS_BASE_DIR", "/tmp/test_aios");
        let mut assistant = AndroidAssistant::new()?;
        std::env::remove_var("AIOS_BASE_DIR");
        let result = assistant.load_model("/nonexistent/model.gguf", "test_model");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_tensor_pool_access() -> anyhow::Result<()> {
        std::env::set_var("AIOS_BASE_DIR", "/tmp/test_aios");
        let assistant = AndroidAssistant::new()?;
        std::env::remove_var("AIOS_BASE_DIR");
        assert!(assistant.get_tensor_pool().is_some());
        Ok(())
    }
}
