use android_module::android_assistant::android_lnn_predictor::{
    AndroidLnnPredictor, LnnError, LoadPrediction, TelemetryData,
};

#[test]
fn test_predictor_creation() -> anyhow::Result<()> {
    let predictor = AndroidLnnPredictor::new();
    assert_eq!(predictor.telemetry_count(), 0);
    Ok(())
}

#[test]
fn test_add_telemetry() -> anyhow::Result<()> {
    let mut predictor = AndroidLnnPredictor::new();
    predictor.add_telemetry(TelemetryData {
        cpu_percent: 50.0,
        memory_mb: 256,
        io_mbps: 10.0,
    });
    assert_eq!(predictor.telemetry_count(), 1);
    Ok(())
}

#[test]
fn test_add_multiple_telemetry() -> anyhow::Result<()> {
    let mut predictor = AndroidLnnPredictor::new();
    for i in 0..10 {
        predictor.add_telemetry(TelemetryData {
            cpu_percent: i as f32 * 10.0,
            memory_mb: 128 + i * 64,
            io_mbps: i as f32 * 5.0,
        });
    }
    assert_eq!(predictor.telemetry_count(), 10);
    Ok(())
}

#[test]
fn test_predict_load_with_data() -> Result<(), Box<dyn std::error::Error>> {
    let mut predictor = AndroidLnnPredictor::new();
    predictor.add_telemetry(TelemetryData {
        cpu_percent: 50.0,
        memory_mb: 256,
        io_mbps: 10.0,
    });
    let prediction = predictor.predict_load(30)?;
    assert!(prediction.predicted_cpu > 0.0);
    assert!(prediction.predicted_cpu <= 100.0);
    assert!(prediction.confidence > 0.0 && prediction.confidence <= 1.0);
    Ok(())
}

#[test]
fn test_predict_load_no_data() {
    let predictor = AndroidLnnPredictor::new();
    let result = predictor.predict_load(30);
    assert!(result.is_err());
}

#[test]
fn test_predict_load_different_horizons() -> Result<(), Box<dyn std::error::Error>> {
    let mut predictor = AndroidLnnPredictor::new();
    predictor.add_telemetry(TelemetryData {
        cpu_percent: 50.0,
        memory_mb: 256,
        io_mbps: 10.0,
    });

    let short = predictor.predict_load(10)?;
    let long = predictor.predict_load(120)?;

    assert!(short.predicted_cpu > 0.0);
    assert!(long.predicted_cpu > 0.0);
    Ok(())
}

#[test]
fn test_predict_load_cpu_clamped() -> Result<(), Box<dyn std::error::Error>> {
    let mut predictor = AndroidLnnPredictor::new();
    predictor.add_telemetry(TelemetryData {
        cpu_percent: 99.0,
        memory_mb: 3500,
        io_mbps: 900.0,
    });
    let prediction = predictor.predict_load(60)?;
    assert!(prediction.predicted_cpu <= 100.0);
    Ok(())
}

#[test]
fn test_telemetry_data_clone() {
    let data = TelemetryData {
        cpu_percent: 75.0,
        memory_mb: 1024,
        io_mbps: 50.0,
    };
    let cloned = data.clone();
    assert_eq!(cloned.cpu_percent, data.cpu_percent);
    assert_eq!(cloned.memory_mb, data.memory_mb);
}

#[test]
fn test_load_prediction_clone() {
    let pred = LoadPrediction {
        predicted_cpu: 60.0,
        predicted_memory_mb: 512,
        confidence: 0.85,
    };
    let cloned = pred.clone();
    assert_eq!(cloned.predicted_cpu, pred.predicted_cpu);
    assert_eq!(cloned.confidence, pred.confidence);
}

#[test]
fn test_telemetry_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let data = TelemetryData {
        cpu_percent: 80.0,
        memory_mb: 2048,
        io_mbps: 100.0,
    };
    let json = serde_json::to_string(&data)?;
    let deserialized: TelemetryData = serde_json::from_str(&json)?;
    assert_eq!(deserialized.cpu_percent, 80.0);
    Ok(())
}

#[test]
fn test_load_prediction_serialization() -> Result<(), Box<dyn std::error::Error>> {
    let pred = LoadPrediction {
        predicted_cpu: 70.0,
        predicted_memory_mb: 1024,
        confidence: 0.9,
    };
    let json = serde_json::to_string(&pred)?;
    let deserialized: LoadPrediction = serde_json::from_str(&json)?;
    assert_eq!(deserialized.confidence, 0.9);
    Ok(())
}

#[test]
fn test_lnn_error_prediction_failed() {
    let err = LnnError::PredictionFailed("no data".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("no data"));
}

#[test]
fn test_buffer_overflow_handling() {
    let mut predictor = AndroidLnnPredictor::new();
    for i in 0..5000 {
        predictor.add_telemetry(TelemetryData {
            cpu_percent: i as f32 % 100.0,
            memory_mb: 256,
            io_mbps: 10.0,
        });
    }
    assert!(predictor.telemetry_count() <= 4096);
}
