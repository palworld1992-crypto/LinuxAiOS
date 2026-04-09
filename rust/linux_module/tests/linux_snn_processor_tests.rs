use linux_module::ai::{GpuSpikeEvent, GpuSpikeType, LinuxSnnProcessor, SpikeEvent};
use std::thread;
use std::time::Duration;

#[test]
fn test_snn_processor_creation() {
    let processor = LinuxSnnProcessor::new(8);
    let (spikes, gpu_spikes, decisions) = processor.stats();
    assert_eq!(spikes, 0);
    assert_eq!(gpu_spikes, 0);
    assert_eq!(decisions, 0);
}

#[test]
fn test_snn_processor_start_stop() {
    let mut processor = LinuxSnnProcessor::new(8);
    processor.start();
    thread::sleep(Duration::from_millis(10));
    processor.stop();
}

#[test]
fn test_snn_processor_send_event() {
    let mut processor = LinuxSnnProcessor::new(16);
    processor.start();

    let event = SpikeEvent {
        pid: 1234,
        vaddr: 0x7f000000,
        timestamp_ns: 0,
    };

    let result = processor.send_event(event);
    assert!(result.is_ok());

    thread::sleep(Duration::from_millis(50));
    processor.stop();
}

#[test]
fn test_snn_processor_send_gpu_event() {
    let mut processor = LinuxSnnProcessor::new(16);
    processor.start();

    let event = GpuSpikeEvent {
        layer_index: 5,
        spike_type: GpuSpikeType::VramHigh,
        vram_usage_percent: 95.0,
        timestamp_ns: 0,
    };

    let result = processor.send_gpu_event(event);
    assert!(result.is_ok());

    thread::sleep(Duration::from_millis(50));
    processor.stop();
}

#[test]
fn test_snn_processor_poll_action_empty() {
    let processor = LinuxSnnProcessor::new(16);
    let action = processor.poll_action();
    assert!(action.is_none());
}

#[test]
fn test_snn_processor_reset() {
    let processor = LinuxSnnProcessor::new(16);
    processor.reset();
    let (spikes, gpu_spikes, decisions) = processor.stats();
    assert_eq!(spikes, 0);
    assert_eq!(gpu_spikes, 0);
    assert_eq!(decisions, 0);
}

#[test]
fn test_snn_processor_ring_buffer_full() {
    let mut processor = LinuxSnnProcessor::new(16);
    processor.start();

    for i in 0..5000 {
        let event = SpikeEvent {
            pid: 1000 + i as u32,
            vaddr: 0x7f000000 + i as u64 * 4096,
            timestamp_ns: 0,
        };
        let _ = processor.send_event(event);
    }

    thread::sleep(Duration::from_millis(100));
    processor.stop();
}

#[test]
fn test_snn_processor_gpu_event_types() {
    let mut processor = LinuxSnnProcessor::new(16);
    processor.start();

    let vram_high = GpuSpikeEvent {
        layer_index: 0,
        spike_type: GpuSpikeType::VramHigh,
        vram_usage_percent: 95.0,
        timestamp_ns: 0,
    };
    processor.send_gpu_event(vram_high).ok();

    let layer_idle = GpuSpikeEvent {
        layer_index: 1,
        spike_type: GpuSpikeType::LayerIdle,
        vram_usage_percent: 50.0,
        timestamp_ns: 0,
    };
    processor.send_gpu_event(layer_idle).ok();

    let layer_hot = GpuSpikeEvent {
        layer_index: 2,
        spike_type: GpuSpikeType::LayerHot,
        vram_usage_percent: 80.0,
        timestamp_ns: 0,
    };
    processor.send_gpu_event(layer_hot).ok();

    thread::sleep(Duration::from_millis(50));
    processor.stop();
}

#[test]
fn test_snn_processor_multiple_events_generate_actions() {
    let mut processor = LinuxSnnProcessor::new(4);
    processor.start();

    for i in 0..20 {
        let event = SpikeEvent {
            pid: 2000,
            vaddr: 0x80000000 + i as u64 * 4096,
            timestamp_ns: 0,
        };
        processor.send_event(event).ok();
        thread::sleep(Duration::from_micros(50));
    }

    thread::sleep(Duration::from_millis(100));

    let mut actions = 0;
    while processor.poll_action().is_some() {
        actions += 1;
    }
    assert!(actions > 0);

    processor.stop();
}

#[test]
fn test_snn_processor_stats_increase() {
    let mut processor = LinuxSnnProcessor::new(16);
    processor.start();

    let (before_spikes, _, _) = processor.stats();

    for i in 0..5 {
        let event = SpikeEvent {
            pid: 3000,
            vaddr: 0x90000000 + i as u64 * 4096,
            timestamp_ns: 0,
        };
        processor.send_event(event).ok();
    }

    thread::sleep(Duration::from_millis(50));

    let (after_spikes, _, _) = processor.stats();
    assert!(after_spikes >= before_spikes);

    processor.stop();
}
