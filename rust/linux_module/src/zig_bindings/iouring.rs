//! Wrapper gọi Zig io_uring wrapper (setup ring, submit SQEs, registered buffers).
//! Phase 3, Section 3.4.5: linux_iouring
//! Sử dụng libloading để load Zig library an toàn, không panic khi chưa implement.

use anyhow::{anyhow, Result};
use libloading::{Library, Symbol};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;

static ZIG_LIBRARY_LOADED: AtomicBool = AtomicBool::new(false);
static IOURING_FNS: OnceLock<IouringFns> = OnceLock::new();

#[repr(C)]
#[derive(Debug)]
pub struct IoUringHandle {
    pub ring_fd: i32,
    pub entries: u32,
    pub enabled: bool,
}

type FfiInitFn = unsafe extern "C" fn(ring: *mut IoUringHandle, entries: u32) -> i32;
type FfiSubmitReadFn = unsafe extern "C" fn(
    ring_fd: i32,
    fd: i32,
    offset: u64,
    buf: *mut u8,
    len: usize,
    user_data: u64,
) -> i32;
type FfiSubmitWriteFn = unsafe extern "C" fn(
    ring_fd: i32,
    fd: i32,
    offset: u64,
    buf: *const u8,
    len: usize,
    user_data: u64,
) -> i32;
type FfiSubmitOpenatFn = unsafe extern "C" fn(
    ring_fd: i32,
    dirfd: i32,
    pathname: *const libc::c_char,
    flags: i32,
    mode: u32,
    user_data: u64,
) -> i32;
type FfiRegisterBuffersFn =
    unsafe extern "C" fn(ring_fd: i32, buffers: *const libc::c_void, nr: u32) -> i32;
type FfiCloseFn = unsafe extern "C" fn(ring_fd: i32) -> i32;
type FfiWaitCqesFn = unsafe extern "C" fn(ring_fd: i32, wait_nr: u32) -> i32;
type FfiPeekCqeFn = unsafe extern "C" fn(ring_fd: i32) -> i32;
type FfiEnableRingFn = unsafe extern "C" fn(ring_fd: i32) -> i32;
type FfiDisableRingFn = unsafe extern "C" fn(ring_fd: i32) -> i32;

struct IouringFns {
    init: FfiInitFn,
    submit_read: FfiSubmitReadFn,
    submit_write: FfiSubmitWriteFn,
    submit_openat: FfiSubmitOpenatFn,
    register_buffers: FfiRegisterBuffersFn,
    close: FfiCloseFn,
    wait_cqes: FfiWaitCqesFn,
    peek_cqe: FfiPeekCqeFn,
    enable_ring: FfiEnableRingFn,
    disable_ring: FfiDisableRingFn,
}

impl IouringFns {
    fn load(library_path: &str) -> anyhow::Result<Self> {
        // SAFETY: Loading a shared library from the given path. The library must be a valid
        // ELF shared object with the expected C ABI symbols. We trust the library path
        // because it's configured by the system administrator.
        let lib = unsafe { Library::new(library_path) }
            .map_err(|e| anyhow!("Failed to load io_uring library: {}", e))?;

        // SAFETY: All symbols are loaded from the validated library. Each function pointer
        // has a known C ABI signature defined in the extern "C" block above.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            let init: Symbol<FfiInitFn> = lib
                .get(b"iouring_init")
                .map_err(|_| anyhow!("Symbol iouring_init not found"))?;
            let submit_read: Symbol<FfiSubmitReadFn> = lib
                .get(b"iouring_submit_read")
                .map_err(|_| anyhow!("Symbol iouring_submit_read not found"))?;
            let submit_write: Symbol<FfiSubmitWriteFn> = lib
                .get(b"iouring_submit_write")
                .map_err(|_| anyhow!("Symbol iouring_submit_write not found"))?;
            let submit_openat: Symbol<FfiSubmitOpenatFn> = lib
                .get(b"iouring_submit_openat")
                .map_err(|_| anyhow!("Symbol iouring_submit_openat not found"))?;
            let register_buffers: Symbol<FfiRegisterBuffersFn> = lib
                .get(b"iouring_register_buffers")
                .map_err(|_| anyhow!("Symbol iouring_register_buffers not found"))?;
            let close: Symbol<FfiCloseFn> = lib
                .get(b"iouring_close")
                .map_err(|_| anyhow!("Symbol iouring_close not found"))?;
            let wait_cqes: Symbol<FfiWaitCqesFn> = lib
                .get(b"iouring_wait_cqes")
                .map_err(|_| anyhow!("Symbol iouring_wait_cqes not found"))?;
            let peek_cqe: Symbol<FfiPeekCqeFn> = lib
                .get(b"iouring_peek_cqe")
                .map_err(|_| anyhow!("Symbol iouring_peek_cqe not found"))?;
            let enable_ring: Symbol<FfiEnableRingFn> = lib
                .get(b"iouring_enable_ring")
                .map_err(|_| anyhow!("Symbol iouring_enable_ring not found"))?;
            let disable_ring: Symbol<FfiDisableRingFn> = lib
                .get(b"iouring_disable_ring")
                .map_err(|_| anyhow!("Symbol iouring_disable_ring not found"))?;

            Ok(Self {
                init: *init,
                submit_read: *submit_read,
                submit_write: *submit_write,
                submit_openat: *submit_openat,
                register_buffers: *register_buffers,
                close: *close,
                wait_cqes: *wait_cqes,
                peek_cqe: *peek_cqe,
                enable_ring: *enable_ring,
                disable_ring: *disable_ring,
            })
        }));
        match result {
            Ok(Ok(fns)) => Ok(fns),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow!("Panic during symbol loading")),
        }
    }
}

pub fn is_iouring_available() -> bool {
    ZIG_LIBRARY_LOADED.load(Ordering::Relaxed)
}

pub fn try_load_iouring(library_path: &str) -> Result<()> {
    match IouringFns::load(library_path) {
        Ok(fns) => {
            if IOURING_FNS.set(fns).is_err() {
                tracing::warn!("io_uring functions already initialized");
            }
            ZIG_LIBRARY_LOADED.store(true, Ordering::Relaxed);
            tracing::info!("io_uring wrapper loaded successfully from {}", library_path);
            Ok(())
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load io_uring wrapper: {}. Using userspace fallback.",
                e
            );
            ZIG_LIBRARY_LOADED.store(false, Ordering::Relaxed);
            Err(anyhow!("io_uring wrapper failed to load: {}", e))
        }
    }
}

fn with_iouring_fns<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&IouringFns) -> Result<R>,
{
    match IOURING_FNS.get() {
        Some(fns) => f(fns),
        None => Err(anyhow!(
            "io_uring wrapper not loaded. Call try_load_iouring() first."
        )),
    }
}

pub fn iouring_init_ring(entries: u32) -> Result<IoUringHandle> {
    let mut ring = IoUringHandle {
        ring_fd: -1,
        entries,
        enabled: false,
    };

    if !is_iouring_available() {
        return Err(anyhow!("io_uring not available - Zig library not loaded"));
    }

    with_iouring_fns(|fns| {
        // SAFETY: ring is a valid mutable pointer to an IoUringHandle. entries is valid.
        // The FFI function initializes the io_uring ring buffer.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.init)(&mut ring, entries)
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in iouring_init")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to init io_uring, code: {}", ret))
        } else {
            ring.enabled = true;
            Ok(ring)
        }
    })
}

pub fn iouring_submit_read(
    ring: &IoUringHandle,
    fd: i32,
    offset: u64,
    buf: &mut [u8],
    user_data: u64,
) -> Result<()> {
    if !is_iouring_available() {
        return Err(anyhow!("io_uring not available"));
    }

    with_iouring_fns(|fns| {
        // SAFETY: ring.ring_fd is valid, buf.as_mut_ptr() points to valid mutable memory of length buf.len().
        // The FFI function submits a read SQE to the io_uring ring.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.submit_read)(
                ring.ring_fd,
                fd,
                offset,
                buf.as_mut_ptr(),
                buf.len(),
                user_data,
            )
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in iouring_submit_read")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to submit read, code: {}", ret))
        } else {
            Ok(())
        }
    })
}

pub fn iouring_submit_write(
    ring: &IoUringHandle,
    fd: i32,
    offset: u64,
    buf: &[u8],
    user_data: u64,
) -> Result<()> {
    if !is_iouring_available() {
        return Err(anyhow!("io_uring not available"));
    }

    with_iouring_fns(|fns| {
        // SAFETY: ring.ring_fd is valid, buf.as_ptr() points to valid const memory of length buf.len().
        // The FFI function submits a write SQE to the io_uring ring.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.submit_write)(ring.ring_fd, fd, offset, buf.as_ptr(), buf.len(), user_data)
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in iouring_submit_write")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to submit write, code: {}", ret))
        } else {
            Ok(())
        }
    })
}

pub fn iouring_submit_openat(
    ring: &IoUringHandle,
    dirfd: i32,
    pathname: &std::ffi::CStr,
    flags: i32,
    mode: u32,
    user_data: u64,
) -> Result<()> {
    if !is_iouring_available() {
        return Err(anyhow!("io_uring not available"));
    }

    with_iouring_fns(|fns| {
        // SAFETY: ring.ring_fd is valid, pathname.as_ptr() points to a valid null-terminated C string.
        // The FFI function submits an openat SQE to the io_uring ring.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.submit_openat)(
                ring.ring_fd,
                dirfd,
                pathname.as_ptr(),
                flags,
                mode,
                user_data,
            )
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in iouring_submit_openat")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to submit openat, code: {}", ret))
        } else {
            Ok(())
        }
    })
}

pub fn iouring_register_buffers(ring: &IoUringHandle, buffers: &[&[u8]]) -> Result<()> {
    if !is_iouring_available() {
        return Err(anyhow!("io_uring not available"));
    }

    with_iouring_fns(|fns| {
        let nr = buffers.len() as u32;
        // SAFETY: ring.ring_fd is valid, buffers.as_ptr() points to an array of valid slices.
        // The FFI function registers these buffers with the io_uring ring.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.register_buffers)(ring.ring_fd, buffers.as_ptr() as *const _, nr)
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in iouring_register_buffers")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to register buffers, code: {}", ret))
        } else {
            Ok(())
        }
    })
}

pub fn iouring_close_ring(ring: &IoUringHandle) -> Result<()> {
    if !is_iouring_available() {
        return Err(anyhow!("io_uring not available"));
    }

    with_iouring_fns(|fns| {
        // SAFETY: ring.ring_fd is a valid file descriptor for an io_uring ring.
        // The FFI function closes the ring and frees resources.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { (fns.close)(ring.ring_fd) }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in iouring_close")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to close io_uring, code: {}", ret))
        } else {
            Ok(())
        }
    })
}

pub fn iouring_wait_cqes(ring: &IoUringHandle, wait_nr: u32) -> Result<()> {
    if !is_iouring_available() {
        return Err(anyhow!("io_uring not available"));
    }

    with_iouring_fns(|fns| {
        // SAFETY: ring.ring_fd is valid. The FFI function waits for completion queue events.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.wait_cqes)(ring.ring_fd, wait_nr)
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in iouring_wait_cqes")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to wait CQEs, code: {}", ret))
        } else {
            Ok(())
        }
    })
}

pub fn iouring_peek_cqe(ring: &IoUringHandle) -> bool {
    if !is_iouring_available() {
        return false;
    }
    match with_iouring_fns(|fns| {
        // SAFETY: ring.ring_fd is valid. The FFI function peeks at the completion queue.
        let result =
            panic::catch_unwind(AssertUnwindSafe(|| unsafe { (fns.peek_cqe)(ring.ring_fd) }));
        match result {
            Ok(val) => Ok(val >= 0),
            Err(_) => Err(anyhow!("Panic in iouring_peek_cqe")),
        }
    }) {
        Ok(available) => available,
        Err(e) => {
            tracing::warn!("Failed to peek CQE: {}", e);
            false
        }
    }
}

pub fn iouring_enable(ring: &mut IoUringHandle) -> Result<()> {
    if !is_iouring_available() {
        return Err(anyhow!("io_uring not available"));
    }

    with_iouring_fns(|fns| {
        // SAFETY: ring is a valid mutable reference, ring.ring_fd is a valid ring file descriptor.
        // The FFI function enables the ring for operation.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.enable_ring)(ring.ring_fd)
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in iouring_enable")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to enable ring, code: {}", ret))
        } else {
            ring.enabled = true;
            Ok(())
        }
    })
}

pub fn iouring_disable(ring: &mut IoUringHandle) -> Result<()> {
    if !is_iouring_available() {
        return Err(anyhow!("io_uring not available"));
    }

    with_iouring_fns(|fns| {
        // SAFETY: ring is a valid mutable reference, ring.ring_fd is a valid ring file descriptor.
        // The FFI function disables the ring.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.disable_ring)(ring.ring_fd)
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in iouring_disable")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to disable ring, code: {}", ret))
        } else {
            ring.enabled = false;
            Ok(())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_iouring_handle_struct() {
        let handle = IoUringHandle {
            ring_fd: -1,
            entries: 256,
            enabled: false,
        };
        assert_eq!(handle.entries, 256);
    }

    #[test]
    fn test_iouring_not_available() {
        assert!(!is_iouring_available());
        let result = iouring_init_ring(256);
        assert!(result.is_err());
    }
}
