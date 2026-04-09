use thiserror::Error;

#[derive(Error, Debug)]
pub enum TokenError {
    #[error("System time error")]
    TimeError,
    #[error("Signature size mismatch")]
    SignatureSize,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct IntentToken {
    pub signal_type: u8,
    pub urgency: u8,
    pub module_id: [u8; 32],
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl IntentToken {
    pub fn new(module_id: [u8; 32], signal_type: u8, urgency: u8) -> Result<Self, TokenError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs())
            .ok_or(TokenError::TimeError)?;

        Ok(Self {
            signal_type,
            urgency,
            module_id,
            timestamp,
            signature: vec![0u8; 2420],
        })
    }

    pub fn is_valid(&self) -> Result<bool, TokenError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_secs())
            .ok_or(TokenError::TimeError)?;

        Ok(now.saturating_sub(self.timestamp) <= 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_validity() -> Result<(), TokenError> {
        let token = IntentToken::new([0u8; 32], 1, 200)?;
        assert!(token.is_valid()?);
        assert_eq!(token.signature.len(), 2420);
        Ok(())
    }
}
