use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModelManagerError {
    #[error("Model not found: {0}")]
    NotFound(String),
    #[error("Failed to load model: {0}")]
    LoadError(String),
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub path: String,
    pub quantization: String,
    pub size_mb: u64,
}

pub struct AndroidModelManager {
    models: std::collections::HashMap<String, ModelInfo>,
}

impl Default for AndroidModelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidModelManager {
    pub fn new() -> Self {
        Self {
            models: std::collections::HashMap::new(),
        }
    }

    pub fn register_model(&mut self, name: &str, path: &str, quantization: &str) {
        let size_mb = if std::path::Path::new(path).exists() {
            match std::fs::metadata(path) {
                Ok(m) => m.len() / (1024 * 1024),
                Err(_) => 0,
            }
        } else {
            0
        };

        self.models.insert(
            name.to_string(),
            ModelInfo {
                name: name.to_string(),
                path: path.to_string(),
                quantization: quantization.to_string(),
                size_mb,
            },
        );
    }

    pub fn get_model(&self, name: &str) -> Option<&ModelInfo> {
        self.models.get(name)
    }

    pub fn list_models(&self) -> Vec<&ModelInfo> {
        self.models.values().collect()
    }

    pub fn remove_model(&mut self, name: &str) -> Option<ModelInfo> {
        self.models.remove(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_model() {
        let mut manager = AndroidModelManager::new();
        manager.register_model("phi-3", "/models/phi-3-int4.gguf", "int4");
        assert!(manager.get_model("phi-3").is_some());
    }

    #[test]
    fn test_list_models() {
        let mut manager = AndroidModelManager::new();
        manager.register_model("phi-3", "/models/phi-3-int4.gguf", "int4");
        manager.register_model("tinyllama", "/models/tinyllama-int4.gguf", "int4");
        assert_eq!(manager.list_models().len(), 2);
    }
}
