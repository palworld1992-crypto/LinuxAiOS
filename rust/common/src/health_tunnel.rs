//! Common health tunnel trait for modules.

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthRecord {
    pub module_id: String,
    pub timestamp: u64,
    pub status: HealthStatus,
    pub potential: f32,
    pub details: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Failed,
    Unknown,
    Supporting,
}

pub trait HealthTunnel: Send + Sync {
    fn record_health(&self, record: HealthRecord) -> Result<()>;
    fn last_health(&self, module_id: &str) -> Option<HealthRecord>;
    fn health_history(&self, module_id: &str, limit: usize) -> Vec<HealthRecord>;
    fn rollback(&self) -> Option<Vec<HealthRecord>>;
}
