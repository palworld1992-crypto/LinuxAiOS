//! System Host – Bootstrap, giám sát, failover, quản lý model

pub mod supervisor;
pub mod health;
pub mod failover;
pub mod scheduler;
pub mod emergency;
pub mod watchdog;
pub mod activator;
pub mod database;
pub mod supervision;
pub mod assistant;
pub mod zig_bindings;
pub mod host_main;

pub use host_main::HostSupport;
pub use supervisor::HostSupervisor;
pub use health::{HostHealthChecker, HealthStatus, HealthAlert, ModuleStatus, AlertLevel, HealthError};
pub use failover::{HostFailoverManager, FailoverState, FailoverEvent, SpikePending, FailoverError};
pub use scheduler::{HostMicroScheduler, SchedulingPolicy, ProcessInfo, SchedulerError};
pub use emergency::{HostEmergencyChannel, EmergencyCommand, EmergencyRequest, EmergencyError};
pub use watchdog::{HostWatchdog, WatchdogError};
pub use activator::{HostModuleActivator, ModuleState, ActivationRequest, ActivationResult, ActivatorError};
pub use database::{HostDatabase, DatabaseEvent, DatabaseError};

use scc::ConnectionManager;
use std::sync::Arc;

pub fn init() -> Arc<HostSupervisor> {
    let conn_mgr = Arc::new(ConnectionManager::new());
    let master_kyber_pub = [0u8; 1568];
    let my_dilithium_priv = [0u8; 4032];
    Arc::new(HostSupervisor::new(
        conn_mgr.clone(),
        master_kyber_pub,
        my_dilithium_priv,
    ))
}
