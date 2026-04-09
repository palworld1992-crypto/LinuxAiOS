//! Anomaly Detector - phát hiện hành vi lệch chuẩn từ Assistant.
//! Phase 3, Section 3.4.4: linux_anomaly_detector
//! Dùng candle với model nhỏ (≤10 MB) chạy trên CPU, phân tích luồng sự kiện
//! từ các Assistant mỗi 100ms. Nếu phát hiện điểm số bất thường vượt ngưỡng,
//! gửi HealthAlert đến Linux Supervisor.

use crate::tensor::TensorPool;
use anyhow::{anyhow, Result};
use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tracing::{info, warn};

/// Event nhận từ Transport Tunnel (qua SCC).
#[derive(Debug, Clone)]
pub struct AnomalyEvent {
    pub source_module: String,
    pub timestamp_ms: u64,
    pub features: Vec<f32>,
    pub metadata: Vec<u8>,
}

/// Alert gửi đến Supervisor khi phát hiện anomaly.
#[derive(Debug, Clone)]
pub struct HealthAlert {
    pub module_id: String,
    pub alert_type: String,
    pub severity: u8,
    pub score: f32,
    pub details: Vec<u8>,
}

/// Anomaly detector dùng candle model (≤10 MB).
pub struct LinuxAnomalyDetector {
    tensor_pool: Arc<DashMap<(), TensorPool>>,
    model_name: String,
    threshold: f32,
    health_tunnel: DashMap<(), Arc<dyn HealthTunnel + Send + Sync>>,
    event_window: DashMap<usize, AnomalyEvent>,
    event_index: AtomicUsize,
    max_window_size: usize,
    inference_interval_ms: u64,
    last_alert: AtomicU64,
    alert_cooldown_ms: u64,
}

impl LinuxAnomalyDetector {
    /// Tạo detector mới.
    ///
    /// - `tensor_pool`: Tensor Pool chứa model anomaly
    /// - `model_name`: Tên model trong Tensor Pool
    /// - `threshold`: Ngưỡng score để coi là anomaly (0.0-1.0)
    /// - `inference_interval_ms`: Khoảng thời gian giữa các lần inference
    /// - `alert_cooldown_ms`: Thời gian tối thiểu giữa 2 alert cùng loại
    pub fn new(
        tensor_pool: Arc<DashMap<(), TensorPool>>,
        model_name: &str,
        threshold: f32,
        inference_interval_ms: u64,
        alert_cooldown_ms: u64,
    ) -> Self {
        Self {
            tensor_pool,
            model_name: model_name.to_string(),
            threshold,
            health_tunnel: DashMap::new(),
            event_window: DashMap::with_capacity(256),
            event_index: AtomicUsize::new(0),
            max_window_size: 256,
            inference_interval_ms,
            last_alert: AtomicU64::new(0),
            alert_cooldown_ms,
        }
    }

    /// Gắn health tunnel để gửi alert.
    pub fn set_health_tunnel(&self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel.insert((), tunnel);
    }

    /// Nhận event từ Assistant qua Transport Tunnel.
    pub fn receive_event(&self, event: AnomalyEvent) {
        let idx = self.event_index.fetch_add(1, Ordering::SeqCst);
        let map_idx = idx % self.max_window_size;

        if self.event_window.len() >= self.max_window_size {
            self.event_window.remove(&map_idx);
        }
        self.event_window.insert(map_idx, event);
    }

    /// Chạy inference trên batch events hiện tại.
    /// Trả về danh sách alerts nếu phát hiện anomaly.
    pub fn run_inference(&self) -> Result<Vec<HealthAlert>> {
        let model_bytes = match self.tensor_pool.get(&()) {
            Some(pool) => pool.get_model_data(&self.model_name),
            None => None,
        };

        let model_bytes = match model_bytes {
            Some(bytes) => bytes,
            None => {
                tracing::warn!("Model '{}' not available in TensorPool", self.model_name);
                return Ok(vec![]);
            }
        };

        if model_bytes.len() > 10 * 1024 * 1024 {
            warn!(
                "Anomaly model size {} bytes exceeds 10 MB limit",
                model_bytes.len()
            );
        }

        if self.event_window.is_empty() {
            return Ok(vec![]);
        }

        let mut alerts = vec![];
        let events: Vec<_> = self
            .event_window
            .iter()
            .map(|r| r.value().clone())
            .collect();

        for event in events {
            let score = self.compute_anomaly_score(&event.features);
            if score > self.threshold {
                let now = event.timestamp_ms;
                let last = self.last_alert.load(Ordering::SeqCst);
                if last > 0 && now.saturating_sub(last) < self.alert_cooldown_ms {
                    continue;
                }

                let alert = HealthAlert {
                    module_id: event.source_module.clone(),
                    alert_type: "anomaly_detected".to_string(),
                    severity: ((score - self.threshold) * 255.0).min(255.0) as u8,
                    score,
                    details: serde_json::to_vec(&serde_json::json!({
                        "event_timestamp": event.timestamp_ms,
                        "feature_count": event.features.len(),
                        "model": self.model_name,
                    }))
                    .map_err(|e| anyhow!("Failed to serialize alert details: {}", e))?,
                };
                alerts.push(alert);

                self.last_alert.store(now, Ordering::SeqCst);
            }
        }

        if !alerts.is_empty() {
            self.send_alerts(&alerts)?;
        }

        Ok(alerts)
    }

    /// Tính anomaly score từ features (giả lập inference).
    /// Trong production, dùng candle để chạy model thật.
    fn compute_anomaly_score(&self, features: &[f32]) -> f32 {
        if features.is_empty() {
            return 0.0;
        }

        let mean: f32 = features.iter().sum::<f32>() / features.len() as f32;
        let variance: f32 =
            features.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / features.len() as f32;
        let stddev = variance.sqrt();

        let max_deviation = features
            .iter()
            .map(|&x| (x - mean).abs())
            .fold(0.0f32, f32::max);

        let z_score = if stddev > 0.0 {
            max_deviation / stddev
        } else {
            0.0
        };

        (z_score / 5.0).min(1.0)
    }

    /// Gửi alerts qua Health Tunnel.
    fn send_alerts(&self, alerts: &[HealthAlert]) -> Result<()> {
        let tunnel = match self.health_tunnel.get(&()) {
            Some(t) => t.value().clone(),
            None => {
                tracing::warn!("Health tunnel not configured");
                return Ok(());
            }
        };

        for alert in alerts {
            let timestamp = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            {
                Ok(d) => d.as_millis() as u64,
                Err(e) => {
                    tracing::warn!("System clock before UNIX_EPOCH: {}", e);
                    0
                }
            };
            let record = HealthRecord {
                module_id: format!("anomaly_{}", alert.module_id),
                timestamp,
                status: HealthStatus::Degraded,
                potential: 1.0 - alert.score as f32,
                details: alert.details.clone(),
            };
            tunnel.record_health(record)?;
            warn!(
                "HealthAlert sent: module={}, severity={}, score={:.3}",
                alert.module_id, alert.severity, alert.score
            );
        }

        Ok(())
    }

    /// Reset trạng thái detector.
    pub fn reset(&self) {
        self.event_window.clear();
        self.last_alert.store(0, Ordering::SeqCst);
        self.event_index.store(0, Ordering::SeqCst);
    }

    /// Lấy số events đang chờ xử lý.
    pub fn pending_events(&self) -> usize {
        self.event_window.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::TensorPool;

    fn create_test_detector() -> Result<LinuxAnomalyDetector, anyhow::Error> {
        let pool: Arc<DashMap<(), TensorPool>> = Arc::new(DashMap::with_capacity(1));
        pool.insert((), TensorPool::new("test_pool", 1024 * 1024)?);
        Ok(LinuxAnomalyDetector::new(
            pool,
            "test_model",
            0.7,
            100,
            1000,
        ))
    }

    #[test]
    fn test_anomaly_detector_creation() -> Result<(), anyhow::Error> {
        let detector = create_test_detector()?;
        assert_eq!(detector.pending_events(), 0);
        Ok(())
    }

    #[test]
    fn test_receive_event() -> Result<(), anyhow::Error> {
        let detector = create_test_detector()?;
        let event = AnomalyEvent {
            source_module: "test".to_string(),
            timestamp_ms: 12345,
            features: vec![0.1, 0.2, 0.3],
            metadata: vec![],
        };
        detector.receive_event(event);
        assert_eq!(detector.pending_events(), 1);
        Ok(())
    }

    #[test]
    fn test_compute_anomaly_score_normal() -> Result<(), anyhow::Error> {
        let detector = create_test_detector()?;
        let score = detector.compute_anomaly_score(&[0.5, 0.5, 0.5, 0.5]);
        assert!(score < 0.5);
        Ok(())
    }

    #[test]
    fn test_compute_anomaly_score_outlier() -> Result<(), anyhow::Error> {
        let detector = create_test_detector()?;
        let score = detector.compute_anomaly_score(&[0.1, 0.1, 0.1, 0.9]);
        assert!(score > 0.0);
        Ok(())
    }

    #[test]
    fn test_compute_anomaly_score_empty() -> Result<(), anyhow::Error> {
        let detector = create_test_detector()?;
        let score = detector.compute_anomaly_score(&[]);
        assert_eq!(score, 0.0);
        Ok(())
    }

    #[test]
    fn test_window_overflow() -> Result<(), anyhow::Error> {
        let detector = create_test_detector()?;
        for i in 0..300 {
            detector.receive_event(AnomalyEvent {
                source_module: "test".to_string(),
                timestamp_ms: i,
                features: vec![0.5],
                metadata: vec![],
            });
        }
        assert_eq!(detector.pending_events(), 256);
        Ok(())
    }
}
