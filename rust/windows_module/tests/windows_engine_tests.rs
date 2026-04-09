use windows_module::translation::{EngineError, RoutingTarget};

#[test]
fn test_routing_target_variants() -> anyhow::Result<()> {
    let _ = RoutingTarget::HybridLibrary(123);
    let _ = RoutingTarget::Wine;
    let _ = RoutingTarget::Kvm;
    Ok(())
}

#[test]
fn test_engine_error_display() -> anyhow::Result<()> {
    let err = EngineError::CacheMiss("test_api".to_string());
    assert_eq!(err.to_string(), "Cache miss for API: test_api");
    Ok(())
}

#[test]
fn test_routing_target_debug() -> anyhow::Result<()> {
    let target = RoutingTarget::Wine;
    assert!(format!("{:?}", target).contains("Wine"));
    Ok(())
}