//! KVM Executor for Windows Module

use libc::{kill, SIGCONT, SIGSTOP, SIGTERM};
use std::process::Child;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum KvmError {
    #[error("KVM not available: {0}")]
    NotAvailable(String),
    #[error("Failed to start VM: {0}")]
    StartError(String),
    #[error("VM process error: {0}")]
    ProcessError(String),
}

#[derive(Clone, Debug)]
pub struct KvmConfig {
    pub kernel: Option<String>,
    pub initrd: Option<String>,
    pub append: Option<String>,
    pub memory_mb: u32,
    pub cpu_count: u32,
    pub disk_image: Option<String>,
    pub gpu_passthrough: bool,
    pub gpu_addr: Option<String>,
    pub use_virtio: bool,
    pub snapshot_dir: Option<String>,
}

impl Default for KvmConfig {
    fn default() -> Self {
        Self {
            kernel: None,
            initrd: None,
            append: Some("console=ttyS0 root=/dev/sda1".to_string()),
            memory_mb: 2048,
            cpu_count: 2,
            disk_image: None,
            gpu_passthrough: false,
            gpu_addr: None,
            use_virtio: true,
            snapshot_dir: None,
        }
    }
}

pub struct KvmExecutor {
    config: Arc<KvmConfig>,
    child: Arc<tokio::sync::Mutex<Option<Child>>>,
    running: AtomicBool,
    vm_pid: AtomicU32,
}

impl KvmExecutor {
    pub fn new() -> Self {
        Self {
            config: Arc::new(KvmConfig::default()),
            child: Arc::new(tokio::sync::Mutex::new(None)),
            running: AtomicBool::new(false),
            vm_pid: AtomicU32::new(0),
        }
    }

    pub fn with_config(config: KvmConfig) -> Self {
        Self {
            config: Arc::new(config),
            child: Arc::new(tokio::sync::Mutex::new(None)),
            running: AtomicBool::new(false),
            vm_pid: AtomicU32::new(0),
        }
    }

    pub fn set_config(&mut self, config: KvmConfig) {
        self.config = Arc::new(config);
    }

    pub fn check_kvm_availability(&self) -> Result<bool, KvmError> {
        let kvm_path = std::path::Path::new("/dev/kvm");
        if !kvm_path.exists() {
            return Err(KvmError::NotAvailable("/dev/kvm not found".to_string()));
        }

        if let Ok(metadata) = std::fs::metadata(kvm_path) {
            if metadata.permissions().readonly() {
                return Err(KvmError::NotAvailable(
                    "/dev/kvm is not readable".to_string(),
                ));
            }
        }

        if let Ok(output) = std::process::Command::new("kvm-ok").output() {
            if output.status.success() {
                return Ok(true);
            }
        }

        tracing::warn!("KVM device exists but kvm-ok failed");
        Ok(false)
    }

    pub async fn start(&self) -> Result<u32, KvmError> {
        if self.running.load(Ordering::Relaxed) {
            let pid = self.vm_pid.load(Ordering::Relaxed);
            return Ok(pid);
        }

        let cfg = self.config.as_ref().clone();

        if cfg.kernel.is_none() && cfg.disk_image.is_none() {
            return Err(KvmError::NotAvailable(
                "No kernel or disk image provided".to_string(),
            ));
        }

        let mut cmd = std::process::Command::new("qemu-system-x86_64");
        cmd.arg("-enable-kvm");

        if let Some(ref kernel) = cfg.kernel {
            cmd.arg("-kernel").arg(kernel);
        }
        if let Some(ref initrd) = cfg.initrd {
            cmd.arg("-initrd").arg(initrd);
        }
        if let Some(ref append) = cfg.append {
            cmd.arg("-append").arg(append);
        }

        cmd.arg("-m").arg(cfg.memory_mb.to_string());
        cmd.arg("-smp").arg(cfg.cpu_count.to_string());

        if let Some(ref disk) = cfg.disk_image {
            cmd.arg("-hda").arg(disk);
        }

        if cfg.gpu_passthrough {
            cmd.arg("-device").arg("vfio-pci,host=01:00.0,x-vga=on");
            if let Some(ref addr) = cfg.gpu_addr {
                cmd.arg(format!("-device vfio-pci,host={}", addr));
            }
        } else if cfg.use_virtio {
            cmd.arg("-vga").arg("virtio");
            cmd.arg("-display").arg("none");
        } else {
            cmd.arg("-vga").arg("std");
        }

        cmd.arg("-nographic");
        cmd.arg("-serial").arg("stdio");
        cmd.arg("-monitor").arg("none");

        let child = cmd
            .spawn()
            .map_err(|e| KvmError::StartError(e.to_string()))?;
        let pid = child.id();

        {
            let mut child_lock = self.child.lock().await;
            *child_lock = Some(child);
        }
        self.running.store(true, Ordering::Relaxed);
        self.vm_pid.store(pid, Ordering::Relaxed);

        info!("KVM started with PID {}", pid);
        Ok(pid)
    }

    pub async fn stop(&self) -> Result<(), KvmError> {
        let pid = self.vm_pid.load(Ordering::Relaxed);
        if pid != 0 {
            unsafe { kill(pid as i32, SIGTERM) };
        }

        {
            let mut child_lock = self.child.lock().await;
            if let Some(mut child) = child_lock.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }

        self.running.store(false, Ordering::Relaxed);
        self.vm_pid.store(0, Ordering::Relaxed);
        info!("KVM stopped");
        Ok(())
    }

    pub async fn pause(&self) -> Result<(), KvmError> {
        let pid = self.vm_pid.load(Ordering::Relaxed);
        if pid != 0 {
            unsafe { kill(pid as i32, SIGSTOP) };
            info!("KVM paused");
        }
        Ok(())
    }

    pub async fn resume(&self) -> Result<(), KvmError> {
        let pid = self.vm_pid.load(Ordering::Relaxed);
        if pid != 0 {
            unsafe { kill(pid as i32, SIGCONT) };
            info!("KVM resumed");
        }
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn get_pid(&self) -> u32 {
        self.vm_pid.load(Ordering::Relaxed)
    }

    pub fn get_status(&self) -> String {
        if self.is_running() {
            "running".to_string()
        } else {
            "stopped".to_string()
        }
    }
}

impl Default for KvmExecutor {
    fn default() -> Self {
        Self::new()
    }
}