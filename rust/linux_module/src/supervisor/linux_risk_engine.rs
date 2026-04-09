//! Risk Assessment Engine – integrates with Health Master Tunnel.

use anyhow::Result;
use common::health_tunnel::{HealthStatus, HealthTunnel};
use common::utils::current_timestamp_ms;
use dashmap::DashMap;
use scc::ConnectionManager;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use tracing::{info, warn};

use super::Proposal;
use crate::health_tunnel_impl::HealthTunnelImpl;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RiskLevel {
    Green,
    Yellow,
    Red,
}

impl RiskLevel {
    pub fn from_score(score: f64) -> Self {
        if score < 0.3 {
            RiskLevel::Green
        } else if score < 0.7 {
            RiskLevel::Yellow
        } else {
            RiskLevel::Red
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            RiskLevel::Green => 0,
            RiskLevel::Yellow => 1,
            RiskLevel::Red => 2,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Reputation {
    pub score: f64,
    pub last_update: u64,
}

#[async_trait::async_trait]
pub trait HealthMasterClient: Send + Sync + 'static {
    async fn publish_risk_level(&self, level: RiskLevel, signature: Vec<u8>) -> Result<()>;
    async fn fetch_reputations(&self) -> Result<HashMap<String, Reputation>>;
}

type DynHealthClient = Arc<dyn HealthMasterClient + Send + Sync + 'static>;

pub struct HealthMasterClientImpl {
    conn_mgr: Arc<ConnectionManager>,
}

impl HealthMasterClientImpl {
    pub fn new(conn_mgr: Arc<ConnectionManager>) -> Self {
        Self { conn_mgr }
    }
}

#[async_trait::async_trait]
impl HealthMasterClient for HealthMasterClientImpl {
    async fn publish_risk_level(&self, level: RiskLevel, signature: Vec<u8>) -> Result<()> {
        let msg = json!({
            "type": "risk_update",
            "level": level.as_u8(),
            "signature": hex::encode(signature),
            "timestamp": current_timestamp_ms(),
        });
        let payload = serde_json::to_vec(&msg)?;

        self.conn_mgr
            .send("health_master_tunnel", payload)
            .map_err(|e| anyhow::anyhow!("Failed to send risk update to health_master_tunnel: {}", e))
    }

    async fn fetch_reputations(&self) -> Result<HashMap<String, Reputation>> {
        Err(anyhow::anyhow!("fetch_reputations not implemented until Phase 4"))
    }
}

pub struct RiskAssessmentEngine {
    health_tunnel: Arc<HealthTunnelImpl>,
    health_master_client: DashMap<String, DynHealthClient>,
    reputations: DashMap<String, Reputation>,
    current_risk: DashMap<String, (RiskLevel, Vec<u8>, u64)>,
}

impl RiskAssessmentEngine {
    pub fn new(health_tunnel: Arc<HealthTunnelImpl>) -> Self {
        Self {
            health_tunnel,
            health_master_client: DashMap::new(),
            reputations: DashMap::new(),
            current_risk: DashMap::new(),
        }
    }

    pub fn set_health_master_client(&self, client: DynHealthClient) {
        self.health_master_client.insert("client".to_string(), client);
    }

    pub fn update_reputation(&self, supervisor_id: &str, new_score: f64) {
        self.reputations.insert(
            supervisor_id.to_string(),
            Reputation {
                score: new_score.clamp(0.0, 1.0),
                last_update: current_timestamp_ms(),
            },
        );
    }

    pub fn evaluate(&self, _proposal: &Proposal, proposer_id: &str) -> f64 {
        let mut score = 0.0;

        if let Some(last_health) = self.health_tunnel.last_health(proposer_id) {
            score += match last_health.status {
                HealthStatus::Healthy => 0.0,
                HealthStatus::Degraded => 0.3,
                HealthStatus::Failed => 0.7,
                HealthStatus::Unknown => 0.5,
                HealthStatus::Supporting => 0.2,
            };
        } else {
            score += 0.5;
        }

        if let Some(rep) = self.reputations.get(proposer_id) {
            score += (1.0 - rep.score) * 0.3;
        } else {
            score += 0.15;
        }

        let all_modules = [
            "linux",
            "windows",
            "android",
            "sih",
            "system_host",
            "browser",
            "adaptive",
        ];
        let mut cluster_risk = 0.0;
        let mut count = 0;

        for module in all_modules {
            if module == proposer_id {
                continue;
            }
            if let Some(rec) = self.health_tunnel.last_health(module) {
                cluster_risk += match rec.status {
                    HealthStatus::Healthy => 0.0,
                    HealthStatus::Degraded => 0.1,
                    HealthStatus::Failed => 0.4,
                    HealthStatus::Unknown => 0.2,
                    HealthStatus::Supporting => 0.1,
                };
                count += 1;
            }
        }

        if count > 0 {
            score += (cluster_risk / count as f64) * 0.2;
        }

        score.clamp(0.0, 1.0)
    }

    pub fn compute_risk_level(&self, score: f64) -> RiskLevel {
        RiskLevel::from_score(score)
    }

    pub async fn publish_current_risk(&self) -> Result<()> {
        let modules = [
            "linux",
            "windows",
            "android",
            "sih",
            "system_host",
            "browser",
            "adaptive",
        ];
        let mut total_score = 0.0;

        for module in modules {
            total_score += if let Some(rec) = self.health_tunnel.last_health(module) {
                match rec.status {
                    HealthStatus::Healthy => 0.0,
                    HealthStatus::Degraded => 0.3,
                    HealthStatus::Failed => 0.7,
                    _ => 0.5,
                }
            } else {
                0.5
            };
        }

        let avg_score = total_score / modules.len() as f64;
        let level = self.compute_risk_level(avg_score);

        let signature = self.sign_risk_level(level)?;

        self.current_risk
            .insert("current".to_string(), (level, signature.clone(), current_timestamp_ms()));

        if let Some(client) = self.health_master_client.get("client") {
            client.publish_risk_level(level, signature).await?;
            info!(target: "risk_engine", "Successfully published risk level {:?} to Master", level);
        } else {
            warn!(target: "risk_engine", "HealthMasterClient not set, skip publishing");
        }

        Ok(())
    }

    pub fn current_risk(&self) -> Option<(RiskLevel, Vec<u8>, u64)> {
        self.current_risk.get("current").map(|r| r.value().clone())
    }

    fn sign_risk_level(&self, level: RiskLevel) -> Result<Vec<u8>> {
        let payload = format!("risk:{}:{}", level.as_u8(), current_timestamp_ms());
        let secret_key = [0u8; 4032];
        scc::crypto::dilithium_sign(&secret_key, payload.as_bytes())
            .map_err(|e| anyhow::anyhow!("Dilithium signing failed: {}", e))
    }
}
