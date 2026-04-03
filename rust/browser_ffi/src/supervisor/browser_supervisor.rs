//! Browser Supervisor – heartbeat, command handler

use anyhow::{anyhow, Result};
use scc::ConnectionManager;
use std::sync::Arc;

pub struct BrowserSupervisor {
    conn_mgr: Arc<ConnectionManager>,
}

impl BrowserSupervisor {
    pub fn new(conn_mgr: Arc<ConnectionManager>) -> Self {
        Self { conn_mgr }
    }

    pub async fn handle_heartbeat(&self) -> Result<()> {
        // Gửi heartbeat qua SCC
        Ok(())
    }

    pub async fn discard_tab(&self, tab_id: u64) -> Result<()> {
        // Tạo CString, tránh unwrap
        let command = std::ffi::CString::new("discard_tab")
            .map_err(|e| anyhow!("Failed to create CString: {}", e))?;
        // SAFETY: Hàm browser_receive_command là FFI từ C++, chỉ nhận con trỏ CString hợp lệ.
        // command.as_ptr() trỏ đến bộ nhớ tĩnh hợp lệ, không bị giải phóng trong quá trình gọi.
        unsafe {
            browser_receive_command(command.as_ptr(), tab_id);
        }
        Ok(())
    }
}

extern "C" {
    fn browser_receive_command(command: *const std::os::raw::c_char, param: u64);
}