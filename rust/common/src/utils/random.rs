use rand::RngCore;

pub fn random_bytes(len: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; len];
    // SECURITY: Use OsRng for cryptographic operations
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    bytes
}
