//! REST API Routes - axum endpoints for adaptive interface

use axum::{
    extract::{Path, Query, State},
    response::Json,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::auth::AuthMiddleware;
use super::state_cache::{ModuleStateEntry, StateCache};
use super::websocket::WebSocketManager;

#[derive(Clone)]
pub struct AppState {
    pub state_cache: Arc<StateCache>,
    pub ws_manager: Arc<WebSocketManager>,
    pub auth: Arc<AuthMiddleware>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProposalRequest {
    pub id: String,
    pub module_id: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModeChangeRequest {
    pub mode: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", axum::routing::get(get_health))
        .route("/api/proposals", axum::routing::get(get_proposals))
        .route(
            "/api/proposals/:id/approve",
            axum::routing::post(approve_proposal),
        )
        .route(
            "/api/proposals/:id/reject",
            axum::routing::post(reject_proposal),
        )
        .route("/api/mode", axum::routing::get(get_mode))
        .route("/api/mode", axum::routing::post(change_mode))
        .route(
            "/api/failover/:module",
            axum::routing::post(trigger_failover),
        )
        .route("/api/logs", axum::routing::get(get_logs))
        .with_state(state)
}

async fn get_health(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<ModuleStateEntry>>> {
    let entries = state.state_cache.get_all_entries();
    Json(ApiResponse {
        success: true,
        data: Some(entries),
        error: None,
    })
}

async fn get_proposals(
    State(_state): State<AppState>,
) -> Json<ApiResponse<Vec<ProposalRequest>>> {
    Json(ApiResponse {
        success: true,
        data: Some(vec![]),
        error: None,
    })
}

async fn approve_proposal(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Json<ApiResponse<()>> {
    Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

async fn reject_proposal(
    State(_state): State<AppState>,
    Path(_id): Path<String>,
) -> Json<ApiResponse<()>> {
    Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

async fn get_mode(
    State(_state): State<AppState>,
) -> Json<ApiResponse<String>> {
    Json(ApiResponse {
        success: true,
        data: Some("Balanced".to_string()),
        error: None,
    })
}

async fn change_mode(
    State(_state): State<AppState>,
    Json(_req): Json<ModeChangeRequest>,
) -> Json<ApiResponse<()>> {
    Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

async fn trigger_failover(
    State(_state): State<AppState>,
    Path(_module): Path<String>,
) -> Json<ApiResponse<()>> {
    Json(ApiResponse {
        success: true,
        data: Some(()),
        error: None,
    })
}

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub module: Option<String>,
    pub level: Option<String>,
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

async fn get_logs(
    State(_state): State<AppState>,
    Query(query): Query<LogQuery>,
) -> Json<ApiResponse<Vec<String>>> {
    let _module_filter = query.module.as_deref();
    let _level_filter = query.level.as_deref();
    let page = match query.page {
        Some(p) => p,
        None => 1,
    };
    let limit = match query.limit {
        Some(l) => l,
        None => 50,
    };
    
    // TODO(Phase 5): Implement real log retrieval from log store
    let logs: Vec<String> = (0..limit)
        .skip((page - 1) * limit)
        .map(|i| format!("log_entry_{}", i))
        .collect();
    
    Json(ApiResponse {
        success: true,
        data: Some(logs),
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response() -> anyhow::Result<()> {
        let resp = ApiResponse {
            success: true,
            data: Some("test".to_string()),
            error: None,
        };
        assert!(resp.success);
        assert!(resp.data.is_some());
        assert!(resp.error.is_none());
        Ok(())
    }

    #[test]
    fn test_api_response_error() -> anyhow::Result<()> {
        let resp: ApiResponse<String> = ApiResponse {
            success: false,
            data: None,
            error: Some("test error".to_string()),
        };
        assert!(!resp.success);
        assert!(resp.data.is_none());
        assert!(resp.error.is_some());
        Ok(())
    }
}
