//! Wrapper gọi Zig cgroup manager (tạo, xóa, set limits).
//! Phase 3, Section 3.4.5: linux_cgroup

use anyhow::{anyhow, Result};
use std::ffi::CString;
use std::panic::{self, AssertUnwindSafe};

extern "C" {
    #[link_name = "cgroup_create"]
    fn ffi_cgroup_create(name: *const libc::c_char) -> i32;
    #[link_name = "cgroup_destroy"]
    fn ffi_cgroup_destroy(name: *const libc::c_char) -> i32;
    #[link_name = "cgroup_freeze"]
    fn ffi_cgroup_freeze(name: *const libc::c_char) -> i32;
    #[link_name = "cgroup_thaw"]
    fn ffi_cgroup_thaw(name: *const libc::c_char) -> i32;
    #[link_name = "cgroup_set_cpu_limit"]
    fn ffi_cgroup_set_cpu_limit(name: *const libc::c_char, quota: u64, period: u64) -> i32;
    #[link_name = "cgroup_set_memory_limit"]
    fn ffi_cgroup_set_memory_limit(name: *const libc::c_char, limit_bytes: u64) -> i32;
    #[link_name = "cgroup_set_io_limit"]
    fn ffi_cgroup_set_io_limit(
        name: *const libc::c_char,
        major: u32,
        minor: u32,
        rbps: u64,
        wbps: u64,
    ) -> i32;
    #[link_name = "cgroup_add_process"]
    fn ffi_cgroup_add_process(name: *const libc::c_char, pid: u32) -> i32;
    #[link_name = "cgroup_get_memory_usage"]
    fn ffi_cgroup_get_memory_usage(name: *const libc::c_char) -> u64;
    #[link_name = "cgroup_get_cpu_usage"]
    fn ffi_cgroup_get_cpu_usage(name: *const libc::c_char) -> u64;
}

/// Tạo cgroup mới với tên cho trước.
pub fn create_cgroup(name: &str) -> Result<()> {
    let c_name = CString::new(name).map_err(|e| anyhow!("Invalid cgroup name: {}", e))?;
    // SAFETY: c_name.as_ptr() points to a valid null-terminated C string.
    // The FFI function creates a cgroup v2 hierarchy with the given name.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_cgroup_create(c_name.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_cgroup_create")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to create cgroup, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Xóa cgroup.
pub fn destroy_cgroup(name: &str) -> Result<()> {
    let c_name = CString::new(name).map_err(|e| anyhow!("Invalid cgroup name: {}", e))?;
    // SAFETY: c_name.as_ptr() points to a valid null-terminated C string.
    // The FFI function destroys the cgroup hierarchy.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_cgroup_destroy(c_name.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_cgroup_destroy")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to destroy cgroup, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Đóng băng cgroup.
pub fn freeze_cgroup(name: &str) -> Result<()> {
    let c_name = CString::new(name).map_err(|e| anyhow!("Invalid cgroup name: {}", e))?;
    // SAFETY: c_name.as_ptr() points to a valid null-terminated C string.
    // The FFI function writes to cgroup.freeze to freeze all tasks.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_cgroup_freeze(c_name.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_cgroup_freeze")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to freeze cgroup, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Giải phóng cgroup.
pub fn thaw_cgroup(name: &str) -> Result<()> {
    let c_name = CString::new(name).map_err(|e| anyhow!("Invalid cgroup name: {}", e))?;
    // SAFETY: c_name.as_ptr() points to a valid null-terminated C string.
    // The FFI function writes to cgroup.freeze to thaw all tasks.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_cgroup_thaw(c_name.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_cgroup_thaw")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to thaw cgroup, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Set giới hạn CPU cho cgroup (quota/period tính bằng microsecond).
pub fn set_cpu_limit(name: &str, quota: u64, period: u64) -> Result<()> {
    let c_name = CString::new(name).map_err(|e| anyhow!("Invalid cgroup name: {}", e))?;
    // SAFETY: c_name.as_ptr() points to a valid null-terminated C string.
    // quota and period are valid u64 values for cgroup cpu.max.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_cgroup_set_cpu_limit(c_name.as_ptr(), quota, period)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_cgroup_set_cpu_limit")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to set CPU limit, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Set giới hạn bộ nhớ cho cgroup (byte).
pub fn set_memory_limit(name: &str, limit_bytes: u64) -> Result<()> {
    let c_name = CString::new(name).map_err(|e| anyhow!("Invalid cgroup name: {}", e))?;
    // SAFETY: c_name.as_ptr() points to a valid null-terminated C string.
    // limit_bytes is a valid u64 value for cgroup memory.max.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_cgroup_set_memory_limit(c_name.as_ptr(), limit_bytes)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_cgroup_set_memory_limit")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to set memory limit, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Set giới hạn I/O cho cgroup.
pub fn set_io_limit(name: &str, major: u32, minor: u32, rbps: u64, wbps: u64) -> Result<()> {
    let c_name = CString::new(name).map_err(|e| anyhow!("Invalid cgroup name: {}", e))?;
    // SAFETY: c_name.as_ptr() points to a valid null-terminated C string.
    // major/minor are valid device numbers, rbps/wbps are valid byte-per-second limits.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_cgroup_set_io_limit(c_name.as_ptr(), major, minor, rbps, wbps)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_cgroup_set_io_limit")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to set IO limit, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Thêm process vào cgroup.
pub fn add_process_to_cgroup(name: &str, pid: u32) -> Result<()> {
    let c_name = CString::new(name).map_err(|e| anyhow!("Invalid cgroup name: {}", e))?;
    // SAFETY: c_name.as_ptr() points to a valid null-terminated C string.
    // pid is a valid process ID. The FFI function writes pid to cgroup.procs.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_cgroup_add_process(c_name.as_ptr(), pid)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_cgroup_add_process")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to add process to cgroup, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Đọc mức sử dụng bộ nhớ của cgroup (byte).
pub fn get_cgroup_memory_usage(name: &str) -> Result<u64> {
    let c_name = CString::new(name).map_err(|e| anyhow!("Invalid cgroup name: {}", e))?;
    // SAFETY: c_name.as_ptr() points to a valid null-terminated C string.
    // The FFI function reads from the cgroup memory.current file and returns a u64.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_cgroup_get_memory_usage(c_name.as_ptr())
    }));
    match result {
        Ok(val) => Ok(val),
        Err(_) => Err(anyhow!("Panic in ffi_cgroup_get_memory_usage")),
    }
}

/// Đọc mức sử dụng CPU của cgroup (microsecond).
pub fn get_cgroup_cpu_usage(name: &str) -> Result<u64> {
    let c_name = CString::new(name).map_err(|e| anyhow!("Invalid cgroup name: {}", e))?;
    // SAFETY: c_name.as_ptr() points to a valid null-terminated C string.
    // The FFI function reads from the cgroup cpu.stat and returns a u64.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_cgroup_get_cpu_usage(c_name.as_ptr())
    }));
    match result {
        Ok(val) => Ok(val),
        Err(_) => Err(anyhow!("Panic in ffi_cgroup_get_cpu_usage")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_cgroup_name() {
        let result = create_cgroup("invalid\0name");
        assert!(result.is_err());
    }
}
