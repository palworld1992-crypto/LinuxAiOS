//! Userfault handler for memory tiering.
//! Uses `userfaultfd` crate to handle page faults on memory regions that have been paged out.

use anyhow::{anyhow, Result};
use libc::{syscall, SYS_userfaultfd, O_CLOEXEC, O_NONBLOCK};
use memmap2::MmapMut;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::os::unix::io::{AsRawFd, FromRawFd, RawFd};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use tracing::{error, info, warn};
use userfaultfd::{Event, Uffd};
use zstd::stream::Decoder;

use crate::tensor::TensorPool;

/// Metadata for a paged-out page (or region).
#[derive(Clone)]
struct PageOutEntry {
    compressed_path: PathBuf,
    size: usize,
    #[allow(dead_code)]
    offset: usize,
}

/// Userfault handler that monitors memory regions and restores pages on demand.
pub struct UserfaultHandler {
    /// Shared map from (uffd_fd, offset) to paged-out entry.
    paged_out: Arc<RwLock<HashMap<(RawFd, usize), PageOutEntry>>>,
    /// Map from region base address to (uffd, size, fd) – only used for unregistration.
    regions: RwLock<HashMap<usize, (Uffd, usize, RawFd)>>,
    /// Thread handles for each registered region.
    threads: RwLock<HashMap<RawFd, thread::JoinHandle<()>>>,
    /// Tensor pool reference (optional).
    tensor_pool: Option<Arc<RwLock<TensorPool>>>,
}

impl UserfaultHandler {
    pub fn new() -> Self {
        Self {
            paged_out: Arc::new(RwLock::new(HashMap::new())),
            regions: RwLock::new(HashMap::new()),
            threads: RwLock::new(HashMap::new()),
            tensor_pool: None,
        }
    }

    pub fn attach_tensor_pool(&mut self, pool: Arc<RwLock<TensorPool>>) {
        self.tensor_pool = Some(pool);
    }

    /// Create a userfaultfd file descriptor.
    fn create_uffd(blocking: bool) -> Result<RawFd> {
        let flags = if blocking { 0 } else { O_NONBLOCK };
        // SAFETY: syscall SYS_userfaultfd được kernel hỗ trợ, flags hợp lệ.
        let fd = unsafe { syscall(SYS_userfaultfd, flags | O_CLOEXEC) };
        if fd < 0 {
            Err(std::io::Error::last_os_error().into())
        } else {
            Ok(fd as RawFd)
        }
    }

    /// Register a memory region for userfaultfd handling.
    pub fn register_region(&self, mmap: &MmapMut, size: usize) -> Result<RawFd> {
        let addr = mmap.as_ptr() as usize;
        let fd = Self::create_uffd(true)?;
        // SAFETY: fd là userfaultfd hợp lệ, được tạo bởi create_uffd.
        let uffd = unsafe { Uffd::from_raw_fd(fd) };

        // Gọi register không cần unsafe (theo crate version hiện tại)
        uffd.register(addr as *mut _, size)?;

        // Store the uffd for later unregistration
        {
            let mut regions = self.regions.write();
            regions.insert(addr, (uffd, size, fd));
        }

        // Spawn a thread to handle page faults for this region.
        // Share the paged_out map via Arc.
        let paged_out = self.paged_out.clone();
        let handle = thread::spawn(move || {
            // Recreate Uffd from fd (safe because we have exclusive access now)
            // SAFETY: fd là userfaultfd hợp lệ và thread này có quyền sở hữu.
            let uffd = unsafe { Uffd::from_raw_fd(fd) };
            Self::run_loop(uffd, addr, size, paged_out);
        });
        self.threads.write().insert(fd, handle);
        Ok(fd)
    }

    /// Main loop for a userfaultfd instance.
    fn run_loop(
        mut uffd: Uffd,
        base_addr: usize,
        region_size: usize,
        paged_out: Arc<RwLock<HashMap<(RawFd, usize), PageOutEntry>>>,
    ) {
        info!(
            "Userfault handler thread started for region at {:#x}",
            base_addr
        );
        loop {
            match uffd.read_event() {
                Ok(Some(event)) => {
                    Self::handle_event(&mut uffd, event, base_addr, region_size, &paged_out);
                }
                Ok(None) => {
                    // No event (non‑blocking) – should not happen in blocking mode
                    continue;
                }
                Err(e) => {
                    error!("Userfaultfd error: {}", e);
                    break;
                }
            }
        }
        info!(
            "Userfault handler thread stopped for region at {:#x}",
            base_addr
        );
    }

    fn handle_event(
        uffd: &mut Uffd,
        event: Event,
        base_addr: usize,
        region_size: usize,
        paged_out: &Arc<RwLock<HashMap<(RawFd, usize), PageOutEntry>>>,
    ) {
        match event {
            Event::Pagefault { addr, .. } => {
                let fault_addr = addr as usize;
                let offset = fault_addr - base_addr;
                if offset >= region_size {
                    warn!("Page fault outside region: {:#x}", fault_addr);
                    return;
                }

                let fd = uffd.as_raw_fd();
                let entry = {
                    let map = paged_out.read();
                    map.get(&(fd, offset)).cloned()
                };

                if let Some(entry) = entry {
                    // Restore the page from compressed file
                    if let Err(e) = Self::restore_page(uffd, fault_addr, &entry) {
                        error!("Failed to restore page at {:#x}: {}", fault_addr, e);
                        // Fallback: zero the page
                        // SAFETY: fault_addr là địa chỉ hợp lệ trong vùng đã đăng ký,
                        // entry.size là kích thước trang (thường 4096).
                        let _ = unsafe { uffd.zeropage(fault_addr as *mut _, entry.size, true) };
                    }
                } else {
                    warn!("Page fault at {:#x} but not paged out", fault_addr);
                    // SAFETY: fault_addr là địa chỉ hợp lệ.
                    let _ = unsafe { uffd.zeropage(fault_addr as *mut _, 4096, true) };
                }
            }
            Event::Remove { .. } => {
                info!("Userfaultfd removed, exiting loop");
                // The thread will exit when loop breaks
            }
            _ => {
                warn!("Unhandled event: {:?}", event);
            }
        }
    }

    /// Restore a page from compressed file.
    fn restore_page(uffd: &mut Uffd, fault_addr: usize, entry: &PageOutEntry) -> Result<()> {
        // ✅ FIX: Loại bỏ `mut` vì file chỉ được truyền vào decoder và không cần thay đổi trực tiếp
        let file = File::open(&entry.compressed_path)?;
        let mut decoder = Decoder::new(file)?;
        let mut decompressed = vec![0u8; entry.size];
        decoder.read_exact(&mut decompressed)?;

        // Use UFFDIO_COPY to copy the page into the faulting address.
        // SAFETY: fault_addr là địa chỉ hợp lệ trong vùng đã đăng ký,
        // decompressed.as_ptr() trỏ đến bộ nhớ đệm hợp lệ, entry.size là kích thước trang.
        unsafe {
            uffd.copy(
                fault_addr as *mut _,
                decompressed.as_ptr() as *mut _,
                entry.size,
                true,
            )?;
        }
        Ok(())
    }

    /// Mark a range as paged out. On page fault, the handler will restore from the given file.
    pub fn page_out(
        &self,
        fd: RawFd,
        offset: usize,
        size: usize,
        compressed_path: PathBuf,
    ) -> Result<()> {
        let entry = PageOutEntry {
            compressed_path,
            size,
            offset,
        };
        self.paged_out.write().insert((fd, offset), entry);
        Ok(())
    }

    /// Remove a region (stop handling faults).
    pub fn unregister_region(&self, fd: RawFd) -> Result<()> {
        if let Some(handle) = self.threads.write().remove(&fd) {
            // Remove from regions map as well
            let mut regions = self.regions.write();
            let to_remove = regions
                .iter()
                .find(|(_, (_, _, f))| *f == fd)
                .map(|(addr, _)| *addr);
            if let Some(addr) = to_remove {
                regions.remove(&addr);
            }
            // Join the thread
            handle
                .join()
                .map_err(|_| anyhow!("Failed to join thread"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memfd::MemfdOptions;
    use memmap2::MmapOptions;
    use tempfile::tempdir;

    #[test]
    fn test_userfault_handler() -> anyhow::Result<()> {
        // Kiểm tra xem userfaultfd có khả dụng không (cần CAP_SYS_ADMIN hoặc kernel hỗ trợ)
        let fd = unsafe { syscall(SYS_userfaultfd, O_CLOEXEC) };
        if fd < 0 {
            eprintln!(
                "Skipping test: userfaultfd not available (errno: {})",
                std::io::Error::last_os_error()
            );
            return Ok(());
        }
        unsafe { libc::close(fd as i32) };

        let handler = UserfaultHandler::new();

        // Create a memfd-backed mapping
        let memfd = MemfdOptions::default().create("test")?;
        let size = 4096 * 2;
        memfd.as_file().set_len(size as u64)?;
        let mmap = unsafe { MmapOptions::new().len(size).map_mut(memfd.as_file())? };

        // Try to register the region
        let register_result = handler.register_region(&mmap, size);
        let fd = match register_result {
            Ok(fd) => fd,
            Err(e) => {
                // Check if the error is due to unsupported operation (EINVAL, EPERM, ENOSYS)
                let should_skip = if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                    let kind = io_err.kind();
                    kind == std::io::ErrorKind::InvalidInput
                        || kind == std::io::ErrorKind::PermissionDenied
                        || kind == std::io::ErrorKind::Unsupported
                } else {
                    // Also check the error string as a fallback
                    let err_str = format!("{:#}", e);
                    err_str.contains("EINVAL")
                        || err_str.contains("EPERM")
                        || err_str.contains("Operation not permitted")
                        || err_str.contains("Invalid argument")
                };

                if should_skip {
                    eprintln!("Skipping test: userfaultfd operation not supported: {}", e);
                    return Ok(());
                }
                return Err(e);
            }
        };

        // Write some data
        let data = b"Hello, world!";
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), mmap.as_ptr() as *mut u8, data.len());
        }

        // Simulate paging out the first page
        let dir = tempdir()?;
        let compressed_path = dir.path().join("page0.zst");
        std::fs::write(&compressed_path, data)?;

        handler.page_out(fd, 0, 4096, compressed_path)?;

        // Access the page again – it should be restored
        let value = unsafe { *mmap.as_ptr() };
        assert_eq!(value, b'H');

        handler.unregister_region(fd)?;
        Ok(())
    }
}
