//! Linux Main – quản lý tài nguyên, eBPF, memory tiering, ...

mod hardware_monitor;
mod linux_degraded_mode;
mod linux_local_failover;
mod linux_main;
mod linux_snapshot_integration;
mod linux_support;
mod linux_support_context;
mod process_manager;
mod snapshot_manager;

pub use hardware_monitor::HardwareMonitor;
pub use linux_degraded_mode::DegradedMode;
pub use linux_local_failover::{FailoverState, LocalFailover};
pub use linux_main::LinuxMain;
pub use linux_snapshot_integration::SnapshotIntegration;
pub use linux_support::{LinuxSupport, SupportStatus};
pub use linux_support_context::LinuxSupportContext;
pub use process_manager::ProcessManager;
pub use snapshot_manager::SnapshotManager;
