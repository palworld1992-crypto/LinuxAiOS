use libc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SeccompError {
    #[error("Failed to apply seccomp filter: {0}")]
    ApplyError(String),
    #[error("Invalid syscall whitelist: {0}")]
    InvalidSyscall(String),
}

pub struct AndroidSeccompFilter {
    allowed_syscalls: Vec<i64>,
}

impl Default for AndroidSeccompFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidSeccompFilter {
    pub fn new() -> Self {
        Self {
            allowed_syscalls: Self::default_whitelist(),
        }
    }

    fn default_whitelist() -> Vec<i64> {
        vec![
            libc::SYS_read,
            libc::SYS_write,
            libc::SYS_open,
            libc::SYS_close,
            libc::SYS_stat,
            libc::SYS_fstat,
            libc::SYS_mmap,
            libc::SYS_mprotect,
            libc::SYS_munmap,
            libc::SYS_brk,
            libc::SYS_ioctl,
            libc::SYS_access,
            libc::SYS_pipe,
            libc::SYS_select,
            libc::SYS_sched_yield,
            libc::SYS_mremap,
            libc::SYS_msync,
            libc::SYS_dup,
            libc::SYS_nanosleep,
            libc::SYS_getpid,
            libc::SYS_socket,
            libc::SYS_connect,
            libc::SYS_sendto,
            libc::SYS_recvfrom,
            libc::SYS_bind,
            libc::SYS_listen,
            libc::SYS_clone,
            libc::SYS_exit,
            libc::SYS_wait4,
            libc::SYS_futex,
            libc::SYS_epoll_create,
            libc::SYS_epoll_ctl,
            libc::SYS_epoll_wait,
            libc::SYS_set_tid_address,
            libc::SYS_clock_gettime,
            libc::SYS_exit_group,
            libc::SYS_getrandom,
            libc::SYS_execve,
        ]
    }

    pub fn apply_filter(&self) -> Result<(), SeccompError> {
        #[cfg(target_os = "linux")]
        {
            // Set no_new_privs first (required for seccomp without CAP_SYS_ADMIN)
            let ret = unsafe { libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) };
            if ret < 0 {
                return Err(SeccompError::ApplyError(
                    "prctl PR_SET_NO_NEW_PRIVS failed".to_string(),
                ));
            }
            // Phase 5: Baseline seccomp setup
            // Full BPF filter will be implemented in later phases using libseccomp
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(())
        }
    }

    pub fn apply_to_pid(&self, pid: i32) -> Result<(), SeccompError> {
        // Only support applying to current process in Phase 5
        if pid == unsafe { libc::getpid() } {
            self.apply_filter()
        } else {
            // Applying to other processes requires ptrace or similar
            Err(SeccompError::ApplyError(
                "Applying seccomp to other processes not supported in Phase 5".to_string(),
            ))
        }
    }

    pub fn get_allowed_syscalls(&self) -> &[i64] {
        &self.allowed_syscalls
    }

    pub fn add_syscall(&mut self, syscall: i64) {
        if !self.allowed_syscalls.contains(&syscall) {
            self.allowed_syscalls.push(syscall);
        }
    }

    pub fn remove_syscall(&mut self, syscall: i64) {
        self.allowed_syscalls.retain(|&s| s != syscall);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seccomp_filter_creation() {
        let filter = AndroidSeccompFilter::new();
        assert!(!filter.get_allowed_syscalls().is_empty());
    }

    #[test]
    fn test_add_syscall() {
        let mut filter = AndroidSeccompFilter::new();
        let initial_len = filter.get_allowed_syscalls().len();
        filter.add_syscall(999);
        assert_eq!(filter.get_allowed_syscalls().len(), initial_len + 1);
    }

    #[test]
    fn test_remove_syscall() {
        let mut filter = AndroidSeccompFilter::new();
        let syscall = libc::SYS_read;
        filter.remove_syscall(syscall);
        assert!(!filter.get_allowed_syscalls().contains(&syscall));
    }
}
