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
use common::health_tunnel::HealthTunnel;

pub use windows::WindowsMain;
pub use windows::WindowsLocalFailover;
pub use windows::WindowsDegradedMode;
pub use supervisor::WindowsSupervisor;

pub struct HealthTunnelImpl;
impl HealthTunnel for HealthTunnelImpl {
    fn record_health(&self, _record: common::health_tunnel::HealthRecord) -> Result<()> {
        Ok(())
    }
    fn last_health(&self, _module_id: &str) -> Option<common::health_tunnel::HealthRecord> {
        None
    }
    fn health_history(
        &self,
        _module_id: &str,
        _limit: usize,
    ) -> Vec<common::health_tunnel::HealthRecord> {
        vec![]
    }
    fn rollback(&self) -> Option<Vec<common::health_tunnel::HealthRecord>> {
        None
    }
}

pub fn init() {
    // Khởi tạo module
}
