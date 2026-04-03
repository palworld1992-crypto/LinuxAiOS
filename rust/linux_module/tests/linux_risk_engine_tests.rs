use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use common::utils::current_timestamp_ms;
use linux_module::health_tunnel_impl::HealthTunnelImpl;
use linux_module::supervisor::linux_risk_engine::{
    HealthMasterClient, RiskAssessmentEngine, RiskLevel,
};
use linux_module::supervisor::Proposal;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn with_temp_base<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let temp_dir = tempfile::tempdir().unwrap();
    let base_path = temp_dir.path().to_str().unwrap();
    std::env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    std::env::remove_var("AIOS_BASE_DIR");
    result
}

struct MockHealthMasterClient {
    published_risk: Arc<RwLock<Option<(RiskLevel, Vec<u8>)>>>,
}

impl MockHealthMasterClient {
    fn new() -> Self {
        Self {
            published_risk: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl HealthMasterClient for MockHealthMasterClient {
    async fn publish_risk_level(&self, level: RiskLevel, signature: Vec<u8>) -> anyhow::Result<()> {
        *self.published_risk.write() = Some((level, signature));
        Ok(())
    }

    async fn fetch_reputations(
        &self,
    ) -> anyhow::Result<HashMap<String, linux_module::supervisor::linux_risk_engine::Reputation>>
    {
        Ok(HashMap::new())
    }
}

#[test]
fn test_risk_engine_evaluate() {
    with_temp_base(|| {
        let health_tunnel = Arc::new(HealthTunnelImpl::new("test_module"));
        let engine = RiskAssessmentEngine::new(health_tunnel.clone());

        // Ghi một số health record cho proposer và các module khác
        let record = HealthRecord {
            module_id: "linux".to_string(),
            timestamp: current_timestamp_ms(),
            status: HealthStatus::Healthy,
            details: vec![],
        };
        health_tunnel.record_health(record).unwrap();

        let record2 = HealthRecord {
            module_id: "windows".to_string(),
            timestamp: current_timestamp_ms(),
            status: HealthStatus::Degraded,
            details: vec![],
        };
        health_tunnel.record_health(record2).unwrap();

        // Cập nhật reputation cho proposer
        engine.update_reputation("linux", 0.8);

        let proposal = Proposal { id: 1 };
        let score = engine.evaluate(&proposal, "linux");
        assert!(score >= 0.0 && score <= 1.0);
        // Score phải nhỏ hơn ngưỡng vì health healthy + reputation cao
        assert!(score < 0.5);
    });
}

#[test]
fn test_risk_engine_high_risk() {
    with_temp_base(|| {
        let health_tunnel = Arc::new(HealthTunnelImpl::new("test_module"));
        let engine = RiskAssessmentEngine::new(health_tunnel.clone());

        // Proposer có health failed
        let record = HealthRecord {
            module_id: "linux".to_string(),
            timestamp: current_timestamp_ms(),
            status: HealthStatus::Failed,
            details: vec![],
        };
        health_tunnel.record_health(record).unwrap();

        // Reputation thấp
        engine.update_reputation("linux", 0.2);

        // Các module khác đều degraded
        for module in [
            "windows",
            "android",
            "sih",
            "system_host",
            "browser",
            "adaptive",
        ] {
            let rec = HealthRecord {
                module_id: module.to_string(),
                timestamp: current_timestamp_ms(),
                status: HealthStatus::Degraded,
                details: vec![],
            };
            health_tunnel.record_health(rec).unwrap();
        }

        let proposal = Proposal { id: 1 };
        let score = engine.evaluate(&proposal, "linux");
        assert!(score > 0.7);
        let level = engine.compute_risk_level(score);
        assert_eq!(level, RiskLevel::Red);
    });
}

#[test]
fn test_publish_current_risk() {
    with_temp_base(|| {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let health_tunnel = Arc::new(HealthTunnelImpl::new("test_module"));
            let engine = RiskAssessmentEngine::new(health_tunnel.clone());

            // Ghi health cho tất cả module (mặc định Unknown)
            for module in [
                "linux",
                "windows",
                "android",
                "sih",
                "system_host",
                "browser",
                "adaptive",
            ] {
                let rec = HealthRecord {
                    module_id: module.to_string(),
                    timestamp: current_timestamp_ms(),
                    status: HealthStatus::Unknown,
                    details: vec![],
                };
                health_tunnel.record_health(rec).unwrap();
            }

            let mock_client = Arc::new(MockHealthMasterClient::new());
            engine.set_health_master_client(mock_client.clone());

            engine.publish_current_risk().await.unwrap();

            let published = mock_client.published_risk.read();
            assert!(published.is_some());
            let (level, _) = published.as_ref().unwrap();
            // Với tất cả Unknown, avg_score = 0.5 => risk level Yellow
            assert_eq!(*level, RiskLevel::Yellow);
        });
    });
}

#[test]
fn test_current_risk_cache() {
    with_temp_base(|| {
        let health_tunnel = Arc::new(HealthTunnelImpl::new("test_module"));
        let engine = RiskAssessmentEngine::new(health_tunnel);
        assert!(engine.current_risk().is_none());

        // Sau khi publish, cache được cập nhật
        // Thực tế cần chạy publish_current_risk, nhưng trong test ta có thể set mock
        // Ở đây chỉ test cache ban đầu
    });
}
