//! Consensus Client – sends proposals and votes to Master Tunnel.

use crate::supervisor::linux_risk_engine::RiskLevel;
use common::utils::current_timestamp_ms;
use scc::crypto::{aes_gcm_encrypt, dilithium_sign, kyber_encaps};
use scc::ConnectionManager;
use serde_json::json;
use std::sync::Arc;

pub struct ConsensusClient {
    conn_mgr: Arc<ConnectionManager>,
    master_kyber_pub: [u8; 1568],
    my_dilithium_priv: [u8; 4032],
}

impl ConsensusClient {
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

    /// Submit a new proposal to Master Tunnel.
    pub async fn submit_proposal(
        &self,
        proposal_data: Vec<u8>,
        risk_level: RiskLevel,
        reputation_score: f64,
    ) -> anyhow::Result<()> {
        // 1. Build proposal struct
        let proposal = json!({
            "proposal_id": current_timestamp_ms(),
            "data": hex::encode(proposal_data),
            "risk_level": risk_level.as_u8(),
            "reputation": reputation_score,
            "timestamp": current_timestamp_ms(),
        });
        let plain = serde_json::to_vec(&proposal)?;

        // 2. Encrypt with Kyber + AES-GCM
        let (ciphertext, shared_secret) = kyber_encaps(&self.master_kyber_pub)
            .map_err(|e| anyhow::anyhow!("Kyber encaps failed: {}", e))?;
        let key: [u8; 32] = shared_secret
            .try_into()
            .map_err(|_| anyhow::anyhow!("Shared secret size mismatch"))?;
        let aad = b"aios-proposal";
        let encrypted = aes_gcm_encrypt(&key, &plain, aad)
            .map_err(|e| anyhow::anyhow!("AES-GCM encrypt failed: {}", e))?;

        // 3. Sign the encrypted payload with Dilithium
        let signature = dilithium_sign(&self.my_dilithium_priv, &encrypted)
            .map_err(|e| anyhow::anyhow!("Dilithium sign failed: {}", e))?;

        // 4. Package final message
        let final_msg = json!({
            "ciphertext": hex::encode(encrypted),
            "signature": hex::encode(signature),
            "kem_ciphertext": hex::encode(ciphertext),
        });
        let payload = serde_json::to_vec(&final_msg)?;

        // 5. Send via SCC to master_tunnel
        self.conn_mgr
            .send("master_tunnel", payload)
            .map_err(|e| anyhow::anyhow!("Failed to send proposal: {}", e))?;

        Ok(())
    }

    /// Submit a vote (existing, kept for compatibility)
    // ... (phần đầu file giữ nguyên)

    /// Submit a vote (existing, kept for compatibility)
    pub fn submit_vote(
        &self,
        proposal_id: u64,
        risk_score: f64,
        risk_level: RiskLevel,
        reputation_score: f64,
    ) {
        let vote = json!({
            "proposal_id": proposal_id,
            "risk_score": risk_score,
            "risk_level": risk_level.as_u8(),
            "reputation_score": reputation_score,
            "timestamp": current_timestamp_ms(),
        });
        // Sử dụng `?` không thể vì hàm trả về (), nhưng ta vẫn xử lý lỗi bằng `map_err` và log
        match serde_json::to_vec(&vote) {
            Ok(payload) => {
                if let Err(e) = self.conn_mgr.send("master_tunnel", payload) {
                    tracing::error!("Failed to send vote: {}", e);
                } else {
                    tracing::info!(
                        "Vote for proposal {}: risk_score={:.2}, risk_level={:?}, rep={:.2}",
                        proposal_id,
                        risk_score,
                        risk_level,
                        reputation_score
                    );
                }
            }
            Err(e) => tracing::error!("Failed to serialize vote: {}", e),
        }
    }
}
