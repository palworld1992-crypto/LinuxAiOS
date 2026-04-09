//! Degraded Mode - Chế độ hoạt động khi Supervisor không hoạt động

use common::health_tunnel::{HealthRecord, HealthTunnel};
use std::sync::Arc;

struct DummyHealthTunnel;

impl HealthTunnel for DummyHealthTunnel {
    fn record_health(&self, _record: HealthRecord) -> anyhow::Result<()> {
        Ok(())
    }

    fn last_health(&self, _module_id: &str) -> Option<HealthRecord> {
        None
    }

    fn health_history(&self, _module_id: &str, _limit: usize) -> Vec<HealthRecord> {
        Vec::new()
    }

    fn rollback(&self) -> Option<Vec<HealthRecord>> {
        None
    }
}

pub struct SihDegradedMode {
    _active: Arc<std::sync::atomic::AtomicBool>,
    _read_only: Arc<std::sync::atomic::AtomicBool>,
    health_tunnel: Arc<dyn HealthTunnel + Send + Sync>,
}

impl Default for SihDegradedMode {
    fn default() -> Self {
        Self {
            _active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            _read_only: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            health_tunnel: Arc::new(DummyHealthTunnel),
        }
    }
}

impl SihDegradedMode {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter(&self) {
        self._active
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn exit(&self) {
        self._active
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_active(&self) -> bool {
        self._active.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn is_read_only(&self) -> bool {
        self._read_only.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn allow_write(&self) {
        self._read_only
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn deny_write(&self) {
        self._read_only
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}
