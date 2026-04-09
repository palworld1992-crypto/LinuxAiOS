use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use common::utils::current_timestamp_ms;
use dashmap::DashMap;
use linux_module::health_tunnel_impl::HealthTunnelImpl;
use linux_module::supervisor::linux_risk_engine::{
    HealthMasterClient, RiskAssessmentEngine, RiskLevel,
};
use linux_module::supervisor::Proposal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn with_temp_base<F, T>(f: F) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<T, Box<dyn std::error::Error>>,
{
    let temp_dir = tempfile::tempdir()?;
    let base_path = temp_dir.path().to_str().ok_or("Invalid path")?;
    std::env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    std::env::remove_var("AIOS_BASE_DIR");
    result
}

type RiskSignature = Vec<u8>;
type RiskRecord = (RiskLevel, RiskSignature);

struct MockHealthMasterClient {
    published_risk: Arc<DashMap<(), Option<RiskRecord>>>,
}

impl MockHealthMasterClient {
    fn new() -> Self {
        Self {
            published_risk: Arc::new(DashMap::with_capacity(1)),
        }
    }
}

#[async_trait::async_trait]
impl HealthMasterClient for MockHealthMasterClient {
    async fn publish_risk_level(&self, level: RiskLevel, signature: Vec<u8>) -> anyhow::Result<()> {
        self.published_risk.insert((), Some((level, signature)));
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
fn test_risk_engine_evaluate() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let health_tunnel = Arc::new(HealthTunnelImpl::new("test_module"));
        let engine = RiskAssessmentEngine::new(health_tunnel.clone());

        let record = HealthRecord {
            module_id: "linux".to_string(),
            timestamp: current_timestamp_ms(),
            status: HealthStatus::Healthy,
            potential: 1.0,
            details: vec![],
        };
        health_tunnel.record_health(record)?;

        let record2 = HealthRecord {
            module_id: "windows".to_string(),
            timestamp: current_timestamp_ms(),
            status: HealthStatus::Degraded,
            potential: 0.5,
            details: vec![],
        };
        health_tunnel.record_health(record2)?;

        engine.update_reputation("linux", 0.8);

        let proposal = Proposal { id: 1 };
        let score = engine.evaluate(&proposal, "linux");
        assert!((0.0..=1.0).contains(&score));
        assert!(score < 0.5);
        Ok(())
    })
}

#[test]
fn test_risk_engine_high_risk() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let health_tunnel = Arc::new(HealthTunnelImpl::new("test_module"));
        let engine = RiskAssessmentEngine::new(health_tunnel.clone());

        let record = HealthRecord {
            module_id: "linux".to_string(),
            timestamp: current_timestamp_ms(),
            status: HealthStatus::Failed,
            potential: 0.0,
            details: vec![],
        };
        health_tunnel.record_health(record)?;

        engine.update_reputation("linux", 0.2);

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
                potential: 0.5,
                details: vec![],
            };
            health_tunnel.record_health(rec)?;
        }

        let proposal = Proposal { id: 1 };
        let score = engine.evaluate(&proposal, "linux");
        assert!(score > 0.7);
        let level = engine.compute_risk_level(score);
        assert_eq!(level, RiskLevel::Red);
        Ok(())
    })
}

#[test]
fn test_publish_current_risk() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let rt = Runtime::new()?;
        rt.block_on(async {
            let health_tunnel = Arc::new(HealthTunnelImpl::new("test_module"));
            let engine = RiskAssessmentEngine::new(health_tunnel.clone());

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
                    potential: 0.5,
                    details: vec![],
                };
                health_tunnel.record_health(rec)?;
            }

            let mock_client = Arc::new(MockHealthMasterClient::new());
            engine.set_health_master_client(mock_client.clone());

            engine.publish_current_risk().await?;

            let published = mock_client.published_risk.get(&());
            assert!(published.is_some());
            let value = published.ok_or("No risk published")?;
            let (level, _) = value.value().as_ref().ok_or("No risk published")?;
            assert_eq!(*level, RiskLevel::Yellow);
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    })
}

#[test]
fn test_current_risk_cache() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let health_tunnel = Arc::new(HealthTunnelImpl::new("test_module"));
        let engine = RiskAssessmentEngine::new(health_tunnel);
        assert!(engine.current_risk().is_none());
        Ok(())
    })
}
