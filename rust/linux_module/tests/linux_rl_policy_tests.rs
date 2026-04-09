use linux_module::ai::{LinuxRlPolicy, RlAction};

#[test]
fn test_rl_action_encode_decode_pageout() -> Result<(), Box<dyn std::error::Error>> {
    let action = RlAction::PageOut(3);
    let encoded = action.encode();
    let decoded = RlAction::decode(encoded).ok_or("decode should succeed")?;
    assert!(matches!(decoded, RlAction::PageOut(3)));
    Ok(())
}

#[test]
fn test_rl_action_encode_decode_prefetch() -> Result<(), Box<dyn std::error::Error>> {
    let action = RlAction::Prefetch(42);
    let encoded = action.encode();
    let decoded = RlAction::decode(encoded).ok_or("decode should succeed")?;
    assert!(matches!(decoded, RlAction::Prefetch(42)));
    Ok(())
}

#[test]
fn test_rl_action_encode_decode_activate_module() -> Result<(), Box<dyn std::error::Error>> {
    let action = RlAction::ActivateModule(100);
    let encoded = action.encode();
    let decoded = RlAction::decode(encoded).ok_or("decode should succeed")?;
    assert!(matches!(decoded, RlAction::ActivateModule(100)));
    Ok(())
}

#[test]
fn test_rl_action_encode_decode_hibernate_module() -> Result<(), Box<dyn std::error::Error>> {
    let action = RlAction::HibernateModule(7);
    let encoded = action.encode();
    let decoded = RlAction::decode(encoded).ok_or("decode should succeed")?;
    assert!(matches!(decoded, RlAction::HibernateModule(7)));
    Ok(())
}

#[test]
fn test_rl_action_decode_invalid() {
    let decoded = RlAction::decode(0x0FFF);
    assert!(decoded.is_none());
}

#[test]
fn test_rl_policy_recommend_wrong_dimension() -> Result<(), Box<dyn std::error::Error>> {
    let policy = LinuxRlPolicy::new(None, 4, 10)?;
    let result = policy.recommend(&[0.5, 0.6]);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn test_rl_policy_recommend_without_model() -> Result<(), Box<dyn std::error::Error>> {
    let policy = LinuxRlPolicy::new(None, 4, 10)?;
    let result = policy.recommend(&[0.9, 0.5, 0.3, 0.2]);
    assert!(result.is_ok());
    let (action, confidence) = result?;
    assert!(matches!(action, RlAction::PageOut(_)));
    assert!(confidence > 0.0 && confidence <= 1.0);
    Ok(())
}

#[test]
fn test_rl_policy_recommend_low_state() -> Result<(), Box<dyn std::error::Error>> {
    let policy = LinuxRlPolicy::new(None, 4, 10)?;
    let result = policy.recommend(&[0.1, 0.2, 0.3, 0.4])?;
    let (action, _confidence) = result;
    // Rule-based policy should return PageOut for low usage
    assert!(matches!(action, RlAction::PageOut(_)));
    Ok(())
}

#[test]
fn test_rl_policy_confidence_threshold() -> Result<(), Box<dyn std::error::Error>> {
    let mut policy = LinuxRlPolicy::new(None, 4, 10)?;
    assert_eq!(policy.get_confidence_threshold(), 0.7);
    policy.set_confidence_threshold(0.5);
    assert_eq!(policy.get_confidence_threshold(), 0.5);
    Ok(())
}

#[test]
fn test_rl_action_encode_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    let actions = [
        RlAction::PageOut(0),
        RlAction::PageOut(255),
        RlAction::Prefetch(0),
        RlAction::Prefetch(4095),
        RlAction::ActivateModule(0),
        RlAction::ActivateModule(4095),
        RlAction::HibernateModule(0),
        RlAction::HibernateModule(4095),
    ];

    for action in &actions {
        let encoded = action.encode();
        let decoded = RlAction::decode(encoded);
        assert!(decoded.is_some());
    }
    Ok(())
}
