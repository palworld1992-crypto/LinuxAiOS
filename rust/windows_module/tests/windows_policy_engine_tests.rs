use windows_module::supervisor::WindowsPolicyEngine;

#[test]
fn test_policy_engine_new() -> anyhow::Result<()> {
    let engine = WindowsPolicyEngine::new();
    assert_eq!(engine.wine_memory_limit(), 2048);
    assert_eq!(engine.kvm_memory_limit(), 4096);
    assert!(engine.hybrid_library_enabled());
    Ok(())
}

#[test]
fn test_policy_engine_default_values() -> anyhow::Result<()> {
    let engine = WindowsPolicyEngine::new();
    assert!(engine.wine_memory_limit() > 0);
    assert!(engine.kvm_memory_limit() > 0);
    Ok(())
}

#[test]
fn test_policy_engine_hybrid_library() -> anyhow::Result<()> {
    let engine = WindowsPolicyEngine::new();
    let _ = engine.hybrid_library_enabled();
    Ok(())
}