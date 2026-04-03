//! GPU Context for Windows Module – Manages GPU ownership via SCC connection manager
use common::health_tunnel::HealthTunnel;
use scc::ConnectionManager;
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum GpuContextError {
    #[error("SCC connection failed: {0}")]
    SccError(String),
    #[error("GPU not available")]
    GpuUnavailable,
}

pub struct WindowsGpuContext {
    _conn_mgr: Arc<ConnectionManager>,
    _health_tunnel: Arc<dyn HealthTunnel>,
    gpu_owner_id: Option<u32>,
}

impl WindowsGpuContext {
    pub fn new(conn_mgr: Arc<ConnectionManager>, health_tunnel: Arc<dyn HealthTunnel>) -> Self {
        Self {
            _conn_mgr: conn_mgr,
            _health_tunnel: health_tunnel,
            gpu_owner_id: None,
        }
    }

    pub fn acquire_gpu(&mut self, owner_id: u32) -> Result<(), GpuContextError> {
        info!("Acquiring GPU ownership for owner {}", owner_id);
        // SCC logic to claim GPU
        self.gpu_owner_id = Some(owner_id);
        Ok(())
    }

    pub fn release_gpu(&mut self) {
        if let Some(id) = self.gpu_owner_id {
            info!("Releasing GPU ownership from owner {}", id);
            self.gpu_owner_id = None;
        }
    }

    pub fn is_gpu_active(&self) -> bool {
        self.gpu_owner_id.is_some()
    }
}
