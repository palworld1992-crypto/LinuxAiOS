use linux_module::ai::LinuxLnnPredictor;

#[test]
fn test_lnn_predictor_creation() {
    let predictor = LinuxLnnPredictor::new(4, 3, 0.1, 100);
    assert_eq!(predictor.last_output().len(), 3);
}

#[test]
fn test_lnn_predictor_prediction() {
    let mut predictor = LinuxLnnPredictor::new(4, 3, 0.1, 100);
    let features = vec![0.5, 0.3, 0.7, 0.2];
    let result = predictor.predict(&features);
    assert_eq!(result.len(), 3);
}

#[test]
fn test_lnn_predictor_dimension_mismatch() {
    let mut predictor = LinuxLnnPredictor::new(4, 3, 0.1, 100);
    let features = vec![0.5, 0.3];
    let result = predictor.predict(&features);
    assert_eq!(result.len(), 3);
    assert!(result.iter().all(|&x| x == 0.0));
}

#[test]
fn test_lnn_predictor_reset() {
    let mut predictor = LinuxLnnPredictor::new(4, 3, 0.1, 100);
    let features = vec![0.5, 0.3, 0.7, 0.2];
    predictor.predict(&features);
    predictor.reset();
    let output = predictor.last_output();
    assert!(output.iter().all(|&x| x == 0.0));
}

#[test]
fn test_lnn_predictor_history_limit() {
    let mut predictor = LinuxLnnPredictor::new(4, 3, 0.1, 5);
    let features = vec![0.5, 0.3, 0.7, 0.2];

    for _ in 0..10 {
        let result = predictor.predict(&features);
        assert_eq!(result.len(), 3);
    }
}

#[test]
fn test_layer_access_recording() {
    let mut predictor = LinuxLnnPredictor::new(4, 8, 0.1, 100).with_num_layers(8);

    for i in 0..8 {
        predictor.record_layer_access(i);
    }

    let suggestions = predictor.predict_layers_to_prefetch(5);
    assert!(suggestions.len() <= 3);
}

#[test]
fn test_layer_prefetch_suggestions_sorted() {
    let mut predictor = LinuxLnnPredictor::new(4, 10, 0.1, 100).with_num_layers(10);

    for _ in 0..20 {
        predictor.record_layer_access(0);
        predictor.record_layer_access(1);
        predictor.record_layer_access(2);
    }

    let suggestions = predictor.predict_layers_to_prefetch(5);
    for i in 1..suggestions.len() {
        assert!(suggestions[i - 1].priority >= suggestions[i].priority);
    }
}

#[test]
fn test_layer_prefetch_truncated_to_three() {
    let mut predictor = LinuxLnnPredictor::new(4, 10, 0.1, 100).with_num_layers(10);

    for i in 0..10 {
        for _ in 0..5 {
            predictor.record_layer_access(i);
        }
    }

    let suggestions = predictor.predict_layers_to_prefetch(5);
    assert!(suggestions.len() <= 3);
}

#[test]
fn test_load_weights_empty() {
    let mut predictor = LinuxLnnPredictor::new(4, 3, 0.1, 100);
    let result = predictor.load_weights(&[]);
    assert!(result.is_ok());
}

#[test]
fn test_load_weights_non_empty() {
    let mut predictor = LinuxLnnPredictor::new(4, 3, 0.1, 100);
    let result = predictor.load_weights(&[1, 2, 3, 4]);
    assert!(result.is_ok());
}

#[test]
fn test_multiple_predictions() {
    let mut predictor = LinuxLnnPredictor::new(4, 3, 0.1, 100);

    for i in 0..5 {
        let features = vec![i as f32 * 0.1, 0.3, 0.7, 0.2];
        let result = predictor.predict(&features);
        assert_eq!(result.len(), 3);
    }
}
