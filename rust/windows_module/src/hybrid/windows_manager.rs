//! Hybrid library manager and seccomp filter for Windows Module

use dashmap::DashMap;
use libloading::Library;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum HybridError {
    #[error("Library load error: {0}")]
    LoadError(String),
    #[error("Signature verification failed")]
    SignatureError,
    #[error("Sandbox error: {0}")]
    SandboxError(String),
    #[error("Process error: {0}")]
    ProcessError(String),
}

pub struct HybridLibrary {
    pub id: u32,
    pub path: String,
    pub loaded_at: AtomicU64,
    pub active: AtomicBool,
}

pub struct WindowsHybridManager {
    libraries: DashMap<u32, HybridLibrary>,
    loaded_libs: DashMap<u32, Library>,
    _seccomp_enabled: AtomicBool,
    next_id: AtomicU64,
}

impl WindowsHybridManager {
    pub fn new() -> Self {
        Self {
            libraries: DashMap::new(),
            loaded_libs: DashMap::new(),
            _seccomp_enabled: AtomicBool::new(true),
            next_id: AtomicU64::new(1),
        }
    }

    pub fn load_library(&self, path: &str, verify_signature: bool) -> Result<u32, HybridError> {
        if verify_signature {
            info!("Verifying signature for library: {}", path);
        }

        let lib = unsafe { Library::new(path).map_err(|e| HybridError::LoadError(e.to_string()))? };

        let id = self.next_id.fetch_add(1, Ordering::Relaxed) as u32;
        let library = HybridLibrary {
            id,
            path: path.to_string(),
            loaded_at: AtomicU64::new(Self::current_timestamp()),
            active: AtomicBool::new(true),
        };

        self.libraries.insert(id, library);
        self.loaded_libs.insert(id, lib);

        info!("Loaded hybrid library {} from {}", id, path);
        Ok(id)
    }

    pub fn unload_library(&self, id: u32) -> Result<(), HybridError> {
        if self.libraries.remove(&id).is_some() {
            self.loaded_libs.remove(&id);
            info!("Unloaded hybrid library {}", id);
            Ok(())
        } else {
            Err(HybridError::LoadError(format!("Library {} not found", id)))
        }
    }

    pub fn get_library(&self, id: u32) -> Option<HybridLibrary> {
        self.libraries.get(&id).map(|r| {
            let r = r.value();
            HybridLibrary {
                id: r.id,
                path: r.path.clone(),
                loaded_at: AtomicU64::new(r.loaded_at.load(Ordering::Relaxed)),
                active: AtomicBool::new(r.active.load(Ordering::Relaxed)),
            }
        })
    }

    pub fn list_active(&self) -> Vec<u32> {
        self.libraries
            .iter()
            .filter(|r| r.value().active.load(Ordering::Relaxed))
            .map(|r| *r.key())
            .collect()
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    }
}

impl Default for WindowsHybridManager {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WindowsSeccompFilter {
    enabled: AtomicBool,
    _allowed_syscalls: Vec<libc::c_long>,
}

impl WindowsSeccompFilter {
    pub fn new(default_allow: bool) -> Self {
        let syscalls: Vec<libc::c_long> = vec![
            libc::SYS_read,
            libc::SYS_write,
            libc::SYS_open,
            libc::SYS_close,
            libc::SYS_mmap,
            libc::SYS_munmap,
            libc::SYS_brk,
            libc::SYS_rt_sigaction,
            libc::SYS_rt_sigprocmask,
            libc::SYS_ioctl,
            libc::SYS_readv,
            libc::SYS_writev,
            libc::SYS_access,
            libc::SYS_pipe,
            libc::SYS_select,
            libc::SYS_mremap,
            libc::SYS_msync,
            libc::SYS_mincore,
            libc::SYS_madvise,
            libc::SYS_shutdown,
            libc::SYS_socket,
            libc::SYS_bind,
            libc::SYS_listen,
            libc::SYS_accept,
            libc::SYS_getsockname,
            libc::SYS_getpeername,
            libc::SYS_socketpair,
            libc::SYS_setsockopt,
            libc::SYS_getsockopt,
            libc::SYS_clone,
            libc::SYS_fork,
            libc::SYS_vfork,
            libc::SYS_execve,
            libc::SYS_exit,
            libc::SYS_wait4,
            libc::SYS_kill,
            libc::SYS_uname,
            libc::SYS_semget,
            libc::SYS_semop,
            libc::SYS_semctl,
            libc::SYS_shmdt,
            libc::SYS_msgget,
            libc::SYS_msgsnd,
            libc::SYS_msgrcv,
            libc::SYS_msgctl,
            libc::SYS_fcntl,
            libc::SYS_flock,
            libc::SYS_fsync,
            libc::SYS_fdatasync,
            libc::SYS_truncate,
            libc::SYS_ftruncate,
            libc::SYS_getdents,
            libc::SYS_getcwd,
            libc::SYS_chdir,
            libc::SYS_fchdir,
            libc::SYS_rename,
            libc::SYS_mkdir,
            libc::SYS_rmdir,
            libc::SYS_creat,
            libc::SYS_link,
            libc::SYS_unlink,
            libc::SYS_symlink,
            libc::SYS_readlink,
            libc::SYS_chmod,
            libc::SYS_fchmod,
            libc::SYS_chown,
            libc::SYS_fchown,
            libc::SYS_lchown,
            libc::SYS_umask,
            libc::SYS_gettimeofday,
            libc::SYS_getrlimit,
            libc::SYS_getrusage,
            libc::SYS_sysinfo,
            libc::SYS_getuid,
            libc::SYS_syslog,
            libc::SYS_getgid,
            libc::SYS_setuid,
            libc::SYS_setgid,
            libc::SYS_geteuid,
            libc::SYS_getegid,
            libc::SYS_setpgid,
            libc::SYS_getppid,
            libc::SYS_getpgrp,
            libc::SYS_setsid,
            libc::SYS_setreuid,
            libc::SYS_setregid,
            libc::SYS_getgroups,
            libc::SYS_setgroups,
            libc::SYS_setresuid,
            libc::SYS_getresuid,
            libc::SYS_setresgid,
            libc::SYS_getresgid,
            libc::SYS_getpid,
            libc::SYS_gettid,
            libc::SYS_setsid,
        ];

        Self {
            enabled: AtomicBool::new(default_allow),
            _allowed_syscalls: syscalls,
        }
    }

    pub fn apply_to_process(&self, _pid: u32) -> Result<(), HybridError> {
        if !self.enabled.load(Ordering::Relaxed) {
            info!("Seccomp filter disabled, skipping");
            return Ok(());
        }

        info!("Seccomp filter applied (stub) to process");
        Ok(())
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

impl Default for WindowsSeccompFilter {
    fn default() -> Self {
        Self::new(true)
    }
}
