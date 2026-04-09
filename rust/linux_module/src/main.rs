use child_tunnel::ChildTunnel;
use dashmap::DashMap;
use health_master_tunnel::{run_server, HealthMasterServer};
use linux_module::{
    HealthTunnelImpl, LinuxMain, LinuxSupervisor, SnapshotManager, SupervisorSharedState,
    TensorPool,
};
use scc::connection::IncomingMessage;
use scc::ConnectionManager;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    scc::init_ada();

    let conn_mgr = Arc::new(ConnectionManager::new());
    let health_tunnel = Arc::new(HealthTunnelImpl::new("linux_module"));
    let snapshot_mgr = Arc::new(SnapshotManager::new(
        std::path::PathBuf::from("/var/lib/aios/snapshots"),
        5,
    ));
    let shared_state: Arc<SupervisorSharedState> = Arc::new(SupervisorSharedState::new());

    let tensor_pool: Arc<DashMap<(), TensorPool>> = Arc::new(DashMap::with_capacity(1));
    match TensorPool::new("linux_model_pool", 128 * 1024 * 1024) {
        Ok(pool) => {
            tensor_pool.insert((), pool);
        }
        Err(e) => {
            anyhow::bail!("Failed to create TensorPool: {}", e);
        }
    }

    let health_master_server = Arc::new(HealthMasterServer::new());
    let (tx, rx) = mpsc::unbounded_channel::<IncomingMessage>();
    conn_mgr.register_handler("health_master_tunnel", tx);

    let server = health_master_server.clone();
    tokio::spawn(async move {
        run_server(server, rx).await;
    });

    let master_kyber_pub = [0u8; 1568];
    let my_dilithium_priv = [0u8; 4032];

    let supervisor = LinuxSupervisor::new(
        conn_mgr.clone(),
        health_tunnel.clone(),
        snapshot_mgr,
        tensor_pool,
        master_kyber_pub,
        my_dilithium_priv,
        Some(shared_state.clone()),
    );

    let child_tunnel = Arc::new(ChildTunnel::default());
    let mut main = LinuxMain::new(conn_mgr.clone(), child_tunnel, Some(shared_state.clone()));
    main.set_health_tunnel(health_tunnel.clone());

    let proposal = linux_module::supervisor::Proposal { id: 1 };
    if let Err(e) = supervisor.handle_proposal(&proposal) {
        info!("Proposal rejected: {}", e);
    } else {
        info!("Proposal accepted");
    }

    main.memory_tiering
        .handle_prediction(&[(1, 1234, 0x7f000000, 4096)]);

    main.hardware_monitor.refresh();
    info!("CPU usage: {}%", main.hardware_monitor.cpu_usage());
    info!(
        "Memory: {} / {} MB",
        main.hardware_monitor.memory_used() / 1024 / 1024,
        main.hardware_monitor.memory_total() / 1024 / 1024
    );

    info!("Linux Module demo finished.");
    Ok(())
}
