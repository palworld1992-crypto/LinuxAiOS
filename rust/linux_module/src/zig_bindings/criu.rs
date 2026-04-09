//! Wrapper gọi Zig CRIU (checkpoint/restore).
//! Phase 3, Section 3.4.5: linux_criu

use anyhow::{anyhow, Result};
use std::ffi::CString;
use std::panic::{self, AssertUnwindSafe};

extern "C" {
    #[link_name = "criu_checkpoint"]
    fn ffi_criu_checkpoint(pid: u32, images_dir: *const libc::c_char) -> i32;
    #[link_name = "criu_restore"]
    fn ffi_criu_restore(images_dir: *const libc::c_char) -> i32;
    #[link_name = "criu_pre_dump"]
    fn ffi_criu_pre_dump(pid: u32, images_dir: *const libc::c_char) -> i32;
    #[link_name = "criu_check"]
    fn ffi_criu_check() -> i32;
    #[link_name = "criu_page_server"]
    fn ffi_criu_page_server(pid: u32, images_dir: *const libc::c_char, port: u32) -> i32;
    #[link_name = "criu_dump_tree"]
    fn ffi_criu_dump_tree(pid: u32, images_dir: *const libc::c_char) -> i32;
    #[link_name = "criu_restore_tree"]
    fn ffi_criu_restore_tree(images_dir: *const libc::c_char, pid: u32) -> i32;
    #[link_name = "criu_images_exist"]
    fn ffi_criu_images_exist(images_dir: *const libc::c_char) -> i32;
}

/// Checkpoint process bằng CRIU.
pub fn checkpoint(pid: u32, images_dir: &str) -> Result<()> {
    let c_dir = CString::new(images_dir).map_err(|e| anyhow!("Invalid images dir: {}", e))?;
    // SAFETY: c_dir.as_ptr() points to a valid null-terminated C string.
    // pid is a valid process ID. The FFI function invokes CRIU checkpoint.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_criu_checkpoint(pid, c_dir.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_criu_checkpoint")),
    };
    if ret < 0 {
        Err(anyhow!("CRIU checkpoint failed, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Restore process từ CRIU images.
pub fn restore(images_dir: &str) -> Result<i32> {
    let c_dir = CString::new(images_dir).map_err(|e| anyhow!("Invalid images dir: {}", e))?;
    // SAFETY: c_dir.as_ptr() points to a valid null-terminated C string.
    // The FFI function invokes CRIU restore from the images directory.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_criu_restore(c_dir.as_ptr())
    }));
    match result {
        Ok(val) => {
            if val < 0 {
                Err(anyhow!("CRIU restore failed, code: {}", val))
            } else {
                Ok(val)
            }
        }
        Err(_) => Err(anyhow!("Panic in ffi_criu_restore")),
    }
}

/// Pre-dump iterative (giảm downtime).
pub fn pre_dump(pid: u32, images_dir: &str) -> Result<()> {
    let c_dir = CString::new(images_dir).map_err(|e| anyhow!("Invalid images dir: {}", e))?;
    // SAFETY: c_dir.as_ptr() points to a valid null-terminated C string.
    // pid is a valid process ID. The FFI function invokes CRIU pre-dump.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_criu_pre_dump(pid, c_dir.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_criu_pre_dump")),
    };
    if ret < 0 {
        Err(anyhow!("CRIU pre-dump failed, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Kiểm tra CRIU có sẵn không.
pub fn criu_available() -> bool {
    // SAFETY: The FFI function checks CRIU availability. No memory safety concerns.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { ffi_criu_check() }));
    match result {
        Ok(val) => val == 0,
        Err(_) => false,
    }
}

/// Checkpoint với page server (tối ưu cho VM lớn).
pub fn checkpoint_with_page_server(pid: u32, images_dir: &str, port: u32) -> Result<()> {
    let c_dir = CString::new(images_dir).map_err(|e| anyhow!("Invalid images dir: {}", e))?;
    // SAFETY: c_dir.as_ptr() points to a valid null-terminated C string.
    // pid is a valid process ID, port is a valid TCP port number.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_criu_page_server(pid, c_dir.as_ptr(), port)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_criu_page_server")),
    };
    if ret < 0 {
        Err(anyhow!("CRIU page server checkpoint failed, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Dump process tree.
pub fn dump_tree(pid: u32, images_dir: &str) -> Result<()> {
    let c_dir = CString::new(images_dir).map_err(|e| anyhow!("Invalid images dir: {}", e))?;
    // SAFETY: c_dir.as_ptr() points to a valid null-terminated C string.
    // pid is a valid process ID.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_criu_dump_tree(pid, c_dir.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_criu_dump_tree")),
    };
    if ret < 0 {
        Err(anyhow!("CRIU dump tree failed, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Restore process tree.
pub fn restore_tree(images_dir: &str, pid: u32) -> Result<()> {
    let c_dir = CString::new(images_dir).map_err(|e| anyhow!("Invalid images dir: {}", e))?;
    // SAFETY: c_dir.as_ptr() points to a valid null-terminated C string.
    // pid is a valid process ID.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_criu_restore_tree(c_dir.as_ptr(), pid)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_criu_restore_tree")),
    };
    if ret < 0 {
        Err(anyhow!("CRIU restore tree failed, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Kiểm tra images có tồn tại không.
pub fn images_exist(images_dir: &str) -> bool {
    let c_dir = match CString::new(images_dir) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_criu_images_exist(c_dir.as_ptr())
    }));
    match result {
        Ok(val) => val != 0,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_images_dir() {
        let result = checkpoint(1234, "invalid\0dir");
        assert!(result.is_err());
    }

    #[test]
    fn test_criu_available() {
        let _ = criu_available();
    }
}
