//! Windows Module Support Functions – Implement SupervisorSupport for WindowsMain

use crate::windows::windows_local_failover::WindowsLocalFailover;
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use common::supervisor_support::{SupervisorSupport, SupportContext, SupportError, SupportStatus};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct WindowsSupport {
    status: AtomicU64,
    active_tasks: AtomicU64,
    health_tunnel: Arc<dyn HealthTunnel>,
    failover: Arc<WindowsLocalFailover>,
    api_profiling_enabled: AtomicBool,
    executor_monitoring_enabled: AtomicBool,
    jit_compilation_allowed: AtomicBool,
}

impl WindowsSupport {
    pub fn new(health_tunnel: Arc<dyn HealthTunnel>, failover: Arc<WindowsLocalFailover>) -> Self {
        Self {
            status: AtomicU64::new(0),
            active_tasks: AtomicU64::new(0),
            health_tunnel,
            failover,
            api_profiling_enabled: AtomicBool::new(false),
            executor_monitoring_enabled: AtomicBool::new(false),
            jit_compilation_allowed: AtomicBool::new(false),
        }
    }

    pub fn start_api_profiling(&self) {
        self.api_profiling_enabled.store(true, Ordering::Relaxed);
        debug!("API profiling enabled via support");
    }

    pub fn stop_api_profiling(&self) {
        self.api_profiling_enabled.store(false, Ordering::Relaxed);
        debug!("API profiling disabled");
    }

    pub fn start_executor_monitoring(&self) {
        self.executor_monitoring_enabled
            .store(true, Ordering::Relaxed);
        debug!("Executor monitoring enabled via support");
    }

    pub fn stop_executor_monitoring(&self) {
        self.executor_monitoring_enabled
            .store(false, Ordering::Relaxed);
        debug!("Executor monitoring disabled");
    }

    pub fn allow_jit_compilation(&self) {
        self.jit_compilation_allowed.store(true, Ordering::Relaxed);
        debug!("JIT compilation allowed via support");
    }

    pub fn disallow_jit_compilation(&self) {
        self.jit_compilation_allowed.store(false, Ordering::Relaxed);
        debug!("JIT compilation disallowed");
    }

    pub fn is_api_profiling_enabled(&self) -> bool {
        self.api_profiling_enabled.load(Ordering::Relaxed)
    }

    pub fn is_executor_monitoring_enabled(&self) -> bool {
        self.executor_monitoring_enabled.load(Ordering::Relaxed)
    }

    pub fn is_jit_allowed(&self) -> bool {
        self.jit_compilation_allowed.load(Ordering::Relaxed)
    }
}

impl SupervisorSupport for WindowsSupport {
    fn is_supervisor_busy(&self) -> bool {
        self.status.load(Ordering::Relaxed) == 1
    }

    fn take_over_operations(&self, context: SupportContext) -> Result<(), SupportError> {
        let current_status = self.status.load(Ordering::Relaxed);
        if current_status == 2 {
            return Err(SupportError::TakeOverFailed(
                "Support is suspended".to_string(),
            ));
        }

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

        self.status.store(1, Ordering::Relaxed);
        self.active_tasks.store(tasks.0 as u64, Ordering::Relaxed);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);

        let record = HealthRecord {
            module_id: "windows".to_string(),
            status: HealthStatus::Healthy,
            potential: 0.5,
            details: format!("Support started with tasks: {:?}", tasks).into_bytes(),
            timestamp,
        };

        let _ = self.health_tunnel.record_health(record);
        info!("Windows support started, tasks: {:?}", tasks);
        Ok(())
    }

    fn delegate_back_operations(&self) -> Result<(), SupportError> {
        self.stop_api_profiling();
        self.stop_executor_monitoring();
        self.disallow_jit_compilation();

        self.status.store(0, Ordering::Relaxed);
        self.active_tasks.store(0, Ordering::Relaxed);

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64);

        let record = HealthRecord {
            module_id: "windows".to_string(),
            status: HealthStatus::Healthy,
            potential: 0.8,
            details: "Support delegated back".to_string().into_bytes(),
            timestamp,
        };

        let _ = self.health_tunnel.record_health(record);
        info!("Windows support ended, operations delegated back");
        Ok(())
    }

    fn support_status(&self) -> SupportStatus {
        match self.status.load(Ordering::Relaxed) {
            0 => SupportStatus::Idle,
            1 => SupportStatus::Supporting,
            _ => SupportStatus::Suspended,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::health_tunnel::HealthRecord;

    struct DummyHealthTunnel;

    impl HealthTunnel for DummyHealthTunnel {
        fn record_health(&self, _record: HealthRecord) -> anyhow::Result<()> {
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
    fn test_support_default_state() -> anyhow::Result<()> {
        let health_tunnel = Arc::new(DummyHealthTunnel);
        let failover = Arc::new(WindowsLocalFailover::new());
        let support = WindowsSupport::new(health_tunnel, failover);
        assert!(!support.is_supervisor_busy());
        assert_eq!(support.support_status(), SupportStatus::Idle);
        Ok(())
    }

    #[test]
    fn test_take_over_operations() -> anyhow::Result<()> {
        let health_tunnel = Arc::new(DummyHealthTunnel);
        let failover = Arc::new(WindowsLocalFailover::new());
        let mut support = WindowsSupport::new(health_tunnel, failover);
        let context = SupportContext::API_PROFILING.union(SupportContext::EXECUTOR_MONITORING);
        support.take_over_operations(context)?;
        assert!(support.is_supervisor_busy());
        assert!(support.is_api_profiling_enabled());
        assert!(support.is_executor_monitoring_enabled());
        Ok(())
    }

    #[test]
    fn test_delegate_back_operations() -> anyhow::Result<()> {
        let health_tunnel = Arc::new(DummyHealthTunnel);
        let failover = Arc::new(WindowsLocalFailover::new());
        let mut support = WindowsSupport::new(health_tunnel, failover);
        let context = SupportContext::JIT_COMPILATION;
        support.take_over_operations(context)?;
        support.delegate_back_operations()?;
        assert!(!support.is_supervisor_busy());
        assert!(!support.is_jit_allowed());
        Ok(())
    }
}
