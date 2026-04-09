use linux_module::supervisor::linux_consensus_client::ConsensusClient;
use linux_module::supervisor::linux_risk_engine::RiskLevel;
use scc::crypto::{dilithium_keypair, kyber_keypair};
use scc::ConnectionManager;
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

#[test]
fn test_consensus_client_creation() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let (master_kyber_pub, _) = kyber_keypair()?;
        let (_, my_dilithium_priv) = dilithium_keypair()?;

        let mut kyber_arr = [0u8; 1568];
        let mut dilithium_arr = [0u8; 4032];
        kyber_arr.copy_from_slice(&master_kyber_pub);
        dilithium_arr.copy_from_slice(&my_dilithium_priv);

        let _client = ConsensusClient::new(conn_mgr, kyber_arr, dilithium_arr);
        Ok(())
    })
}

#[test]
fn test_submit_vote_no_panic() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let conn_mgr = Arc::new(ConnectionManager::new());
        let (master_kyber_pub, _) = kyber_keypair()?;
        let (_, my_dilithium_priv) = dilithium_keypair()?;

        let mut kyber_arr = [0u8; 1568];
        let mut dilithium_arr = [0u8; 4032];
        kyber_arr.copy_from_slice(&master_kyber_pub);
        dilithium_arr.copy_from_slice(&my_dilithium_priv);

        let client = ConsensusClient::new(conn_mgr, kyber_arr, dilithium_arr);
        client.submit_vote(123, 0.75, RiskLevel::Yellow, 0.85);
        Ok(())
    })
}

#[test]
fn test_submit_proposal_async() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let rt = Runtime::new()?;
        rt.block_on(async {
            let conn_mgr = Arc::new(ConnectionManager::new());
            let (master_kyber_pub, _) = kyber_keypair()?;
            let (_, my_dilithium_priv) = dilithium_keypair()?;

            let mut kyber_arr = [0u8; 1568];
            let mut dilithium_arr = [0u8; 4032];
            kyber_arr.copy_from_slice(&master_kyber_pub);
            dilithium_arr.copy_from_slice(&my_dilithium_priv);

            let client = ConsensusClient::new(conn_mgr, kyber_arr, dilithium_arr);
            let proposal_data = b"test proposal".to_vec();
            let result = client
                .submit_proposal(proposal_data, RiskLevel::Green, 0.9)
                .await;
            assert!(result.is_err());
            Ok::<_, Box<dyn std::error::Error>>(())
        })
    })
}
