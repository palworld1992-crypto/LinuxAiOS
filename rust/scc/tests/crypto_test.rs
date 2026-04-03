//! Integration tests for quantum-safe crypto (Kyber, Dilithium, AES-GCM, HMAC)
//! These tests call the Ada/SPARK crypto engine via FFI.

use scc::crypto::{
    aes_gcm_decrypt, aes_gcm_encrypt, dilithium_keypair, dilithium_sign, dilithium_verify,
    hmac_sha256, kyber_decaps, kyber_encaps, kyber_keypair,
};

#[test]
#[ignore = "Requires Ada crypto libraries, may segfault if not available"]
fn test_kyber_keypair_encaps_decaps() {
    let (pubkey, seckey) = kyber_keypair().expect("Kyber keypair generation failed");
    let (ciphertext, shared_secret_enc) =
        kyber_encaps(&pubkey).expect("Kyber encapsulation failed");
    let shared_secret_dec = kyber_decaps(&seckey, &ciphertext).expect("Kyber decapsulation failed");
    assert_eq!(shared_secret_enc, shared_secret_dec);
}

#[test]
#[ignore = "Requires Ada crypto libraries, may segfault if not available"]
fn test_dilithium_sign_verify() {
    let (pubkey, seckey) = dilithium_keypair().expect("Dilithium keypair generation failed");
    let message = b"AIOS test message";
    let signature = dilithium_sign(&seckey, message).expect("Dilithium signing failed");
    assert!(
        dilithium_verify(&pubkey, message, &signature).unwrap_or(false),
        "Signature verification failed"
    );
}

#[test]
#[ignore = "Requires Ada crypto libraries, may segfault if not available"]
fn test_aes_gcm() {
    let key = [0x01u8; 32];
    let plaintext = b"Hello, AIOS!";
    let aad = b"additional data";

    let ciphertext = aes_gcm_encrypt(&key, plaintext, aad).expect("AES-GCM encryption failed");
    let decrypted = aes_gcm_decrypt(&key, &ciphertext, aad).expect("AES-GCM decryption failed");
    assert_eq!(plaintext.as_slice(), decrypted.as_slice());
}

#[test]
#[ignore = "Requires Ada crypto libraries, may segfault if not available"]
fn test_hmac_sha256() {
    let key = [0x02u8; 32];
    let data = b"test data";
    let mac = hmac_sha256(&key, data).expect("HMAC-SHA256 failed");
    assert_eq!(mac.len(), 32);
}
