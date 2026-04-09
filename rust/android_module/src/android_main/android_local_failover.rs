use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use tracing::warn;

#[derive(Error, Debug)]
pub enum FailoverError {
    #[error("Failed to handle supervisor failure: {0}")]
    SupervisorFailure(String),
    #[error("Failed to accept new supervisor: {0}")]
    AcceptSupervisor(String),
    #[error("Heartbeat timeout exceeded")]
    HeartbeatTimeout,
}

pub struct AndroidLocalFailover {
    supervisor_heartbeat: AtomicU64,
    is_failover_active: AtomicBool,
    last_failure_timestamp: AtomicU64,
    heartbeat_timeout_secs: u64,
    failure_count: AtomicU64,
}

impl Default for AndroidLocalFailover {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidLocalFailover {
    fn get_current_timestamp() -> u64 {
        match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(e) => {
                warn!("SystemTime before UNIX_EPOCH: {}, using 0", e);
                0
            }
        }
    }

    pub fn new() -> Self {
        Self {
            supervisor_heartbeat: AtomicU64::new(Self::get_current_timestamp()),
            is_failover_active: AtomicBool::new(false),
            last_failure_timestamp: AtomicU64::new(0),
            heartbeat_timeout_secs: 30,
            failure_count: AtomicU64::new(0),
        }
    }

    pub fn with_timeout(timeout_secs: u64) -> Self {
        let mut failover = Self::new();
        failover.heartbeat_timeout_secs = timeout_secs;
        failover
    }

    pub fn record_heartbeat(&self) {
        self.supervisor_heartbeat
            .store(Self::get_current_timestamp(), Ordering::SeqCst);
    }

    pub fn check_heartbeat(&self) -> bool {
        let now = Self::get_current_timestamp();
        let last = self.supervisor_heartbeat.load(Ordering::SeqCst);
        now.saturating_sub(last) < self.heartbeat_timeout_secs
    }

    pub fn handle_supervisor_failure(&self) -> Result<(), FailoverError> {
        if !self.check_heartbeat() {
            self.is_failover_active.store(true, Ordering::SeqCst);
            self.last_failure_timestamp
                .store(Self::get_current_timestamp(), Ordering::SeqCst);
            self.failure_count.fetch_add(1, Ordering::SeqCst);
            return Err(FailoverError::HeartbeatTimeout);
        }
        Ok(())
    }

    pub fn accept_new_supervisor(&self) -> Result<(), FailoverError> {
        self.is_failover_active.store(false, Ordering::SeqCst);
        self.record_heartbeat();
        self.failure_count.store(0, Ordering::SeqCst);
        Ok(())
    }

    pub fn is_failover_active(&self) -> bool {
        self.is_failover_active.load(Ordering::SeqCst)
    }

    pub fn get_failure_count(&self) -> u64 {
        self.failure_count.load(Ordering::SeqCst)
    }

    pub fn get_last_failure_timestamp(&self) -> u64 {
        self.last_failure_timestamp.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_failover_creation() -> anyhow::Result<()> {
        let _ = AndroidLocalFailover::new();
        Ok(())
    }

    #[test]
    fn test_heartbeat_recording() -> anyhow::Result<()> {
        let failover = AndroidLocalFailover::new();
        assert!(failover.check_heartbeat());
        failover.record_heartbeat();
        assert!(failover.check_heartbeat());
        Ok(())
    }

    #[test]
    fn test_handle_supervisor_failure_with_timeout() -> anyhow::Result<()> {
        let failover = AndroidLocalFailover::with_timeout(0);
        let result = failover.handle_supervisor_failure();
        assert!(result.is_err());
        assert!(failover.is_failover_active());
        Ok(())
    }

    #[test]
    fn test_accept_new_supervisor() -> anyhow::Result<()> {
        let failover = AndroidLocalFailover::with_timeout(0);
        let _ = failover.handle_supervisor_failure();
        assert!(failover.is_failover_active());
        failover.accept_new_supervisor()?;
        assert!(!failover.is_failover_active());
        Ok(())
    }

    #[test]
    fn test_failure_count() -> anyhow::Result<()> {
        let failover = AndroidLocalFailover::with_timeout(0);
        let _ = failover.handle_supervisor_failure();
        assert_eq!(failover.get_failure_count(), 1);
        let _ = failover.handle_supervisor_failure();
        assert_eq!(failover.get_failure_count(), 2);
        Ok(())
    }
}
