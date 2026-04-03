use linux_module::anomaly::{AnomalyDetector, MlAnomalyDetector};
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
fn test_anomaly_detector() {
    let detector = AnomalyDetector::new(6, 2.0);
    // Feed normal values
    for _ in 0..6 {
        assert!(!detector.feed(0.5));
    }
    // Feed anomaly
    assert!(detector.feed(2.0));
}

#[test]
fn test_ml_anomaly_detector() {
    with_temp_base(|| {
        let tensor_pool = std::sync::Arc::new(parking_lot::RwLock::new(
            linux_module::tensor::TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        let detector = MlAnomalyDetector::new(tensor_pool, "anomaly_model", 0.7);
        // Creation succeeds even if model not yet loaded
        assert!(detector.is_ok());
        let detector = detector.unwrap();
        // Without model, predict returns false
        assert!(!detector.predict(&[0.8, 0.9, 0.7]));
    });
}
