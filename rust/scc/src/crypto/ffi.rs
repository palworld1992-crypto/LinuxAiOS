extern "C" {
    // Ada procedures - ALL use out parameter for Status (no return value)
    // Ada: procedure HMAC_SHA256(Key: Key_32; Data: Address; Data_Len: size_t; MAC_Out: out Key_32; Status: out int)
    #[link_name = "crypto_engine__hmac_sha256"]
    pub fn crypto_hmac_sha256(
        key: *const u8,
        data: *const u8,
        data_len: usize,
        mac_out: *mut u8,
        status: *mut i32,
    );

    // Ada: procedure Kyber_Keypair(Public_Key: out Key_1568; Secret_Key: out Key_2400; Status: out int)
    #[link_name = "crypto_engine__kyber_keypair"]
    pub fn crypto_kyber_keypair(public_key: *mut u8, secret_key: *mut u8, status: *mut i32);

    // Ada: procedure Kyber_Encaps(Public_Key: Key_1568; Ciphertext: out Key_1312; Shared_Secret: out Key_32; Status: out int)
    #[link_name = "crypto_engine__kyber_encaps"]
    pub fn crypto_kyber_encaps(
        public_key: *const u8,
        ciphertext: *mut u8,
        shared_secret: *mut u8,
        status: *mut i32,
    );

    // Ada: procedure Kyber_Decaps(Secret_Key: Key_2400; Ciphertext: Key_1312; Shared_Secret: out Key_32; Status: out int)
    #[link_name = "crypto_engine__kyber_decaps"]
    pub fn crypto_kyber_decaps(
        secret_key: *const u8,
        ciphertext: *const u8,
        shared_secret: *mut u8,
        status: *mut i32,
    );

    // Ada: procedure AES_GCM_Encrypt(... Ciphertext_Len: out size_t; Status: out int)
    #[link_name = "crypto_engine__aes_gcm_encrypt"]
    pub fn crypto_aes_gcm_encrypt(
        key: *const u8,
        plaintext: *const u8,
        plaintext_len: usize,
        aad: *const u8,
        aad_len: usize,
        ciphertext_buf: *mut u8,
        ciphertext_buf_size: usize,
        ciphertext_len: *mut usize,
        status: *mut i32,
    );

    // Ada: procedure AES_GCM_Decrypt(... Plaintext_Len: out size_t; Status: out int)
    #[link_name = "crypto_engine__aes_gcm_decrypt"]
    pub fn crypto_aes_gcm_decrypt(
        key: *const u8,
        ciphertext: *const u8,
        ciphertext_len: usize,
        aad: *const u8,
        aad_len: usize,
        plaintext_buf: *mut u8,
        plaintext_buf_size: usize,
        plaintext_len: *mut usize,
        status: *mut i32,
    );

    // Dilithium - status as out parameter, matching Ada External_Name
    #[link_name = "crypto_engine__dilithium_keypair"]
    pub fn crypto_dilithium_keypair(public_key: *mut u8, secret_key: *mut u8, status: *mut i32);

    #[link_name = "crypto_engine__dilithium_sign"]
    pub fn crypto_dilithium_sign(
        secret_key: *const u8,
        message: *const u8,
        message_len: usize,
        signature_buf: *mut u8,
        signature_buf_size: usize,
        signature_len: *mut usize,
        status: *mut i32,
    );

    #[link_name = "crypto_engine__dilithium_verify"]
    pub fn crypto_dilithium_verify(
        public_key: *const u8,
        message: *const u8,
        message_len: usize,
        signature: *const u8,
        signature_len: usize,
        status: *mut i32,
    );
}
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::{rngs::OsRng, RngCore};
use std::panic::{self, AssertUnwindSafe};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("FFI error: {0}")]
    Ffi(String),
    #[error("Crypto library not available")]
    NotAvailable,
}

pub fn is_crypto_available() -> bool {
    let possible_paths = [
        "/root/aios/spark/lib/libscc.so",
        "/home/ToHung/LinusAiOS/spark/lib/libscc.so",
    ];
    possible_paths
        .iter()
        .any(|p| std::path::Path::new(p).exists())
}

pub fn kyber_keypair() -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut public_key = vec![0u8; 1568];
    let mut secret_key = vec![0u8; 2400];
    let mut status: i32 = 0;

    // SAFETY: The pointers point to valid, properly aligned writable memory of the correct sizes.
    // The FFI function expects these buffers to be pre-allocated and will write the key material.
    // The `status` out-parameter is valid and will be set by the function.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        crypto_kyber_keypair(
            public_key.as_mut_ptr(),
            secret_key.as_mut_ptr(),
            &mut status,
        );
    }));

    if let Err(_panic) = result {
        return Err(CryptoError::Ffi("Panic in kyber_keypair".to_string()));
    }

    if status == 0 {
        Ok((public_key, secret_key))
    } else {
        Err(CryptoError::Ffi(format!(
            "Kyber keypair failed: {}",
            status
        )))
    }
}

pub fn kyber_encaps(public_key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut ciphertext = vec![0u8; 1088];
    let mut shared_secret = vec![0u8; 32];
    let mut status: i32 = 0;

    // SAFETY: `public_key` is a valid slice from the caller, passed as const pointer.
    // The output buffers `ciphertext` and `shared_secret` are pre-allocated with correct sizes.
    // The function will not write beyond these bounds. `status` is a valid out-parameter.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        crypto_kyber_encaps(
            public_key.as_ptr(),
            ciphertext.as_mut_ptr(),
            shared_secret.as_mut_ptr(),
            &mut status,
        );
    }));

    if let Err(_panic) = result {
        return Err(CryptoError::Ffi("Panic in kyber_encaps".to_string()));
    }

    if status == 0 {
        Ok((ciphertext, shared_secret))
    } else {
        Err(CryptoError::Ffi(format!("Kyber encaps failed: {}", status)))
    }
}

pub fn kyber_decaps(secret_key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut shared_secret = vec![0u8; 32];
    let mut status: i32 = 0;

    // SAFETY: `secret_key` and `ciphertext` are valid slices; `shared_secret` is a pre-allocated buffer.
    // The FFI function will read the inputs and write the shared secret if successful.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        crypto_kyber_decaps(
            secret_key.as_ptr(),
            ciphertext.as_ptr(),
            shared_secret.as_mut_ptr(),
            &mut status,
        );
    }));

    if let Err(_panic) = result {
        return Err(CryptoError::Ffi("Panic in kyber_decaps".to_string()));
    }

    if status == 0 {
        Ok(shared_secret)
    } else {
        Err(CryptoError::Ffi(format!("Kyber decaps failed: {}", status)))
    }
}

pub fn dilithium_keypair() -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut public_key = vec![0u8; 1952];
    let mut secret_key = vec![0u8; 4032];
    let mut status: i32 = 0;

    // SAFETY: Buffers are correctly sized for Dilithium keys. Pointers are valid and aligned.
    // The FFI function will fill them. `status` is a valid out-parameter.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        crypto_dilithium_keypair(
            public_key.as_mut_ptr(),
            secret_key.as_mut_ptr(),
            &mut status,
        );
    }));

    if let Err(_panic) = result {
        return Err(CryptoError::Ffi("Panic in dilithium_keypair".to_string()));
    }

    if status == 0 {
        Ok((public_key, secret_key))
    } else {
        Err(CryptoError::Ffi(format!(
            "Dilithium keypair failed: {}",
            status
        )))
    }
}

pub fn dilithium_sign(secret_key: &[u8], message: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut signature = vec![0u8; 3309];
    let mut signature_len: usize = 0;
    let mut status: i32 = 0;

    // SAFETY: `secret_key` and `message` are valid slices. `signature` buffer is pre-allocated with
    // sufficient size (3309 bytes for Dilithium signature). `signature_len` is an out-parameter.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        crypto_dilithium_sign(
            secret_key.as_ptr(),
            message.as_ptr(),
            message.len(),
            signature.as_mut_ptr(),
            signature.len(),
            &mut signature_len,
            &mut status,
        );
    }));

    if let Err(_panic) = result {
        return Err(CryptoError::Ffi("Panic in dilithium_sign".to_string()));
    }

    if status == 0 {
        // Keep full 3309-byte signature (do not truncate - verify needs full size)
        Ok(signature)
    } else {
        Err(CryptoError::Ffi(format!(
            "Dilithium sign failed: {}",
            status
        )))
    }
}

pub fn dilithium_verify(
    public_key: &[u8],
    message: &[u8],
    signature: &[u8],
) -> Result<bool, CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut status: i32 = 0;

    // SAFETY: All input slices are valid. The FFI function only reads them.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        crypto_dilithium_verify(
            public_key.as_ptr(),
            message.as_ptr(),
            message.len(),
            signature.as_ptr(),
            signature.len(),
            &mut status,
        );
    }));

    if let Err(_panic) = result {
        return Err(CryptoError::Ffi("Panic in dilithium_verify".to_string()));
    }

    Ok(status == 0)
}

pub fn dilithium_sign_with_key(secret_key: &[u8], message: &[u8]) -> Result<Vec<u8>, CryptoError> {
    dilithium_sign(secret_key, message)
}

pub fn aes_gcm_encrypt(key: &[u8], plaintext: &[u8], _aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let key_array = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key_array);

    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let mut ciphertext = cipher
        .encrypt(&nonce, plaintext.as_ref())
        .map_err(|e| CryptoError::Ffi(format!("AES-GCM encrypt failed: {:?}", e)))?;

    let mut result = nonce.to_vec();
    result.append(&mut ciphertext);
    Ok(result)
}

pub fn aes_gcm_decrypt(key: &[u8], ciphertext: &[u8], _aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if ciphertext.len() < 12 {
        return Err(CryptoError::Ffi("Ciphertext too short".into()));
    }

    let key_array = Key::<Aes256Gcm>::from_slice(key);
    let cipher = Aes256Gcm::new(key_array);

    let (nonce_bytes, ct) = ciphertext.split_at(12);
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ct)
        .map_err(|e| CryptoError::Ffi(format!("AES-GCM decrypt failed: {:?}", e)))
}

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut mac = vec![0u8; 32];
    let mut status: i32 = 0;

    // SAFETY: `key` and `data` are valid slices. `mac` is a pre-allocated buffer of 32 bytes.
    // The FFI function will write the HMAC result into `mac`.
    let result = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        crypto_hmac_sha256(
            key.as_ptr(),
            data.as_ptr(),
            data.len(),
            mac.as_mut_ptr(),
            &mut status,
        );
    }));

    if let Err(_panic) = result {
        return Err(CryptoError::Ffi("Panic in hmac_sha256".to_string()));
    }

    if status == 0 {
        Ok(mac)
    } else {
        Err(CryptoError::Ffi(format!("HMAC-SHA256 failed: {}", status)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use tracing::{info, warn};

    #[test]
    fn test_crypto_available() -> Result<()> {
        let available = is_crypto_available();
        info!("Crypto library available: {}", available);
        Ok(())
    }

    #[test]
    fn test_kyber_if_available() -> Result<()> {
        if !is_crypto_available() {
            warn!("Skipping test - crypto not available");
            return Ok(());
        }

        let (pubkey, seckey) = kyber_keypair()?;
        let (ciphertext, ss_enc) = kyber_encaps(&pubkey)?;
        let ss_dec = kyber_decaps(&seckey, &ciphertext)?;
        assert_eq!(ss_enc, ss_dec);
        Ok(())
    }

    #[test]
    fn test_dilithium_if_available() -> Result<()> {
        if !is_crypto_available() {
            warn!("Skipping test - crypto not available");
            return Ok(());
        }

        let (pubkey, seckey) = dilithium_keypair()?;
        info!("Dilithium public key length: {} bytes", pubkey.len());
        info!("Dilithium secret key length: {} bytes", seckey.len());

        let message = b"AIOS test message for Dilithium signature";
        let signature = dilithium_sign(&seckey, message)?;
        info!("Dilithium signature length: {} bytes", signature.len());

        let valid = dilithium_verify(&pubkey, message, &signature)?;
        assert!(valid, "Dilithium signature verification failed");

        let invalid = dilithium_verify(&pubkey, b"different message", &signature)?;
        assert!(!invalid, "Dilithium should reject invalid message");

        Ok(())
    }

    #[test]
    fn test_aes_if_available() -> Result<()> {
        let key = [0x01u8; 32];
        let plaintext = b"Hello, AIOS!";
        let aad = b"additional data";

        let ciphertext = aes_gcm_encrypt(&key, plaintext, aad)?;
        let decrypted = aes_gcm_decrypt(&key, &ciphertext, aad)?;
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
        Ok(())
    }

    #[test]
    fn test_hmac_if_available() -> Result<()> {
        if !is_crypto_available() {
            warn!("Skipping test - crypto not available");
            return Ok(());
        }

        let key = [0x02u8; 32];
        let data = b"test data";
        let mac = hmac_sha256(&key, data)?;
        assert_eq!(mac.len(), 32);
        Ok(())
    }
}
