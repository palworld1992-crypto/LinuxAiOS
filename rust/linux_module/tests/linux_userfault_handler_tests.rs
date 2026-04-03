use linux_module::memory::UserfaultHandler;
use memfd::MemfdOptions;
use memmap2::MmapOptions;
use std::env;
use std::fs;
use tempfile::tempdir;

fn with_temp_base<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path().to_str().unwrap();
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

#[test]
fn test_userfault_handler_register_and_page_out() {
    // Kiểm tra userfaultfd có khả dụng không
    use libc::{syscall, SYS_userfaultfd, O_CLOEXEC};
    let fd = unsafe { syscall(SYS_userfaultfd, O_CLOEXEC) };
    if fd < 0 {
        eprintln!("Skipping test: userfaultfd not available");
        return;
    }
    unsafe { libc::close(fd as i32) };

    with_temp_base(|| {
        let handler = UserfaultHandler::new();

        // Tạo vùng nhớ memfd
        let memfd = MemfdOptions::default().create("test").unwrap();
        let size = 4096 * 2;
        memfd.as_file().set_len(size as u64).unwrap();
        let mmap = unsafe {
            MmapOptions::new()
                .len(size)
                .map_mut(memfd.as_file())
                .unwrap()
        };

        // Ghi dữ liệu test
        let data = b"Hello, userfault!";
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), mmap.as_ptr() as *mut u8, data.len());
        }

        // Đăng ký vùng nhớ
        let fd = handler.register_region(&mmap, size).unwrap();

        // Tạo file nén giả lập
        let dir = tempdir().unwrap();
        let compressed_path = dir.path().join("page0.zst");
        fs::write(&compressed_path, data).unwrap();

        // Đánh dấu page đã paged out
        handler.page_out(fd, 0, 4096, compressed_path).unwrap();

        // Truy cập page để kích hoạt page fault (sẽ được khôi phục)
        let value = unsafe { *mmap.as_ptr() };
        assert_eq!(value, b'H');

        // Hủy đăng ký
        handler.unregister_region(fd).unwrap();
    });
}

#[test]
fn test_userfault_handler_multiple_regions() {
    use libc::{syscall, SYS_userfaultfd, O_CLOEXEC};
    let fd = unsafe { syscall(SYS_userfaultfd, O_CLOEXEC) };
    if fd < 0 {
        eprintln!("Skipping test: userfaultfd not available");
        return;
    }
    unsafe { libc::close(fd as i32) };

    with_temp_base(|| {
        let handler = UserfaultHandler::new();

        // Tạo 2 vùng nhớ riêng
        let memfd1 = MemfdOptions::default().create("test1").unwrap();
        let memfd2 = MemfdOptions::default().create("test2").unwrap();
        let size = 4096;
        memfd1.as_file().set_len(size as u64).unwrap();
        memfd2.as_file().set_len(size as u64).unwrap();

        let mmap1 = unsafe {
            MmapOptions::new()
                .len(size)
                .map_mut(memfd1.as_file())
                .unwrap()
        };
        let mmap2 = unsafe {
            MmapOptions::new()
                .len(size)
                .map_mut(memfd2.as_file())
                .unwrap()
        };

        let fd1 = handler.register_region(&mmap1, size).unwrap();
        let fd2 = handler.register_region(&mmap2, size).unwrap();

        // Tạo file nén cho mỗi region
        let dir = tempdir().unwrap();
        let data1 = b"Region 1 data";
        let data2 = b"Region 2 data";
        let path1 = dir.path().join("region1.zst");
        let path2 = dir.path().join("region2.zst");
        fs::write(&path1, data1).unwrap();
        fs::write(&path2, data2).unwrap();

        handler.page_out(fd1, 0, 4096, path1).unwrap();
        handler.page_out(fd2, 0, 4096, path2).unwrap();

        // Kiểm tra cả hai vùng đều có thể phục hồi
        let val1 = unsafe { *mmap1.as_ptr() };
        let val2 = unsafe { *mmap2.as_ptr() };
        assert_eq!(val1, b'R');
        assert_eq!(val2, b'R');

        handler.unregister_region(fd1).unwrap();
        handler.unregister_region(fd2).unwrap();
    });
}
