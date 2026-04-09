use common::supervisor_support::{
    SupervisorSupport as SupervisorSupportTrait, SupportContext, SupportError, SupportStatus,
};
use dashmap::DashSet;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AndroidSupportError {
    #[error("Support not active")]
    NotActive,
    #[error("Support error: {0}")]
    OperationError(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SupportTask {
    ContainerMonitoring,
    HybridLibrarySupervision,
}

pub struct AndroidSupport {
    active_tasks: DashSet<SupportTask>,
    status: AtomicU8,
    container_monitor_enabled: AtomicBool,
    hybrid_library_supervision_enabled: AtomicBool,
}

const STATUS_IDLE: u8 = 0;
const STATUS_SUPPORTING: u8 = 1;
const STATUS_SUSPENDED: u8 = 2;

impl Default for AndroidSupport {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidSupport {
    pub fn new() -> Self {
        Self {
            active_tasks: DashSet::new(),
            status: AtomicU8::new(STATUS_IDLE),
            container_monitor_enabled: AtomicBool::new(false),
            hybrid_library_supervision_enabled: AtomicBool::new(false),
        }
    }

    pub fn start_support(&self) {
        self.status.store(STATUS_SUPPORTING, Ordering::Relaxed);
    }

    pub fn stop_support(&self) {
        self.status.store(STATUS_IDLE, Ordering::Relaxed);
        self.active_tasks.clear();
        self.container_monitor_enabled
            .store(false, Ordering::Relaxed);
        self.hybrid_library_supervision_enabled
            .store(false, Ordering::Relaxed);
    }

    pub fn is_supporting(&self) -> bool {
        self.status.load(Ordering::Relaxed) == STATUS_SUPPORTING
    }

    pub fn enable_task(&self, task: SupportTask) -> Result<(), AndroidSupportError> {
        if !self.is_supporting() {
            return Err(AndroidSupportError::NotActive);
        }
        match task {
            SupportTask::ContainerMonitoring => self
                .container_monitor_enabled
                .store(true, Ordering::Relaxed),
            SupportTask::HybridLibrarySupervision => self
                .hybrid_library_supervision_enabled
                .store(true, Ordering::Relaxed),
        }
        self.active_tasks.insert(task);
        Ok(())
    }

    pub fn disable_task(&self, task: &SupportTask) {
        self.active_tasks.remove(task);
        match task {
            SupportTask::ContainerMonitoring => self
                .container_monitor_enabled
                .store(false, Ordering::Relaxed),
            SupportTask::HybridLibrarySupervision => self
                .hybrid_library_supervision_enabled
                .store(false, Ordering::Relaxed),
        }
    }

    pub fn is_task_active(&self, task: &SupportTask) -> bool {
        self.active_tasks.contains(task)
    }

    pub fn get_active_tasks(&self) -> Vec<SupportTask> {
        self.active_tasks.iter().map(|r| r.clone()).collect()
    }

    pub fn is_container_monitor_enabled(&self) -> bool {
        self.container_monitor_enabled.load(Ordering::Relaxed)
    }

    pub fn is_hybrid_library_supervision_enabled(&self) -> bool {
        self.hybrid_library_supervision_enabled
            .load(Ordering::Relaxed)
    }

    fn get_status(&self) -> SupportStatus {
        match self.status.load(Ordering::Relaxed) {
            STATUS_IDLE => SupportStatus::Idle,
            STATUS_SUPPORTING => SupportStatus::Supporting,
            STATUS_SUSPENDED => SupportStatus::Suspended,
            _ => SupportStatus::Idle,
        }
    }
}

impl SupervisorSupportTrait for AndroidSupport {
    fn is_supervisor_busy(&self) -> bool {
        self.status.load(Ordering::Relaxed) == STATUS_SUPPORTING
    }

    fn take_over_operations(&self, context: SupportContext) -> Result<(), SupportError> {
        if self.status.load(Ordering::Relaxed) == STATUS_SUSPENDED {
            return Err(SupportError::TakeOverFailed(
                "Support is suspended".to_string(),
            ));
        }

        self.status.store(STATUS_SUPPORTING, Ordering::Relaxed);

        if context.contains(SupportContext::CONTAINER_MONITORING) {
            self.container_monitor_enabled
                .store(true, Ordering::Relaxed);
            self.active_tasks.insert(SupportTask::ContainerMonitoring);
        }

        if context.contains(SupportContext::HYBRID_LIBRARY_SUPERVISION) {
            self.hybrid_library_supervision_enabled
                .store(true, Ordering::Relaxed);
            self.active_tasks
                .insert(SupportTask::HybridLibrarySupervision);
        }

        Ok(())
    }

    fn delegate_back_operations(&self) -> Result<(), SupportError> {
        self.container_monitor_enabled
            .store(false, Ordering::Relaxed);
        self.hybrid_library_supervision_enabled
            .store(false, Ordering::Relaxed);
        self.active_tasks.clear();
        self.status.store(STATUS_IDLE, Ordering::Relaxed);
        Ok(())
    }

    fn support_status(&self) -> SupportStatus {
        self.get_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_support_creation() -> anyhow::Result<()> {
        let support = AndroidSupport::new();
        assert!(!support.is_supporting());
        assert_eq!(support.support_status(), SupportStatus::Idle);
        Ok(())
    }

    #[test]
    fn test_start_stop_support() -> anyhow::Result<()> {
        let mut support = AndroidSupport::new();
        support.start_support();
        assert!(support.is_supporting());
        support.stop_support();
        assert!(!support.is_supporting());
        Ok(())
    }

    #[test]
    fn test_enable_task_requires_support() -> anyhow::Result<()> {
        let mut support = AndroidSupport::new();
        let result = support.enable_task(SupportTask::ContainerMonitoring);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_enable_task_when_supporting() -> anyhow::Result<()> {
        let mut support = AndroidSupport::new();
        support.start_support();
        support.enable_task(SupportTask::ContainerMonitoring)?;
        assert!(support.is_task_active(&SupportTask::ContainerMonitoring));
        assert!(support.is_container_monitor_enabled());
        Ok(())
    }

    #[test]
    fn test_supervisor_support_trait_take_over() -> anyhow::Result<()> {
        let mut support = AndroidSupport::new();
        let context = SupportContext::CONTAINER_MONITORING;
        support.take_over_operations(context)?;
        assert!(support.is_supporting());
        assert!(support.is_container_monitor_enabled());
        Ok(())
    }

    #[test]
    fn test_supervisor_support_trait_delegate_back() -> anyhow::Result<()> {
        let mut support = AndroidSupport::new();
        let context = SupportContext::CONTAINER_MONITORING;
        support.take_over_operations(context)?;
        support.delegate_back_operations()?;
        assert_eq!(support.support_status(), SupportStatus::Idle);
        assert!(!support.is_container_monitor_enabled());
        Ok(())
    }
}
