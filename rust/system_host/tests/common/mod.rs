//! Common test utilities for system_host integration tests

use std::time::Duration;

/// Wait for async operations to settle
pub fn settle() {
    std::thread::sleep(Duration::from_millis(50));
}

/// Generate a random HMAC key for testing
pub fn test_hmac_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    for (i, b) in key.iter_mut().enumerate() {
        *b = (i % 256) as u8;
    }
    key
}

/// Generate a random Kyber public key for testing
pub fn test_kyber_pub() -> [u8; 1568] {
    [0u8; 1568]
}

/// Generate a random Dilithium private key for testing
pub fn test_dilithium_priv() -> [u8; 4032] {
    [0u8; 4032]
}
