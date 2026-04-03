use linux_module::{HealthTunnelImpl, LinuxMain, LinuxSupervisor, SnapshotManager, TensorPool};
use parking_lot::RwLock;
use scc::ConnectionManager;
use std::sync::Arc;
use tracing::info;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let conn_mgr = Arc::new(ConnectionManager::new());
    let health_tunnel = Arc::new(HealthTunnelImpl::new("linux_module"));
    let snapshot_mgr = Arc::new(SnapshotManager::new(
        std::path::PathBuf::from("/var/lib/aios/snapshots"),
        5,
    ));

    // Correctly initialize TensorPool within RwLock and unwrap the Result from TensorPool::new
    let tensor_pool = Arc::new(RwLock::new(TensorPool::new(
        "linux_model_pool",
        128 * 1024 * 1024,
    )?));

    // Placeholder keys – in production they come from Master Tunnel registration.
    let master_kyber_pub = [0u8; 1568];
    let my_dilithium_priv = [0u8; 4032];

    let supervisor = LinuxSupervisor::new(
        conn_mgr.clone(),
        health_tunnel,
        snapshot_mgr,
        tensor_pool,
        master_kyber_pub,
        my_dilithium_priv,
    );

    let mut main = LinuxMain::new(conn_mgr.clone());

    // Demo proposal handling
    let proposal = linux_module::supervisor::Proposal { id: 1 };
    if let Err(e) = supervisor.handle_proposal(&proposal) {
        info!("Proposal rejected: {}", e);
    } else {
        info!("Proposal accepted");
    }

    // Demo memory tiering
    main.memory_tiering
        .handle_prediction(&[(1, 1234, 0x7f000000, 4096)]);

    // Demo hardware monitor
    main.hardware_monitor.refresh();
    info!("CPU usage: {}%", main.hardware_monitor.cpu_usage());
    info!(
        "Memory: {} / {} MB",
        main.hardware_monitor.memory_used() / 1024 / 1024,
        main.hardware_monitor.memory_total() / 1024 / 1024
    );

    println!("Linux Module demo finished.");
    Ok(()) // ✅ Thêm Ok(()) để trả về Result
}
