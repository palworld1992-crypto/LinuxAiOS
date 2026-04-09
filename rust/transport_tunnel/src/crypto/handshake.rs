use hmac_sha256::HMAC;
use scc::crypto::ffi::{self, CryptoError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HandshakeError {
    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),
    #[error("Invalid peer")]
    InvalidPeer,
    #[error("Timeout")]
    Timeout,
    #[error("Crypto library not available")]
    NotAvailable,
    #[error("KDF error: {0}")]
    KdfError(String),
}

pub struct Handshake {
    public_key: Option<Vec<u8>>,
    secret_key: Option<Vec<u8>>,
}

impl Handshake {
    pub fn new() -> Self {
        Self {
            public_key: None,
            secret_key: None,
        }
    }

    pub fn initiate(&mut self) -> Result<Vec<u8>, HandshakeError> {
        let (public_key, secret_key) =
            ffi::kyber_keypair().map_err(|e| HandshakeError::KeyExchangeFailed(e.to_string()))?;

        let (ciphertext, shared_secret) = ffi::kyber_encaps(&public_key)
            .map_err(|e| HandshakeError::KeyExchangeFailed(e.to_string()))?;

        self.public_key = Some(public_key);
        self.secret_key = Some(secret_key);

        let mut initiator_data = ciphertext;
        initiator_data.extend_from_slice(&shared_secret);
        Ok(initiator_data)
    }

    pub fn complete(&self, response: &[u8]) -> Result<[u8; 32], HandshakeError> {
        let secret_key = self
            .secret_key
            .as_ref()
            .ok_or_else(|| HandshakeError::InvalidPeer)?;

        if response.len() < 1088 {
            return Err(HandshakeError::InvalidPeer);
        }

        let ciphertext = &response[..1088];

        let shared_secret = ffi::kyber_decaps(secret_key, ciphertext)
            .map_err(|e| HandshakeError::KeyExchangeFailed(e.to_string()))?;

        if shared_secret.len() != 32 {
            return Err(HandshakeError::KeyExchangeFailed(
                "Invalid shared secret length".to_string(),
            ));
        }

        let mut key = [0u8; 32];
        key.copy_from_slice(&shared_secret[..32]);
        Ok(key)
    }

    pub fn derive_session_key(&self, peer_data: &[u8]) -> Result<[u8; 32], HandshakeError> {
        let shared_secret = if peer_data.len() > 1088 {
            &peer_data[1088..]
        } else {
            return Err(HandshakeError::InvalidPeer);
        };

        let mut info = b"AIOS-Transport-Tunnel-v1".to_vec();
        info.extend_from_slice(peer_data);

        let hmac_result = HMAC::mac(&shared_secret[..32], &info);
        let mut key = [0u8; 32];
        key.copy_from_slice(&hmac_result[..32]);
        Ok(key)
    }

    pub fn get_public_key(&self) -> Option<&[u8]> {
        self.public_key.as_deref()
    }
}

impl Default for Handshake {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handshake_initiate() -> anyhow::Result<()> {
        if !scc::crypto::ffi::is_crypto_available() {
            tracing::warn!("Skipping test - crypto not available");
            return Ok(());
        }

        let mut hs = Handshake::new();
        let initiator_data = hs.initiate()?;

        assert!(
            initiator_data.len() >= 1088 + 32,
            "Initiator data should contain ciphertext + shared_secret"
        );
        assert!(hs.get_public_key().is_some());

        Ok(())
    }

    #[test]
    fn test_handshake_complete() -> anyhow::Result<()> {
        if !scc::crypto::ffi::is_crypto_available() {
            tracing::warn!("Skipping test - crypto not available");
            return Ok(());
        }

        let mut initiator = Handshake::new();
        let mut responder = Handshake::new();

        let init_data = initiator.initiate()?;
        let resp_data = responder.initiate()?;

        let shared_from_init = initiator.complete(&resp_data)?;
        let shared_from_resp = responder.complete(&init_data)?;

        assert_eq!(
            shared_from_init, shared_from_resp,
            "Shared secrets should match"
        );

        Ok(())
    }

    #[test]
    fn test_derive_session_key() -> anyhow::Result<()> {
        if !scc::crypto::ffi::is_crypto_available() {
            tracing::warn!("Skipping test - crypto not available");
            return Ok(());
        }

        let mut hs = Handshake::new();
        let init_data = hs.initiate()?;

        let session_key = hs.derive_session_key(&init_data)?;
        assert_eq!(session_key.len(), 32);

        Ok(())
    }
}
