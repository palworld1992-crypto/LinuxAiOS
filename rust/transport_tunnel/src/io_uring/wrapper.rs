use std::os::unix::io::RawFd;

pub struct IoUringWrapper;

impl IoUringWrapper {
    pub fn submit_read(fd: RawFd, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        // Fallback: dùng pread
        let ret = unsafe { libc::pread(fd, buf.as_mut_ptr() as *mut _, buf.len(), offset as i64) };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }
}
