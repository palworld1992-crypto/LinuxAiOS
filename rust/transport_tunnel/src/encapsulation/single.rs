use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Key,
};
use anyhow::anyhow;
use std::sync::atomic::{AtomicU64, Ordering}; // thêm import

pub struct SingleEncapsulator;

impl SingleEncapsulator {
    pub fn encapsulate(
        key: &[u8; 32],
        nonce_base: &[u8; 12],
        counter: &AtomicU64,
        payload: &[u8],
        aad: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        let ctr = counter.fetch_add(1, Ordering::Relaxed);
        let mut nonce_bytes = *nonce_base;
        for i in 0..8 {
            nonce_bytes[12 - 8 + i] ^= (ctr >> (i * 8)) as u8;
        }
        let nonce = GenericArray::from_slice(&nonce_bytes);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

        let ciphertext = cipher
            .encrypt(nonce, Payload { msg: payload, aad })
            .map_err(|e| anyhow!("AES-GCM encryption failed: {}", e))?;
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        Ok(result)
    }

    pub fn decapsulate(key: &[u8; 32], data: &[u8], aad: &[u8]) -> Option<Vec<u8>> {
        if data.len() < 12 {
            return None;
        }
        let (nonce_bytes, ciphertext) = data.split_at(12);
        let nonce = GenericArray::from_slice(nonce_bytes);
        let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));

        cipher
            .decrypt(
                nonce,
                Payload {
                    msg: ciphertext,
                    aad,
                },
            )
            .ok()
    }
}
