//! Model manager for System Host – quản lý model ONNX, xác minh chữ ký

use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

pub struct HostModelManager {
    _models_dir: PathBuf,
}

impl Default for HostModelManager {
    fn default() -> Self {
        Self::new()
    }
}

impl HostModelManager {
    pub fn new() -> Self {
        Self {
            _models_dir: PathBuf::from("/var/lib/aios/models"),
        }
    }

    // TODO(Phase 7): Dilithium signature verification via spark::crypto
    pub fn verify_signature(&self, model_path: &PathBuf, _signature: &[u8]) -> Result<bool> {
        unimplemented!("Phase 7: Dilithium signature verification via spark::crypto")
    }

    pub fn deploy_model(&self, model_name: &str, model_path: &PathBuf) -> Result<()> {
        info!("Deploying model {} from {:?}", model_name, model_path);
        Ok(())
    }
}
