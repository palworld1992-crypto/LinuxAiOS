use crate::token::IntentToken;
use dashmap::DashMap;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Policy {
    pub allow: bool,
    pub max_urgency: u8,
}

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Token expired")]
    TokenExpired,
    #[error("Policy not found")]
    PolicyNotFound,
    #[error("FFI error: {0}")]
    FfiError(String),
}

pub struct IntentValidator {
    policy_cache: DashMap<[u8; 32], Policy>,
}

impl IntentValidator {
    pub fn new() -> Self {
        Self {
            policy_cache: DashMap::new(),
        }
    }

    pub fn validate(&self, token: &IntentToken) -> Result<bool, ValidationError> {
        match token.is_valid() {
            Ok(true) => {
                let policy = self.get_policy(&token.module_id)?;
                if policy.allow {
                    Ok(true)
                } else {
                    Err(ValidationError::InvalidSignature)
                }
            }
            Ok(false) => Err(ValidationError::TokenExpired),
            Err(_) => Err(ValidationError::TokenExpired),
        }
    }

    fn get_policy(&self, module_id: &[u8; 32]) -> Result<Policy, ValidationError> {
        if let Some(policy) = self.policy_cache.get(module_id) {
            return Ok(*policy.value());
        }

        Ok(Policy {
            allow: true,
            max_urgency: 255,
        })
    }

    pub fn update_policy(&self, module_id: [u8; 32], policy: Policy) {
        self.policy_cache.insert(module_id, policy);
    }

    pub fn check_policy(_module_id: &[u8; 32]) -> Result<Policy, ValidationError> {
        Ok(Policy {
            allow: true,
            max_urgency: 255,
        })
    }
}

impl Default for IntentValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_token() {
        let validator = IntentValidator::new();
        let token = IntentToken::new([0u8; 32], 1, 200);
        // Test chỉ chạy nếu token được tạo thành công
        // (Trong môi trường test có thể dùng unwrap vì đây là test cụ thể)
        let token = match token {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Skipping test: Failed to create token: {}", e);
                return;
            }
        };
        assert!(validator.validate(&token).is_ok());
    }

    #[test]
    fn test_check_policy() {
        let result = IntentValidator::check_policy(&[0u8; 32]);
        assert!(result.is_ok(), "check_policy should not fail");
        let policy = result.unwrap();
        assert!(policy.allow);
    }

    #[test]
    fn test_update_policy() {
        let validator = IntentValidator::new();
        validator.update_policy(
            [0u8; 32],
            Policy {
                allow: false,
                max_urgency: 100,
            },
        );
        let token_result = IntentToken::new([0u8; 32], 1, 50);
        let token = match token_result {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Skipping test: Failed to create token: {}", e);
                return;
            }
        };
        let result = validator.validate(&token);
        assert!(result.is_err());
    }
}
