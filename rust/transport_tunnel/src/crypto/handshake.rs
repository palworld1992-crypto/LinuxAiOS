use thiserror::Error;

#[derive(Error, Debug)]
pub enum HandshakeError {
    #[error("Key exchange failed")]
    KeyExchangeFailed,
    #[error("Invalid peer")]
    InvalidPeer,
    #[error("Timeout")]
    Timeout,
}

pub struct Handshake;

impl Handshake {
    pub fn new() -> Self {
        Self
    }

    pub fn initiate(&self) -> Result<Vec<u8>, HandshakeError> {
        Ok(vec![0u8; 32])
    }

    pub fn complete(&self, _response: &[u8]) -> Result<[u8; 32], HandshakeError> {
        Ok([0u8; 32])
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
    fn test_handshake() {
        let hs = Handshake::new();
        let init = hs.initiate().unwrap();
        assert_eq!(init.len(), 32);
    }
}
