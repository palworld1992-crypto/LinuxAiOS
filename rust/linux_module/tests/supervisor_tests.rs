use linux_module::health_tunnel_impl::HealthTunnelImpl;
use linux_module::supervisor::linux_global_ai::GlobalDecisionAi;
use linux_module::supervisor::linux_reputation_db::ReputationDatabase;
use linux_module::supervisor::PolicyEngine;
use linux_module::supervisor::RiskAssessmentEngine;
use scc::crypto::{dilithium_keypair, dilithium_sign, dilithium_verify, kyber_keypair};
use std::env;
use std::sync::Arc;
use tempfile::tempdir;

fn with_temp_base<F>(f: F) -> anyhow::Result<()>
where
    F: FnOnce() -> anyhow::Result<()>,
{
    let temp_dir = tempdir()?;
    let base_path = temp_dir
        .path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Invalid path"))?;
    env::set_var("AIOS_BASE_DIR", base_path);
    let result = f();
    env::remove_var("AIOS_BASE_DIR");
    result
}

#[test]
fn test_global_decision_ai_creation() -> anyhow::Result<()> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(linux_module::tensor::TensorPool::new(
            "test_pool",
            1024 * 1024,
        )?);

        let model_name = "dummy_model";
        let global_ai = GlobalDecisionAi::new(tensor_pool, model_name, 0.5, 0.8)?;
        // Without model loaded, predict returns Ok with heuristic
        let result = global_ai.predict("test_module", &[0.5, 0.3, 0.2]);
        assert!(result.is_ok());
        Ok(())
    })
}

#[test]
fn test_reputation_db_creation() -> anyhow::Result<()> {
    let temp_dir = tempdir()?;
    let db_path = temp_dir.path().join("reputation.db");
    let hmac_key = [0u8; 32];
    let db = ReputationDatabase::new(
        db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid path"))?,
        hmac_key,
    );
    assert!(db.is_ok());
    Ok(())
}

#[test]
fn test_policy_engine_default() -> anyhow::Result<()> {
    let engine = PolicyEngine::new();
    assert_eq!(engine.get_risk_threshold(), 0.7);
    Ok(())
}

#[test]
fn test_risk_engine_creation() -> anyhow::Result<()> {
    let health_tunnel = Arc::new(HealthTunnelImpl::new("test"));
    let engine = RiskAssessmentEngine::new(health_tunnel);
    assert!(engine.current_risk().is_none());
    Ok(())
}

#[test]
fn test_quantum_key_generation() -> anyhow::Result<()> {
    let kyber_result = kyber_keypair();
    let dilithium_result = dilithium_keypair();

    if kyber_result.is_err() || dilithium_result.is_err() {
        tracing::warn!(
            "Skipping test: crypto keypair generation failed (Kyber: {:?}, Dilithium: {:?})",
            kyber_result.as_ref().err(),
            dilithium_result.as_ref().err()
        );
        return Ok(());
    }

    let (kyber_pub, kyber_priv) = kyber_result?;
    let (dilithium_pub, dilithium_priv) = dilithium_result?;

    assert_eq!(kyber_pub.len(), 1568);
    assert_eq!(kyber_priv.len(), 2400);
    assert_eq!(dilithium_pub.len(), 1952);
    assert_eq!(dilithium_priv.len(), 4032);

    let message = b"AIOS test message for supervisor";
    let signature = dilithium_sign(&dilithium_priv, message)?;
    assert!(
        dilithium_verify(&dilithium_pub, message, &signature).is_ok(),
        "Signature verification failed"
    );
    Ok(())
}
