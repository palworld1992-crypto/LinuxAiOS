//! Adaptive Interface Backend – REST API, WebSocket, state cache

pub mod main_component;
pub mod supervisor;
pub mod api;
pub mod assistant;
pub mod support;

pub use main_component::AdaptiveMain;
pub use supervisor::AdaptiveSupervisor;
pub use api::{AppState, AuthMiddleware, Claims, ApiResponse, ModuleStateEntry, StateCache, WebSocketManager};
pub use assistant::{AdaptiveAssistant, AdaptiveLnnPredictor, AdaptiveRlPolicy, UserAccessLog, ProposalNotification};
pub use support::{AdaptiveSupport, AdaptiveSupportContext};

pub fn init() {}
