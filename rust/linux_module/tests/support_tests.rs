use child_tunnel::ChildTunnel;
use common::supervisor_support::{SupervisorSupport, SupportContext, SupportStatus};
use linux_module::main_component::LinuxMain;
use scc::ConnectionManager;
use std::sync::Arc;

#[test]
fn test_support_flow() -> Result<(), Box<dyn std::error::Error>> {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let child_tunnel = Arc::new(ChildTunnel::default());
    let main: Arc<LinuxMain> = Arc::new(LinuxMain::new(conn_mgr, child_tunnel, None));
    let support: Arc<dyn SupervisorSupport> = main.clone();

    assert!(!support.is_supervisor_busy());

    let ctx = SupportContext::MEMORY_TIERING.union(SupportContext::HEALTH_CHECK);
    support.take_over_operations(ctx)?;

    assert_eq!(support.support_status(), SupportStatus::Supporting);

    support.delegate_back_operations()?;

    assert_eq!(support.support_status(), SupportStatus::Idle);

    Ok(())
}
