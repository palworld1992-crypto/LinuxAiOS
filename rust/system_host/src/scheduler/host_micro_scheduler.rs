//! Host Micro Scheduler - CPU pinning and process scheduling

use crate::zig_bindings::host_cpu_pinning::HostCpuPinning;
use dashmap::DashMap;
use libc;
use std::sync::atomic::{AtomicBool, Ordering};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    RoundRobin,
    Priority,
    CpuPinned,
    Dynamic,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_affinity: Option<u64>,
    pub priority: i32,
    pub policy: SchedulingPolicy,
}

#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Failed to pin process: {0}")]
    PinFailed(String),
    #[error("Process not found: {0}")]
    ProcessNotFound(u32),
    #[error("FFI error: {0}")]
    FfiError(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct HostMicroScheduler {
    // DashMap: lock-free concurrent map thay RwLock<HashMap>
    processes: DashMap<u32, ProcessInfo>,
    default_policy: SchedulingPolicy,
    enabled: AtomicBool,
    cpu_pinner: HostCpuPinning, // Phase 7: Zig CPU pinning via FFI
}

impl HostMicroScheduler {
    pub fn new(default_policy: SchedulingPolicy) -> Self {
        Self {
            processes: DashMap::new(),
            default_policy,
            enabled: AtomicBool::new(true),
            cpu_pinner: HostCpuPinning::new(), // Phase 7: initialize Zig FFI
        }
    }

    pub fn register_process(&self, pid: u32, name: String) {
        self.processes.insert(
            pid,
            ProcessInfo {
                pid,
                name,
                cpu_affinity: None,
                priority: 0,
                policy: self.default_policy,
            },
        );
    }

    pub fn unregister_process(&self, pid: u32) {
        self.processes.remove(&pid);
    }

    pub fn pin_process(&self, pid: u32, core_mask: u64) -> Result<(), SchedulerError> {
        if let Some(mut info) = self.processes.get_mut(&pid) {
            info.cpu_affinity = Some(core_mask);
            info.policy = SchedulingPolicy::CpuPinned;

            #[cfg(target_os = "linux")]
            {
                if let Err(e) = self.pin_process_native(pid, core_mask) {
                    return Err(SchedulerError::PinFailed(e.to_string()));
                }
            }

            Ok(())
        } else {
            Err(SchedulerError::ProcessNotFound(pid))
        }
    }

    #[cfg(target_os = "linux")]
    fn pin_process_native(&self, pid: u32, core_mask: u64) -> Result<(), std::io::Error> {
        // Phase 7: Use Zig FFI for CPU pinning instead of taskset
        self.cpu_pinner.pin_thread_to_core(pid, core_mask).map_err(
            |e: crate::zig_bindings::host_cpu_pinning::CpuPinningError| {
                std::io::Error::other(e.to_string())
            },
        )
    }

    pub fn set_priority(&self, pid: u32, priority: i32) -> Result<(), SchedulerError> {
        if let Some(mut info) = self.processes.get_mut(&pid) {
            info.priority = priority;

            #[cfg(target_os = "linux")]
            {
                let nice_value = priority.clamp(-20, 19);
                let ret = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as u32, nice_value) };
                if ret != 0 {
                    return Err(SchedulerError::PinFailed(
                        std::io::Error::last_os_error().to_string(),
                    ));
                }
            }

            Ok(())
        } else {
            Err(SchedulerError::ProcessNotFound(pid))
        }
    }

    pub fn set_policy(&self, pid: u32, policy: SchedulingPolicy) -> Result<(), SchedulerError> {
        if let Some(mut info) = self.processes.get_mut(&pid) {
            info.policy = policy;
            Ok(())
        } else {
            Err(SchedulerError::ProcessNotFound(pid))
        }
    }

    pub fn get_process(&self, pid: u32) -> Option<ProcessInfo> {
        // None: PID không tồn tại trong danh sách process đã đăng ký
        self.processes.get(&pid).map(|r| r.clone())
    }

    pub fn get_all_processes(&self) -> Vec<ProcessInfo> {
        self.processes.iter().map(|r| r.clone()).collect()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
    }

    pub fn get_default_policy(&self) -> SchedulingPolicy {
        self.default_policy
    }

    pub fn pin_current_thread(&self, core_id: u32) -> Result<(), SchedulerError> {
        let core_mask = 1u64 << core_id;

        #[cfg(target_os = "linux")]
        {
            let pid = std::process::id();
            self.pin_process(pid, core_mask)?;
        }

        Ok(())
    }
}

impl Default for HostMicroScheduler {
    fn default() -> Self {
        Self::new(SchedulingPolicy::Dynamic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() -> anyhow::Result<()> {
        let scheduler = HostMicroScheduler::default();
        assert!(scheduler.is_enabled());
        assert_eq!(scheduler.get_default_policy(), SchedulingPolicy::Dynamic);
        Ok(())
    }

    #[test]
    fn test_register_process() -> anyhow::Result<()> {
        let scheduler = HostMicroScheduler::default();

        scheduler.register_process(1234, "test_process".to_string());

        let info = scheduler.get_process(1234);
        assert!(info.is_some());
        assert_eq!(info.map(|i| i.name), Some("test_process".to_string()));

        Ok(())
    }

    #[test]
    fn test_set_enabled() -> anyhow::Result<()> {
        let scheduler = HostMicroScheduler::default();

        scheduler.set_enabled(false);
        assert!(!scheduler.is_enabled());

        scheduler.set_enabled(true);
        assert!(scheduler.is_enabled());

        Ok(())
    }
}
