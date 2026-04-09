use linux_module::memory::UserfaultHandler;
use memfd::MemfdOptions;
use memmap2::MmapOptions;
use std::env;
use std::fs;
use tempfile::tempdir;

fn with_temp_base<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    let temp_dir = tempdir()?;
    let base_path = temp_dir
        .path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

#[test]
fn test_userfault_handler_register_and_page_out() -> anyhow::Result<()> {
    use libc::{syscall, SYS_userfaultfd, O_CLOEXEC};
    // SAFETY: syscall with known valid parameters (SYS_userfaultfd, O_CLOEXEC flag)
    let fd = unsafe { syscall(SYS_userfaultfd, O_CLOEXEC) };
    if fd < 0 {
        tracing::info!("Skipping test: userfaultfd not available");
        return Ok(());
    }
    // SAFETY: fd is a valid file descriptor from syscall, close is safe
    unsafe { libc::close(fd as i32) };

    with_temp_base(|| {
        let handler = UserfaultHandler::new();
        let memfd = MemfdOptions::default().create("test")?;
        let size = 4096 * 2;
        memfd.as_file().set_len(size as u64)?;
        // SAFETY: mmap with correct size and valid file descriptor
        let mmap = unsafe { MmapOptions::new().len(size).map_mut(memfd.as_file())? };
        let data = b"Hello, userfault!";
        // SAFETY: copy to mapped memory region that we own, non-overlapping
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), mmap.as_ptr() as *mut u8, data.len());
        }
        let fd = handler.register_region(&mmap, size)?;
        let dir = tempdir()?;
        let compressed_path = dir.path().join("page0.zst");
        fs::write(&compressed_path, data)?;
        handler.page_out(fd, 0, 4096, compressed_path)?;
        // SAFETY: reading from mapped memory is safe after page_out
        let value = unsafe { *mmap.as_ptr() };
        assert_eq!(value, b'H');
        handler.unregister_region(fd)?;
        Ok(())
    })
}

#[test]
fn test_userfault_handler_multiple_regions() -> anyhow::Result<()> {
    use libc::{syscall, SYS_userfaultfd, O_CLOEXEC};
    // SAFETY: syscall with known valid parameters
    let fd = unsafe { syscall(SYS_userfaultfd, O_CLOEXEC) };
    if fd < 0 {
        tracing::info!("Skipping test: userfaultfd not available");
        return Ok(());
    }
    // SAFETY: fd is valid from syscall
    unsafe { libc::close(fd as i32) };

    with_temp_base(|| {
        let handler = UserfaultHandler::new();
        let memfd1 = MemfdOptions::default().create("test1")?;
        let memfd2 = MemfdOptions::default().create("test2")?;
        let size = 4096;
        memfd1.as_file().set_len(size as u64)?;
        memfd2.as_file().set_len(size as u64)?;
        // SAFETY: mmap with valid file descriptors and correct size
        let mmap1 = unsafe { MmapOptions::new().len(size).map_mut(memfd1.as_file())? };
        // SAFETY: mmap with valid file descriptor
        let mmap2 = unsafe { MmapOptions::new().len(size).map_mut(memfd2.as_file())? };
        let fd1 = handler.register_region(&mmap1, size)?;
        let fd2 = handler.register_region(&mmap2, size)?;
        let dir = tempdir()?;
        let data1 = b"Region 1 data";
        let data2 = b"Region 2 data";
        let path1 = dir.path().join("region1.zst");
        let path2 = dir.path().join("region2.zst");
        fs::write(&path1, data1)?;
        fs::write(&path2, data2)?;
        handler.page_out(fd1, 0, 4096, path1)?;
        handler.page_out(fd2, 0, 4096, path2)?;
        // SAFETY: reading from mapped memory regions
        let val1 = unsafe { *mmap1.as_ptr() };
        let val2 = unsafe { *mmap2.as_ptr() };
        assert_eq!(val1, b'R');
        assert_eq!(val2, b'R');
        handler.unregister_region(fd1)?;
        handler.unregister_region(fd2)?;
        Ok(())
    })
}
