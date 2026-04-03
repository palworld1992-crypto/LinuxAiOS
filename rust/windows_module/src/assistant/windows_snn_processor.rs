//! SNN Processor for Windows Module – Leaky Integrate-and-Fire neuron model

use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SnnError {
    #[error("Neuron error: {0}")]
    NeuronError(String),
    #[error("Spike processing error")]
    SpikeError,
}

#[derive(Clone, Debug)]
pub enum SnnAction {
    PageOut,
    Prefetch,
    Discard,
    FallbackToWine,
}

#[derive(Clone, Debug)]
pub struct SpikeEvent {
    pub event_type: String,
    pub timestamp: u64,
    pub address: u64,
}

#[derive(Debug)]
pub struct NeuronState {
    pub potential: f32,
    pub threshold: f32,
    pub refractory: u32,
    pub spiked: AtomicBool,
}

impl NeuronState {
    pub fn new(threshold: f32) -> Self {
        Self {
            potential: 0.0,
            threshold,
            refractory: 0,
            spiked: AtomicBool::new(false),
        }
    }
}

impl Clone for NeuronState {
    fn clone(&self) -> Self {
        Self {
            potential: self.potential,
            threshold: self.threshold,
            refractory: self.refractory,
            spiked: AtomicBool::new(self.spiked.load(Ordering::Relaxed)),
        }
    }
}

impl NeuronState {
    pub fn integrate(&mut self, input: f32, dt: f32, tau: f32) {
        if self.refractory > 0 {
            self.refractory -= 1;
            self.potential = 0.0;
            return;
        }

        let d_potential = (-self.potential + input) / tau * dt;
        self.potential += d_potential;

        if self.potential >= self.threshold {
            self.spiked.store(true, Ordering::Relaxed);
            self.potential = 0.0;
            self.refractory = 10;
        }
    }

    pub fn fire(&self) -> bool {
        self.spiked.swap(false, Ordering::Relaxed)
    }
}

pub struct WindowsSnnProcessor {
    buffer: RwLock<VecDeque<SpikeEvent>>,
    neuron: RwLock<NeuronState>,
    spike_count: AtomicU64,
    action_count: AtomicU64,
    enabled: AtomicBool,
}

impl WindowsSnnProcessor {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer: RwLock::new(VecDeque::with_capacity(buffer_size)),
            neuron: RwLock::new(NeuronState::new(1.0)),
            spike_count: AtomicU64::new(0),
            action_count: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    pub fn push_event(&self, event: SpikeEvent) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let mut buffer = self.buffer.write();
        if buffer.len() >= buffer.capacity() {
            let _ = buffer.pop_front();
        }
        buffer.push_back(event);
    }

    pub fn process(&self) -> Option<SnnAction> {
        let event = {
            let mut buffer = self.buffer.write();
            buffer.pop_front()
        };

        if let Some(event) = event {
            self.process_event(&event);
            return self.decide_action();
        }

        None
    }

    fn process_event(&self, event: &SpikeEvent) {
        let input = match event.event_type.as_str() {
            "page_fault" => 0.8,
            "memory_pressure" => 0.9,
            "io_wait" => 0.5,
            "cache_miss" => 0.3,
            _ => 0.1,
        };

        let mut neuron = self.neuron.write();
        neuron.integrate(input, 0.01, 0.5);

        if neuron.fire() {
            self.spike_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn decide_action(&self) -> Option<SnnAction> {
        let spikes = self.spike_count.load(Ordering::Relaxed);
        let neuron = self.neuron.read();

        if spikes > 100 && neuron.potential > 0.8 {
            self.action_count.fetch_add(1, Ordering::Relaxed);
            Some(SnnAction::PageOut)
        } else if spikes > 50 {
            self.action_count.fetch_add(1, Ordering::Relaxed);
            Some(SnnAction::Prefetch)
        } else if spikes > 200 {
            self.action_count.fetch_add(1, Ordering::Relaxed);
            Some(SnnAction::FallbackToWine)
        } else {
            None
        }
    }

    pub fn get_spike_count(&self) -> u64 {
        self.spike_count.load(Ordering::Relaxed)
    }

    pub fn get_action_count(&self) -> u64 {
        self.action_count.load(Ordering::Relaxed)
    }

    pub fn reset(&self) {
        self.spike_count.store(0, Ordering::Relaxed);
        self.action_count.store(0, Ordering::Relaxed);
        self.neuron.write().potential = 0.0;
    }

    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snn_creation() {
        let processor = WindowsSnnProcessor::new(1024);
        assert!(processor.is_enabled());
    }

    #[test]
    fn test_push_event() {
        let processor = WindowsSnnProcessor::new(1024);
        let event = SpikeEvent {
            event_type: "page_fault".to_string(),
            timestamp: 1000,
            address: 0x1000,
        };
        processor.push_event(event);
    }

    #[test]
    fn test_neuron_fire() {
        let mut state = NeuronState::new(1.0);
        state.integrate(1.5, 0.01, 0.5);
        assert!(state.fire());
    }
}
