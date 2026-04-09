//! Executor Orchestrator for Windows Module – manages Wine and KVM executors

use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum OrchestratorError {
    #[error("Executor not found: {0}")]
    NotFound(String),
    #[error("Failed to start executor: {0}")]
    StartFailed(String),
    #[error("Failed to stop executor: {0}")]
    StopFailed(String),
}

#[derive(Clone, Debug)]
pub enum ExecutorType {
    Wine,
    Kvm,
}

#[derive(Clone, Debug)]
pub struct ExecutorInfo {
    pub id: String,
    pub exe_type: ExecutorType,
    pub pid: Option<u32>,
    pub started_at: u64,
    pub active: bool,
}

pub struct ExecutorOrchestrator {
    executors: DashMap<String, ExecutorInfo>,
    active_executor: AtomicU64,
    wine_pid: AtomicU32,
    kvm_pid: AtomicU32,
}

impl ExecutorOrchestrator {
    pub fn new() -> Self {
        Self {
            executors: DashMap::new(),
            active_executor: AtomicU64::new(0),
            wine_pid: AtomicU32::new(0),
            kvm_pid: AtomicU32::new(0),
        }
    }

    pub fn start_executor(
        &self,
        id: &str,
        exe_type: ExecutorType,
        config: &ExecutorConfig,
    ) -> Result<u32, OrchestratorError> {
        let pid = match exe_type {
            ExecutorType::Wine => self.start_wine(config)?,
            ExecutorType::Kvm => self.start_kvm(config)?,
        };

        let info = ExecutorInfo {
            id: id.to_string(),
            exe_type,
            pid: Some(pid),
            started_at: Self::current_timestamp(),
            active: true,
        };

        self.executors.insert(id.to_string(), info);

        let id_ptr = id.as_ptr() as u64;
        self.active_executor.store(id_ptr, Ordering::Relaxed);

        info!("Started executor {} with PID {}", id, pid);
        Ok(pid)
    }

    fn start_wine(&self, config: &ExecutorConfig) -> Result<u32, OrchestratorError> {
        let mut cmd = std::process::Command::new("wine");
        if let Some(ref prefix) = config.wine_prefix {
            cmd.env("WINEPREFIX", prefix);
        }
        if let Some(ref program) = config.wine_program {
            cmd.arg(program);
        }
        if config.wine_server_timeout > 0 {
            cmd.env("WINESERVER", format!("-t {}", config.wine_server_timeout));
        }

        let child = cmd
            .spawn()
            .map_err(|e| OrchestratorError::StartFailed(e.to_string()))?;

        let pid = child.id();
        self.wine_pid.store(pid, Ordering::Relaxed);

        Ok(pid)
    }

    fn start_kvm(&self, config: &ExecutorConfig) -> Result<u32, OrchestratorError> {
        let current_kvm = self.kvm_pid.load(Ordering::Relaxed);
        if current_kvm != 0 {
            return Ok(current_kvm);
        }

        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        cmd.arg("-enable-kvm");
        if let Some(ref kernel) = config.kvm_kernel {
            cmd.arg("-kernel").arg(kernel);
        }
        if let Some(ref initrd) = config.kvm_initrd {
            cmd.arg("-initrd").arg(initrd);
        }
        if let Some(ref append) = config.kvm_append {
            cmd.arg("-append").arg(append);
        }
        if let Some(mem) = config.kvm_memory {
            cmd.arg("-m").arg(mem.to_string());
        }
        if let Some(cpu) = config.kvm_cpu {
            if cpu > 0 {
                cmd.arg("-smp").arg(cpu.to_string());
            }
        }

        if let Some(ref disk) = config.kvm_disk {
            cmd.arg("-hda").arg(disk);
        }

        if config.kvm_gpu_passthrough {
            cmd.arg("-vfio-pci");
            if let Some(ref gpu) = config.kvm_gpu_addr {
                cmd.arg(gpu);
            }
        } else if config.kvm_use_virtio {
            cmd.arg("-vga").arg("virtio");
            cmd.arg("-display").arg("none");
        } else {
            cmd.arg("-vga").arg("std");
        }

        cmd.arg("-nographic");
        cmd.arg("-serial").arg("stdio");

        let child = cmd
            .spawn()
            .map_err(|e| OrchestratorError::StartFailed(e.to_string()))?;

        let pid = child.id();
        self.kvm_pid.store(pid, Ordering::Relaxed);

        Ok(pid)
    }

    pub fn stop_executor(&self, id: &str) -> Result<(), OrchestratorError> {
        let mut info = self
            .executors
            .get_mut(id)
            .ok_or_else(|| OrchestratorError::NotFound(id.to_string()))?;

        if let Some(pid) = info.pid {
            let _ = std::process::Command::new("kill")
                .arg(pid.to_string())
                .output();
        }

        info.active = false;
        info.pid = None;

        Ok(())
    }

    pub fn get_active_executor(&self) -> Option<String> {
        let ptr = self.active_executor.load(Ordering::Relaxed);
        if ptr == 0 {
            return None;
        }
        unsafe {
            let s = std::slice::from_raw_parts(ptr as *const u8, 20);
            std::str::from_utf8(s)
                .ok()
                .map(|s| s.trim_end_matches('\0').to_string())
        }
    }

    pub fn switch_executor(&self, from_id: &str, to_id: &str) -> Result<(), OrchestratorError> {
        self.stop_executor(from_id)?;
        self.start_executor(to_id, ExecutorType::Kvm, &ExecutorConfig::default())?;
        Ok(())
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    }
}

impl Default for ExecutorOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub struct ExecutorConfig {
    pub wine_prefix: Option<String>,
    pub wine_program: Option<String>,
    pub wine_server_timeout: u32,
    pub kvm_kernel: Option<String>,
    pub kvm_initrd: Option<String>,
    pub kvm_append: Option<String>,
    pub kvm_memory: Option<u32>,
    pub kvm_cpu: Option<u32>,
    pub kvm_disk: Option<String>,
    pub kvm_gpu_passthrough: bool,
    pub kvm_gpu_addr: Option<String>,
    pub kvm_use_virtio: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            wine_prefix: None,
            wine_program: None,
            wine_server_timeout: 60,
            kvm_kernel: None,
            kvm_initrd: None,
            kvm_append: Some("console=ttyS0".to_string()),
            kvm_memory: Some(2048),
            kvm_cpu: Some(2),
            kvm_disk: None,
            kvm_gpu_passthrough: false,
            kvm_gpu_addr: None,
            kvm_use_virtio: true,
        }
    }
}
