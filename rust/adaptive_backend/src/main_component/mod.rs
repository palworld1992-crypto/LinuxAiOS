//! Main component modules for Adaptive Interface

mod adaptive_degraded_mode;
mod adaptive_local_failover;
mod adaptive_main;

pub use adaptive_degraded_mode::AdaptiveDegradedMode;
pub use adaptive_local_failover::AdaptiveLocalFailover;
pub use adaptive_main::AdaptiveMain;
