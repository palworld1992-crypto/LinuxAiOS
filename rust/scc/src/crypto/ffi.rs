extern "C" {
    pub fn crypto_aes_gcm_encrypt(
        key: *const u8,
        plaintext: *const u8,
        plaintext_len: usize,
        aad: *const u8,
        aad_len: usize,
        ciphertext_out: *mut *mut u8,
        ciphertext_len: *mut usize,
    ) -> i32;

    pub fn crypto_aes_gcm_decrypt(
        key: *const u8,
        ciphertext: *const u8,
        ciphertext_len: usize,
        aad: *const u8,
        aad_len: usize,
        plaintext_out: *mut *mut u8,
        plaintext_len: *mut usize,
    ) -> i32;

    pub fn crypto_hmac_sha256(
        key: *const u8,
        data: *const u8,
        data_len: usize,
        mac_out: *mut u8,
    ) -> i32;

    pub fn crypto_kyber_keypair(public_key: *mut u8, secret_key: *mut u8) -> i32;

    pub fn crypto_kyber_encaps(
        public_key: *const u8,
        ciphertext: *mut u8,
        shared_secret: *mut u8,
    ) -> i32;

    pub fn crypto_kyber_decaps(
        secret_key: *const u8,
        ciphertext: *const u8,
        shared_secret: *mut u8,
    ) -> i32;

    pub fn crypto_dilithium_keypair(public_key: *mut u8, secret_key: *mut u8) -> i32;

    pub fn crypto_dilithium_sign(
        secret_key: *const u8,
        message: *const u8,
        message_len: usize,
        signature_out: *mut *mut u8,
        signature_len: *mut usize,
    ) -> i32;

    pub fn crypto_dilithium_verify(
        public_key: *const u8,
        message: *const u8,
        message_len: usize,
        signature: *const u8,
        signature_len: usize,
    ) -> i32;

    pub fn crypto_free_buffer(ptr: *mut u8);
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("FFI error: {0}")]
    Ffi(String),
    #[error("Allocation error")]
    Alloc,
    #[error("Crypto library not available")]
    NotAvailable,
}

pub fn is_crypto_available() -> bool {
    unsafe {
        let mut pk = [0u8; 1568];
        let mut sk = [0u8; 2400];
        crypto_kyber_keypair(pk.as_mut_ptr(), sk.as_mut_ptr()) == 0
    }
}

pub fn kyber_keypair() -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut public_key = vec![0u8; 1568];
    let mut secret_key = vec![0u8; 2400];

    let ret = unsafe { crypto_kyber_keypair(public_key.as_mut_ptr(), secret_key.as_mut_ptr()) };

    if ret == 0 {
        Ok((public_key, secret_key))
    } else {
        Err(CryptoError::Ffi(format!("Kyber keypair failed: {}", ret)))
    }
}

pub fn kyber_encaps(public_key: &[u8]) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut ciphertext = vec![0u8; 1088];
    let mut shared_secret = vec![0u8; 32];

    let ret = unsafe {
        crypto_kyber_encaps(
            public_key.as_ptr(),
            ciphertext.as_mut_ptr(),
            shared_secret.as_mut_ptr(),
        )
    };

    if ret == 0 {
        Ok((ciphertext, shared_secret))
    } else {
        Err(CryptoError::Ffi(format!("Kyber encaps failed: {}", ret)))
    }
}

pub fn kyber_decaps(secret_key: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut shared_secret = vec![0u8; 32];

    let ret = unsafe {
        crypto_kyber_decaps(
            secret_key.as_ptr(),
            ciphertext.as_ptr(),
            shared_secret.as_mut_ptr(),
        )
    };

    if ret == 0 {
        Ok(shared_secret)
    } else {
        Err(CryptoError::Ffi(format!("Kyber decaps failed: {}", ret)))
    }
}

pub fn dilithium_keypair() -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut public_key = vec![0u8; 1952];
    let mut secret_key = vec![0u8; 4000];

    let ret = unsafe { crypto_dilithium_keypair(public_key.as_mut_ptr(), secret_key.as_mut_ptr()) };

    if ret == 0 {
        Ok((public_key, secret_key))
    } else {
        Err(CryptoError::Ffi(format!(
            "Dilithium keypair failed: {}",
            ret
        )))
    }
}

pub fn dilithium_sign(secret_key: &[u8], message: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut signature_ptr: *mut u8 = std::ptr::null_mut();
    let mut signature_len: usize = 0;

    let ret = unsafe {
        crypto_dilithium_sign(
            secret_key.as_ptr(),
            message.as_ptr(),
            message.len(),
            &mut signature_ptr,
            &mut signature_len,
        )
    };

    if ret == 0 && !signature_ptr.is_null() {
        let signature =
            unsafe { std::slice::from_raw_parts(signature_ptr, signature_len).to_vec() };
        unsafe { crypto_free_buffer(signature_ptr) };
        Ok(signature)
    } else {
        Err(CryptoError::Ffi(format!("Dilithium sign failed: {}", ret)))
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

    let ret = unsafe {
        crypto_dilithium_verify(
            public_key.as_ptr(),
            message.as_ptr(),
            message.len(),
            signature.as_ptr(),
            signature.len(),
        )
    };

    Ok(ret == 0)
}

pub fn aes_gcm_encrypt(key: &[u8], plaintext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut ciphertext_ptr: *mut u8 = std::ptr::null_mut();
    let mut ciphertext_len: usize = 0;

    let ret = unsafe {
        crypto_aes_gcm_encrypt(
            key.as_ptr(),
            plaintext.as_ptr(),
            plaintext.len(),
            aad.as_ptr(),
            aad.len(),
            &mut ciphertext_ptr,
            &mut ciphertext_len,
        )
    };

    if ret == 0 && !ciphertext_ptr.is_null() {
        let ciphertext =
            unsafe { std::slice::from_raw_parts(ciphertext_ptr, ciphertext_len).to_vec() };
        unsafe { crypto_free_buffer(ciphertext_ptr) };
        Ok(ciphertext)
    } else {
        Err(CryptoError::Ffi(format!("AES-GCM encrypt failed: {}", ret)))
    }
}

pub fn aes_gcm_decrypt(key: &[u8], ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut plaintext_ptr: *mut u8 = std::ptr::null_mut();
    let mut plaintext_len: usize = 0;

    let ret = unsafe {
        crypto_aes_gcm_decrypt(
            key.as_ptr(),
            ciphertext.as_ptr(),
            ciphertext.len(),
            aad.as_ptr(),
            aad.len(),
            &mut plaintext_ptr,
            &mut plaintext_len,
        )
    };

    if ret == 0 && !plaintext_ptr.is_null() {
        let plaintext =
            unsafe { std::slice::from_raw_parts(plaintext_ptr, plaintext_len).to_vec() };
        unsafe { crypto_free_buffer(plaintext_ptr) };
        Ok(plaintext)
    } else {
        Err(CryptoError::Ffi(format!("AES-GCM decrypt failed: {}", ret)))
    }
}

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    if !is_crypto_available() {
        return Err(CryptoError::NotAvailable);
    }

    let mut mac = vec![0u8; 32];

    let ret =
        unsafe { crypto_hmac_sha256(key.as_ptr(), data.as_ptr(), data.len(), mac.as_mut_ptr()) };

    if ret == 0 {
        Ok(mac)
    } else {
        Err(CryptoError::Ffi(format!("HMAC-SHA256 failed: {}", ret)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crypto_available() {
        let available = is_crypto_available();
        println!("Crypto library available: {}", available);
    }

    #[test]
    fn test_kyber_if_available() {
        if !is_crypto_available() {
            eprintln!("Skipping test - crypto not available");
            return;
        }

        let (pubkey, seckey) = kyber_keypair().expect("Kyber keypair failed");
        let (ciphertext, ss_enc) = kyber_encaps(&pubkey).expect("Kyber encaps failed");
        let ss_dec = kyber_decaps(&seckey, &ciphertext).expect("Kyber decaps failed");
        assert_eq!(ss_enc, ss_dec);
    }

    #[test]
    fn test_dilithium_if_available() {
        if !is_crypto_available() {
            eprintln!("Skipping test - crypto not available");
            return;
        }

        let (pubkey, seckey) = dilithium_keypair().expect("Dilithium keypair failed");
        let message = b"AIOS test message";
        let signature = dilithium_sign(&seckey, message).expect("Dilithium sign failed");
        assert!(
            dilithium_verify(&pubkey, message, &signature).expect("Verify failed"),
            "Signature verification failed"
        );
    }

    #[test]
    fn test_aes_if_available() {
        if !is_crypto_available() {
            eprintln!("Skipping test - crypto not available");
            return;
        }

        let key = [0x01u8; 32];
        let plaintext = b"Hello, AIOS!";
        let aad = b"additional data";

        let ciphertext = aes_gcm_encrypt(&key, plaintext, aad).expect("AES-GCM encryption failed");
        let decrypted = aes_gcm_decrypt(&key, &ciphertext, aad).expect("AES-GCM decryption failed");
        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_hmac_if_available() {
        if !is_crypto_available() {
            eprintln!("Skipping test - crypto not available");
            return;
        }

        let key = [0x02u8; 32];
        let data = b"test data";
        let mac = hmac_sha256(&key, data).expect("HMAC-SHA256 failed");
        assert_eq!(mac.len(), 32);
    }
}
