use memmap2::{MmapMut, MmapOptions};
use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

pub struct SharedMemoryRegion {
    _file: File,
    mmap: MmapMut,
    size: usize,
}

impl SharedMemoryRegion {
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
            _file: file,
            mmap,
            size,
        })
    }

    pub fn open(name: &str, size: usize) -> io::Result<Self> {
        let path = Path::new("/dev/shm").join(name);
        let file = OpenOptions::new().read(true).write(true).open(&path)?;
        let mmap = unsafe { MmapOptions::new().map_mut(&file)? };
        Ok(Self {
            _file: file,
            mmap,
            size,
        })
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
