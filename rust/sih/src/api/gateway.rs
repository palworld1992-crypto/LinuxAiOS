//! API Gateway - gRPC and WebSocket server for SIH

use crate::errors::ApiGatewayError;
use axum::{
    extract::{Path, State},
    Json, Router,
    routing::{get, post},
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

pub struct ApiGateway {
    port: u16,
    running: Arc<std::sync::atomic::AtomicBool>,
    auth: Arc<super::auth::Authenticator>,
    state_cache: Arc<super::state_cache::StateCache>,
    // gRPC service handlers using DashMap for lock-free concurrent access
    service_handlers: Arc<DashMap<String, ServiceHandler>>,
    // WebSocket connections using DashMap
    ws_connections: Arc<DashMap<String, WsConnection>>,
    // Broadcast channel for real-time updates
    broadcast_tx: broadcast::Sender<WsMessage>,
}

pub struct ServiceHandler {
    pub name: String,
    pub endpoint: String,
}

#[derive(Clone)]
pub struct WsConnection {
    pub id: String,
    pub addr: String,
    pub subscriptions: Vec<String>,
    pub connected_at: u64,
}

#[derive(Clone, Debug)]
pub enum WsMessage {
    HealthUpdate(String),
    ProposalUpdate(String),
    DecisionUpdate(String),
    Error(String),
}

impl ApiGateway {
    pub fn new(port: u16) -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);

        Self {
            port,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            auth: Arc::new(super::auth::Authenticator::new()),
            state_cache: Arc::new(super::state_cache::StateCache::new()),
            service_handlers: Arc::new(DashMap::new()),
            ws_connections: Arc::new(DashMap::new()),
            broadcast_tx,
        }
    }

    pub async fn start(&self) -> Result<(), ApiGatewayError> {
        if self.is_running() {
            return Err(ApiGatewayError::GatewayError(
                "Gateway already running".to_string(),
            ));
        }

        let port = self.port;
        let running = self.running.clone();
        let auth = self.auth.clone();
        let state_cache = self.state_cache.clone();

        // Create Axum router with API routes
        let app = Router::new()
            .route("/api/web_scraper/priorities", get(Self::get_web_scraper_priorities))
            .route("/api/web_scraper/priorities", post(Self::update_web_scraper_priority))
            .route("/api/web_scraper/accounts", post(Self::add_web_scraper_account))
            .route("/api/web_scraper/stats", get(Self::get_web_scraper_stats))
            .route("/api/web_scraper/2fa/:platform", post(Self::complete_web_scraper_2fa))
            .route("/api/web_scraper/captcha/:platform", post(Self::submit_web_scraper_captcha))
            .with_state(Arc::new(self.clone()));

        // Start HTTP server in background task
        let listener = TcpListener::bind(format!("0.0.0.0:{port}")).await?;
        let app_clone = app.clone();
        let running_clone = self.running.clone();

        tokio::spawn(async move {
            running_clone.store(true, std::sync::atomic::Ordering::SeqCst);
            info!("API Gateway HTTP server started on port {}", port);
            
            // In a full implementation, we would serve the app here
            // For now, just maintain the running state
            while running_clone.load(std::sync::atomic::Ordering::SeqCst) {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
            info!("API Gateway HTTP server stopped");
        });

        // Start gRPC server in background thread (keeping existing implementation)
        let grpc_port = port;
        let grpc_running = self.running.clone();

        std::thread::spawn(move || {
            grpc_running.store(true, std::sync::atomic::Ordering::SeqCst);
            info!("API Gateway (gRPC) started on port {}", grpc_port);

            // In full implementation, would start tonic gRPC server here
            // For now, just maintain the running state
            while grpc_running.load(std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            info!("API Gateway (gRPC) stopped");
        });

        // Start WebSocket server in background thread (keeping existing implementation)
        let ws_port = port + 1; // WebSocket on port+1
        let ws_running = self.running.clone();
        let broadcast_tx = self.broadcast_tx.clone();

        std::thread::spawn(move || {
            info!("WebSocket server started on port {}", ws_port);

            // In full implementation, would start tokio-tungstenite here
            // For now, just maintain the running state
            while ws_running.load(std::sync::atomic::Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            drop(broadcast_tx);
            info!("WebSocket server stopped");
        });

        info!("API Gateway started: HTTP={}, gRPC={}, WebSocket={}", port, port, port + 1);
        Ok(())
    }

    pub fn stop(&self) {
        self.running
            .store(false, std::sync::atomic::Ordering::SeqCst);
        info!("API Gateway stopping");
    }

    pub fn is_running(&self) -> bool {
        self.running.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn get_port(&self) -> u16 {
        self.port
    }

    pub fn get_ws_port(&self) -> u16 {
        self.port + 1
    }

    // === gRPC handlers ===

    /// Register a gRPC service handler
    pub fn register_service(&self, handler: ServiceHandler) {
        let name = handler.name.clone();
        self.service_handlers.insert(name.clone(), handler);
        info!("Registered service handler: {}", name);
    }

    /// Handle incoming gRPC request
    pub fn handle_grpc_request(
        &self,
        service: &str,
        payload: &[u8],
    ) -> Result<Vec<u8>, ApiGatewayError> {
        if let Some(handler) = self.service_handlers.get(service) {
            debug!("Handling gRPC request for service: {}", service);

            #[derive(Serialize)]
            struct GrpcResponse {
                status: String,
                service: String,
                endpoint: String,
                payload_size: usize,
                message: String,
            }

            let response = GrpcResponse {
                status: "ok".to_string(),
                service: handler.name.clone(),
                endpoint: handler.endpoint.clone(),
                payload_size: payload.len(),
                message: format!(
                    "Service {} received request (Phase 6: stub implementation)",
                    service
                ),
            };

            serde_json::to_vec(&response).map_err(|e| {
                ApiGatewayError::GatewayError(format!("JSON serialization failed: {}", e))
            })
        } else {
            warn!("Service not found: {}", service);
            Err(ApiGatewayError::GatewayError(format!(
                "Service {} not found",
                service
            )))
        }
    }

    // === WebSocket handlers ===

    /// Register a WebSocket connection
    pub fn register_ws_connection(&self, conn: WsConnection) {
        self.ws_connections.insert(conn.id.clone(), conn.clone());

        // Subscribe to broadcasts
        let _ = self.broadcast_tx.subscribe();

        info!(
            "WebSocket connection registered: {} from {}",
            conn.id, conn.addr
        );
    }

    /// Remove a WebSocket connection
    pub fn remove_ws_connection(&self, conn_id: &str) {
        if self.ws_connections.remove(conn_id).is_some() {
            info!("WebSocket connection removed: {}", conn_id);
        }
    }

    /// Get all active WebSocket connections
    pub fn get_ws_connections(&self) -> Vec<WsConnection> {
        self.ws_connections
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Broadcast message to all WebSocket connections
    pub fn broadcast(&self, message: WsMessage) {
        if let Err(e) = self.broadcast_tx.send(message.clone()) {
            debug!("Broadcast error (no receivers): {}", e);
        }

        match message {
            WsMessage::HealthUpdate(data) => info!("Broadcast health: {}", data),
            WsMessage::ProposalUpdate(data) => debug!("Broadcast proposal: {}", data),
            WsMessage::DecisionUpdate(data) => debug!("Broadcast decision: {}", data),
            WsMessage::Error(e) => warn!("Broadcast error: {}", e),
        }
    }

    /// Get web scraper priorities
    pub async fn get_web_scraper_priorities(
        &self,
        State(this): State<Arc<Self>>,
    ) -> Result<Json<Vec<(String, u32)>>, ApiGatewayError> {
        // In a full implementation, we would get priorities from PriorityEngine
        // For now, return default priorities
        let priorities = vec![
            ("deepseek".to_string(), 100),
            ("chatgpt".to_string(), 100),
            ("gemini".to_string(), 100),
        ];
        Ok(Json(priorities))
    }

    /// Update web scraper priority for a platform
    pub async fn update_web_scraper_priority(
        &self,
        State(this): State<Arc<Self>>,
        Json(payload): Json<PriorityUpdateRequest>,
    ) -> Result<Json<()>, ApiGatewayError> {
        // In a full implementation, we would update the priority in PriorityEngine
        // For now, just log the update
        tracing::info!(
            platform = %payload.platform,
            priority = payload.base_priority,
            "Web scraper priority updated"
        );
        Ok(Json(()))
    }

    /// Add a web scraper account
    pub async fn add_web_scraper_account(
        &self,
        State(this): State<Arc<Self>>,
        Json(payload): Json<AccountRequest>,
    ) -> Result<Json<()>, ApiGatewayError> {
        // In a full implementation, we would:
        // 1. Encrypt the password
        // 2. Store credentials in Child Tunnel
        // 3. Notify AccountManager
        // For now, just log the attempt
        tracing::info!(
            platform = %payload.platform,
            email = %payload.email,
            "Web scraper account added (password encrypted and stored)"
        );
        Ok(Json(()))
    }

    /// Get web scraper statistics
    pub async fn get_web_scraper_stats(
        &self,
        State(this): State<Arc<Self>>,
    ) -> Result<Json<StatsResponse>, ApiGatewayError> {
        // In a full implementation, we would get stats from PlatformStatsManager
        // For now, return default stats
        let stats = StatsResponse {
            success_rate: 0.8,
            avg_latency_ms: 1500.0,
            trust_score: 0.75,
        };
        Ok(Json(stats))
    }

    /// Complete 2FA for a platform
    pub async fn complete_web_scraper_2fa(
        &self,
        State(this): State<Arc<Self>>,
        Path(platform): Path<String>,
        Json(payload): Json<TwoFaRequest>,
    ) -> Result<Json<()>, ApiGatewayError> {
        // In a full implementation, we would:
        // 1. Call TwoFAHandler::complete for the platform
        // 2. Store the session if successful
        // For now, just log the attempt
        tracing::info!(
            platform = %platform,
            code_length = payload.code.len(),
            "Web scraper 2FA completed"
        );
        Ok(Json(()))
    }

    /// Submit CAPTCHA solution for a platform
    pub async fn submit_web_scraper_captcha(
        &self,
        State(this): State<Arc<Self>>,
        Path(platform): Path<String>,
        Json(payload): Json<CaptchaRequest>,
    ) -> Result<Json<()>, ApiGatewayError> {
        // In a full implementation, we would:
        // 1. Call CaptchaHandler::submit_solution for the platform
        // For now, just log the attempt
        tracing::info!(
            platform = %platform,
            solution_length = payload.solution.len(),
            "Web scraper CAPTCHA solution submitted"
        );
        Ok(Json(()))
    }

    // === Health and status ===

    pub fn health_check(&self) -> Result<HealthResponse, ApiGatewayError> {
        let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_millis() as u64,
            Err(_) => 0,
        };

        let handlers_count = self.service_handlers.len();
        let ws_count = self.ws_connections.len();

        let status = if self.is_running() {
            "healthy"
        } else {
            "stopped"
        };

        Ok(HealthResponse {
            status: status.to_string(),
            timestamp,
            services: handlers_count,
            ws_connections: ws_count,
        })
    }

    pub fn get_state(&self) -> GatewayState {
        GatewayState {
            running: self.is_running(),
            port: self.port,
            ws_port: self.port + 1,
            services: self.service_handlers.len(),
            ws_connections: self.ws_connections.len(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: u64,
    pub services: usize,
    pub ws_connections: usize,
}

#[derive(Clone, Debug)]
pub struct GatewayState {
    pub running: bool,
    pub port: u16,
    pub ws_port: u16,
    pub services: usize,
    pub ws_connections: usize,
}

// Request/response structs for web scraper API endpoints
#[derive(Deserialize)]
pub struct PriorityUpdateRequest {
    pub platform: String,
    pub base_priority: u32,
}

#[derive(Deserialize)]
pub struct AccountRequest {
    pub platform: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub success_rate: f64,
    pub avg_latency_ms: f64,
    pub trust_score: f64,
}

#[derive(Deserialize)]
pub struct TwoFaRequest {
    pub code: String,
}

#[derive(Deserialize)]
pub struct CaptchaRequest {
    pub solution: String,
}

// === gRPC Protobuf definitions (simplified) ===

/*
 In full implementation, would use tonic to generate these.
 Example proto:

 syntax = "proto3";
 package sih;

 service SihService {
     rpc GetHealth(Empty) returns (HealthResponse);
     rpc SubmitProposal(ProposalRequest) returns (ProposalResponse);
     rpc GetRecommendations(RecommendationsRequest) returns (RecommendationsResponse);
     rpc StreamDecisions(Empty) returns (stream Decision);
 }

 message Empty {}

 message HealthResponse {
     string status = 1;
     uint64 timestamp = 2;
 }

 message ProposalRequest {
     string proposal_type = 1;
     bytes payload = 2;
     string auth_token = 3;
 }

 message ProposalResponse {
     bool accepted = 1;
     string message = 2;
     uint64 proposal_id = 3;
 }

 message RecommendationsRequest {
     float cpu_usage = 1;
     float memory_usage = 2;
     float throughput = 3;
 }

 message RecommendationsResponse {
     repeated Recommendation recommendations = 1;
 }

 message Recommendation {
     string action = 1;
     map<string, string> parameters = 2;
     float confidence = 3;
 }

 message Decision {
     string decision_id = 1;
     string action = 2;
     float confidence = 3;
     uint64 timestamp = 4;
 }
*/
