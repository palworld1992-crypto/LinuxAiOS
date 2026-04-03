//! Handshake Hybrid KEM (Kyber) + KDF (HMAC‑SHA256) để thỏa thuận khóa phiên.

use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use scc::crypto::{dilithium_sign, dilithium_verify, kyber_decaps, kyber_encaps};
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// Khóa phiên dùng cho AES‑256‑GCM.
#[derive(Clone)]
pub struct SessionKey {
    pub key: [u8; 32],
    pub nonce_base: [u8; 12],
    pub expiration: u64,
}

impl SessionKey {
    pub fn from_master_secret(master: &[u8]) -> Result<Self> {
        let mut mac =
            HmacSha256::new_from_slice(master).map_err(|e| anyhow!("HMAC init failed: {}", e))?;
        mac.update(b"aes-key");
        let key = mac.finalize().into_bytes();
        let mut key_arr = [0u8; 32];
        key_arr.copy_from_slice(&key[..32]);

        let mut mac =
            HmacSha256::new_from_slice(master).map_err(|e| anyhow!("HMAC init failed: {}", e))?;
        mac.update(b"nonce-base");
        let nonce = mac.finalize().into_bytes();
        let mut nonce_arr = [0u8; 12];
        nonce_arr.copy_from_slice(&nonce[..12]);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(Self {
            key: key_arr,
            nonce_base: nonce_arr,
            expiration: now + 60_000,
        })
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        now >= self.expiration
    }
}

pub fn client_handshake(
    peer_kyber_pub: &[u8; 1568],
    my_dilithium_priv: &[u8; 4032],
) -> Result<(SessionKey, Vec<u8>)> {
    let (ciphertext, shared_secret) =
        kyber_encaps(peer_kyber_pub).map_err(|e| anyhow!("Kyber encaps failed: {}", e))?;
    let signature = dilithium_sign(my_dilithium_priv, &ciphertext)
        .map_err(|e| anyhow!("Dilithium sign failed: {}", e))?;

    let session = SessionKey::from_master_secret(&shared_secret)?;

    let mut handshake_msg = Vec::with_capacity(1312 + 3309);
    handshake_msg.extend_from_slice(&ciphertext);
    handshake_msg.extend_from_slice(&signature);
    Ok((session, handshake_msg))
}

pub fn server_handshake(
    my_kyber_priv: &[u8; 2400],
    peer_dilithium_pub: &[u8; 1952],
    handshake_msg: &[u8],
) -> Result<SessionKey> {
    if handshake_msg.len() != 1312 + 3309 {
        return Err(anyhow!("Invalid handshake message length"));
    }
    let (ciphertext, signature) = handshake_msg.split_at(1312);
    let ciphertext_arr: [u8; 1312] = ciphertext
        .try_into()
        .map_err(|_| anyhow!("ciphertext size mismatch"))?;

    let signature_arr: [u8; 3309] = signature
        .try_into()
        .map_err(|_| anyhow!("signature size mismatch"))?;

    let verified = dilithium_verify(peer_dilithium_pub, ciphertext, &signature_arr)
        .map_err(|e| anyhow!("Dilithium verify failed: {}", e))?;
    if !verified {
        return Err(anyhow!("Dilithium signature verification failed"));
    }

    let shared_secret = kyber_decaps(my_kyber_priv, &ciphertext_arr)
        .map_err(|e| anyhow!("Kyber decaps failed: {}", e))?;

    Ok(SessionKey::from_master_secret(&shared_secret)?)
}
