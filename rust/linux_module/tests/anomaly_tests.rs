use dashmap::DashMap;
use linux_module::anomaly::{AnomalyDetector, MlAnomalyDetector};
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
fn test_anomaly_detector_basic() {
    let detector = AnomalyDetector::new(6, 2.0);
    for _ in 0..6 {
        assert!(!detector.feed(0.5));
    }
}

#[test]
fn test_anomaly_detector_detects_anomaly() {
    let detector = AnomalyDetector::new(10, 2.0);
    for i in 0..10 {
        detector.feed(0.5 + (i as f32) * 0.01);
    }
    assert!(detector.feed(2.0));
}

#[test]
fn test_anomaly_detector_reset() {
    let detector = AnomalyDetector::new(6, 2.0);
    for _ in 0..10 {
        detector.feed(0.5);
    }
    detector.reset();
    for _ in 0..5 {
        assert!(!detector.feed(0.5));
    }
}

#[test]
fn test_ml_anomaly_detector_creation() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(DashMap::with_capacity(1));
        let pool = TensorPool::new("test_pool", 1024 * 1024)?;
        tensor_pool.insert((), pool);
        let result = MlAnomalyDetector::new(tensor_pool, "anomaly_model", 0.7);
        assert!(result.is_ok());
        Ok(())
    })
}

#[test]
fn test_ml_anomaly_detector_predict_without_model() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(DashMap::with_capacity(1));
        let pool = TensorPool::new("test_pool", 1024 * 1024)?;
        tensor_pool.insert((), pool);
        let detector = MlAnomalyDetector::new(tensor_pool, "nonexistent_model", 0.7)?;
        let features = vec![0.1f32; 16];
        let result = detector.predict(&features);
        assert!(!result);
        Ok(())
    })
}

#[test]
fn test_ml_anomaly_detector_predict_with_model() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(DashMap::with_capacity(1));
        let pool = TensorPool::new("test_pool", 1024 * 1024)?;
        tensor_pool.insert((), pool);
        let detector = MlAnomalyDetector::new(tensor_pool, "anomaly_model", 0.7)?;
        let features = vec![0.1f32; 16];
        let result = detector.predict(&features);
        assert!(!result);
        Ok(())
    })
}

#[test]
fn test_ml_anomaly_detector_short_features() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(DashMap::with_capacity(1));
        let pool = TensorPool::new("test_pool", 1024 * 1024)?;
        tensor_pool.insert((), pool);
        let detector = MlAnomalyDetector::new(tensor_pool, "anomaly_model", 0.7)?;
        let features = vec![0.1f32; 4];
        let result = detector.predict(&features);
        assert!(!result);
        Ok(())
    })
}

#[test]
fn test_ml_anomaly_detector_returns_bool() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(DashMap::with_capacity(1));
        let pool = TensorPool::new("test_pool", 1024 * 1024)?;
        tensor_pool.insert((), pool);
        let detector = MlAnomalyDetector::new(tensor_pool, "anomaly_model", 0.7)?;
        let features = vec![10.0f32; 16];
        let _result = detector.predict(&features);
        Ok(())
    })
}
