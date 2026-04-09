use aes::Aes256Gcm;
use ctr::Ctr128BE;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Invalid key length")]
    InvalidKeyLength,
    #[error("Invalid nonce")]
    InvalidNonce,
}

pub struct ChannelCrypto {
    key: [u8; 32],
    nonce: std::sync::atomic::AtomicU64,
}

impl ChannelCrypto {
    pub fn new(key: [u8; 32]) -> Self {
        Self {
            key,
            nonce: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let nonce_val = self
            .nonce
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut nonce = [0u8; 12];
        nonce[..8].copy_from_slice(&nonce_val.to_be_bytes());

        let cipher =
            Aes256Gcm::new_from_slice(&self.key).map_err(|_| CryptoError::InvalidKeyLength)?;

        let nonce = aes::cipher::Nonce::<Aes256Gcm>::from_slice(&nonce);
        let tag = cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        let mut result = Vec::with_capacity(12 + plaintext.len() + 16);
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&tag);
        Ok(result)
    }

    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if ciphertext.len() < 28 {
            return Err(CryptoError::InvalidNonce);
        }

        let mut nonce = [0u8; 12];
        nonce.copy_from_slice(&ciphertext[..12]);

        let cipher =
            Aes256Gcm::new_from_slice(&self.key).map_err(|_| CryptoError::InvalidKeyLength)?;

        let nonce = aes::cipher::Nonce::<Aes256Gcm>::from_slice(&nonce);
        cipher
            .decrypt(nonce, &ciphertext[12..])
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn test_encrypt_decrypt() -> Result<()> {
        let key = [0u8; 32];
        let crypto = ChannelCrypto::new(key);

        let plaintext = b"Hello, World!";
        let encrypted = crypto.encrypt(plaintext)?;
        let decrypted = crypto.decrypt(&encrypted)?;

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
        Ok(())
    }
}
