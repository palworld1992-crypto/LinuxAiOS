//! Windows Module – Lớp tương thích Windows (Wine, KVM)

#[path = "main/mod.rs"]
pub mod windows;
pub mod supervisor;

pub mod assistant;
pub mod executor;
pub mod ffi;
pub mod hybrid;
pub mod jit;
pub mod mapper;
pub mod profiling;
pub mod security;
pub mod translation;

use anyhow::Result;
use child_tunnel::ChildTunnel;
use common::health_tunnel::{HealthRecord, HealthTunnel as CommonHealthTunnel};
use health_tunnel::HealthTunnel as RealHealthTunnel;
use std::sync::Arc;
use tracing::info;

pub use windows::WindowsMain;
pub use windows::WindowsLocalFailover;
pub use windows::WindowsDegradedMode;
pub use supervisor::WindowsSupervisor;

/// Health tunnel implementation using real health_tunnel crate.
/// Stores health records locally and can sync to Health Master Tunnel later.
pub struct HealthTunnelImpl {
    inner: Arc<RealHealthTunnel>,
}

impl HealthTunnelImpl {
    pub fn new(module_id: &str) -> Self {
        Self {
            inner: Arc::new(RealHealthTunnel::new(module_id)),
        }
    }
}

impl CommonHealthTunnel for HealthTunnelImpl {
    fn record_health(&self, record: HealthRecord) -> Result<()> {
        // Store in local health tunnel (snapshots)
        self.inner.record_health(record)?;
        // TODO(Phase 6): Also send via SCC to Health Master Tunnel
        Ok(())
    }

    fn last_health(&self, module_id: &str) -> Option<HealthRecord> {
        self.inner.last_health(module_id)
    }

    fn health_history(&self, module_id: &str, limit: usize) -> Vec<HealthRecord> {
        self.inner.health_history(module_id, limit)
    }

    fn rollback(&self) -> Option<Vec<HealthRecord>> {
        self.inner.rollback().map(|snapshot| snapshot.components.into_values().collect())
    }
}

/// Initialize Windows Module.
/// Should be called once during system startup.
pub fn init(child_tunnel: Arc<ChildTunnel>) -> Result<()> {
    info!("Initializing Windows Module");
    
    // Register Windows Module main component with Child Tunnel
    let component_id = "windows_module".to_string();
    child_tunnel.update_state(component_id, vec![], true)?;
    
    // TODO(Phase 6): Initialize other components (executor, assistant, etc.)
    // For now, just log success.
    info!("Windows Module initialized successfully");
    Ok(())
}