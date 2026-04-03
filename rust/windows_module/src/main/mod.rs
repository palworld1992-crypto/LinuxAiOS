//! Windows Main component

mod windows_degraded_mode;
mod windows_local_failover;
mod windows_main; // ← file windows_main.rs (đã rename)
pub mod windows_support;
pub mod windows_support_context;

pub use windows_degraded_mode::WindowsDegradedMode;
pub use windows_local_failover::WindowsLocalFailover;
pub use windows_main::WindowsMain;
