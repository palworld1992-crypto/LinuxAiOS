//! GPU Context for Windows Module – Manages GPU ownership via SCC connection manager

use common::health_tunnel::HealthTunnel;
use scc::ConnectionManager;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Error, Debug)]
pub enum GpuContextError {
    #[error("SCC connection failed: {0}")]
    SccError(String),
    #[error("GPU not available")]
    GpuUnavailable,
    #[error("GPU request timeout")]
    Timeout,
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuRequest {
    pub request_id: u64,
    pub owner_id: u32,
    pub owner_type: String,
    pub gpu_index: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GpuResponse {
    pub request_id: u64,
    pub success: bool,
    pub gpu_index: u32,
    pub error_message: Option<String>,
}

pub struct WindowsGpuContext {
    conn_mgr: Arc<ConnectionManager>,
    health_tunnel: Arc<dyn HealthTunnel>,
    gpu_owner_id: AtomicU32,
    gpu_index: AtomicU32,
    gpu_available: AtomicBool,
    pending_request_id: AtomicU64,
}

impl WindowsGpuContext {
    pub fn new(conn_mgr: Arc<ConnectionManager>, health_tunnel: Arc<dyn HealthTunnel>) -> Self {
        Self {
            conn_mgr,
            health_tunnel,
            gpu_owner_id: AtomicU32::new(0),
            gpu_index: AtomicU32::new(0),
            gpu_available: AtomicBool::new(false),
            pending_request_id: AtomicU64::new(0),
        }
    }

    pub fn acquire_gpu(&self, owner_id: u32, owner_type: &str) -> Result<u32, GpuContextError> {
        info!(
            "Acquiring GPU ownership for owner {} ({})",
            owner_id, owner_type
        );

        if self.gpu_available.load(Ordering::Relaxed)
            && self.gpu_owner_id.load(Ordering::Relaxed) != 0
        {
            let current_owner = self.gpu_owner_id.load(Ordering::Relaxed);
            if current_owner != owner_id {
                warn!(
                    "GPU already owned by {}, cannot acquire for {}",
                    current_owner, owner_id
                );
                return Err(GpuContextError::GpuUnavailable);
            }
            return Ok(self.gpu_index.load(Ordering::Relaxed));
        }

        let request_id = self.pending_request_id.fetch_add(1, Ordering::Relaxed) + 1;

        let request = GpuRequest {
            request_id,
            owner_id,
            owner_type: owner_type.to_string(),
            gpu_index: 0,
        };

        let payload =
            serde_json::to_vec(&request).map_err(|e| GpuContextError::SccError(e.to_string()))?;

        match self.conn_mgr.send("linux_module", payload) {
            Ok(()) => {
                info!("GPU acquire request {} sent to linux_module", request_id);

                self.gpu_owner_id.store(owner_id, Ordering::Relaxed);
                self.gpu_index.store(0, Ordering::Relaxed);
                self.gpu_available.store(true, Ordering::Relaxed);

                Ok(0)
            }
            Err(e) => {
                error!("Failed to send GPU acquire request: {}", e);
                Err(GpuContextError::SccError(e.to_string()))
            }
        }
    }

    pub fn release_gpu(&self, owner_id: u32) -> Result<(), GpuContextError> {
        let current_owner = self.gpu_owner_id.load(Ordering::Relaxed);

        if current_owner != owner_id {
            warn!(
                "GPU owned by {}, cannot release from {}",
                current_owner, owner_id
            );
            return Err(GpuContextError::SccError("Not the GPU owner".to_string()));
        }

        info!("Releasing GPU ownership from owner {}", owner_id);

        let request_id = self.pending_request_id.fetch_add(1, Ordering::Relaxed) + 1;

        let request = GpuRequest {
            request_id,
            owner_id,
            owner_type: "windows_module".to_string(),
            gpu_index: self.gpu_index.load(Ordering::Relaxed),
        };

        let payload =
            serde_json::to_vec(&request).map_err(|e| GpuContextError::SccError(e.to_string()))?;

        match self.conn_mgr.send("linux_module", payload) {
            Ok(()) => {
                info!("GPU release request {} sent to linux_module", request_id);

                self.gpu_owner_id.store(0, Ordering::Relaxed);
                self.gpu_index.store(0, Ordering::Relaxed);
                self.gpu_available.store(false, Ordering::Relaxed);

                Ok(())
            }
            Err(e) => {
                error!("Failed to send GPU release request: {}", e);
                Err(GpuContextError::SccError(e.to_string()))
            }
        }
    }

    pub fn is_gpu_active(&self) -> bool {
        self.gpu_available.load(Ordering::Relaxed)
    }

    pub fn get_gpu_owner(&self) -> Option<u32> {
        let owner = self.gpu_owner_id.load(Ordering::Relaxed);
        if owner != 0 {
            Some(owner)
        } else {
            None
        }
    }

    pub fn get_gpu_index(&self) -> Option<u32> {
        if self.gpu_available.load(Ordering::Relaxed) {
            Some(self.gpu_index.load(Ordering::Relaxed))
        } else {
            None
        }
    }

    pub fn check_gpu_available(&self) -> bool {
        info!("Checking GPU availability via SCC");

        let request_id = self.pending_request_id.fetch_add(1, Ordering::Relaxed) + 1;

        let request = GpuRequest {
            request_id,
            owner_id: 0,
            owner_type: "check".to_string(),
            gpu_index: 0,
        };

        let payload = match serde_json::to_vec(&request) {
            Ok(p) => p,
            Err(e) => {
                error!("Failed to serialize GPU check request: {}", e);
                return false;
            }
        };

        match self.conn_mgr.send("linux_module", payload) {
            Ok(()) => {
                info!("GPU availability check request sent");
                true
            }
            Err(e) => {
                warn!("Failed to check GPU availability: {}", e);
                false
            }
        }
    }
}
