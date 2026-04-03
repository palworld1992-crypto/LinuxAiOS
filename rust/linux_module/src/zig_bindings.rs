//! FFI bindings to Zig functions.

use anyhow::{anyhow, Result};
use std::ffi::CStr;
use std::os::raw::c_char;

extern "C" {
    // eBPF cold page tracker
    pub fn zig_load_coldpage_program(obj_path: *const c_char) -> i32;
    pub fn zig_attach_coldpage_program(prog_fd: i32) -> i32;
    pub fn zig_compress_and_store(pid: u32, addr: u64, len: usize, path: *const c_char) -> i32;

    // eBPF IPC router
    pub fn zig_init_ipc_router(prog_path: *const c_char) -> i32;
    pub fn zig_update_route(map_fd: i32, src_peer: u64, dst_sock: u32) -> i32;
    pub fn zig_remove_route(map_fd: i32, src_peer: u64) -> i32;
    pub fn zig_set_sockmap_prog(map_fd: i32, prog_fd: i32) -> i32;

    // Cgroup control
    pub fn zig_cgroup_freeze(path: *const c_char) -> i32;
    pub fn zig_cgroup_thaw(path: *const c_char) -> i32;
}

/// Safe wrapper for loading cold page detection eBPF program.
pub fn load_coldpage_program(obj_path: &CStr) -> Result<i32> {
    let fd = unsafe { zig_load_coldpage_program(obj_path.as_ptr()) };
    if fd < 0 {
        Err(anyhow!("Failed to load coldpage program, code: {}", fd))
    } else {
        Ok(fd)
    }
}

/// Safe wrapper for attaching cold page detection eBPF program.
pub fn attach_coldpage_program(prog_fd: i32) -> Result<()> {
    let ret = unsafe { zig_attach_coldpage_program(prog_fd) };
    if ret < 0 {
        Err(anyhow!("Failed to attach coldpage program, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Safe wrapper for compressing and storing a cold page.
pub fn compress_and_store(pid: u32, addr: u64, len: usize, path: &CStr) -> Result<()> {
    let ret = unsafe { zig_compress_and_store(pid, addr, len, path.as_ptr()) };
    if ret < 0 {
        Err(anyhow!("Failed to compress and store, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Safe wrapper for initializing the eBPF IPC router.
pub fn init_ipc_router(prog_path: &CStr) -> Result<i32> {
    let fd = unsafe { zig_init_ipc_router(prog_path.as_ptr()) };
    if fd < 0 {
        Err(anyhow!("Failed to init IPC router, code: {}", fd))
    } else {
        Ok(fd)
    }
}

/// Safe wrapper to update a route in the eBPF map.
pub fn update_route(map_fd: i32, src_peer: u64, dst_sock: u32) -> Result<()> {
    let ret = unsafe { zig_update_route(map_fd, src_peer, dst_sock) };
    if ret < 0 {
        Err(anyhow!("Failed to update route, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Safe wrapper to remove a route.
pub fn remove_route(map_fd: i32, src_peer: u64) -> Result<()> {
    let ret = unsafe { zig_remove_route(map_fd, src_peer) };
    if ret < 0 {
        Err(anyhow!("Failed to remove route, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Safe wrapper to set sockmap program.
pub fn set_sockmap_prog(map_fd: i32, prog_fd: i32) -> Result<()> {
    let ret = unsafe { zig_set_sockmap_prog(map_fd, prog_fd) };
    if ret < 0 {
        Err(anyhow!("Failed to set sockmap prog, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Safe wrapper to freeze a cgroup.
pub fn cgroup_freeze(path: &CStr) -> Result<()> {
    let ret = unsafe { zig_cgroup_freeze(path.as_ptr()) };
    if ret < 0 {
        Err(anyhow!("Failed to freeze cgroup, code: {}", ret))
    } else {
        Ok(())
    }
}

/// Safe wrapper to thaw a cgroup.
pub fn cgroup_thaw(path: &CStr) -> Result<()> {
    let ret = unsafe { zig_cgroup_thaw(path.as_ptr()) };
    if ret < 0 {
        Err(anyhow!("Failed to thaw cgroup, code: {}", ret))
    } else {
        Ok(())
    }
}
