//! Windows Module Support Functions – Implement SupervisorSupport for WindowsMain

use crate::windows_main::windows_local_failover::WindowsLocalFailover;
use common::health_tunnel::{HealthRecord, HealthTunnel};
use common::supervisor_support::{SupervisorSupport, SupportContext, SupportError, SupportStatus};
use parking_lot::RwLock;
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct WindowsSupport {
    status: RwLock<SupportStatus>,
    active_tasks: RwLock<SupportContext>,
    health_tunnel: Arc<dyn HealthTunnel>,
    failover: Arc<WindowsLocalFailover>,
    api_profiling_enabled: RwLock<bool>,
    executor_monitoring_enabled: RwLock<bool>,
    jit_compilation_allowed: RwLock<bool>,
}

impl WindowsSupport {
    pub fn new(health_tunnel: Arc<dyn HealthTunnel>, failover: Arc<WindowsLocalFailover>) -> Self {
        Self {
            status: RwLock::new(SupportStatus::Idle),
            active_tasks: RwLock::new(SupportContext::NONE),
            health_tunnel,
            failover,
            api_profiling_enabled: RwLock::new(false),
            executor_monitoring_enabled: RwLock::new(false),
            jit_compilation_allowed: RwLock::new(false),
        }
    }

    pub fn start_api_profiling(&self) {
        *self.api_profiling_enabled.write() = true;
        debug!("API profiling enabled via support");
    }

    pub fn stop_api_profiling(&self) {
        *self.api_profiling_enabled.write() = false;
        debug!("API profiling disabled");
    }

    pub fn start_executor_monitoring(&self) {
        *self.executor_monitoring_enabled.write() = true;
        debug!("Executor monitoring enabled via support");
    }

    pub fn stop_executor_monitoring(&self) {
        *self.executor_monitoring_enabled.write() = false;
        debug!("Executor monitoring disabled");
    }

    pub fn allow_jit_compilation(&self) {
        *self.jit_compilation_allowed.write() = true;
        debug!("JIT compilation allowed via support");
    }

    pub fn disallow_jit_compilation(&self) {
        *self.jit_compilation_allowed.write() = false;
        debug!("JIT compilation disallowed");
    }

    pub fn is_api_profiling_enabled(&self) -> bool {
        *self.api_profiling_enabled.read()
    }

    pub fn is_executor_monitoring_enabled(&self) -> bool {
        *self.executor_monitoring_enabled.read()
    }

    pub fn is_jit_allowed(&self) -> bool {
        *self.jit_compilation_allowed.read()
    }
}

impl SupervisorSupport for WindowsSupport {
    fn is_supervisor_busy(&self) -> bool {
        matches!(*self.status.read(), SupportStatus::Supporting)
    }

    fn take_over_operations(&mut self, context: SupportContext) -> Result<(), SupportError> {
        let current_status = *self.status.read();

        if current_status == SupportStatus::Suspended {
            return Err(SupportError::TakeOverFailed(
                "Support is suspended".to_string(),
            ));
        }

        // Notify local failover
        if let Err(e) = self.failover.handle_supervisor_failure() {
            warn!("Failover notification failed: {}", e);
        }

        let tasks = context;

        if tasks.contains(SupportContext::API_PROFILING) {
            self.start_api_profiling();
        }

        if tasks.contains(SupportContext::EXECUTOR_MONITORING) {
            self.start_executor_monitoring();
        }

        if tasks.contains(SupportContext::JIT_COMPILATION) {
            self.allow_jit_compilation();
        }

        *self.status.write() = SupportStatus::Supporting;
        *self.active_tasks.write() = tasks;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let record = HealthRecord {
            module_id: "windows_main".to_string(),
            status: common::health_tunnel::HealthStatus::Healthy,
            potential: 0.5,
            details: format!("Support started with tasks: {:?}", tasks).into_bytes(),
            timestamp,
        };

        let _ = self.health_tunnel.record_health(record);
        info!("Windows support started, tasks: {:?}", tasks);

        Ok(())
    }

    fn delegate_back_operations(&mut self) -> Result<(), SupportError> {
        self.stop_api_profiling();
        self.stop_executor_monitoring();
        self.disallow_jit_compilation();

        *self.status.write() = SupportStatus::Idle;
        *self.active_tasks.write() = SupportContext::NONE;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let record = HealthRecord {
            module_id: "windows_main".to_string(),
            status: common::health_tunnel::HealthStatus::Healthy,
            potential: 0.8,
            details: "Support delegated back".to_string().into_bytes(),
            timestamp,
        };

        let _ = self.health_tunnel.record_health(record);
        info!("Windows support ended, operations delegated back");

        Ok(())
    }

    fn support_status(&self) -> SupportStatus {
        *self.status.read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyHealthTunnel;

    impl HealthTunnel for DummyHealthTunnel {
        fn record_health(&self, _record: HealthRecord) -> std::result::Result<(), anyhow::Error> {
            Ok(())
        }
        fn last_health(&self, _module_id: &str) -> Option<HealthRecord> {
            None
        }
        fn health_history(&self, _module_id: &str, _limit: usize) -> Vec<HealthRecord> {
            vec![]
        }
        fn rollback(&self) -> Option<Vec<HealthRecord>> {
            None
        }
    }

    #[test]
    fn test_support_default_state() {
        let health_tunnel = Arc::new(DummyHealthTunnel);
        let failover = Arc::new(WindowsLocalFailover::new());
        let support = WindowsSupport::new(health_tunnel, failover);

        assert!(!support.is_supervisor_busy());
        assert_eq!(support.support_status(), SupportStatus::Idle);
    }

    #[test]
    fn test_take_over_operations() {
        let health_tunnel = Arc::new(DummyHealthTunnel);
        let failover = Arc::new(WindowsLocalFailover::new());
        let mut support = WindowsSupport::new(health_tunnel, failover);

        let context = SupportContext::API_PROFILING.union(SupportContext::EXECUTOR_MONITORING);

        assert!(support.take_over_operations(context).is_ok());
        assert!(support.is_supervisor_busy());
        assert!(support.is_api_profiling_enabled());
        assert!(support.is_executor_monitoring_enabled());
    }

    #[test]
    fn test_delegate_back_operations() {
        let health_tunnel = Arc::new(DummyHealthTunnel);
        let failover = Arc::new(WindowsLocalFailover::new());
        let mut support = WindowsSupport::new(health_tunnel, failover);

        let context = SupportContext::JIT_COMPILATION;
        let _ = support.take_over_operations(context);

        assert!(support.delegate_back_operations().is_ok());
        assert!(!support.is_supervisor_busy());
        assert!(!support.is_jit_allowed());
    }
}
