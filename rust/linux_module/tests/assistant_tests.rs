use child_tunnel::ChildTunnel;
use dashmap::DashMap;
use linux_module::ai::{AssistantConfig, LinuxAssistant, LinuxSnnProcessor, RlState, SpikeEvent};
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
fn test_init_models() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(DashMap::with_capacity(1));
        let pool = TensorPool::new("test_pool", 1024 * 1024)?;
        tensor_pool.insert((), pool);

        let config = AssistantConfig {
            lnn_input_dim: 3,
            lnn_output_dim: 3,
            rl_state_dim: 3,
            rl_action_dim: 4,
            inference_interval_ms: 100,
            spike_threshold: 0.7,
        };
        let child_tunnel = Arc::new(ChildTunnel::default());
        let assistant = LinuxAssistant::new(tensor_pool, config, None, None, child_tunnel);
        let result = assistant.init_models();
        assert!(result.is_ok());
        Ok(())
    })
}

#[test]
fn test_predict_spike() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(DashMap::with_capacity(1));
        let pool = TensorPool::new("test_pool", 1024 * 1024)?;
        tensor_pool.insert((), pool);

        let config = AssistantConfig {
            lnn_input_dim: 3,
            lnn_output_dim: 3,
            rl_state_dim: 3,
            rl_action_dim: 4,
            inference_interval_ms: 100,
            spike_threshold: 0.7,
        };
        let child_tunnel = Arc::new(ChildTunnel::default());
        let assistant = LinuxAssistant::new(tensor_pool, config, None, None, child_tunnel);
        assistant.init_models()?;
        let features = vec![0.5, 0.6, 0.7];
        let result = assistant.predict_spike(&features);
        assert!(result.is_ok());
        let (cpu, ram, io) = result?;
        assert!((0.0..=1.0).contains(&cpu));
        assert!((0.0..=1.0).contains(&ram));
        assert!((0.0..=1.0).contains(&io));
        Ok(())
    })
}

#[test]
fn test_propose_policy() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let tensor_pool = Arc::new(DashMap::with_capacity(1));
        let pool = TensorPool::new("test_pool", 1024 * 1024)?;
        tensor_pool.insert((), pool);

        let config = AssistantConfig {
            lnn_input_dim: 3,
            lnn_output_dim: 3,
            rl_state_dim: 3,
            rl_action_dim: 4,
            inference_interval_ms: 100,
            spike_threshold: 0.7,
        };
        let child_tunnel = Arc::new(ChildTunnel::default());
        let assistant = LinuxAssistant::new(tensor_pool, config, None, None, child_tunnel);
        assistant.init_models()?;
        let state = RlState {
            cpu_load: 0.8,
            mem_usage: 0.9,
            page_fault_rate: 0.1,
            active_modules: vec!["windows".to_string()],
        };
        let action = assistant.propose_policy(state);
        let _ = action;
        Ok(())
    })
}

#[test]
fn test_snn_processor() -> Result<(), Box<dyn std::error::Error>> {
    with_temp_base(|| {
        let mut snn = LinuxSnnProcessor::new(16);
        snn.start();
        let event = SpikeEvent {
            pid: 1234,
            vaddr: 0x1000,
            timestamp_ns: 0,
        };
        snn.send_event(event)?;
        std::thread::sleep(std::time::Duration::from_millis(50));
        snn.stop();
        Ok(())
    })
}
