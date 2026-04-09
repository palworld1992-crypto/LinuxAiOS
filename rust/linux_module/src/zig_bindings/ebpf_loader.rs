//! Wrapper an toàn gọi `ebpf_load_program` từ Zig.
//! Phase 3, Section 3.4.5: linux_ebpf_loader

use anyhow::{anyhow, Result};
use std::ffi::CString;
use std::panic::{self, AssertUnwindSafe};

extern "C" {
    #[link_name = "ebpf_load_program"]
    fn ffi_ebpf_load_program(prog_path: *const libc::c_char, prog_type: u32) -> i32;
    #[link_name = "ebpf_create_sockmap"]
    fn ffi_ebpf_create_sockmap(max_entries: u32) -> i32;
    #[link_name = "ebpf_create_hash_map"]
    fn ffi_ebpf_create_hash_map(max_entries: u32) -> i32;
    #[link_name = "ebpf_update_map_elem"]
    fn ffi_ebpf_update_map_elem(map_fd: i32, key: *const u8, value: *const u8) -> i32;
    #[link_name = "ebpf_lookup_map_elem"]
    fn ffi_ebpf_lookup_map_elem(map_fd: i32, key: *const u8, value: *mut u8) -> i32;
    #[link_name = "ebpf_delete_map_elem"]
    fn ffi_ebpf_delete_map_elem(map_fd: i32, key: *const u8) -> i32;
    #[link_name = "ebpf_attach_sockmap"]
    fn ffi_ebpf_attach_sockmap(map_fd: i32, prog_fd: i32) -> i32;
    #[link_name = "ebpf_is_supported"]
    fn ffi_ebpf_is_supported() -> i32;

    #[link_name = "zig_compress_and_store"]
    fn ffi_zig_compress_and_store(
        pid: u32,
        addr: u64,
        len: usize,
        path: *const libc::c_char,
    ) -> i32;
    #[link_name = "zig_load_coldpage_program"]
    fn ffi_zig_load_coldpage_program(obj_path: *const libc::c_char) -> i32;
    #[link_name = "zig_attach_coldpage_program"]
    fn ffi_zig_attach_coldpage_program(prog_fd: i32) -> i32;
    #[link_name = "zig_init_ipc_router"]
    fn ffi_zig_init_ipc_router(prog_path: *const libc::c_char) -> i32;
    #[link_name = "zig_update_route"]
    fn ffi_zig_update_route(map_fd: i32, src_peer: u64, dst_sock: u32) -> i32;
    #[link_name = "zig_remove_route"]
    fn ffi_zig_remove_route(map_fd: i32, src_peer: u64) -> i32;
    #[link_name = "zig_set_sockmap_prog"]
    fn ffi_zig_set_sockmap_prog(map_fd: i32, prog_fd: i32) -> i32;
    #[link_name = "zig_cgroup_freeze"]
    fn ffi_zig_cgroup_freeze(path: *const libc::c_char) -> i32;
    #[link_name = "zig_cgroup_thaw"]
    fn ffi_zig_cgroup_thaw(path: *const libc::c_char) -> i32;
}

/// Load eBPF program từ file ELF.
/// Trả về program fd nếu thành công, ngược lại lỗi.
pub fn load_ebpf_program(prog_path: &str, prog_type: u32) -> Result<i32> {
    let c_path = CString::new(prog_path).map_err(|e| anyhow!("Invalid path: {}", e))?;
    // SAFETY: c_path.as_ptr() points to a valid null-terminated C string.
    // The FFI function is defined in Zig and uses callconv(.C).
    // prog_type is a valid eBPF program type enum value.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_ebpf_load_program(c_path.as_ptr(), prog_type)
    }));
    let fd = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_ebpf_load_program")),
    };
    if fd < 0 {
        Err(anyhow!("Failed to load eBPF program, code: {}", fd))
    } else {
        Ok(fd)
    }
}

/// Tạo sockmap với số lượng entry tối đa.
pub fn create_sockmap(max_entries: u32) -> Result<i32> {
    // SAFETY: max_entries is a valid u32 count. The FFI function creates a BPF_MAP_TYPE_SOCKMAP.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_ebpf_create_sockmap(max_entries)
    }));
    let fd = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_ebpf_create_sockmap")),
    };
    if fd < 0 {
        Err(anyhow!("Failed to create sockmap, code: {}", fd))
    } else {
        Ok(fd)
    }
}

/// Tạo hash map với số lượng entry tối đa.
pub fn create_hash_map(max_entries: u32) -> Result<i32> {
    // SAFETY: max_entries is a valid u32 count. The FFI function creates a BPF_MAP_TYPE_HASH.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_ebpf_create_hash_map(max_entries)
    }));
    let fd = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_ebpf_create_hash_map")),
    };
    if fd < 0 {
        Err(anyhow!("Failed to create hash map, code: {}", fd))
    } else {
        Ok(fd)
    }
}

/// Cập nhật phần tử trong eBPF map.
pub fn update_map_elem(map_fd: i32, key: &[u8], value: &[u8]) -> Result<()> {
    // SAFETY: key.as_ptr() and value.as_ptr() point to valid slices of length key.len() and value.len().
    // map_fd is a valid eBPF map file descriptor.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_ebpf_update_map_elem(map_fd, key.as_ptr(), value.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_ebpf_update_map_elem")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to update map element, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Tra cứu phần tử từ eBPF map.
pub fn lookup_map_elem(map_fd: i32, key: &[u8], value: &mut [u8]) -> Result<()> {
    // SAFETY: key.as_ptr() points to a valid key slice. value.as_mut_ptr() points to a mutable buffer
    // of at least value.len() bytes, which the FFI function will write into.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_ebpf_lookup_map_elem(map_fd, key.as_ptr(), value.as_mut_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_ebpf_lookup_map_elem")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to lookup map element, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Xóa phần tử khỏi eBPF map.
pub fn delete_map_elem(map_fd: i32, key: &[u8]) -> Result<()> {
    // SAFETY: key.as_ptr() points to a valid key slice. map_fd is a valid eBPF map file descriptor.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_ebpf_delete_map_elem(map_fd, key.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_ebpf_delete_map_elem")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to delete map element, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Attach sockmap program.
pub fn attach_sockmap(map_fd: i32, prog_fd: i32) -> Result<()> {
    // SAFETY: map_fd and prog_fd are valid eBPF file descriptors.
    // The FFI function attaches the program to the sockmap.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_ebpf_attach_sockmap(map_fd, prog_fd)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_ebpf_attach_sockmap")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to attach sockmap, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Kiểm tra xem kernel có hỗ trợ eBPF không.
pub fn is_ebpf_supported() -> bool {
    // SAFETY: The FFI function queries kernel capabilities and returns a boolean.
    // No memory safety concerns - it only reads kernel state.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { ffi_ebpf_is_supported() }));
    match result {
        Ok(val) => val != 0,
        Err(_) => {
            tracing::error!("Panic in ffi_ebpf_is_supported, returning false");
            false
        }
    }
}

// ========== Legacy Zig bindings (backward compatibility) ==========

/// Compress and store a cold page.
pub fn compress_and_store(pid: u32, addr: u64, len: usize, path: &std::ffi::CStr) -> Result<()> {
    // SAFETY: path.as_ptr() points to a valid null-terminated C string.
    // pid, addr, len describe a valid memory region to compress.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_zig_compress_and_store(pid, addr, len, path.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_zig_compress_and_store")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to compress and store, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Load coldpage eBPF program (legacy).
pub fn load_coldpage_program(obj_path: &std::ffi::CStr) -> Result<i32> {
    // SAFETY: obj_path.as_ptr() points to a valid null-terminated C string
    // containing the path to an eBPF ELF object file.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_zig_load_coldpage_program(obj_path.as_ptr())
    }));
    let fd = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_zig_load_coldpage_program")),
    };
    if fd < 0 {
        Err(anyhow!("Failed to load coldpage program, code: {}", fd))
    } else {
        Ok(fd)
    }
}

/// Attach coldpage eBPF program (legacy).
pub fn attach_coldpage_program(prog_fd: i32) -> Result<()> {
    // SAFETY: prog_fd is a valid eBPF program file descriptor.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_zig_attach_coldpage_program(prog_fd)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_zig_attach_coldpage_program")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to attach coldpage program, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Initialize IPC router (legacy).
pub fn init_ipc_router(prog_path: &std::ffi::CStr) -> Result<i32> {
    // SAFETY: prog_path.as_ptr() points to a valid null-terminated C string.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_zig_init_ipc_router(prog_path.as_ptr())
    }));
    let fd = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_zig_init_ipc_router")),
    };
    if fd < 0 {
        Err(anyhow!("Failed to init IPC router, code: {}", fd))
    } else {
        Ok(fd)
    }
}

/// Update route in eBPF map (legacy).
pub fn update_route(map_fd: i32, src_peer: u64, dst_sock: u32) -> Result<()> {
    // SAFETY: map_fd is a valid eBPF map file descriptor. src_peer and dst_sock are valid route values.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_zig_update_route(map_fd, src_peer, dst_sock)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_zig_update_route")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to update route, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Remove route from eBPF map (legacy).
pub fn remove_route(map_fd: i32, src_peer: u64) -> Result<()> {
    // SAFETY: map_fd is a valid eBPF map file descriptor. src_peer is a valid route key.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_zig_remove_route(map_fd, src_peer)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_zig_remove_route")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to remove route, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Set sockmap program (legacy).
pub fn set_sockmap_prog(map_fd: i32, prog_fd: i32) -> Result<()> {
    // SAFETY: map_fd and prog_fd are valid eBPF file descriptors.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_zig_set_sockmap_prog(map_fd, prog_fd)
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_zig_set_sockmap_prog")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to set sockmap prog, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Freeze a cgroup (legacy).
pub fn cgroup_freeze(path: &std::ffi::CStr) -> Result<()> {
    // SAFETY: path.as_ptr() points to a valid null-terminated C string representing cgroup path.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_zig_cgroup_freeze(path.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_zig_cgroup_freeze")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to freeze cgroup, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Thaw a cgroup (legacy).
pub fn cgroup_thaw(path: &std::ffi::CStr) -> Result<()> {
    // SAFETY: path.as_ptr() points to a valid null-terminated C string representing cgroup path.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        ffi_zig_cgroup_thaw(path.as_ptr())
    }));
    let ret = match result {
        Ok(val) => val,
        Err(_) => return Err(anyhow!("Panic in ffi_zig_cgroup_thaw")),
    };
    if ret < 0 {
        Err(anyhow!("Failed to thaw cgroup, code: {}", ret))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_support_check() {
        // This will return false if Zig lib is not linked (expected in unit tests)
        let _ = is_ebpf_supported();
    }

    #[test]
    fn test_invalid_path() {
        let result = load_ebpf_program("/nonexistent/path\0", 0);
        assert!(result.is_err());
    }
}
