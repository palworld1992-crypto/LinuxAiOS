use common::utils::current_timestamp_ms;
use scc::crypto::{aes_gcm_encrypt, dilithium_sign, kyber_encaps};
use serde_json::json;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("Failed to sign proposal: {0}")]
    SigningError(String),
    #[error("Failed to send proposal: {0}")]
    SendError(String),
    #[error("Consensus rejected: {0}")]
    Rejected(String),
}

pub struct AndroidConsensusClient {
    conn_mgr: Arc<scc::ConnectionManager>,
    module_id: String,
    master_kyber_pub: [u8; 1568],
    my_dilithium_priv: [u8; 4032],
}

impl AndroidConsensusClient {
    pub fn new(
        conn_mgr: Arc<scc::ConnectionManager>,
        module_id: &str,
        master_kyber_pub: [u8; 1568],
        my_dilithium_priv: [u8; 4032],
    ) -> Self {
        Self {
            conn_mgr,
            module_id: module_id.to_string(),
            master_kyber_pub,
            my_dilithium_priv,
        }
    }

    pub fn send_proposal(
        &self,
        proposal_type: &str,
        metadata: &serde_json::Value,
    ) -> Result<String, ConsensusError> {
        // 1. Build proposal struct
        let proposal = json!({
            "module_id": self.module_id,
            "proposal_type": proposal_type,
            "timestamp": current_timestamp_ms(),
            "metadata": metadata,
        });
        let plain =
            serde_json::to_vec(&proposal).map_err(|e| ConsensusError::SendError(e.to_string()))?;

        // 2. Encrypt with Kyber + AES-GCM
        let (ciphertext, shared_secret) = kyber_encaps(&self.master_kyber_pub)
            .map_err(|e| ConsensusError::SendError(format!("Kyber encaps failed: {}", e)))?;
        let key: [u8; 32] = shared_secret
            .try_into()
            .map_err(|_| ConsensusError::SendError("Shared secret size mismatch".to_string()))?;
        let aad = b"android-proposal";
        let encrypted = aes_gcm_encrypt(&key, &plain, aad)
            .map_err(|e| ConsensusError::SendError(format!("AES-GCM encrypt failed: {}", e)))?;

        // 3. Sign the encrypted payload with Dilithium
        let signature = dilithium_sign(&self.my_dilithium_priv, &encrypted)
            .map_err(|e| ConsensusError::SigningError(format!("Dilithium sign failed: {}", e)))?;

        // 4. Package final message
        let final_msg = json!({
            "ciphertext": hex::encode(encrypted),
            "signature": hex::encode(signature),
            "kem_ciphertext": hex::encode(ciphertext),
        });
        let payload =
            serde_json::to_vec(&final_msg).map_err(|e| ConsensusError::SendError(e.to_string()))?;

        // 5. Send via SCC to master_tunnel
        self.conn_mgr
            .send("master_tunnel", payload)
            .map_err(|e| ConsensusError::SendError(format!("Failed to send proposal: {}", e)))?;

        Ok(format!(
            "proposal-{}-{}",
            self.module_id,
            uuid::Uuid::new_v4()
        ))
    }

    pub fn get_module_id(&self) -> &str {
        &self.module_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_client_creation() {
        let conn_mgr = Arc::new(scc::ConnectionManager::new());
        let master_kyber_pub = [0u8; 1568];
        let my_dilithium_priv = [0u8; 4032];
        let client = AndroidConsensusClient::new(
            conn_mgr,
            "android-01",
            master_kyber_pub,
            my_dilithium_priv,
        );
        assert_eq!(client.get_module_id(), "android-01");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consensus_client_creation() {
        let client = AndroidConsensusClient::new("android-01");
        assert_eq!(client.get_module_id(), "android-01");
    }
}
