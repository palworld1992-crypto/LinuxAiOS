//! Spiking Neural Network (SNN) processor for real‑time event handling.
//! Uses Leaky Integrate‑and‑Fire (LIF) neurons to respond to eBPF spikes and GPU events.
//!
//! Per spec Section 3.10.4: Thêm logic xử lý spike từ GPU monitor:
//! khi VRAM đầy hoặc layer nguội, phát spike → gọi `demote_layer_to_ram` hoặc `demote_layer_to_nvme`.

use crossbeam::channel::{bounded, Receiver, Sender};
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use tracing::{info, warn};

const SPIKE_CHANNEL_SIZE: usize = 4096;
const ACTION_CHANNEL_SIZE: usize = 1024;
const STATS_MAP_SIZE: usize = 64;

#[derive(Debug, Clone, Copy)]
pub struct SpikeEvent {
    pub pid: u32,
    pub vaddr: u64,
    pub timestamp_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GpuSpikeType {
    VramHigh,
    LayerIdle,
    LayerHot,
}

#[derive(Debug, Clone, Copy)]
pub struct GpuSpikeEvent {
    pub layer_index: usize,
    pub spike_type: GpuSpikeType,
    pub vram_usage_percent: f32,
    pub timestamp_ns: u64,
}

pub type SnnAction = (u32, u64);

#[derive(Clone)]
struct LifNeuron {
    v: f32,
    leak: f32,
    threshold: f32,
    reset: f32,
    weight: f32,
}

impl LifNeuron {
    fn new(leak: f32, threshold: f32, reset: f32, weight: f32) -> Self {
        Self {
            v: 0.0,
            leak,
            threshold,
            reset,
            weight,
        }
    }

    fn step(&mut self, input: f32) -> bool {
        self.v = self.v * self.leak + input * self.weight;
        if self.v >= self.threshold {
            self.v = self.reset;
            true
        } else {
            false
        }
    }
}

pub struct LinuxSnnProcessor {
    neurons: DashMap<usize, LifNeuron>,
    spike_tx: Sender<SpikeEvent>,
    spike_rx: Receiver<SpikeEvent>,
    gpu_spike_tx: Sender<GpuSpikeEvent>,
    gpu_spike_rx: Receiver<GpuSpikeEvent>,
    action_tx: Sender<SnnAction>,
    action_rx: Receiver<SnnAction>,
    running: Arc<AtomicBool>,
    thread_handle: Option<thread::JoinHandle<()>>,
    stats: DashMap<String, u64>,
}

impl LinuxSnnProcessor {
    pub fn new(num_neurons: usize) -> Self {
        let neurons = DashMap::with_capacity(num_neurons);
        for i in 0..num_neurons {
            neurons.insert(i, LifNeuron::new(0.95, 1.0, 0.0, 1.0));
        }

        let (spike_tx, spike_rx) = bounded(SPIKE_CHANNEL_SIZE);
        let (gpu_spike_tx, gpu_spike_rx) = bounded(SPIKE_CHANNEL_SIZE);
        let (action_tx, action_rx) = bounded(ACTION_CHANNEL_SIZE);

        Self {
            neurons,
            spike_tx,
            spike_rx,
            gpu_spike_tx,
            gpu_spike_rx,
            action_tx,
            action_rx,
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: None,
            stats: DashMap::with_capacity(STATS_MAP_SIZE),
        }
    }

    pub fn send_event(&self, event: SpikeEvent) -> Result<(), String> {
        match self.spike_tx.send(event) {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("Spike channel full: {:?}", e);
                Err("Spike channel full".to_string())
            }
        }
    }

    pub fn send_gpu_event(&self, event: GpuSpikeEvent) -> Result<(), String> {
        match self.gpu_spike_tx.send(event) {
            Ok(_) => Ok(()),
            Err(e) => {
                warn!("GPU spike channel full: {:?}", e);
                Err("GPU spike channel full".to_string())
            }
        }
    }

    pub fn poll_action(&self) -> Option<SnnAction> {
        self.action_rx.try_recv().ok()
    }

    pub fn start(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            return;
        }
        self.running.store(true, Ordering::Relaxed);
        let running = self.running.clone();
        let spike_rx = self.spike_rx.clone();
        let gpu_spike_rx = self.gpu_spike_rx.clone();
        let action_tx = self.action_tx.clone();
        let neurons = self.neurons.clone();
        let stats = self.stats.clone();

        let handle = thread::spawn(move || {
            let mut last_stats_log = Instant::now();
            let neuron_len = neurons.len();

            while running.load(Ordering::Relaxed) {
                let event = spike_rx.recv_timeout(Duration::from_micros(100)).ok();

                if let Some(event) = event {
                    let mut counter = stats.entry("total_spikes".to_string()).or_insert(0);
                    *counter += 1;

                    stats.insert(
                        "last_spike_time".to_string(),
                        Instant::now().elapsed().as_nanos() as u64,
                    );

                    let idx = (event.vaddr as usize) % neuron_len;
                    if let Some(mut neuron_ref) = neurons.get_mut(&idx) {
                        if neuron_ref.step(1.0) {
                            let action = (event.pid, event.vaddr);
                            if let Err(e) = action_tx.send(action) {
                                warn!("Action channel overflow: {:?}", e);
                            }
                            let mut counter =
                                stats.entry("decisions_made".to_string()).or_insert(0);
                            *counter += 1;
                        }
                    }
                }

                let gpu_event = gpu_spike_rx.recv_timeout(Duration::from_micros(100)).ok();

                if let Some(gpu_event) = gpu_event {
                    let mut counter = stats.entry("total_gpu_spikes".to_string()).or_insert(0);
                    *counter += 1;

                    stats.insert(
                        "last_spike_time".to_string(),
                        Instant::now().elapsed().as_nanos() as u64,
                    );

                    let idx = gpu_event.layer_index % neuron_len;
                    if let Some(mut neuron_ref) = neurons.get_mut(&idx) {
                        let spike_input = match gpu_event.spike_type {
                            GpuSpikeType::VramHigh => 2.0,
                            GpuSpikeType::LayerIdle => 1.5,
                            GpuSpikeType::LayerHot => 0.5,
                        };

                        if neuron_ref.step(spike_input) {
                            let action = (0, gpu_event.layer_index as u64);
                            if let Err(e) = action_tx.send(action) {
                                warn!("Action channel overflow: {:?}", e);
                            }
                            let mut counter =
                                stats.entry("decisions_made".to_string()).or_insert(0);
                            *counter += 1;
                        }
                    }
                }

                if event.is_none() && gpu_event.is_none() {
                    thread::sleep(Duration::from_micros(100));
                }

                if last_stats_log.elapsed() >= Duration::from_secs(10) {
                    let total_spikes = *stats.entry("total_spikes".to_string()).or_insert(0);
                    let total_gpu_spikes =
                        *stats.entry("total_gpu_spikes".to_string()).or_insert(0);
                    let decisions_made = *stats.entry("decisions_made".to_string()).or_insert(0);
                    info!(
                        "SNN stats: spikes={}, gpu_spikes={}, decisions={}",
                        total_spikes, total_gpu_spikes, decisions_made
                    );
                    last_stats_log = Instant::now();
                }
            }
            info!("SNN processor stopped.");
        });
        self.thread_handle.replace(handle);
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn reset(&self) {
        for mut neuron_ref in self.neurons.iter_mut() {
            neuron_ref.v = 0.0;
        }
    }

    pub fn stats(&self) -> (u64, u64, u64) {
        let total_spikes = *self.stats.entry("total_spikes".to_string()).or_insert(0);
        let total_gpu_spikes = *self
            .stats
            .entry("total_gpu_spikes".to_string())
            .or_insert(0);
        let decisions_made = *self.stats.entry("decisions_made".to_string()).or_insert(0);
        (total_spikes, total_gpu_spikes, decisions_made)
    }
}

impl Drop for LinuxSnnProcessor {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_snn_processor() {
        let mut processor = LinuxSnnProcessor::new(16);
        processor.start();

        for i in 0..10 {
            let event = SpikeEvent {
                pid: 1234,
                vaddr: 0x7f000000 + i * 4096,
                timestamp_ns: 0,
            };
            let _ = processor.send_event(event);
            thread::sleep(Duration::from_micros(10));
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
    fn test_snn_gpu_event() {
        let mut processor = LinuxSnnProcessor::new(16);
        processor.start();

        let event = GpuSpikeEvent {
            layer_index: 5,
            spike_type: GpuSpikeType::VramHigh,
            vram_usage_percent: 95.0,
            timestamp_ns: 0,
        };
        let _ = processor.send_gpu_event(event);

        thread::sleep(Duration::from_millis(50));
        processor.stop();
    }
}
