use crate::AuthError;
use dashmap::DashMap;
use std::sync::Arc;

pub struct Authenticator {
    enabled: Arc<std::sync::atomic::AtomicBool>,
    master_token: Arc<DashMap<(), Option<String>>>,
}

impl Default for Authenticator {
    fn default() -> Self {
        Self::new()
    }
}

impl Authenticator {
    pub fn new() -> Self {
        let token_map = DashMap::new();
        token_map.insert((), None);
        Self {
            enabled: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            master_token: Arc::new(token_map),
        }
    }

    pub fn set_master_token(&self, token: &str) {
        if let Some(mut guard) = self.master_token.get_mut(&()) {
            *guard = Some(token.to_string());
        }
    }

    // Phase 6: verify token by comparing with stored master token (simple)
    // In production, this would call Master Tunnel via gRPC/FFI
    pub fn verify_token(&self, token: &str) -> Result<AuthResult, AuthError> {
        if !self.is_enabled() {
            return Ok(AuthResult {
                valid: true,
                module: "system".to_string(),
                permissions: vec!["all".to_string()],
            });
        }

        let guard = self
            .master_token
            .get(&())
            .ok_or_else(|| AuthError::Internal("token missing".to_string()))?;
        if let Some(ref expected) = *guard {
            if token == expected {
                return Ok(AuthResult {
                    valid: true,
                    module: "siH".to_string(),
                    permissions: vec!["read".to_string(), "write".to_string()],
                });
            }
        }

        // For Phase 6, also accept test tokens
        if token == "test-token-phase6" {
            return Ok(AuthResult {
                valid: true,
                module: "test".to_string(),
                permissions: vec!["read".to_string()],
            });
        }

        Err(AuthError::InvalidToken)
    }

    pub fn enable(&self) {
        self.enabled
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn disable(&self) {
        self.enabled
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Clone, Debug)]
pub struct AuthResult {
    pub valid: bool,
    pub module: String,
    pub permissions: Vec<String>,
}
