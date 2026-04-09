//! Host CPU Pinning - Zig bindings for CPU affinity

use libc::pid_t;
use thiserror::Error;
use tracing::{debug, warn};

#[derive(Error, Debug)]
pub enum CpuPinningError {
    #[error("Pin failed: {0}")]
    PinFailed(String),
    #[error("FFI error: {0}")]
    FfiError(String),
    #[error("Invalid parameter: {0}")]
    InvalidParam(String),
}

extern "C" {
    fn pin_thread_to_core(pid: pid_t, core_mask: u64) -> i32;
    fn get_thread_affinity(pid: pid_t, core_mask: *mut u64) -> i32;
    fn pin_current_thread(core: u32) -> i32;
    fn unpin_thread(pid: pid_t) -> i32;
    fn get_cpu_count() -> i32;
    fn get_current_cpu() -> i32;
    fn pin_thread_range(pid: pid_t, start_core: u32, num_cores: u32) -> i32;
    fn get_available_cores(buffer: *mut u32, max_count: usize) -> i32;
    fn is_core_online(core: u32) -> bool;
    fn pin_thread_to_numa_node(node: i32) -> i32;
}

static ZIG_BINDINGS_AVAILABLE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn check_zig_bindings() -> bool {
    unsafe {
        let count = get_cpu_count();
        count > 0
    }
}

pub struct HostCpuPinning {
    native_available: bool,
}

impl HostCpuPinning {
    pub fn new() -> Self {
        let native_available = check_zig_bindings();
        ZIG_BINDINGS_AVAILABLE.store(native_available, std::sync::atomic::Ordering::SeqCst);

        if !native_available {
            warn!("Zig CPU pinning bindings not available, using native fallback");
        }

        Self { native_available }
    }

    pub fn pin_thread_to_core(&self, pid: u32, core_mask: u64) -> Result<(), CpuPinningError> {
        if core_mask == 0 {
            return Err(CpuPinningError::InvalidParam(
                "core_mask cannot be 0".to_string(),
            ));
        }

        if !self.native_available {
            return self.pin_thread_native(pid, core_mask);
        }

        let result = unsafe { pin_thread_to_core(pid as pid_t, core_mask) };
        if result == 0 {
            debug!("Pinned thread {} to cores {:#x}", pid, core_mask);
            Ok(())
        } else {
            Err(CpuPinningError::PinFailed(format!(
                "pin_thread_to_core failed with code {}",
                result
            )))
        }
    }

    pub fn get_current_affinity(&self, pid: u32) -> Result<u64, CpuPinningError> {
        if !self.native_available {
            return self.get_affinity_native(pid);
        }

        let mut core_mask: u64 = 0;
        let result = unsafe { get_thread_affinity(pid as pid_t, &mut core_mask) };
        if result == 0 {
            debug!("Got affinity for pid {}: {:#x}", pid, core_mask);
            Ok(core_mask)
        } else {
            Err(CpuPinningError::FfiError(format!(
                "get_thread_affinity failed with code {}",
                result
            )))
        }
    }

    pub fn pin_current_thread_to_core(&self, core: u32) -> Result<(), CpuPinningError> {
        if !self.native_available {
            return Err(CpuPinningError::PinFailed(
                "Native tools not available".to_string(),
            ));
        }

        let result = unsafe { pin_current_thread(core) };
        if result == 0 {
            debug!("Pinned current thread to core {}", core);
            Ok(())
        } else {
            Err(CpuPinningError::PinFailed(format!(
                "pin_current_thread failed with code {}",
                result
            )))
        }
    }

    pub fn unpin_thread(&self, pid: u32) -> Result<(), CpuPinningError> {
        if !self.native_available {
            return Err(CpuPinningError::PinFailed(
                "Native tools not available".to_string(),
            ));
        }

        let result = unsafe { unpin_thread(pid as pid_t) };
        if result == 0 {
            debug!("Unpinned thread {}", pid);
            Ok(())
        } else {
            Err(CpuPinningError::PinFailed(format!(
                "unpin_thread failed with code {}",
                result
            )))
        }
    }

    pub fn get_cpu_count(&self) -> i32 {
        if self.native_available {
            unsafe { get_cpu_count() }
        } else {
            num_cpus::get().try_into().ok().map_or(1, |v| v)
        }
    }

    pub fn get_current_cpu(&self) -> Result<i32, CpuPinningError> {
        if !self.native_available {
            return Err(CpuPinningError::FfiError(
                "Zig bindings not available".to_string(),
            ));
        }

        let cpu = unsafe { get_current_cpu() };
        Ok(cpu)
    }

    pub fn pin_thread_range(
        &self,
        pid: u32,
        start_core: u32,
        num_cores: u32,
    ) -> Result<(), CpuPinningError> {
        if !self.native_available {
            return Err(CpuPinningError::PinFailed(
                "Native tools not available".to_string(),
            ));
        }

        let result = unsafe { pin_thread_range(pid as pid_t, start_core, num_cores) };
        if result == 0 {
            debug!(
                "Pinned thread {} to cores {} to {}",
                pid,
                start_core,
                start_core + num_cores - 1
            );
            Ok(())
        } else {
            Err(CpuPinningError::PinFailed(format!(
                "pin_thread_range failed with code {}",
                result
            )))
        }
    }

    pub fn get_available_cores(&self) -> Result<Vec<u32>, CpuPinningError> {
        if !self.native_available {
            let count = num_cpus::get();
            return Ok((0..count as u32).collect());
        }

        let mut buffer = vec![0u32; 256];
        let count = unsafe { get_available_cores(buffer.as_mut_ptr(), buffer.len()) };
        if count > 0 {
            Ok(buffer[..count as usize].to_vec())
        } else {
            Err(CpuPinningError::FfiError(
                "get_available_cores failed".to_string(),
            ))
        }
    }

    pub fn is_core_online(&self, core: u32) -> bool {
        if self.native_available {
            unsafe { is_core_online(core) }
        } else {
            core < num_cpus::get() as u32
        }
    }

    pub fn pin_thread_to_numa_node(&self, node: i32) -> Result<(), CpuPinningError> {
        if !self.native_available {
            return Err(CpuPinningError::PinFailed(
                "Native tools not available".to_string(),
            ));
        }

        let result = unsafe { pin_thread_to_numa_node(node) };
        if result == 0 {
            debug!("Pinned current thread to NUMA node {}", node);
            Ok(())
        } else {
            Err(CpuPinningError::PinFailed(format!(
                "pin_thread_to_numa_node failed with code {}",
                result
            )))
        }
    }

    pub fn is_available(&self) -> bool {
        self.native_available
    }

    fn pin_thread_native(&self, pid: u32, core_mask: u64) -> Result<(), CpuPinningError> {
        use std::process::Command;

        let cores: Vec<String> = (0..64)
            .filter(|i| (core_mask >> i) & 1 == 1)
            .map(|i| i.to_string())
            .collect();

        if cores.is_empty() {
            return Err(CpuPinningError::InvalidParam(
                "No cores specified in mask".to_string(),
            ));
        }

        let mask_str = if cores.len() == 1 {
            cores[0].clone()
        } else {
            format!("{}", core_mask)
        };

        let output = Command::new("taskset")
            .args(["-c", "-p", &mask_str, &pid.to_string()])
            .output()
            .map_err(|e| CpuPinningError::PinFailed(e.to_string()))?;

        if output.status.success() {
            debug!("Pinned thread {} to cores {} (native)", pid, mask_str);
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(CpuPinningError::PinFailed(format!(
                "taskset failed: {}",
                stderr
            )))
        }
    }

    fn get_affinity_native(&self, pid: u32) -> Result<u64, CpuPinningError> {
        use std::process::Command;

        let output = Command::new("taskset")
            .args(["-c", "-p", &pid.to_string()])
            .output()
            .map_err(|e| CpuPinningError::FfiError(e.to_string()))?;

        if !output.status.success() {
            return Err(CpuPinningError::FfiError("taskset failed".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.split_whitespace().collect();

        if parts.len() < 4 {
            return Err(CpuPinningError::FfiError(
                "Unexpected taskset output".to_string(),
            ));
        }

        let mask_str = match parts.last() {
            Some(s) => s,
            None => {
                return Err(CpuPinningError::FfiError(
                    "Unexpected empty taskset output".to_string(),
                ));
            }
        };
        let mask = u64::from_str_radix(mask_str.trim_start_matches("0x"), 16)
            .map_err(|e| CpuPinningError::FfiError(format!("Failed to parse mask: {}", e)))?;

        Ok(mask)
    }
}

impl Default for HostCpuPinning {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() -> anyhow::Result<()> {
        let pinning = HostCpuPinning::default();
        let _ = pinning.is_available();
        Ok(())
    }

    #[test]
    fn test_pin_current_thread() -> anyhow::Result<()> {
        let pinning = HostCpuPinning::default();
        let cpu_count = pinning.get_cpu_count();

        if cpu_count <= 1 {
            tracing::info!("Skipping test - only 1 CPU available");
            return Ok(());
        }

        if !pinning.is_available() {
            tracing::info!("Skipping test - Zig bindings not available");
            return Ok(());
        }

        let core = 0;
        pinning.pin_current_thread_to_core(core)?;

        let current = pinning.get_current_cpu()?;
        tracing::info!("Current CPU after pin: {}", current);

        Ok(())
    }

    #[test]
    fn test_get_affinity() -> anyhow::Result<()> {
        let pinning = HostCpuPinning::default();

        let pid = std::process::id();
        let mask = pinning.get_current_affinity(pid)?;

        tracing::info!("Current affinity for pid {}: {:#x}", pid, mask);
        assert!(mask != 0, "Affinity mask should not be zero");

        Ok(())
    }

    #[test]
    fn test_pin_to_core_range() -> anyhow::Result<()> {
        let pinning = HostCpuPinning::default();
        let cpu_count = pinning.get_cpu_count();

        if cpu_count <= 2 {
            tracing::info!("Skipping test - need at least 3 CPUs");
            return Ok(());
        }

        if !pinning.is_available() {
            tracing::info!("Skipping test - Zig bindings not available");
            return Ok(());
        }

        let pid = std::process::id();
        pinning.pin_thread_range(pid, 0, 2)?;

        let mask = pinning.get_current_affinity(pid)?;
        tracing::info!("Affinity after range pin: {:#x}", mask);

        Ok(())
    }
}
