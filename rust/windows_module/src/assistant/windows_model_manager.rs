//! Model Manager for Windows Assistant – manages INT4/GGUF models

use dashmap::DashMap;
use parking_lot::RwLock;
use std::path::Path;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("Model not found: {0}")]
    NotFound(String),
    #[error("Load error: {0}")]
    LoadError(String),
    #[error("Unload error: {0}")]
    UnloadError(String),
}

#[derive(Clone, Debug)]
pub struct ModelInfo {
    pub id: String,
    pub path: String,
    pub size_mb: u64,
    pub loaded: bool,
    pub ref_count: u32,
}

pub struct WindowsModelManager {
    models: DashMap<String, ModelInfo>,
    active_model: RwLock<Option<String>>,
    max_memory_mb: u64,
    current_memory_mb: RwLock<u64>,
}

impl WindowsModelManager {
    pub fn new(max_memory_mb: u64) -> Self {
        Self {
            models: DashMap::new(),
            active_model: RwLock::new(None),
            max_memory_mb,
            current_memory_mb: RwLock::new(0),
        }
    }

    pub fn register_model(&self, id: &str, path: &str) -> Result<ModelInfo, ModelError> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(ModelError::NotFound(path.display().to_string()));
        }

        let size_mb = if let Ok(metadata) = std::fs::metadata(path) {
            metadata.len() / (1024 * 1024)
        } else {
            0
        };

        let info = ModelInfo {
            id: id.to_string(),
            path: path.display().to_string(),
            size_mb,
            loaded: false,
            ref_count: 0,
        };

        self.models.insert(id.to_string(), info.clone());
        info!("Registered model {} ({} MB)", id, size_mb);

        Ok(info)
    }

    pub fn load_model(&self, id: &str) -> Result<ModelInfo, ModelError> {
        let mut memory = self.current_memory_mb.write();
        let model = self
            .models
            .get(id)
            .ok_or_else(|| ModelError::NotFound(id.to_string()))?;

        let model = model.clone();

        if model.loaded {
            return Ok(model);
        }

        if *memory + model.size_mb > self.max_memory_mb {
            return Err(ModelError::LoadError("Not enough memory".to_string()));
        }

        *memory += model.size_mb;

        let mut info = model.clone();
        info.loaded = true;
        info.ref_count = 1;

        self.models.insert(id.to_string(), info.clone());
        *self.active_model.write() = Some(id.to_string());

        info!("Loaded model {} (total memory: {} MB)", id, *memory);
        Ok(info)
    }

    pub fn unload_model(&self, id: &str) -> Result<(), ModelError> {
        let mut memory = self.current_memory_mb.write();

        let model = self
            .models
            .get(id)
            .ok_or_else(|| ModelError::NotFound(id.to_string()))?;

        let size_mb = model.value().size_mb;

        *memory = memory.saturating_sub(size_mb);

        let mut info = model.value().clone();
        info.loaded = false;
        info.ref_count = 0;

        self.models.insert(id.to_string(), info);

        if let Some(ref active) = *self.active_model.read() {
            if active == id {
                *self.active_model.write() = None;
            }
        }

        info!("Unloaded model {} (total memory: {} MB)", id, *memory);
        Ok(())
    }

    pub fn get_model(&self, id: &str) -> Option<ModelInfo> {
        self.models.get(id).map(|r| r.clone())
    }

    pub fn list_models(&self) -> Vec<ModelInfo> {
        self.models.iter().map(|r| r.clone()).collect()
    }

    pub fn get_active_model(&self) -> Option<String> {
        self.active_model.read().clone()
    }

    pub fn get_memory_usage(&self) -> (u64, u64) {
        let current = *self.current_memory_mb.read();
        (current, self.max_memory_mb)
    }
}
