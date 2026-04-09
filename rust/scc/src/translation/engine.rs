use memmap2::MmapMut;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct ShmHandle {
    pub id: String,
    pub size: usize,
    pub fd: i32,
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

        let fd_i32: i32 = fd
            .try_into()
            .map_err(|_| TranslationError::CreateFailed("fd overflow".to_string()))?;

        // SAFETY: fd is a valid file descriptor from memfd_create. ftruncate is safe here.
        if unsafe { libc::ftruncate(fd_i32, size as libc::off_t) } != 0 {
            // SAFETY: fd is valid and we're just closing it after ftruncate failed
            unsafe { libc::close(fd_i32) };
            return Err(TranslationError::CreateFailed(
                "ftruncate failed".to_string(),
            ));
        }

        // SAFETY: mmap with appropriate size using map_anon for anonymous memory
        let mmap =
            MmapMut::map_anon(size).map_err(|e| TranslationError::MapFailed(e.to_string()))?;

        let id = format!(
            "shm_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_nanos())
                .ok_or_else(|| TranslationError::CreateFailed("System time error".into()))?
        );

        Ok((
            ShmHandle {
                id,
                size,
                fd: fd_i32,
            },
            mmap,
        ))
    }

    pub fn open_region(_id: &str) -> Result<ShmHandle, TranslationError> {
        // TODO(Phase 3): Implement actual region opening via /proc/self/fd or similar
        Err(TranslationError::InvalidHandle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_region() -> Result<(), TranslationError> {
        let result = TranslationEngine::create_region(4096);
        assert!(
            result.is_ok(),
            "Failed to create region: {:?}",
            result.err()
        );
        let (_handle, _mmap) = result?;
        // Verify handle properties
        let result = TranslationEngine::create_region(4096);
        let (handle, _) = result?;
        assert_eq!(handle.size, 4096);
        Ok(())
    }
}
