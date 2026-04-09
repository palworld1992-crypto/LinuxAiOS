//! Model Manager for Windows Assistant – manages INT4/GGUF models

use dashmap::DashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
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

#[derive(Debug)]
pub struct ModelInfo {
    pub id: String,
    pub path: String,
    pub size_mb: u64,
    pub loaded: bool,
    pub ref_count: AtomicU32,
}

impl Clone for ModelInfo {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            path: self.path.clone(),
            size_mb: self.size_mb,
            loaded: self.loaded,
            ref_count: AtomicU32::new(self.ref_count.load(Ordering::Relaxed)),
        }
    }
}

pub struct WindowsModelManager {
    models: DashMap<String, ModelInfo>,
    active_model: AtomicU64,
    max_memory_mb: u64,
    current_memory_mb: AtomicU64,
}

impl WindowsModelManager {
    pub fn new(max_memory_mb: u64) -> Self {
        Self {
            models: DashMap::new(),
            active_model: AtomicU64::new(0),
            max_memory_mb,
            current_memory_mb: AtomicU64::new(0),
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
            ref_count: AtomicU32::new(0),
        };

        self.models.insert(id.to_string(), info.clone());
        info!("Registered model {} ({} MB)", id, size_mb);
        Ok(info)
    }

    pub fn load_model(&self, id: &str) -> Result<(), ModelError> {
        let mut model = self
            .models
            .get_mut(id)
            .ok_or_else(|| ModelError::NotFound(id.to_string()))?;

        model.loaded = true;
        self.current_memory_mb
            .fetch_add(model.size_mb, Ordering::Relaxed);
        Ok(())
    }

    pub fn unload_model(&self, id: &str) -> Result<(), ModelError> {
        let mut model = self
            .models
            .get_mut(id)
            .ok_or_else(|| ModelError::NotFound(id.to_string()))?;

        if model.ref_count.load(Ordering::Relaxed) > 0 {
            return Err(ModelError::UnloadError("Model in use".to_string()));
        }

        model.loaded = false;
        self.current_memory_mb
            .fetch_sub(model.size_mb, Ordering::Relaxed);
        Ok(())
    }

    pub fn get_active_model(&self) -> Option<String> {
        let id_ptr = self.active_model.load(Ordering::Relaxed);
        if id_ptr == 0 {
            return None;
        }
        unsafe {
            let s = std::slice::from_raw_parts(id_ptr as *const u8, 20);
            std::str::from_utf8(s)
                .ok()
                .map(|s| s.trim_end_matches('\0').to_string())
        }
    }

    pub fn set_active_model(&self, id: &str) {
        let id_ptr = id.as_ptr() as u64;
        self.active_model.store(id_ptr, Ordering::Relaxed);
    }

    pub fn get_memory_usage(&self) -> u64 {
        self.current_memory_mb.load(Ordering::Relaxed)
    }

    pub fn is_memory_available(&self, size_mb: u64) -> bool {
        self.current_memory_mb.load(Ordering::Relaxed) + size_mb <= self.max_memory_mb
    }

    pub fn list_models(&self) -> Vec<ModelInfo> {
        self.models.iter().map(|r| r.value().clone()).collect()
    }
}

impl Default for WindowsModelManager {
    fn default() -> Self {
        Self::new(4096)
    }
}
