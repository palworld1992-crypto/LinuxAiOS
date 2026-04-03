use linux_module::supervisor::PolicyEngine;
use std::env;
use tempfile::tempdir;

fn with_temp_base<F, T>(f: F) -> T
where
    F: FnOnce() -> T,
{
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path().to_str().unwrap();
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

#[test]
fn test_policy_engine_default() {
    with_temp_base(|| {
        let engine = PolicyEngine::new();
        assert_eq!(engine.get_risk_threshold(), 0.7);
    });
}
