use linux_module::supervisor::linux_global_ai::{GlobalDecisionAi, ModuleState};
use linux_module::tensor::TensorPool;
use parking_lot::RwLock;
use sha2::{Digest, Sha256};
use std::env;
use std::sync::Arc;
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
fn test_global_decision_ai_creation_model_not_found() {
    with_temp_base(|| {
        let tensor_pool = Arc::new(RwLock::new(
            TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        let result = GlobalDecisionAi::new(tensor_pool, "nonexistent_model", 0.5, 0.8);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("not currently active"));
        }
    });
}

#[test]
fn test_global_decision_ai_with_model() {
    with_temp_base(|| {
        let tensor_pool = Arc::new(RwLock::new(
            TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        // Load dummy model into pool
        let dummy_data = vec![1u8; 1024];
        let hash = Sha256::digest(&dummy_data).to_vec();
        {
            let mut pool = tensor_pool.write();
            pool.load_model("test_model", &dummy_data, "1.0", hash)
                .unwrap();
        }

        let ai = GlobalDecisionAi::new(tensor_pool, "test_model", 0.3, 0.7).unwrap();
        let features = vec![0.2, 0.4, 0.1];
        let prediction = ai.predict("linux", &features).unwrap();
        assert_eq!(prediction.module_name, "linux");
        // With low score (0.233) < 0.3 => Active
        assert_eq!(prediction.state, ModuleState::Active);
        assert!(prediction.confidence > 0.0);
    });
}

#[test]
fn test_global_decision_ai_stub_transition() {
    with_temp_base(|| {
        let tensor_pool = Arc::new(RwLock::new(
            TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        let dummy_data = vec![1u8; 1024];
        let hash = Sha256::digest(&dummy_data).to_vec();
        {
            let mut pool = tensor_pool.write();
            pool.load_model("test_model", &dummy_data, "1.0", hash)
                .unwrap();
        }

        let ai = GlobalDecisionAi::new(tensor_pool, "test_model", 0.3, 0.7).unwrap();
        // score = (0.5+0.6+0.4)/3 = 0.5 => Stub
        let features = vec![0.5, 0.6, 0.4];
        let prediction = ai.predict("windows", &features).unwrap();
        assert_eq!(prediction.state, ModuleState::Stub);
    });
}

#[test]
fn test_global_decision_ai_hibernated_transition() {
    with_temp_base(|| {
        let tensor_pool = Arc::new(RwLock::new(
            TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        let dummy_data = vec![1u8; 1024];
        let hash = Sha256::digest(&dummy_data).to_vec();
        {
            let mut pool = tensor_pool.write();
            pool.load_model("test_model", &dummy_data, "1.0", hash)
                .unwrap();
        }

        let ai = GlobalDecisionAi::new(tensor_pool, "test_model", 0.3, 0.7).unwrap();
        // score = (0.8+0.9+0.7)/3 = 0.8 => Hibernated
        let features = vec![0.8, 0.9, 0.7];
        let prediction = ai.predict("android", &features).unwrap();
        assert_eq!(prediction.state, ModuleState::Hibernated);
    });
}

#[test]
fn test_ensure_model_active() {
    with_temp_base(|| {
        let tensor_pool = Arc::new(RwLock::new(
            TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        let dummy_data = vec![1u8; 1024];
        let hash = Sha256::digest(&dummy_data).to_vec();
        {
            let mut pool = tensor_pool.write();
            pool.load_model("test_model", &dummy_data, "1.0", hash)
                .unwrap();
            // Deactivate it
            pool.deactivate_model("test_model").unwrap();
        }

        let ai = GlobalDecisionAi::new(tensor_pool.clone(), "test_model", 0.3, 0.7).unwrap();
        // Model is offline, predict should fail
        let features = vec![0.2, 0.3, 0.4];
        let result = ai.predict("linux", &features);
        assert!(result.is_err());

        // Ensure model active should restore it
        ai.ensure_model_active().unwrap();
        // Now predict should work
        let prediction = ai.predict("linux", &features).unwrap();
        assert_eq!(prediction.state, ModuleState::Active);
    });
}
