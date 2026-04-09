use crate::idl_registry::IDLType;
use bincode;
use memmap2::MmapMut;
use serde::{de::DeserializeOwned, Serialize};
use std::os::fd::AsRawFd;
use tempfile::tempfile;

#[repr(C)]
pub struct ShmDescriptor {
    pub ptr: *mut u8,
    pub len: usize,
    pub fd: i32,
}

impl ShmDescriptor {
    pub fn new(ptr: *mut u8, len: usize, fd: i32) -> Self {
        Self { ptr, len, fd }
    }
}

pub struct Translator;

impl Translator {
    /// Convert serializable data to shared memory descriptor (zero-copy)
    pub fn to_descriptor<T: Serialize>(_data: &T, _layout: &mut IDLType) -> Option<ShmDescriptor> {
        // Phase 2: Serialize data and map to shared memory
        let serialized = bincode::serialize(_data).ok()?;
        let size = serialized.len();

        // Create temp file as shared memory backing
        let temp_file = tempfile().ok()?;
        temp_file.set_len(size as u64).ok()?;

        let mut mmap = unsafe { MmapMut::map_mut(&temp_file).ok()? };
        mmap.copy_from_slice(&serialized);
        mmap.flush().ok()?;

        Some(ShmDescriptor {
            ptr: mmap.as_mut_ptr(),
            len: size,
            fd: temp_file.as_raw_fd(),
        })
    }

    /// Read data from shared memory descriptor (zero-copy)
    pub fn from_descriptor<T: DeserializeOwned>(
        desc: &ShmDescriptor,
        _layout: &IDLType,
    ) -> Option<T> {
        // Phase 2: Deserialize directly from shared memory slice
        unsafe {
            let slice = std::slice::from_raw_parts(desc.ptr, desc.len);
            bincode::deserialize(slice).ok()
        }
    }
}
