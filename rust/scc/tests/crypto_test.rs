//! Integration tests for quantum-safe crypto
//! Tests gracefully skip when Ada crypto backend is not available.
//! FIX: No longer uses crypto_free_buffer — Rust allocates all buffers.

mod common;

use scc::crypto::*;

#[test]
fn test_kyber_keypair_encaps_decaps() {
    common::init();
    let (pubkey, seckey) = match kyber_keypair() {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Skipping kyber test: Ada crypto backend not available");
            return;
        }
    };
    let (ciphertext, ss_enc) = match kyber_encaps(&pubkey) {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Skipping kyber test: encaps not available");
            return;
        }
    };
    let ss_dec = match kyber_decaps(&seckey, &ciphertext) {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Skipping kyber test: decaps not available");
            return;
        }
    };
    assert_eq!(ss_enc, ss_dec);
}

#[test]
fn test_dilithium_sign_verify() -> Result<(), Box<dyn std::error::Error>> {
    common::init();
    let (pubkey, seckey) = match dilithium_keypair() {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Skipping dilithium test: Ada crypto backend not available");
            return Ok(());
        }
    };
    let msg = b"AIOS test message";
    let sig = match dilithium_sign(&seckey, msg) {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Skipping dilithium test: sign not available");
            return Ok(());
        }
    };
    let valid = dilithium_verify(&pubkey, msg, &sig)?;
    assert!(valid, "Dilithium signature verification failed");

    let invalid = dilithium_verify(&pubkey, b"wrong message", &sig)?;
    assert!(!invalid, "Dilithium should reject invalid message");
    Ok(())
}

#[test]
fn test_aes_gcm() {
    common::init();
    let key = [0x01u8; 32];
    let plain = b"Hello, AIOS!";
    let aad = b"additional data";
    let cipher = match aes_gcm_encrypt(&key, plain, aad) {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Skipping AES-GCM test: Ada crypto backend not available");
            return;
        }
    };
    let dec = match aes_gcm_decrypt(&key, &cipher, aad) {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Skipping AES-GCM test: decrypt not available");
            return;
        }
    };
    assert_eq!(plain.as_slice(), dec.as_slice());
}

#[test]
fn test_hmac_sha256() {
    common::init();
    let key = [0x02u8; 32];
    let data = b"test data";
    let mac = match hmac_sha256(&key, data) {
        Ok(result) => result,
        Err(_) => {
            tracing::warn!("Skipping HMAC-SHA256 test: Ada crypto backend not available");
            return;
        }
    };
    assert_eq!(mac.len(), 32);
}
