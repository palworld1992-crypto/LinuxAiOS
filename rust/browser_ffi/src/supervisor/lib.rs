//! Browser Module FFI

pub mod supervisor;

use supervisor::BrowserSupervisor;

pub fn init(conn_mgr: std::sync::Arc<scc::ConnectionManager>) -> BrowserSupervisor {
    BrowserSupervisor::new(conn_mgr)
}