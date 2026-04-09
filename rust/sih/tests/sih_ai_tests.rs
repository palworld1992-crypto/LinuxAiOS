use anyhow::Result;
use sih::ai::sih_rl_policy::{PolicyAction, RlContext};
use sih::ai::{SihLnnPredictor, SihRlPolicy};

#[test]
fn test_lnn_predictor_creation() -> Result<()> {
    let predictor = SihLnnPredictor::new(100, 5)?;
    assert!(!predictor.is_loaded());
    Ok(())
}

#[test]
fn test_lnn_predictor_record_query() -> Result<()> {
    let predictor = SihLnnPredictor::new(100, 5)?;
    predictor.record_query("test query".to_string());
    Ok(())
}

#[test]
fn test_lnn_predictor_load_model() -> Result<()> {
    let mut predictor = SihLnnPredictor::new(100, 5)?;
    let result = predictor.load_model("/path/to/model");
    assert!(result.is_ok());
    assert!(predictor.is_loaded());
    Ok(())
}

#[test]
fn test_rl_policy_creation() -> Result<()> {
    let policy = SihRlPolicy::new()?;
    assert!(!policy.is_loaded());
    Ok(())
}

#[test]
fn test_rl_policy_default() -> Result<()> {
    let policy = SihRlPolicy::default()?;
    assert!(!policy.is_loaded());
    Ok(())
}

#[test]
fn test_rl_policy_load_policy() -> Result<()> {
    let mut policy = SihRlPolicy::new()?;
    let result = policy.load_policy("test_policy");
    assert!(result.is_ok());
    assert!(policy.is_loaded());
    Ok(())
}

#[test]
fn test_rl_policy_evaluate_without_model() -> Result<()> {
    let policy = SihRlPolicy::new()?;
    let context = RlContext {
        source_trust: 0.8,
        historical_accuracy: 0.9,
        rollback_count: 0,
        popularity: None,
    };
    let action = policy.evaluate(&context);
    assert!(matches!(action, PolicyAction::AdjustThreshold(_)));
    Ok(())
}

#[test]
fn test_rl_policy_set_trust_threshold() -> Result<()> {
    let policy = SihRlPolicy::new()?;
    policy.set_trust_threshold(0.75);
    assert!((policy.get_trust_threshold() - 0.75).abs() < 0.001);
    Ok(())
}

#[test]
fn test_rl_policy_get_trust_threshold_default() -> Result<()> {
    let policy = SihRlPolicy::new()?;
    let threshold = policy.get_trust_threshold();
    assert!(threshold > 0.0 && threshold <= 1.0);
    Ok(())
}

#[test]
fn test_policy_action_debug() -> Result<()> {
    let action = PolicyAction::AdjustThreshold(0.5);
    let debug = format!("{:?}", action);
    assert!(debug.contains("AdjustThreshold"));
    Ok(())
}

#[test]
fn test_policy_action_prioritize_source() -> Result<()> {
    let action = PolicyAction::PrioritizeSource("source-1".to_string());
    let debug = format!("{:?}", action);
    assert!(debug.contains("PrioritizeSource"));
    Ok(())
}

#[test]
fn test_policy_action_reject_source() -> Result<()> {
    let action = PolicyAction::RejectSource("bad-source".to_string());
    let debug = format!("{:?}", action);
    assert!(debug.contains("RejectSource"));
    Ok(())
}

#[test]
fn test_rl_context_clone() -> Result<()> {
    let context = RlContext {
        source_trust: 0.9,
        historical_accuracy: 0.85,
        rollback_count: 2,
        popularity: None,
    };
    let cloned = context.clone();
    assert_eq!(cloned.source_trust, context.source_trust);
    assert_eq!(cloned.rollback_count, context.rollback_count);
    Ok(())
}
