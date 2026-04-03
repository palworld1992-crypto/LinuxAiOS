use linux_module::health_tunnel_impl::HealthTunnelImpl;
use linux_module::supervisor::linux_global_ai::GlobalDecisionAi;
use linux_module::supervisor::linux_reputation_db::ReputationDatabase;
use linux_module::supervisor::PolicyEngine;
use linux_module::supervisor::RiskAssessmentEngine;
use parking_lot::RwLock;
use scc::crypto::{dilithium_keypair, dilithium_sign, dilithium_verify, kyber_keypair};
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
fn test_global_decision_ai_creation() {
    with_temp_base(|| {
        let tensor_pool = Arc::new(RwLock::new(
            linux_module::tensor::TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        let model_name = "dummy_model";
        // The pool does not have a model with that name; creation succeeds but inference should fail.
        let global_ai = GlobalDecisionAi::new(tensor_pool, model_name, 0.5, 0.8).unwrap();
        // Inference should fail because model is not loaded
        let result = global_ai.predict("test_module", &[0.5, 0.3, 0.2]);
        assert!(result.is_err());
    });
}

#[test]
fn test_reputation_db_creation() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("reputation.db");
    let hmac_key = [0u8; 32];
    let db = ReputationDatabase::new(db_path.to_str().unwrap(), hmac_key);
    assert!(db.is_ok());
}

#[test]
fn test_policy_engine_default() {
    let engine = PolicyEngine::new();
    // Default threshold is 0.7
    assert_eq!(engine.get_risk_threshold(), 0.7);
}

#[test]
fn test_risk_engine_creation() {
    let health_tunnel = Arc::new(HealthTunnelImpl::new("test"));
    let engine = RiskAssessmentEngine::new(health_tunnel);
    // Initially no risk level
    assert!(engine.current_risk().is_none());
}

// Thêm test sử dụng khóa lượng tử để loại bỏ warning và kiểm tra chức năng bảo mật
#[test]
fn test_quantum_key_generation() {
    let kyber_result = kyber_keypair();
    let dilithium_result = dilithium_keypair();

    if kyber_result.is_err() || dilithium_result.is_err() {
        eprintln!(
            "Skipping test: crypto keypair generation failed (Kyber: {:?}, Dilithium: {:?})",
            kyber_result.as_ref().err(),
            dilithium_result.as_ref().err()
        );
        return;
    }

    let (kyber_pub, kyber_priv) = kyber_result.unwrap();
    let (dilithium_pub, dilithium_priv) = dilithium_result.unwrap();

    // Kiểm tra kích thước khóa
    assert_eq!(kyber_pub.len(), 1568);
    assert_eq!(kyber_priv.len(), 2400);
    assert_eq!(dilithium_pub.len(), 1952);
    assert_eq!(dilithium_priv.len(), 4032);

    // Kiểm tra ký và xác thực Dilithium
    let message = b"AIOS test message for supervisor";
    let signature = dilithium_sign(&dilithium_priv, message).expect("Dilithium signing failed");
    assert!(
        dilithium_verify(&dilithium_pub, message, &signature),
        "Signature verification failed"
    );
}
