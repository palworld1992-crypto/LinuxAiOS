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
        // SAFETY: The file is opened with read/write permissions, has valid size,
        // and is kept alive via _file field. MmapMut guarantees exclusive access.
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
        // SAFETY: The file exists and is opened with read/write permissions.
        // MmapMut guarantees exclusive access. File is kept alive via _file field.
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

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_shm_create_and_write() -> io::Result<()> {
        let name = "test_shm_create";
        let size = 4096;

        let mut shm = SharedMemory::create(name, size)?;
        assert_eq!(shm.len(), size);

        let ptr = shm.as_mut_ptr();
        // SAFETY: ptr points to a valid mmap region of at least `size` bytes (4096).
        // Writing 4 bytes is within bounds. No other thread accesses this memory.
        unsafe {
            std::ptr::write_bytes(ptr, 0xAB, 4);
        }

        fs::remove_file(Path::new("/dev/shm").join(name)).ok();
        Ok(())
    }

    #[test]
    fn test_shm_reopen() -> io::Result<()> {
        let name = "test_shm_reopen";
        let size = 4096;

        let mut shm1 = SharedMemory::create(name, size)?;
        // SAFETY: shm1.as_mut_ptr() points to a valid mmap region of at least `size` bytes.
        // Writing 4 bytes is within bounds. No other thread accesses this memory.
        unsafe {
            std::ptr::write_bytes(shm1.as_mut_ptr(), 0xCD, 4);
        }

        let shm2 = SharedMemory::open(name, size)?;
        let ptr = shm2.as_ptr();
        // SAFETY: ptr points to a valid mmap region previously written with 0xCD bytes.
        // Reading 1 byte is safe because the memory is initialized and valid.
        let val = unsafe { std::ptr::read(ptr) };
        assert_eq!(val, 0xCD);

        fs::remove_file(Path::new("/dev/shm").join(name)).ok();
        Ok(())
    }

    #[test]
    fn test_shm_read_write() -> io::Result<()> {
        let name = "test_shm_read_write";
        let size = 4096;

        let mut shm = SharedMemory::create(name, size)?;

        let data = b"Hello, Shared Memory!";
        let ptr = shm.as_mut_ptr();
        // SAFETY: ptr points to a valid mmap region of at least `size` bytes (4096).
        // Copying data.len() (19) bytes is well within bounds. No concurrent access.
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), ptr, data.len());
        }

        let read_ptr = shm.as_ptr();
        // SAFETY: read_ptr points to the same valid mmap region, just written with
        // data.len() bytes. Creating a slice of that length is safe.
        let read_data = unsafe { std::slice::from_raw_parts(read_ptr, data.len()) };

        assert_eq!(&read_data[..data.len()], &data[..]);

        fs::remove_file(Path::new("/dev/shm").join(name)).ok();
        Ok(())
    }
}
