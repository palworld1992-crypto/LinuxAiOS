use scc::token::IntentToken;
use scc::validation::{IntentValidator, Policy, ValidationError};

fn make_token(
    module_id: [u8; 32],
    signal: u8,
    urgency: u8,
) -> Result<IntentToken, Box<dyn std::error::Error>> {
    let token = IntentToken::new(module_id, signal, urgency)?;
    Ok(token)
}

#[test]
fn test_validator_creation() {
    let _validator = IntentValidator::new();
}

#[test]
fn test_validate_valid_token() -> Result<(), Box<dyn std::error::Error>> {
    let validator = IntentValidator::new();
    let token = make_token([0u8; 32], 1, 200)?;
    let result = validator.validate(&token)?;
    assert!(result);
    Ok(())
}

#[test]
fn test_update_policy() -> Result<(), Box<dyn std::error::Error>> {
    let validator = IntentValidator::new();
    let module_id = [0xAAu8; 32];
    let policy = Policy {
        allow: true,
        max_urgency: 100,
    };

    validator.update_policy(module_id, policy);

    let token = make_token(module_id, 1, 50)?;
    let result = validator.validate(&token)?;
    assert!(result);
    Ok(())
}

#[test]
fn test_validate_with_denied_policy() -> Result<(), Box<dyn std::error::Error>> {
    let validator = IntentValidator::new();
    let module_id = [0xBBu8; 32];

    validator.update_policy(
        module_id,
        Policy {
            allow: false,
            max_urgency: 100,
        },
    );

    let token = make_token(module_id, 1, 50)?;
    let result = validator.validate(&token);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_check_policy_default() -> Result<(), Box<dyn std::error::Error>> {
    let policy = IntentValidator::check_policy(&[0u8; 32])?;
    assert!(policy.allow);
    assert_eq!(policy.max_urgency, 255);
    Ok(())
}

#[test]
fn test_policy_copy() {
    let policy = Policy {
        allow: true,
        max_urgency: 128,
    };
    let copied = policy;
    assert_eq!(policy.allow, copied.allow);
    assert_eq!(policy.max_urgency, copied.max_urgency);
}

#[test]
fn test_policy_debug() {
    let policy = Policy {
        allow: true,
        max_urgency: 200,
    };
    let debug = format!("{:?}", policy);
    assert!(debug.contains("Policy"));
}

#[test]
fn test_validation_error_invalid_signature() {
    let err = ValidationError::InvalidSignature;
    let msg = format!("{}", err);
    assert!(msg.contains("signature"));
}

#[test]
fn test_validation_error_token_expired() {
    let err = ValidationError::TokenExpired;
    let msg = format!("{}", err);
    assert!(msg.contains("expired"));
}

#[test]
fn test_validation_error_policy_not_found() {
    let err = ValidationError::PolicyNotFound;
    let msg = format!("{}", err);
    assert!(msg.contains("Policy"));
}

#[test]
fn test_validation_error_ffi_error() {
    let err = ValidationError::FfiError("test ffi error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("test ffi error"));
}

#[test]
fn test_validation_error_debug() {
    let err = ValidationError::InvalidSignature;
    let debug = format!("{:?}", err);
    assert!(debug.contains("InvalidSignature"));
}

#[test]
fn test_multiple_policy_updates() -> Result<(), Box<dyn std::error::Error>> {
    let validator = IntentValidator::new();

    for i in 0..10 {
        let module_id = [i as u8; 32];
        validator.update_policy(
            module_id,
            Policy {
                allow: i % 2 == 0,
                max_urgency: (i * 25) as u8,
            },
        );
    }

    let token = make_token([0u8; 32], 1, 50)?;
    let result = validator.validate(&token)?;
    assert!(result);
    Ok(())
}
