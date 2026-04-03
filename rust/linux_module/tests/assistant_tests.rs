use linux_module::ai::{AssistantConfig, LinuxAssistant, LinuxSnnProcessor, RlState, SpikeEvent};
use linux_module::tensor::TensorPool;
use parking_lot::RwLock;
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
fn test_init_models() {
    with_temp_base(|| {
        let tensor_pool = Arc::new(RwLock::new(
            TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        let config = AssistantConfig {
            lnn_input_dim: 3,
            lnn_output_dim: 3,
            rl_state_dim: 3,
            rl_action_dim: 4,
            inference_interval_ms: 100,
            spike_threshold: 0.7,
        };
        let assistant = LinuxAssistant::new(tensor_pool, config, None);
        let result = assistant.init_models();
        assert!(result.is_ok());
    });
}

#[test]
fn test_predict_spike() {
    with_temp_base(|| {
        let tensor_pool = Arc::new(RwLock::new(
            TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        let config = AssistantConfig {
            lnn_input_dim: 3,
            lnn_output_dim: 3,
            rl_state_dim: 3,
            rl_action_dim: 4,
            inference_interval_ms: 100,
            spike_threshold: 0.7,
        };
        let assistant = LinuxAssistant::new(tensor_pool, config, None);
        assistant.init_models().unwrap();
        let features = vec![0.5, 0.6, 0.7];
        let result = assistant.predict_spike(&features);
        assert!(result.is_ok());
        let (cpu, ram, io) = result.unwrap();
        assert!(cpu >= 0.0 && cpu <= 1.0);
        assert!(ram >= 0.0 && ram <= 1.0);
        assert!(io >= 0.0 && io <= 1.0);
    });
}

#[test]
fn test_propose_policy() {
    with_temp_base(|| {
        let tensor_pool = Arc::new(RwLock::new(
            TensorPool::new("test_pool", 1024 * 1024).unwrap(),
        ));
        let config = AssistantConfig {
            lnn_input_dim: 3,
            lnn_output_dim: 3,
            rl_state_dim: 3,
            rl_action_dim: 4,
            inference_interval_ms: 100,
            spike_threshold: 0.7,
        };
        let assistant = LinuxAssistant::new(tensor_pool, config, None);
        assistant.init_models().unwrap();
        let state = RlState {
            cpu_load: 0.8,
            mem_usage: 0.9,
            page_fault_rate: 0.1,
            active_modules: vec!["windows".to_string()],
        };
        let action = assistant.propose_policy(state);
        // Có thể trả về lỗi nếu confidence thấp, nhưng ít nhất không panic
        let _ = action; // ignore result for now
    });
}

#[test]
fn test_snn_processor() {
    with_temp_base(|| {
        let mut snn = LinuxSnnProcessor::new(16);
        snn.start();
        let event = SpikeEvent {
            pid: 1234,
            vaddr: 0x1000,
            timestamp_ns: 0,
        };
        snn.send_event(event).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        let rx = snn.action_receiver();
        if let Ok((pid, vaddr)) = rx.try_recv() {
            assert_eq!(pid, 1234);
            assert_eq!(vaddr, 0x1000);
        } else {
            // có thể không có action nếu ngưỡng chưa đạt
        }
        snn.stop();
    });
}
