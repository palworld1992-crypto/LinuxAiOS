//! Adaptive Interface Backend – REST API, WebSocket, state cache

pub mod main_component;
pub mod supervisor;

pub use main_component::AdaptiveMain;
pub use supervisor::AdaptiveSupervisor;

pub fn init() {}
