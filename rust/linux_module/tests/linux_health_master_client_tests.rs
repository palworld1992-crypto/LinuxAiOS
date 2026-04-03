use linux_module::supervisor::linux_risk_engine::{
    HealthMasterClient, HealthMasterClientImpl, RiskLevel,
};
use scc::ConnectionManager;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[test]
fn test_health_master_client_publish_risk() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let client = HealthMasterClientImpl::new(conn_mgr.clone());

        // Mock: đăng ký một peer "health_master_tunnel" để nhận message
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        conn_mgr.register_peer("health_master_tunnel".to_string(), tx);

        let signature = vec![1u8; 64];
        let result = client
            .publish_risk_level(RiskLevel::Yellow, signature.clone())
            .await;

        // Vì không có thực tế xử lý, nhưng send sẽ thành công vì peer đã đăng ký
        assert!(result.is_ok());

        // Kiểm tra message đã được gửi
        let received = rx.recv().await.unwrap();
        let msg: serde_json::Value = serde_json::from_slice(&received).unwrap();
        assert_eq!(msg["type"], "risk_update");
        assert_eq!(msg["level"], 1); // RiskLevel::Yellow = 1
        assert_eq!(msg["signature"], hex::encode(&signature));
    });
}

#[test]
fn test_health_master_client_publish_risk_no_peer() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let client = HealthMasterClientImpl::new(conn_mgr.clone());

        // Không đăng ký peer, gửi sẽ thất bại
        let result = client.publish_risk_level(RiskLevel::Red, vec![]).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("peer not found") || err_msg.contains("Failed to send"));
    });
}

#[test]
fn test_health_master_client_fetch_reputations() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let client = HealthMasterClientImpl::new(conn_mgr);
        let result = client.fetch_reputations().await;
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_empty());
    });
}
