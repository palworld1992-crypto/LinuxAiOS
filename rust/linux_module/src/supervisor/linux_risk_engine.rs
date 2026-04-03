//! Risk Assessment Engine – integrates with Health Master Tunnel.

use anyhow::Result;
use common::health_tunnel::{HealthStatus, HealthTunnel};
use common::utils::current_timestamp_ms;
use parking_lot::RwLock;
use scc::ConnectionManager;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use tracing::{info, warn};

use super::Proposal;
use crate::health_tunnel_impl::HealthTunnelImpl;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RiskLevel {
    Green,  // low risk
    Yellow, // medium risk
    Red,    // high risk
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

/// Trait for client that communicates with Health Master Tunnel.
#[async_trait::async_trait]
pub trait HealthMasterClient: Send + Sync + 'static {
    async fn publish_risk_level(&self, level: RiskLevel, signature: Vec<u8>) -> Result<()>;
    async fn fetch_reputations(&self) -> Result<HashMap<String, Reputation>>;
}
type DynHealthClient = Arc<dyn HealthMasterClient + Send + Sync + 'static>;
/// Concrete client that sends risk levels via SCC to Health Master Tunnel.
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

        // Gửi qua ConnectionManager tới tunnel tương ứng
        self.conn_mgr
            .send("health_master_tunnel", payload)
            .map_err(|e| {
                anyhow::anyhow!("Failed to send risk update to health_master_tunnel: {}", e)
            })
    }

    async fn fetch_reputations(&self) -> Result<HashMap<String, Reputation>> {
        // Mocking: Trong môi trường thực tế, đây sẽ là một request/response qua SCC
        Ok(HashMap::new())
    }
}

// ==================== RiskAssessmentEngine ====================

pub struct RiskAssessmentEngine {
    health_tunnel: Arc<HealthTunnelImpl>,
    health_master_client: RwLock<Option<DynHealthClient>>,
    reputations: RwLock<HashMap<String, Reputation>>,
    current_risk: RwLock<Option<(RiskLevel, Vec<u8>, u64)>>,
}

impl RiskAssessmentEngine {
    pub fn new(health_tunnel: Arc<HealthTunnelImpl>) -> Self {
        Self {
            health_tunnel,
            health_master_client: RwLock::new(None),
            reputations: RwLock::new(HashMap::new()),
            current_risk: RwLock::new(None),
        }
    }

    /// Đăng ký client giao tiếp với Master Tunnel.
    pub fn set_health_master_client(&self, client: DynHealthClient) {
        let mut lock = self.health_master_client.write();
        *lock = Some(client);
    }

    pub fn update_reputation(&self, supervisor_id: &str, new_score: f64) {
        let mut reputations = self.reputations.write();
        reputations.insert(
            supervisor_id.to_string(),
            Reputation {
                score: new_score.clamp(0.0, 1.0),
                last_update: current_timestamp_ms(),
            },
        );
    }

    /// Đánh giá rủi ro (0.0 -> 1.0) dựa trên lịch sử sức khỏe và danh tiếng.
    pub fn evaluate(&self, _proposal: &Proposal, proposer_id: &str) -> f64 {
        let mut score = 0.0;

        // 1. Health của người đề xuất
        if let Some(last_health) = self.health_tunnel.last_health(proposer_id) {
            score += match last_health.status {
                HealthStatus::Healthy => 0.0,
                HealthStatus::Degraded => 0.3,
                HealthStatus::Failed => 0.7,
                HealthStatus::Unknown => 0.5,
                HealthStatus::Supporting => 0.2,
            };
        } else {
            score += 0.5; // Chưa có dữ liệu -> mặc định rủi ro trung bình
        }

        // 2. Danh tiếng (Reputation) của người đề xuất
        if let Some(rep) = self.reputations.read().get(proposer_id) {
            // Danh tiếng càng thấp (gần 0), rủi ro cộng thêm càng cao
            score += (1.0 - rep.score) * 0.3;
        } else {
            score += 0.15;
        }

        // 3. Phân tích sức khỏe tổng thể của cụm giám sát
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

    /// Tính toán và phát tán mức độ rủi ro hiện tại của hệ thống.
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

        // Ký số mức độ rủi ro (Placeholder cho Dilithium)
        let signature = self.sign_risk_level(level)?;

        // Cập nhật bộ nhớ đệm cục bộ
        {
            let mut cache = self.current_risk.write();
            *cache = Some((level, signature.clone(), current_timestamp_ms()));
        }

        // Gửi tới Master Tunnel nếu client đã được đăng ký
        let client_opt = self.health_master_client.read();
        if let Some(client) = client_opt.as_ref() {
            client.publish_risk_level(level, signature).await?;
            info!(target: "risk_engine", "Successfully published risk level {:?} to Master", level);
        } else {
            warn!(target: "risk_engine", "HealthMasterClient not set, skip publishing");
        }

        Ok(())
    }

    pub fn current_risk(&self) -> Option<(RiskLevel, Vec<u8>, u64)> {
        self.current_risk.read().clone()
    }

    fn sign_risk_level(&self, level: RiskLevel) -> Result<Vec<u8>> {
        // TODO: Tích hợp KMS để ký bằng Dilithium
        let _payload = format!("risk:{}:{}", level.as_u8(), current_timestamp_ms());
        let dummy_sig = vec![0u8; 3309];
        Ok(dummy_sig)
    }
}
