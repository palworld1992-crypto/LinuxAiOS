use common::health_tunnel::{HealthRecord, HealthTunnel};
use common::supervisor_support::{SupervisorSupport, SupportContext, SupportError, SupportStatus};
use std::sync::Arc;

pub struct SihSupport {
    active: Arc<std::sync::atomic::AtomicBool>,
    embedding_enabled: Arc<std::sync::atomic::AtomicBool>,
    decision_history_enabled: Arc<std::sync::atomic::AtomicBool>,
    hardware_collection_enabled: Arc<std::sync::atomic::AtomicBool>,
    health_tunnel: Arc<dyn HealthTunnel + Send + Sync>,
}

impl Default for SihSupport {
    fn default() -> Self {
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

        let health_tunnel = Arc::new(DummyHealthTunnel);
        Self {
            active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            embedding_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            decision_history_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            hardware_collection_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            health_tunnel,
        }
    }
}

impl SihSupport {
    pub fn new() -> Self {
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

        Self {
            active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            embedding_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            decision_history_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            hardware_collection_enabled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            health_tunnel: Arc::new(DummyHealthTunnel),
        }
    }

    pub fn start_support(&self) {
        self.active.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn stop_support(&self) {
        self.active
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn enable_embedding(&self) {
        self.embedding_enabled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn disable_embedding(&self) {
        self.embedding_enabled
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn enable_decision_history(&self) {
        self.decision_history_enabled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn disable_decision_history(&self) {
        self.decision_history_enabled
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn enable_hardware_collection(&self) {
        self.hardware_collection_enabled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn disable_hardware_collection(&self) {
        self.hardware_collection_enabled
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_embedding_enabled(&self) -> bool {
        self.embedding_enabled
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn is_decision_history_enabled(&self) -> bool {
        self.decision_history_enabled
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn is_hardware_collection_enabled(&self) -> bool {
        self.hardware_collection_enabled
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl SupervisorSupport for SihSupport {
    fn is_supervisor_busy(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn take_over_operations(&mut self, _context: SupportContext) -> Result<(), SupportError> {
        self.active.store(true, std::sync::atomic::Ordering::SeqCst);
        let _ = self.health_tunnel.record_health(HealthRecord {
            module_id: "sih_support".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            status: common::health_tunnel::HealthStatus::Healthy,
            potential: 1.0,
            details: b"support_taken_over".to_vec(),
        });
        Ok(())
    }

    fn delegate_back_operations(&mut self) -> Result<(), SupportError> {
        self.active
            .store(false, std::sync::atomic::Ordering::SeqCst);
        let _ = self.health_tunnel.record_health(HealthRecord {
            module_id: "sih_support".to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            status: common::health_tunnel::HealthStatus::Healthy,
            potential: 1.0,
            details: b"support_delegated_back".to_vec(),
        });
        Ok(())
    }

    fn support_status(&self) -> SupportStatus {
        if self.active.load(std::sync::atomic::Ordering::SeqCst) {
            SupportStatus::Supporting
        } else {
            SupportStatus::Idle
        }
    }
}
