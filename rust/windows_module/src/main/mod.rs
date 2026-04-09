//! Windows Main component

mod windows_degraded_mode;
mod windows_local_failover;
mod windows_main;
mod windows_support;
mod windows_support_context;

pub use windows_degraded_mode::WindowsDegradedMode;
pub use windows_local_failover::WindowsLocalFailover;
pub use windows_main::WindowsMain;
pub use windows_support::WindowsSupport;
pub use windows_support_context::WindowsSupportContext;