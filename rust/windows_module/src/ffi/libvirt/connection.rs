use std::ffi::{CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;
use std::process::Command;
use tracing::{info, warn};

use super::bindings::{
    virConnectClose, virConnectGetNumOfDomains, virConnectOpen, virGetLastError,
};
use super::error::LibvirtError;
use super::ffi::VirConnect;
use super::types::DomainInfo;

pub struct LibvirtBindings {
    connection: AtomicPtr<VirConnect>,
    connected: AtomicBool,
    uri: OnceLock<String>,
}

// SAFETY: VirConnect is managed through AtomicPtr and accessed from one thread at a time.
unsafe impl Send for LibvirtBindings {}
// SAFETY: AtomicPtr and OnceLock ensure safe concurrent access.
unsafe impl Sync for LibvirtBindings {}

impl LibvirtBindings {
    pub fn new() -> Self {
        Self {
            connection: AtomicPtr::new(ptr::null_mut()),
            connected: AtomicBool::new(false),
            uri: OnceLock::new(),
        }
    }

    pub fn connect(&self, uri: Option<&str>) -> Result<bool, LibvirtError> {
        let uri_str = match uri {
            Some(u) => u.to_string(),
            None => "qemu:///system".to_string(),
        };
        self.uri.set(uri_str.clone()).ok();

        self.attempt_connect(&uri_str)
    }

    fn attempt_connect(&self, uri_str: &str) -> Result<bool, LibvirtError> {
        let c_uri =
            CString::new(uri_str).map_err(|e| LibvirtError::ConnectionFailed(e.to_string()))?;
        info!("Connecting to libvirt: {}", uri_str);

        // SAFETY: c_uri is a valid null-terminated C string. virConnectOpen is thread-safe.
        let conn = super::utils::catch_ffi(|| unsafe { virConnectOpen(c_uri.as_ptr()) })?;

        if conn.is_null() {
            let error = Self::get_last_error();
            warn!("Failed to connect: {}", error);
            self.connected
                .store(false, std::sync::atomic::Ordering::Relaxed);
            Ok(false)
        } else {
            self.connection.store(conn, Ordering::Relaxed);
            self.connected.store(true, Ordering::Relaxed);
            info!("Connected to libvirt");
            Ok(true)
        }
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed) && !self.connection.load(Ordering::Relaxed).is_null()
    }

    pub fn ensure_connected(&self) -> Result<(), LibvirtError> {
        if self.is_connected() {
            return Ok(());
        }

        let uri = self
            .uri
            .get()
            .cloned()
            .unwrap_or_else(|| "qemu:///system".to_string());
        let mut attempts = 0;
        let max_attempts = 10;

        while attempts < max_attempts {
            info!(
                "Attempting to reconnect to libvirt (attempt {}/{})",
                attempts + 1,
                max_attempts
            );
            match self.attempt_connect(&uri) {
                Ok(true) => {
                    info!("Reconnected to libvirt successfully");
                    return Ok(());
                }
                Ok(false) => {
                    warn!(
                        "Connection attempt {} failed, retrying in 1s...",
                        attempts + 1
                    );
                }
                Err(e) => {
                    warn!("Connection error: {}, retrying in 1s...", e);
                }
            }
            attempts += 1;
            thread::sleep(Duration::from_secs(1));
        }

        Err(LibvirtError::ConnectionFailed(format!(
            "Failed to reconnect to libvirt after {} attempts",
            max_attempts
        )))
    }

    pub(crate) fn get_conn(&self) -> *mut VirConnect {
        self.connection.load(Ordering::Relaxed)
    }

    pub fn list_domains(&self) -> Result<Vec<DomainInfo>, LibvirtError> {
        self.ensure_connected()?;

        // Try to enumerate domain names using `virsh list --all --name`, then fetch details
        // via the existing FFI lookup. This avoids adding extra low-level FFI bindings
        // for listing if not available.
        let uri = self
            .uri
            .get()
            .cloned()
            .unwrap_or_else(|| "qemu:///system".to_string());

        let mut cmd = Command::new("virsh");
        if uri != "qemu:///system" {
            cmd.arg("-c").arg(uri);
        }
        cmd.arg("list").arg("--all").arg("--name");

        let output = cmd
            .output()
            .map_err(|e| LibvirtError::OperationFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(LibvirtError::OperationFailed(stderr));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut domains = Vec::new();
        for line in stdout.lines().map(|s| s.trim()).filter(|s| !s.is_empty()) {
            match self.get_domain(line) {
                Ok(Some(info)) => domains.push(info),
                Ok(None) => warn!("Domain listed by virsh but not found via FFI: {}", line),
                Err(e) => warn!("Error fetching domain {}: {}", line, e),
            }
        }

        Ok(domains)
    }

    fn get_domain_by_id(&self, id: i32) -> Result<DomainInfo, LibvirtError> {
        Ok(DomainInfo {
            id: Some(id),
            name: format!("domain-{}", id),
            state: super::types::DomainState::Unknown,
            cpu_time: 0,
            memory_bytes: 0,
        })
    }

    pub(crate) fn get_last_error() -> String {
        // SAFETY: virGetLastError returns thread-local error pointer, safe to call.
        let error_ptr = match super::utils::catch_ffi(|| unsafe { virGetLastError() }) {
            Ok(ptr) => ptr,
            Err(_) => return "Unknown error".to_string(),
        };
        if error_ptr.is_null() {
            return "Unknown error".to_string();
        }
        // SAFETY: error_ptr is non-null (checked above), points to valid VirError.
        let error = unsafe { &*error_ptr };
        if error.message.is_null() {
            return "No error message".to_string();
        }
        // SAFETY: error.message is non-null (checked above), points to valid C string.
        unsafe { CStr::from_ptr(error.message).to_string_lossy().to_string() }
    }

    pub fn disconnect(&self) -> Result<(), LibvirtError> {
        let conn = self.get_conn();
        if !conn.is_null() {
            // SAFETY: conn is valid (non-null check above).
            super::utils::catch_ffi(|| unsafe { virConnectClose(conn) })?;
            self.connection.store(ptr::null_mut(), Ordering::Relaxed);
            self.connected.store(false, Ordering::Relaxed);
            info!("Disconnected from libvirt");
        }
        Ok(())
    }
}

impl Drop for LibvirtBindings {
    fn drop(&mut self) {
        let _ = self.disconnect();
    }
}

impl Default for LibvirtBindings {
    fn default() -> Self {
        Self::new()
    }
}
