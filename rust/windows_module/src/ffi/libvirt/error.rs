use thiserror::Error;

#[derive(Error, Debug)]
pub enum LibvirtError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Domain not found: {0}")]
    DomainNotFound(String),
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    #[error("Config error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() -> anyhow::Result<()> {
        let err = LibvirtError::ConnectionFailed("test".to_string());
        assert!(err.to_string().contains("Connection failed"));
        Ok(())
    }
}