use linux_module::main_component::{LinuxMain, LinuxSupport, LinuxSupportContext, SupportStatus};
use parking_lot::RwLock;
use scc::ConnectionManager;
use std::sync::Arc;

#[test]
fn test_support_flow() {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let main = Arc::new(RwLock::new(LinuxMain::new(conn_mgr)));
    let mut support = LinuxSupport::new(main.clone());
    assert!(!support.is_supervisor_busy()); // giả sử supervisor không bận
    let ctx = LinuxSupportContext {
        memory_tiering: true,
        health_check: true,
        cgroups: false,
    };
    support.take_over_operations(ctx).unwrap();
    assert_eq!(support.support_status(), SupportStatus::Supporting);
    support.delegate_back_operations().unwrap();
    assert_eq!(support.support_status(), SupportStatus::Idle);
}
