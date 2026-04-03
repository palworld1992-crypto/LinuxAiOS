use linux_module::supervisor::linux_consensus_client::ConsensusClient;
use linux_module::supervisor::linux_risk_engine::RiskLevel;
use scc::crypto::{dilithium_keypair, kyber_keypair};
use scc::ConnectionManager;
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

#[test]
fn test_consensus_client_creation() {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let (master_kyber_pub, _) = kyber_keypair().unwrap();
        let (_, my_dilithium_priv) = dilithium_keypair().unwrap();

        let _client = ConsensusClient::new(conn_mgr, master_kyber_pub, my_dilithium_priv);
        // Chỉ cần không panic là được
    });
}

#[test]
fn test_submit_vote_no_panic() {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let (master_kyber_pub, _) = kyber_keypair().unwrap();
        let (_, my_dilithium_priv) = dilithium_keypair().unwrap();

        let client = ConsensusClient::new(conn_mgr, master_kyber_pub, my_dilithium_priv);
        client.submit_vote(123, 0.75, RiskLevel::Yellow, 0.85);
        // Không có gửi thật vì master_tunnel chưa đăng ký, nhưng không panic
    });
}

#[test]
fn test_submit_proposal_async() {
    with_temp_base(|| {
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let conn_mgr = Arc::new(ConnectionManager::new());
            let (master_kyber_pub, _) = kyber_keypair().unwrap();
            let (_, my_dilithium_priv) = dilithium_keypair().unwrap();

            let client = ConsensusClient::new(conn_mgr, master_kyber_pub, my_dilithium_priv);
            let proposal_data = b"test proposal".to_vec();
            // Gửi proposal (sẽ gửi đến master_tunnel, nhưng không có peer nên trả lỗi)
            let result = client
                .submit_proposal(proposal_data, RiskLevel::Green, 0.9)
                .await;
            assert!(result.is_err()); // vì master_tunnel chưa đăng ký
        });
    });
}
