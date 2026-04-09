use scc::BlockchainLightClient;

#[test]
fn test_blockchain_light_client_creation() {
    let _client = BlockchainLightClient::new();
}

#[test]
fn test_blockchain_light_client_default() {
    let _client = BlockchainLightClient::default();
}
