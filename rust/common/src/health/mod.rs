use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthErrorCode {
    Ok,
    Unknown,
    FileNotFound,
    PermissionDenied,
    DatabaseCorrupt,
    ConnectionTimeout,
    PeerUnavailable,
    IntegrityViolation,
    AuthenticationFailed,
    CacheFull,
    OutOfMemory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthError {
    pub code: HealthErrorCode,
    pub message: String,
    pub remediation: String,
    pub timestamp: u64,
}

impl HealthError {
    pub fn new(code: HealthErrorCode, message: &str, remediation: &str) -> Self {
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(_) => {
                warn!("System time before UNIX EPOCH, using 0 as timestamp");
                0
            }
        };
        Self {
            code,
            message: message.to_string(),
            remediation: remediation.to_string(),
            timestamp,
        }
    }
}
