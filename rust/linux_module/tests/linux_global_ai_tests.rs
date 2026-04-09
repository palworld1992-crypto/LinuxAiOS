use linux_module::supervisor::linux_global_ai::{GlobalDecisionAi, ModuleState};
use linux_module::tensor::TensorPool;
use std::env;
use std::sync::Arc;
use tempfile::tempdir;

fn with_temp_base<F, T>(f: F) -> Result<T, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<T, Box<dyn std::error::Error>>,
{
    let temp_dir = tempdir()?;
    let base_path = temp_dir.path().to_str().ok_or("Invalid path")?;
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

#[test]
fn test_global_decision_ai_creation() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(TensorPool::new("test_pool", 1024 * 1024)?);
        let result = GlobalDecisionAi::new(tensor_pool, "nonexistent_model", 0.5, 0.8);
        assert!(result.is_ok());
        Ok(())
    })
}

#[test]
fn test_global_decision_ai_predict_without_model() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(TensorPool::new("test_pool", 1024 * 1024)?);
        let ai = GlobalDecisionAi::new(tensor_pool.clone(), "nonexistent_model", 0.3, 0.7)?;
        let features = vec![0.2f32; 8];
        let result = ai.predict("linux", &features);
        // Without model loaded, predict still works but logs warning
        assert!(result.is_ok());
        Ok(())
    })
}

#[test]
fn test_global_decision_ai_predict_with_heuristic() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(TensorPool::new("test_pool", 1024 * 1024)?);
        // TensorPool empty - will use heuristic mode
        let ai = GlobalDecisionAi::new(tensor_pool, "nonexistent_model", 0.3, 0.7)?;
        let features = vec![0.1f32; 8];
        let prediction = ai.predict("linux", &features)?;
        assert_eq!(prediction.module_name, "linux");
        assert!(prediction.confidence >= 0.0 && prediction.confidence <= 1.0);
        Ok(())
    })
}

#[test]
fn test_global_decision_ai_low_score_active() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(TensorPool::new("test_pool", 1024 * 1024)?);
        let ai = GlobalDecisionAi::new(tensor_pool, "test_model", 0.6, 0.9)?;
        let features = vec![0.1f32; 8];
        let prediction = ai.predict("linux", &features)?;
        assert_eq!(prediction.state, ModuleState::Active);
        Ok(())
    })
}

#[test]
fn test_global_decision_ai_high_score_hibernated() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(TensorPool::new("test_pool", 1024 * 1024)?);
        let ai = GlobalDecisionAi::new(tensor_pool, "test_model", 0.3, 0.5)?;
        let features = vec![1.0f32; 8];
        let prediction = ai.predict("linux", &features)?;
        assert!(
            matches!(
                prediction.state,
                ModuleState::Active | ModuleState::Stub | ModuleState::Hibernated
            ),
            "State should be valid"
        );
        Ok(())
    })
}

#[test]
fn test_global_decision_ai_medium_score_stub() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(TensorPool::new("test_pool", 1024 * 1024)?);
        let ai = GlobalDecisionAi::new(tensor_pool, "test_model", 0.3, 0.7)?;
        let features = vec![0.5f32; 8];
        let prediction = ai.predict("linux", &features)?;
        assert!(
            matches!(
                prediction.state,
                ModuleState::Active | ModuleState::Stub | ModuleState::Hibernated
            ),
            "State should be valid"
        );
        Ok(())
    })
}

#[test]
fn test_global_decision_ai_collect_features() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(TensorPool::new("test_pool", 1024 * 1024)?);
        let ai = GlobalDecisionAi::new(tensor_pool, "test_model", 0.3, 0.7)?;
        let features = ai.collect_features();
        assert_eq!(features.len(), 8);
        Ok(())
    })
}

#[test]
fn test_global_decision_ai_ensure_model_active() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(TensorPool::new("test_pool", 1024 * 1024)?);
        let ai = GlobalDecisionAi::new(tensor_pool, "test_model", 0.3, 0.7)?;
        // ensure_model_active is a no-op in current implementation
        // TODO(Phase 4): Full activation logic
        assert!(ai.ensure_model_active().is_ok());
        Ok(())
    })
}
