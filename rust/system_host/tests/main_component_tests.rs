//! Integration tests for main component (HostMain, HostDegradedMode, HostLocalFailover)

use scc::ConnectionManager;
use std::sync::Arc;
use system_host::main_component::{HostDegradedMode, HostLocalFailover};
use system_host::HostMain;

#[test]
fn test_host_main_creation() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let main = HostMain::new(conn_mgr);

    assert_eq!(main.get_potential(), 1.0);
    assert!(!main.is_degraded());
    assert_eq!(main.get_status(), "normal");
}

#[test]
fn test_host_main_potential_calculation() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let mut main = HostMain::new(conn_mgr);

    main.calculate_potential(0.9, 30.0, 40.0, 0.8);
    assert!(main.get_potential() > 0.0);
    assert!(main.get_potential() <= 1.0);
}

#[test]
fn test_host_main_take_over() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let mut main = HostMain::new(conn_mgr);

    let result = main.take_over();
    assert!(result.is_ok());
}

#[test]
fn test_host_main_delegate_back() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let mut main = HostMain::new(conn_mgr);

    let result = main.delegate_back(12345);
    assert!(result.is_ok());
}

#[test]
fn test_degraded_mode_lifecycle() {
    let dm = HostDegradedMode::new();
    assert!(!dm.is_active());

    dm.enter();
    assert!(dm.is_active());

    dm.exit();
    assert!(!dm.is_active());
}

#[test]
fn test_local_failover_lifecycle() {
    let mut failover = HostLocalFailover::new();

    let result = failover.handle_supervisor_failure();
    assert!(result.is_ok());

    let result = failover.accept_new_supervisor(54321);
    assert!(result.is_ok());
}

#[test]
fn test_init_function() {
    let (supervisor, main) = system_host::init();
    assert!(Arc::strong_count(&supervisor) >= 1);
    assert!(Arc::strong_count(&main) >= 1);
}
