//! Adaptive Support - SupervisorSupport implementation for AdaptiveMain

use common::supervisor_support::{SupervisorSupport, SupportContext, SupportError, SupportStatus};
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

pub struct AdaptiveSupport {
    active: AtomicBool,
    state_cache_enabled: AtomicBool,
    websocket_enabled: AtomicBool,
    support_tasks: DashMap<String, Instant>,
}

impl AdaptiveSupport {
    pub fn new() -> Self {
        Self {
            active: AtomicBool::new(false),
            state_cache_enabled: AtomicBool::new(false),
            websocket_enabled: AtomicBool::new(false),
            support_tasks: DashMap::new(),
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

    pub fn is_state_cache_enabled(&self) -> bool {
        self.state_cache_enabled.load(Ordering::SeqCst)
    }

    pub fn is_websocket_enabled(&self) -> bool {
        self.websocket_enabled.load(Ordering::SeqCst)
    }
}

impl SupervisorSupport for AdaptiveSupport {
    fn is_supervisor_busy(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    fn take_over_operations(&self, context: SupportContext) -> Result<(), SupportError> {
        self.active.store(true, Ordering::SeqCst);

        if context.contains(SupportContext::HEALTH_CHECK) {
            self.state_cache_enabled.store(true, Ordering::SeqCst);
            self.support_tasks
                .insert("state_cache".to_string(), Instant::now());
        }

        if context.contains(SupportContext::MEMORY_TIERING) {
            self.websocket_enabled.store(true, Ordering::SeqCst);
            self.support_tasks
                .insert("websocket".to_string(), Instant::now());
        }

        Ok(())
    }

    fn delegate_back_operations(&self) -> Result<(), SupportError> {
        self.state_cache_enabled.store(false, Ordering::SeqCst);
        self.websocket_enabled.store(false, Ordering::SeqCst);
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

impl Default for AdaptiveSupport {
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
        let support = AdaptiveSupport::default();
        assert!(!support.is_active());
        assert!(!support.is_state_cache_enabled());
        assert!(!support.is_websocket_enabled());
        Ok(())
    }

    #[test]
    fn test_start_stop_support() -> anyhow::Result<()> {
        let support = AdaptiveSupport::default();

        support.start_support();
        assert!(support.is_active());

        support.stop_support();
        assert!(!support.is_active());

        Ok(())
    }

    #[test]
    fn test_take_over_operations() -> anyhow::Result<()> {
        let mut support = AdaptiveSupport::default();
        let context = SupportContext::HEALTH_CHECK.union(SupportContext::MEMORY_TIERING);

        support.take_over_operations(context)?;

        assert!(support.is_active());
        assert!(support.is_state_cache_enabled());
        assert!(support.is_websocket_enabled());

        Ok(())
    }

    #[test]
    fn test_delegate_back_operations() -> anyhow::Result<()> {
        let mut support = AdaptiveSupport::default();
        support.start_support();
        assert!(support.is_active());

        support.delegate_back_operations()?;

        assert!(!support.is_active());
        assert!(!support.is_state_cache_enabled());
        assert!(!support.is_websocket_enabled());

        Ok(())
    }

    #[test]
    fn test_support_status() -> anyhow::Result<()> {
        let support = AdaptiveSupport::default();
        assert_eq!(support.support_status(), SupportStatus::Idle);

        support.start_support();
        assert_eq!(support.support_status(), SupportStatus::Supporting);

        support.stop_support();
        assert_eq!(support.support_status(), SupportStatus::Idle);

        Ok(())
    }
}
