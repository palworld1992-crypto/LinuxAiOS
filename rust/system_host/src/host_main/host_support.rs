//! Host Support - SupervisorSupport implementation for HostMain
//! Implements support functions: health checker, micro scheduler, watchdog

use common::supervisor_support::{SupervisorSupport, SupportContext, SupportError, SupportStatus};
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

pub struct HostSupport {
    active: AtomicBool,
    health_checker_enabled: AtomicBool,
    micro_scheduler_enabled: AtomicBool,
    watchdog_enabled: AtomicBool,
    support_tasks: Arc<DashMap<String, Instant>>,
}

impl HostSupport {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            health_checker_enabled: AtomicBool::new(false),
            micro_scheduler_enabled: AtomicBool::new(false),
            watchdog_enabled: AtomicBool::new(false),
            support_tasks: Arc::new(DashMap::new()),
        }
    }

    pub fn start_support(&self) {
        self.active.store(true, Ordering::SeqCst);
    }

    pub fn stop_support(&self) {
        self.active.store(false, Ordering::SeqCst);
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn is_health_checker_enabled(&self) -> bool {
        self.health_checker_enabled.load(Ordering::SeqCst)
    }

    pub fn is_micro_scheduler_enabled(&self) -> bool {
        self.micro_scheduler_enabled.load(Ordering::SeqCst)
    }

    pub fn is_watchdog_enabled(&self) -> bool {
        self.watchdog_enabled.load(Ordering::SeqCst)
    }
}

impl SupervisorSupport for HostSupport {
    fn is_supervisor_busy(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    fn take_over_operations(&self, context: SupportContext) -> Result<(), SupportError> {
        self.active.store(true, Ordering::SeqCst);

        if context.contains(SupportContext::HEALTH_CHECK) {
            self.health_checker_enabled.store(true, Ordering::SeqCst);
            self.support_tasks
                .insert("health_checker".to_string(), Instant::now());
        }

        if context.contains(SupportContext::MICRO_SCHEDULER) {
            self.micro_scheduler_enabled.store(true, Ordering::SeqCst);
            self.support_tasks
                .insert("micro_scheduler".to_string(), Instant::now());
        }

        if context.contains(SupportContext::WATCHDOG) {
            self.watchdog_enabled.store(true, Ordering::SeqCst);
            self.support_tasks
                .insert("watchdog".to_string(), Instant::now());
        }

        Ok(())
    }

    fn delegate_back_operations(&self) -> Result<(), SupportError> {
        self.health_checker_enabled.store(false, Ordering::SeqCst);
        self.micro_scheduler_enabled.store(false, Ordering::SeqCst);
        self.watchdog_enabled.store(false, Ordering::SeqCst);
        self.active.store(false, Ordering::SeqCst);
        self.support_tasks.clear();
        Ok(())
    }

    fn support_status(&self) -> SupportStatus {
        if self.active.load(Ordering::SeqCst) {
            SupportStatus::Supporting
        } else {
            SupportStatus::Idle
        }
    }
}

impl Default for HostSupport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::supervisor_support::types::SupportContext;

    #[test]
    fn test_support_creation() -> anyhow::Result<()> {
        let support = HostSupport::default();
        assert!(!support.is_active());
        assert!(!support.is_health_checker_enabled());
        assert!(!support.is_micro_scheduler_enabled());
        assert!(!support.is_watchdog_enabled());
        Ok(())
    }

    #[test]
    fn test_start_stop_support() -> anyhow::Result<()> {
        let support = HostSupport::default();

        support.start_support();
        assert!(support.is_active());

        support.stop_support();
        assert!(!support.is_active());

        Ok(())
    }

    #[test]
    fn test_take_over_operations() -> anyhow::Result<()> {
        let mut support = HostSupport::default();
        let context = SupportContext::HEALTH_CHECK.union(SupportContext::MICRO_SCHEDULER);

        support.take_over_operations(context)?;

        assert!(support.is_active());
        assert!(support.is_health_checker_enabled());
        assert!(support.is_micro_scheduler_enabled());
        assert!(!support.is_watchdog_enabled());

        Ok(())
    }

    #[test]
    fn test_delegate_back_operations() -> anyhow::Result<()> {
        let mut support = HostSupport::default();
        support.start_support();
        assert!(support.is_active());

        support.delegate_back_operations()?;

        assert!(!support.is_active());
        assert!(!support.is_health_checker_enabled());
        assert!(!support.is_micro_scheduler_enabled());
        assert!(!support.is_watchdog_enabled());

        Ok(())
    }

    #[test]
    fn test_support_status() -> anyhow::Result<()> {
        let support = HostSupport::default();
        assert_eq!(support.support_status(), SupportStatus::Idle);

        support.start_support();
        assert_eq!(support.support_status(), SupportStatus::Supporting);

        support.stop_support();
        assert_eq!(support.support_status(), SupportStatus::Idle);

        Ok(())
    }
}
