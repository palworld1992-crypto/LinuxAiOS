use crate::errors::ModelManagerError;
use dashmap::DashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::warn;

pub struct SihModelManager {
    _models_dir: PathBuf,
    active_models: Arc<DashMap<String, ModelHandle>>,
}

#[derive(Clone, Debug)]
struct ModelHandle {
    _name: String,
    path: PathBuf,
    _loaded_at: i64,
    _size_bytes: u64,
}

impl SihModelManager {
    pub fn new(models_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&models_dir).ok();

        Self {
            _models_dir: models_dir,
            active_models: Arc::new(DashMap::new()),
        }
    }

    pub fn load_model(&self, name: &str, path: &PathBuf) -> Result<(), ModelManagerError> {
        let metadata = std::fs::metadata(path)?;

        let loaded_at = match std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
        {
            Ok(d) => d.as_secs() as i64,
            Err(e) => {
                warn!("System clock before UNIX_EPOCH: {}", e);
                0
            }
        };

        let handle = ModelHandle {
            _name: name.to_string(),
            path: path.clone(),
            _loaded_at: loaded_at,
            _size_bytes: metadata.len(),
        };

        self.active_models.insert(name.to_string(), handle);
        Ok(())
    }

    pub fn unload_model(&self, name: &str) -> Result<(), ModelManagerError> {
        self.active_models.remove(name);
        Ok(())
    }

    pub fn get_model_path(&self, name: &str) -> Option<PathBuf> {
        self.active_models.get(name).map(|r| r.value().path.clone())
    }

    pub fn list_models(&self) -> Vec<String> {
        self.active_models.iter().map(|r| r.key().clone()).collect()
    }

    pub fn has_model(&self, name: &str) -> bool {
        self.active_models.contains_key(name)
    }
}
