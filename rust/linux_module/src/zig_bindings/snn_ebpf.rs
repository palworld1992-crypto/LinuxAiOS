//! Wrapper gọi eBPF program cold_page_detector để nhận spike events
//! và chuyển sang SNN processor.
//! Phase 3, Section 3.4.5: linux_snn_ebpf
//! Sử dụng libloading để load eBPF loader an toàn, fallback dùng mincore khi eBPF không khả dụng.

use anyhow::{anyhow, Result};
use libloading::{Library, Symbol};
use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Instant;

static EBPF_LIBRARY_LOADED: AtomicBool = AtomicBool::new(false);
static EBPF_FNS: OnceLock<EbpfFns> = OnceLock::new();

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ColdPageEvent {
    pub pid: u32,
    pub addr: u64,
    pub timestamp: u64,
    pub access_count: u32,
}

type FfiInitFn = unsafe extern "C" fn(ring_buf_fd: i32, prog_fd: i32) -> i32;
type FfiReadEventsFn =
    unsafe extern "C" fn(ring_buf_fd: i32, events: *mut ColdPageEvent, max_events: u32) -> i32;
type FfiFallbackFn = unsafe extern "C" fn(
    addr: u64,
    len: usize,
    cold_pages: *mut ColdPageEvent,
    max_cold: u32,
) -> i32;
type FfiEnableFn = unsafe extern "C" fn() -> i32;
type FfiDisableFn = unsafe extern "C" fn() -> i32;
type FfiStatsFn = unsafe extern "C" fn() -> u64;

struct EbpfFns {
    init: FfiInitFn,
    read_events: FfiReadEventsFn,
    fallback: FfiFallbackFn,
    enable: FfiEnableFn,
    disable: FfiDisableFn,
    stats: FfiStatsFn,
}

impl EbpfFns {
    fn load(library_path: &str) -> anyhow::Result<Self> {
        // SAFETY: Loading a shared library from the given path. The library must be a valid
        // ELF shared object with the expected C ABI symbols.
        let lib = unsafe { Library::new(library_path) }
            .map_err(|e| anyhow!("Failed to load eBPF library: {}", e))?;

        // SAFETY: All symbols are loaded from the validated library. Each function pointer
        // has a known C ABI signature defined in the extern "C" block above.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            let init: Symbol<FfiInitFn> = lib
                .get(b"ebpf_init_coldpage_detector")
                .map_err(|_| anyhow!("Symbol ebpf_init_coldpage_detector not found"))?;
            let read_events: Symbol<FfiReadEventsFn> = lib
                .get(b"ebpf_read_coldpage_events")
                .map_err(|_| anyhow!("Symbol ebpf_read_coldpage_events not found"))?;
            let fallback: Symbol<FfiFallbackFn> = lib
                .get(b"fallback_check_cold_pages")
                .map_err(|_| anyhow!("Symbol fallback_check_cold_pages not found"))?;
            let enable: Symbol<FfiEnableFn> = lib
                .get(b"cold_page_detector_enable_tracking")
                .map_err(|_| anyhow!("Symbol cold_page_detector_enable_tracking not found"))?;
            let disable: Symbol<FfiDisableFn> = lib
                .get(b"cold_page_detector_disable_tracking")
                .map_err(|_| anyhow!("Symbol cold_page_detector_disable_tracking not found"))?;
            let stats: Symbol<FfiStatsFn> = lib
                .get(b"cold_page_detector_get_stats")
                .map_err(|_| anyhow!("Symbol cold_page_detector_get_stats not found"))?;

            Ok(Self {
                init: *init,
                read_events: *read_events,
                fallback: *fallback,
                enable: *enable,
                disable: *disable,
                stats: *stats,
            })
        }));
        match result {
            Ok(Ok(fns)) => Ok(fns),
            Ok(Err(e)) => Err(e),
            Err(_) => Err(anyhow!("Panic during eBPF symbol loading")),
        }
    }
}

pub fn is_ebpf_available() -> bool {
    EBPF_LIBRARY_LOADED.load(Ordering::Relaxed)
}

pub fn try_load_ebpf(library_path: &str) -> Result<()> {
    match EbpfFns::load(library_path) {
        Ok(fns) => {
            if EBPF_FNS.set(fns).is_err() {
                tracing::warn!("eBPF functions already initialized");
            }
            EBPF_LIBRARY_LOADED.store(true, Ordering::Relaxed);
            tracing::info!(
                "eBPF cold page detector loaded successfully from {}",
                library_path
            );
            Ok(())
        }
        Err(e) => {
            tracing::warn!(
                "Failed to load eBPF library: {}. Using userspace fallback.",
                e
            );
            EBPF_LIBRARY_LOADED.store(false, Ordering::Relaxed);
            Err(anyhow!("eBPF library failed to load: {}", e))
        }
    }
}

fn with_ebpf_fns<F, R>(f: F) -> Result<R>
where
    F: FnOnce(&EbpfFns) -> Result<R>,
{
    match EBPF_FNS.get() {
        Some(fns) => f(fns),
        None => Err(anyhow!(
            "eBPF library not loaded. Call try_load_ebpf() first."
        )),
    }
}

pub fn init_coldpage_detector(ring_buf_fd: i32, prog_fd: i32) -> Result<()> {
    if !is_ebpf_available() {
        return Err(anyhow!("eBPF not available - library not loaded"));
    }

    with_ebpf_fns(|fns| {
        // SAFETY: ring_buf_fd and prog_fd are valid file descriptors from the eBPF subsystem.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.init)(ring_buf_fd, prog_fd)
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in init_coldpage_detector")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to init coldpage detector, code: {}", ret))
        } else {
            Ok(())
        }
    })
}

pub fn read_coldpage_events(ring_buf_fd: i32, events: &mut [ColdPageEvent]) -> Result<usize> {
    if !is_ebpf_available() {
        return Err(anyhow!("eBPF not available"));
    }

    with_ebpf_fns(|fns| {
        // SAFETY: events.as_mut_ptr() points to valid memory of size events.len() * size_of::<ColdPageEvent>().
        // The FFI function will fill the events buffer.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.read_events)(ring_buf_fd, events.as_mut_ptr(), events.len() as u32)
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in read_coldpage_events")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to read coldpage events, code: {}", ret))
        } else {
            Ok(ret as usize)
        }
    })
}

pub fn fallback_check_cold_pages(
    addr: u64,
    len: usize,
    max_cold: u32,
) -> Result<Vec<ColdPageEvent>> {
    if !is_ebpf_available() {
        return userspace_cold_page_detection(addr, len, max_cold as usize);
    }

    with_ebpf_fns(|fns| {
        let mut events = vec![
            ColdPageEvent {
                pid: 0,
                addr: 0,
                timestamp: 0,
                access_count: 0,
            };
            max_cold as usize
        ];

        // SAFETY: addr and len are valid memory region. events.as_mut_ptr() points to valid buffer.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
            (fns.fallback)(addr, len, events.as_mut_ptr(), max_cold)
        }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in fallback_check_cold_pages")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to check cold pages, code: {}", ret))
        } else {
            events.truncate(ret as usize);
            Ok(events)
        }
    })
}

fn userspace_cold_page_detection(
    addr: u64,
    len: usize,
    max_cold: usize,
) -> Result<Vec<ColdPageEvent>> {
    let page_size = 4096;
    let num_pages = len.div_ceil(page_size);
    let mut events = Vec::with_capacity(num_pages.min(max_cold));
    let now = Instant::now();

    for i in 0..num_pages.min(max_cold) {
        let page_addr = addr + (i as u64 * page_size as u64);
        if let Some(access_count) = check_page_access_via_mincore(page_addr) {
            if access_count == 0 {
                events.push(ColdPageEvent {
                    pid: 0,
                    addr: page_addr,
                    timestamp: now.elapsed().as_millis() as u64,
                    access_count: 0,
                });
            }
        }
    }

    Ok(events)
}

fn check_page_access_via_mincore(page_addr: u64) -> Option<u32> {
    use std::fs::File;
    use std::os::unix::io::AsRawFd;

    let page_size = 4096;
    let mut vec = vec![0u8; page_size];

    let file = match File::open("/proc/self/mem") {
        Ok(f) => f,
        Err(_) => return None,
    };

    // SAFETY: All pointers and file descriptor are valid:
    // - `file.as_raw_fd()` is a valid open file descriptor for /proc/self/mem
    // - `vec.as_mut_ptr()` points to a buffer of `page_size` bytes
    // - `page_addr` is within the process address space and properly aligned
    let result = unsafe {
        libc::pread(
            file.as_raw_fd(),
            vec.as_mut_ptr() as *mut libc::c_void,
            page_size,
            page_addr as libc::off_t,
        )
    };

    if result > 0 {
        Some(1)
    } else {
        Some(0)
    }
}

pub fn enable_tracking() -> Result<()> {
    if !is_ebpf_available() {
        tracing::warn!("eBPF not available, cannot enable tracking");
        return Ok(());
    }

    with_ebpf_fns(|fns| {
        // SAFETY: The FFI function enables eBPF tracking globally. No parameters, no memory access.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { (fns.enable)() }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in enable_tracking")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to enable tracking, code: {}", ret))
        } else {
            Ok(())
        }
    })
}

pub fn disable_tracking() -> Result<()> {
    if !is_ebpf_available() {
        return Ok(());
    }

    with_ebpf_fns(|fns| {
        // SAFETY: The FFI function disables eBPF tracking globally. No parameters, no memory access.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { (fns.disable)() }));
        let ret = match result {
            Ok(val) => val,
            Err(_) => return Err(anyhow!("Panic in disable_tracking")),
        };
        if ret < 0 {
            Err(anyhow!("Failed to disable tracking, code: {}", ret))
        } else {
            Ok(())
        }
    })
}

pub fn get_stats() -> u64 {
    if !is_ebpf_available() {
        return 0;
    }
    match with_ebpf_fns(|fns| {
        // SAFETY: The FFI function returns a u64 statistic. No parameters, no side effects.
        let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe { (fns.stats)() }));
        match result {
            Ok(val) => Ok(val),
            Err(_) => Err(anyhow!("Panic in get_stats")),
        }
    }) {
        Ok(stats) => stats,
        Err(e) => {
            tracing::warn!("Failed to get eBPF stats: {}", e);
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cold_page_event_struct() {
        let event = ColdPageEvent {
            pid: 1234,
            addr: 0x7f000000,
            timestamp: 12345678,
            access_count: 5,
        };
        assert_eq!(event.pid, 1234);
        assert_eq!(event.addr, 0x7f000000);
    }

    #[test]
    fn test_ebpf_not_available() {
        assert!(!is_ebpf_available());
        let result = enable_tracking();
        assert!(result.is_ok());
    }
}
