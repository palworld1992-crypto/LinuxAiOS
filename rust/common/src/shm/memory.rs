use memmap2::{MmapMut, MmapOptions};
use std::fs::OpenOptions;
use std::io;
use std::path::Path;

pub struct SharedMemory {
    _file: Option<std::fs::File>, // giữ file descriptor để tránh bị drop
    mmap: MmapMut,
    size: usize,
}

impl SharedMemory {
    pub fn create(name: &str, size: usize) -> io::Result<Self> {
        let path = Path::new("/dev/shm").join(name);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        file.set_len(size as u64)?;
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        Ok(Self {
            _file: Some(file),
            mmap,
            size,
        })
    }

    pub fn open(name: &str, size: usize) -> io::Result<Self> {
        let path = Path::new("/dev/shm").join(name);
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        Ok(Self {
            _file: Some(file),
            mmap,
            size,
        })
    }

    /// Tạo SharedMemory từ MmapMut (dùng cho memfd)
    pub fn from_mmap(mmap: MmapMut, size: usize) -> Self {
        Self {
            _file: None,
            mmap,
            size,
        }
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.mmap.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.mmap.as_mut_ptr()
    }

    pub fn len(&self) -> usize {
        self.size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_shm_create_and_write() {
        let name = "test_shm_create";
        let size = 4096;

        let result = SharedMemory::create(name, size);
        if result.is_err() {
            return;
        }

        let mut shm = result.unwrap();
        assert_eq!(shm.len(), size);

        let ptr = shm.as_mut_ptr();
        unsafe {
            std::ptr::write_bytes(ptr, 0xAB, 4);
        }

        fs::remove_file(Path::new("/dev/shm").join(name)).ok();
    }

    #[test]
    fn test_shm_reopen() {
        let name = "test_shm_reopen";
        let size = 4096;

        let create_result = SharedMemory::create(name, size);
        if create_result.is_err() {
            return;
        }

        let mut shm1 = create_result.unwrap();
        unsafe {
            std::ptr::write_bytes(shm1.as_mut_ptr(), 0xCD, 4);
        }

        let open_result = SharedMemory::open(name, size);
        if open_result.is_err() {
            fs::remove_file(Path::new("/dev/shm").join(name)).ok();
            return;
        }

        let shm2 = open_result.unwrap();
        let ptr = shm2.as_ptr();
        let val = unsafe { std::ptr::read(ptr) };
        assert_eq!(val, 0xCD);

        fs::remove_file(Path::new("/dev/shm").join(name)).ok();
    }

    #[test]
    fn test_shm_read_write() {
        let name = "test_shm_read_write";
        let size = 4096;

        let result = SharedMemory::create(name, size);
        if result.is_err() {
            return;
        }

        let mut shm = result.unwrap();

        let data = b"Hello, Shared Memory!";
        let ptr = shm.as_mut_ptr();
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        }

        let read_ptr = shm.as_ptr();
        let read_data = unsafe { std::slice::from_raw_parts(read_ptr, data.len()) };

        assert_eq!(&read_data[..data.len()], &data[..]);

        fs::remove_file(Path::new("/dev/shm").join(name)).ok();
    }
}
