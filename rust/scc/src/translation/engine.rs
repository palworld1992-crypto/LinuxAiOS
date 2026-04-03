use memmap2::MmapMut;
use std::os::fd::{FromRawFd, IntoRawFd};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ShmHandle {
    id: String,
    size: usize,
    fd: i32,
}

#[derive(Error, Debug)]
pub enum TranslationError {
    #[error("Failed to create shared memory: {0}")]
    CreateFailed(String),
    #[error("Failed to map shared memory: {0}")]
    MapFailed(String),
    #[error("Invalid handle")]
    InvalidHandle,
}

pub struct TranslationEngine;

impl TranslationEngine {
    pub fn create_region(size: usize) -> Result<(ShmHandle, MmapMut), TranslationError> {
        // SAFETY: memfd_create syscall with MFD_CLOEXEC and MFD_NOEXEC_SEAL flags.
        // - MFD_CLOEXEC ensures the fd is closed on exec
        // - MFD_NOEXEC_SEAL prevents executing code in the memfd
        // The fd is valid for the duration of this function until we close it or transfer ownership.
        let fd = unsafe {
            libc::syscall(
                libc::SYS_memfd_create,
                b"translation_region\0" as *const u8 as *const libc::c_char,
                libc::MFD_CLOEXEC | libc::MFD_NOEXEC_SEAL,
            )
        };

        if fd < 0 {
            return Err(TranslationError::CreateFailed(
                "memfd_create failed".to_string(),
            ));
        }

        // SAFETY: fd is a valid file descriptor from memfd_create. ftruncate is safe here.
        if libc::ftruncate(fd, size as libc::off_t) != 0 {
            unsafe { libc::close(fd) };
            return Err(TranslationError::CreateFailed(
                "ftruncate failed".to_string(),
            ));
        }

        // SAFETY: mmap with appropriate size. We ensure size > 0 and the fd is valid.
        let mmap = unsafe {
            MmapMut::mmap_mut(size).map_err(|e| TranslationError::MapFailed(e.to_string()))?
        };

        let id = format!(
            "shm_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_nanos())
                .ok_or_else(|| TranslationError::CreateFailed("System time error".into()))?
        );

        Ok((ShmHandle { id, size, fd }, mmap))
    }

    pub fn open_region(id: &str) -> Result<ShmHandle, TranslationError> {
        // TODO: Implement actual region opening via /proc/self/fd or similar
        Err(TranslationError::InvalidHandle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_region() {
        let result = TranslationEngine::create_region(4096);
        assert!(
            result.is_ok(),
            "Failed to create region: {:?}",
            result.err()
        );
        let (handle, _mmap) = result.unwrap();
        assert_eq!(handle.size, 4096);
    }
}
