use anyhow::bail;
use linux_module::supervisor::linux_risk_engine::{
    HealthMasterClient, HealthMasterClientImpl, RiskLevel,
};
use scc::ConnectionManager;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[test]
fn test_health_master_client_publish_risk() -> Result<(), Box<dyn std::error::Error>> {
    let rt = Runtime::new()?;
    rt.block_on(async {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let client = HealthMasterClientImpl::new(conn_mgr.clone());

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        conn_mgr.register_peer("health_master_tunnel".to_string(), tx);

        let signature = vec![1u8; 64];
        let result = client
            .publish_risk_level(RiskLevel::Yellow, signature.clone())
            .await;

        assert!(result.is_ok());

        let received = rx.recv().await.ok_or("No message received")?;
        let msg: serde_json::Value = serde_json::from_slice(&received)?;
        assert_eq!(msg["type"], "risk_update");
        assert_eq!(msg["level"], 1);
        assert_eq!(msg["signature"], hex::encode(&signature));
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}

#[test]
fn test_health_master_client_publish_risk_no_peer() -> anyhow::Result<()> {
    let rt = Runtime::new()?;
    rt.block_on(async {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let client = HealthMasterClientImpl::new(conn_mgr.clone());

        let result = client.publish_risk_level(RiskLevel::Red, vec![]).await;
        assert!(result.is_err());
        let err = match result {
            Ok(_) => bail!("expected error"),
            Err(e) => e,
        };
        let err_msg = err.to_string();
        assert!(err_msg.contains("peer not found") || err_msg.contains("Failed to send"));
        Ok(())
    })
}

#[test]
fn test_health_master_client_fetch_reputations() -> Result<(), Box<dyn std::error::Error>> {
    let rt = Runtime::new()?;
    rt.block_on(async {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let client = HealthMasterClientImpl::new(conn_mgr);
        let result = client.fetch_reputations().await;
        assert!(result.is_ok());
        let map = result?;
        assert!(map.is_empty());
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}
