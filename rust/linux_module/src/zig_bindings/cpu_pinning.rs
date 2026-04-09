//! Wrapper gọi Zig `pin_thread_to_core`.
//! Phase 3, Section 3.4.5: linux_cpu_pinning

use anyhow::{anyhow, Result};
use std::panic::{self, AssertUnwindSafe};

extern "C" {
    #[link_name = "pin_thread_to_core"]
    fn ffi_pin_thread_to_core(pid: u32, core_mask: u64) -> i32;
    #[link_name = "pin_current_thread"]
    fn ffi_pin_current_thread(core: u32) -> i32;
    #[link_name = "get_thread_affinity"]
    fn ffi_get_thread_affinity(pid: u32, core_mask: *mut u64) -> i32;
    #[link_name = "unpin_thread"]
    fn ffi_unpin_thread(pid: u32) -> i32;
    #[link_name = "pin_thread_range"]
    fn ffi_pin_thread_range(pid: u32, start_core: u32, num_cores: u32) -> i32;
    #[link_name = "pin_thread_to_numa_node"]
    fn ffi_pin_thread_to_numa_node(node: u32) -> i32;
    #[link_name = "get_cpu_count"]
    fn ffi_get_cpu_count() -> u32;
    #[link_name = "get_current_cpu"]
    fn ffi_get_current_cpu() -> i32;
    #[link_name = "get_available_cores"]
    fn ffi_get_available_cores(mask: *mut u64) -> i32;
    #[link_name = "is_core_online"]
    fn ffi_is_core_online(core: u32) -> i32;
    #[link_name = "get_numa_node"]
    fn ffi_get_numa_node(core: u32) -> i32;
    #[link_name = "get_core_frequency"]
    fn ffi_get_core_frequency(core: u32) -> u64;
    #[link_name = "get_cpu_usage"]
    fn ffi_get_cpu_usage() -> f64;
}


/// Pin thread vào CPU core bằng bitmask.
pub fn pin_thread(pid: u32, core_mask: u64) -> Result<()> {
	// SAFETY: The FFI function calls sched_setaffinity with a valid pid and core_mask.
    // No memory safety concerns - it only sets CPU affinity.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_pin_thread_to_core(pid, core_mask)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_pin_thread_to_core")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to pin thread, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Pin thread hiện tại vào một core cụ thể.
pub fn pin_current(core: u32) -> Result<()> {
    // SAFETY: The FFI function calls sched_setaffinity with the current thread's pid.
    // core is a valid CPU core index.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { ffi_pin_current_thread(core) }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_pin_current_thread")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to pin current thread, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Đọc affinity hiện tại của thread.
pub fn get_affinity(pid: u32) -> Result<u64> {
    let mut mask: u64 = 0;
    
	// SAFETY: &mut mask is a valid mutable pointer to a u64. The FFI function writes the core mask.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_get_thread_affinity(pid, &mut mask)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_get_thread_affinity")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to get thread affinity, code: {}", ret))
    } else {
        Ok(mask)
    }
}

/// Bỏ pin thread (restore all-CPU affinity).
pub fn unpin(pid: u32) -> Result<()> {
    // SAFETY: The FFI function resets CPU affinity to all cores for the given pid.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { ffi_unpin_thread(pid) }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_unpin_thread")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to unpin thread, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Pin thread vào một khoảng cores liên tiếp.
pub fn pin_range(pid: u32, start_core: u32, num_cores: u32) -> Result<()> {
    
	// SAFETY: The FFI function sets CPU affinity to a range of cores [start_core, start_core+num_cores).
    // pid is a valid process ID, start_core and num_cores are valid core indices.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_pin_thread_range(pid, start_core, num_cores)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_pin_thread_range")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to pin thread range, code: {}", ret))
    } else {
        Ok(())
    }
}


/// Pin thread vào NUMA node.
pub fn pin_numa(_pid: u32, node: u32) -> Result<()> {
	// SAFETY: The FFI function sets CPU affinity to all cores in the given NUMA node.
    // node is a valid NUMA node index.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_pin_thread_to_numa_node(node)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_pin_thread_to_numa_node")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to pin to NUMA node, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Lấy số lượng CPU cores.
pub fn cpu_count() -> u32 {
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { ffi_get_cpu_count() }));
    match result {
        Ok(val) => val,
        Err(_) => {
            tracing::error!("Panic in ffi_get_cpu_count, returning 0");
            0
        }
    }
}

/// Lấy CPU hiện tại đang chạy thread này.
pub fn current_cpu() -> i32 {
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { ffi_get_current_cpu() }));
    match result {
        Ok(val) => val,
        Err(_) => {
            tracing::error!("Panic in ffi_get_current_cpu, returning -1");
            -1
        }
    }
}

/// Lấy mask của các cores available.
pub fn available_cores() -> Result<u64> {
    let mut mask: u64 = 0;
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_get_available_cores(&mut mask)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_get_available_cores")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to get available cores, code: {}", ret))
    } else {
        Ok(mask)
    }
}

/// Kiểm tra core có online không.
pub fn is_online(core: u32) -> bool {
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { ffi_is_core_online(core) }));
    match result {
        Ok(val) => val != 0,
        Err(_) => {
            tracing::error!("Panic in ffi_is_core_online, returning false");
            false
        }
    }
}

/// Lấy NUMA node của core.
pub fn numa_node(core: u32) -> i32 {
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { ffi_get_numa_node(core) }));
    match result {
        Ok(val) => val,
        Err(_) => {
            tracing::error!("Panic in ffi_get_numa_node, returning -1");
            -1
        }
    }
}

/// Lấy tần số core (MHz).
pub fn core_frequency(core: u32) -> u64 {
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { ffi_get_core_frequency(core) }));
    match result {
        Ok(val) => val,
        Err(_) => {
            tracing::error!("Panic in ffi_get_core_frequency, returning 0");
            0
        }
    }
}

/// Lấy CPU usage hiện tại (%).
pub fn cpu_usage() -> f64 {
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { ffi_get_cpu_usage() }));
    match result {
        Ok(val) => val,
        Err(_) => {
            tracing::error!("Panic in ffi_get_cpu_usage, returning 0.0");
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_count() {
        let count = cpu_count();
        assert!(count > 0);
    }

    #[test]
    fn test_current_cpu() {
        let cpu = current_cpu();
        assert!(cpu >= 0);
    }
}
