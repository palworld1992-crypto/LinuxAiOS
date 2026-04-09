//! Wrapper để sử dụng Health Tunnel với trait định nghĩa trong `common`.

use crate::HealthTunnel;
use anyhow::Result;
use common::health_tunnel::{HealthRecord, HealthTunnel as CommonHealthTunnel};
use std::sync::Arc;

pub struct HealthTunnelWrapper {
    inner: Arc<HealthTunnel>,
}

impl HealthTunnelWrapper {
    pub fn new(module_id: &str) -> Self {
        Self {
            inner: Arc::new(HealthTunnel::new(module_id)),
        }
    }
}

impl CommonHealthTunnel for HealthTunnelWrapper {
    fn record_health(&self, record: HealthRecord) -> Result<()> {
        self.inner.record_health(record)
    }

    fn last_health(&self, module_id: &str) -> Option<HealthRecord> {
        self.inner.last_health(module_id)
    }

    fn health_history(&self, module_id: &str, limit: usize) -> Vec<HealthRecord> {
        self.inner.health_history(module_id, limit)
    }

    fn rollback(&self) -> Option<Vec<HealthRecord>> {
        self.inner
            .rollback()
            .map(|snapshot| snapshot.components.into_values().collect())
    }
}
