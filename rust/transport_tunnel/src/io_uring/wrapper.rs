use std::os::unix::io::RawFd;

pub struct IoUringWrapper;

impl IoUringWrapper {
    pub fn submit_read(fd: RawFd, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        // Fallback: dùng pread
        let ret = unsafe {
            // SAFETY: pread is called with a valid fd, a mutable buffer pointer with correct length,
            // and a valid offset. The buffer is guaranteed to be valid for the duration of the call
            // and pread does not retain the pointer after returning.
            libc::pread(
                fd,
                buf.as_mut_ptr() as *mut _,
                buf.len() as libc::size_t,
                offset as libc::off_t,
            )
        };
        if ret < 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(ret as usize)
        }
    }
}
