//! Integration tests for quantum-safe crypto

mod common;

use scc::crypto::*;

#[test]
fn test_kyber_keypair_encaps_decaps() {
    common::init();
    let (pubkey, seckey) = kyber_keypair().expect("Kyber keypair failed");
    let (ciphertext, ss_enc) = kyber_encaps(&pubkey).expect("Encaps failed");
    let ss_dec = kyber_decaps(&seckey, &ciphertext).expect("Decaps failed");
    assert_eq!(ss_enc, ss_dec);
}

#[test]
fn test_dilithium_sign_verify() {
    common::init();
    let (pubkey, seckey) = dilithium_keypair().expect("Dilithium keypair failed");
    let msg = b"AIOS test message";
    let sig = dilithium_sign(&seckey, msg).expect("Sign failed");
    assert!(dilithium_verify(&pubkey, msg, &sig).unwrap_or(false));
}

#[test]
fn test_aes_gcm() {
    common::init();
    let key = [0x01u8; 32];
    let plain = b"Hello, AIOS!";
    let aad = b"additional data";
    let cipher = aes_gcm_encrypt(&key, plain, aad).expect("Encrypt failed");
    let dec = aes_gcm_decrypt(&key, &cipher, aad).expect("Decrypt failed");
    assert_eq!(plain.as_slice(), dec.as_slice());
}

#[test]
fn test_hmac_sha256() {
    common::init();
    let key = [0x02u8; 32];
    let data = b"test data";
    let mac = hmac_sha256(&key, data).expect("HMAC failed");
    assert_eq!(mac.len(), 32);
}
