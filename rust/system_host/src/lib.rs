//! System Host – Bootstrap, giám sát, failover, quản lý model

pub mod main_component;
pub mod supervisor;

pub use main_component::HostMain;
pub use supervisor::HostSupervisor;

use scc::ConnectionManager;
use std::sync::Arc;

pub fn init() -> (Arc<HostSupervisor>, Arc<HostMain>) {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let master_kyber_pub = [0u8; 1568];
    let my_dilithium_priv = [0u8; 4032];
    let supervisor = Arc::new(HostSupervisor::new(
        conn_mgr.clone(),
        master_kyber_pub,
        my_dilithium_priv,
    ));
    let main = Arc::new(HostMain::new(conn_mgr.clone()));
    (supervisor, main)
}
