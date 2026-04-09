use scc::token::IntentToken;
use scc::CapabilityToken;
use scc::TokenError;

fn make_token(
    module_id: [u8; 32],
    signal: u8,
    urgency: u8,
) -> Result<IntentToken, Box<dyn std::error::Error>> {
    let token = IntentToken::new(module_id, signal, urgency)?;
    Ok(token)
}

#[test]
fn test_intent_token_creation() -> Result<(), Box<dyn std::error::Error>> {
    let module_id = [0xABu8; 32];
    let token = IntentToken::new(module_id, 1, 200)?;
    assert_eq!(token.signal_type, 1);
    assert_eq!(token.urgency, 200);
    assert_eq!(token.module_id, module_id);
    assert_eq!(token.signature.len(), 2420);
    Ok(())
}

#[test]
fn test_intent_token_validity() -> Result<(), Box<dyn std::error::Error>> {
    let token = make_token([0u8; 32], 1, 200)?;
    let valid = token.is_valid()?;
    assert!(valid);
    Ok(())
}

#[test]
fn test_intent_token_max_urgency() -> Result<(), Box<dyn std::error::Error>> {
    let token = make_token([0u8; 32], 255, 255)?;
    assert_eq!(token.urgency, 255);
    assert_eq!(token.signal_type, 255);
    Ok(())
}

#[test]
fn test_intent_token_min_urgency() -> Result<(), Box<dyn std::error::Error>> {
    let token = make_token([0u8; 32], 0, 0)?;
    assert_eq!(token.urgency, 0);
    assert_eq!(token.signal_type, 0);
    Ok(())
}

#[test]
fn test_intent_token_default_signature() -> Result<(), Box<dyn std::error::Error>> {
    let token = make_token([0u8; 32], 1, 100)?;
    assert_eq!(token.signature.len(), 2420);
    assert!(token.signature.iter().all(|&b| b == 0));
    Ok(())
}

#[test]
fn test_intent_token_different_module_ids() -> Result<(), Box<dyn std::error::Error>> {
    let module_a = [0xAAu8; 32];
    let module_b = [0xBBu8; 32];

    let token_a = make_token(module_a, 1, 100)?;
    let token_b = make_token(module_b, 1, 100)?;

    assert_ne!(token_a.module_id, token_b.module_id);
    Ok(())
}

#[test]
fn test_intent_token_clone() -> Result<(), Box<dyn std::error::Error>> {
    let token = make_token([0xCCu8; 32], 5, 150)?;
    let cloned = token.clone();

    assert_eq!(token.signal_type, cloned.signal_type);
    assert_eq!(token.urgency, cloned.urgency);
    assert_eq!(token.module_id, cloned.module_id);
    assert_eq!(token.signature.len(), cloned.signature.len());
    Ok(())
}

#[test]
fn test_intent_token_debug() -> Result<(), Box<dyn std::error::Error>> {
    let token = make_token([0xDDu8; 32], 1, 200)?;
    let debug_str = format!("{:?}", token);
    assert!(debug_str.contains("IntentToken"));
    Ok(())
}

#[test]
fn test_capability_token_creation() {
    let cap = CapabilityToken::new();
    let _ = cap;
}

#[test]
fn test_capability_token_default() {
    let cap = CapabilityToken;
    let _ = cap;
}

#[test]
fn test_token_error_time_error() {
    let err = TokenError::TimeError;
    let msg = format!("{}", err);
    assert!(msg.contains("time"));
}

#[test]
fn test_token_error_signature_size() {
    let err = TokenError::SignatureSize;
    let msg = format!("{}", err);
    assert!(msg.contains("Signature"));
}
