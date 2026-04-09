//! API module for Adaptive Backend

mod auth;
mod routes;
mod state_cache;
mod websocket;

pub use auth::{AuthMiddleware, Claims};
pub use routes::{create_router, AppState, ApiResponse};
pub use state_cache::{ModuleStateEntry, StateCache};
pub use websocket::WebSocketManager;
