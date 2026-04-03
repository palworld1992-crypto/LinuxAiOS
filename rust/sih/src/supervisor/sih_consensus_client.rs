//! Consensus client for SIH

use anyhow::{anyhow, Result};
use common::utils::current_timestamp_ms;
use scc::crypto::{aes_gcm_encrypt, dilithium_sign, kyber_encaps};
use scc::ConnectionManager;
use serde_json::json;
use std::sync::Arc;

pub struct SihConsensusClient {
    conn_mgr: Arc<ConnectionManager>,
    master_kyber_pub: [u8; 1568],
    my_dilithium_priv: [u8; 4032],
}

impl SihConsensusClient {
    pub fn new(
        conn_mgr: Arc<ConnectionManager>,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
    ) -> Self {
        Self {
            conn_mgr,
            master_kyber_pub,
            my_dilithium_priv,
        }
    }

    pub async fn submit_proposal(&self, proposal_data: Vec<u8>) -> Result<()> {
        let proposal = json!({
            "proposal_id": current_timestamp_ms(),
            "data": hex::encode(proposal_data),
            "timestamp": current_timestamp_ms(),
        });
        let plain = serde_json::to_vec(&proposal)?;
        let (ciphertext, shared_secret) = kyber_encaps(&self.master_kyber_pub)
            .map_err(|e| anyhow!("Kyber encaps failed: {}", e))?;
        let key: [u8; 32] = shared_secret
            .try_into()
            .map_err(|_| anyhow!("Shared secret size mismatch"))?;
        let encrypted = aes_gcm_encrypt(&key, &plain, b"aios-proposal")
            .map_err(|e| anyhow!("AES-GCM encrypt failed: {}", e))?;
        let signature = dilithium_sign(&self.my_dilithium_priv, &encrypted)
            .map_err(|e| anyhow!("Dilithium sign failed: {}", e))?;
        let final_msg = json!({
            "ciphertext": hex::encode(encrypted),
            "signature": hex::encode(signature),
            "kem_ciphertext": hex::encode(ciphertext),
        });
        let payload = serde_json::to_vec(&final_msg)?;
        self.conn_mgr
            .send("master_tunnel", payload)
            .map_err(|e| anyhow!("Failed to send proposal: {}", e))?;
        Ok(())
    }
}
