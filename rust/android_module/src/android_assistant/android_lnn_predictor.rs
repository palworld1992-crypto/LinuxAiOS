use ringbuf::traits::{Consumer, Observer, Producer};
use ringbuf::HeapRb;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LnnError {
    #[error("Prediction failed: {0}")]
    PredictionFailed(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TelemetryData {
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub io_mbps: f32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LoadPrediction {
    pub predicted_cpu: f32,
    pub predicted_memory_mb: u64,
    pub confidence: f32,
}

pub struct AndroidLnnPredictor {
    telemetry_buffer: HeapRb<TelemetryData>,
    weights: [f32; 3],
}

impl Default for AndroidLnnPredictor {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidLnnPredictor {
    pub fn new() -> Self {
        Self {
            telemetry_buffer: HeapRb::new(4096),
            weights: [0.4, 0.35, 0.25],
        }
    }

    pub fn add_telemetry(&mut self, data: TelemetryData) {
        if self.telemetry_buffer.is_full() {
            let _ = self.telemetry_buffer.try_pop();
        }
        let _ = self.telemetry_buffer.try_push(data);
    }

    pub fn predict_load(&self, horizon_seconds: u32) -> Result<LoadPrediction, LnnError> {
        let samples: Vec<_> = self.telemetry_buffer.iter().collect();
        if samples.is_empty() {
            return Err(LnnError::PredictionFailed("No telemetry data".to_string()));
        }

        let n = samples.len() as f32;
        let avg_cpu = samples.iter().map(|s| s.cpu_percent).sum::<f32>() / n;
        let avg_memory = samples.iter().map(|s| s.memory_mb as f32).sum::<f32>() / n;
        let avg_io = samples.iter().map(|s| s.io_mbps).sum::<f32>() / n;

        let cpu_weight = self.weights[0];
        let memory_weight = self.weights[1];
        let io_weight = self.weights[2];

        let cpu_trend = if n > 1.0 {
            let last_cpu = match samples.last() {
                Some(s) => s.cpu_percent,
                None => 0.0, // Should not happen due to empty check, but safe default
            };
            let first_cpu = match samples.first() {
                Some(s) => s.cpu_percent,
                None => 0.0,
            };
            (last_cpu - first_cpu) / n
        } else {
            0.0
        };

        let horizon_factor = (horizon_seconds as f32 / 60.0).min(2.0);
        let predicted_cpu = (avg_cpu + cpu_trend * horizon_factor).clamp(0.0, 100.0);
        let predicted_memory = avg_memory + (self.weights[1] * horizon_factor * 10.0);

        let weighted_score = cpu_weight * avg_cpu / 100.0
            + memory_weight * avg_memory / 4096.0
            + io_weight * avg_io / 1000.0;
        let confidence = (weighted_score * 2.0).clamp(0.1, 0.95);

        Ok(LoadPrediction {
            predicted_cpu,
            predicted_memory_mb: predicted_memory as u64,
            confidence,
        })
    }

    pub fn telemetry_count(&self) -> usize {
        self.telemetry_buffer.occupied_len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictor_creation() {
        let predictor = AndroidLnnPredictor::new();
        assert_eq!(predictor.telemetry_count(), 0);
    }

    #[test]
    fn test_add_telemetry() {
        let mut predictor = AndroidLnnPredictor::new();
        predictor.add_telemetry(TelemetryData {
            cpu_percent: 50.0,
            memory_mb: 256,
            io_mbps: 10.0,
        });
        assert_eq!(predictor.telemetry_count(), 1);
    }

    #[test]
    fn test_predict_load() -> anyhow::Result<()> {
        let mut predictor = AndroidLnnPredictor::new();
        predictor.add_telemetry(TelemetryData {
            cpu_percent: 50.0,
            memory_mb: 256,
            io_mbps: 10.0,
        });
        let prediction = predictor.predict_load(30)?;
        assert!(prediction.predicted_cpu > 0.0);
        Ok(())
    }
}
