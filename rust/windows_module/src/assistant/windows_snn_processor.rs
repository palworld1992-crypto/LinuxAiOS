//! SNN Processor for Windows Module – Leaky Integrate-and-Fire neuron model
//!
//! According to thietkemoi.txt Phase 4.4.8:
//! - SNN xử lý sự kiện từ Wine/KVM (page fault, memory pressure)
//! - Quyết định fallback hoặc discard page
//! - Dùng ringbuf SPSC để nhận event
//! - Mô phỏng LIF neuron
//!
//! When spike xảy ra, SNN đưa ra action: "discard VM cache" hoặc "fallback to Wine".

use anyhow::Result;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use thiserror::Error;
use tracing::{debug, info};

#[derive(Error, Debug)]
pub enum SnnError {
    #[error("Neuron error: {0}")]
    NeuronError(String),
    #[error("Spike processing error")]
    SpikeError,
    #[error("Ring buffer error: {0}")]
    RingBufferError(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SnnAction {
    PageOut,
    Prefetch,
    Discard,
    FallbackToWine,
    MigrateThread(u32),
    PinCurrentThread(u32),
}

impl SnnAction {
    pub fn priority(&self) -> u8 {
        match self {
            SnnAction::PageOut => 1,
            SnnAction::Discard => 2,
            SnnAction::FallbackToWine => 3,
            SnnAction::Prefetch => 4,
            SnnAction::MigrateThread(_) => 5,
            SnnAction::PinCurrentThread(_) => 6,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpikeEvent {
    pub event_type: SpikeEventType,
    pub timestamp: u64,
    pub address: u64,
    pub source: EventSource,
    pub severity: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SpikeEventType {
    PageFault,
    MemoryPressure,
    IoWait,
    CacheMiss,
    EptViolation,
    CpuStarvation,
    NetworkLatency,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EventSource {
    Wine,
    Kvm,
    Host,
}

impl SpikeEventType {
    pub fn to_weight(&self) -> f32 {
        match self {
            SpikeEventType::PageFault => 0.7,
            SpikeEventType::MemoryPressure => 0.9,
            SpikeEventType::IoWait => 0.4,
            SpikeEventType::CacheMiss => 0.3,
            SpikeEventType::EptViolation => 0.8,
            SpikeEventType::CpuStarvation => 0.5,
            SpikeEventType::NetworkLatency => 0.4,
        }
    }
}

#[derive(Debug)]
pub struct NeuronState {
    pub potential: f32,
    pub threshold: f32,
    pub refractory_counter: u32,
    pub spiked: AtomicBool,
    pub last_update: u64,
}

impl NeuronState {
    pub fn new(threshold: f32) -> Self {
        Self {
            potential: 0.0,
            threshold,
            refractory_counter: 0,
            spiked: AtomicBool::new(false),
            last_update: 0,
        }
    }

    pub fn integrate(&mut self, input: f32, dt: f32, tau: f32) {
        if self.refractory_counter > 0 {
            self.refractory_counter -= 1;
            self.potential = 0.0;
            return;
        }

        let d_potential = (-self.potential + input) / tau * dt;
        self.potential += d_potential;

        if self.potential >= self.threshold {
            self.spiked.store(true, Ordering::Relaxed);
            self.potential = 0.0;
            self.refractory_counter = 10;
        }
    }

    pub fn fire(&self) -> bool {
        self.spiked.swap(false, Ordering::Relaxed)
    }

    pub fn reset(&mut self) {
        self.potential = 0.0;
        self.refractory_counter = 0;
        self.spiked.store(false, Ordering::Relaxed);
    }
}

impl Clone for NeuronState {
    fn clone(&self) -> Self {
        Self {
            potential: self.potential,
            threshold: self.threshold,
            refractory_counter: self.refractory_counter,
            spiked: AtomicBool::new(self.spiked.load(Ordering::Relaxed)),
            last_update: self.last_update,
        }
    }
}

pub struct WindowsSnnProcessor {
    event_buffer: Vec<SpikeEvent>,
    buffer_size: usize,
    write_pos: AtomicU64,
    neurons: DashMap<String, NeuronState>,
    spike_count: AtomicU64,
    action_count: AtomicU64,
    enabled: AtomicBool,
    spike_threshold: AtomicU64,
    tau: f32,
    dt: f32,
}

impl WindowsSnnProcessor {
    pub fn new(buffer_size: usize) -> Self {
        let processor = Self {
            event_buffer: Vec::with_capacity(buffer_size),
            buffer_size,
            write_pos: AtomicU64::new(0),
            neurons: DashMap::new(),
            spike_count: AtomicU64::new(0),
            action_count: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
            spike_threshold: AtomicU64::new(10),
            tau: 0.5,
            dt: 0.01,
        };

        processor.initialize_neurons();
        processor
    }

    fn initialize_neurons(&self) {
        self.neurons
            .insert("memory".to_string(), NeuronState::new(0.7));
        self.neurons.insert("io".to_string(), NeuronState::new(0.5));
        self.neurons
            .insert("cpu".to_string(), NeuronState::new(0.6));
        self.neurons
            .insert("network".to_string(), NeuronState::new(0.4));
        self.neurons
            .insert("global".to_string(), NeuronState::new(0.8));

        info!(
            "SNN processor initialized with {} neurons",
            self.neurons.len()
        );
    }

    pub fn push_event(&mut self, event: SpikeEvent) -> Result<(), SnnError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        let pos = self.write_pos.fetch_add(1, Ordering::Relaxed) as usize;
        let idx = pos % self.buffer_size;
        let event_type = event.event_type.clone();

        if idx >= self.event_buffer.len() {
            self.event_buffer.push(event);
        } else {
            self.event_buffer[idx] = event;
        }

        debug!("Pushed event: {:?} at pos {}", event_type, pos);
        Ok(())
    }

    pub fn push_events(&mut self, events: &[SpikeEvent]) -> Result<(), SnnError> {
        for event in events {
            self.push_event(event.clone())?;
        }
        Ok(())
    }

    pub fn process_batch(&self) -> Vec<SnnAction> {
        let events = self.read_recent_events();
        if events.is_empty() {
            return Vec::new();
        }

        let mut actions = Vec::new();

        for event in &events {
            self.process_event(event);
        }

        if let Some(action) = self.decide_action() {
            actions.push(action);
            self.action_count.fetch_add(1, Ordering::Relaxed);
        }

        actions
    }

    pub fn process(&self) -> Option<SnnAction> {
        let events = self.read_recent_events();
        if events.is_empty() {
            return None;
        }

        for event in &events {
            self.process_event(event);
        }

        let action = self.decide_action();
        if action.is_some() {
            self.action_count.fetch_add(1, Ordering::Relaxed);
        }
        action
    }

    fn read_recent_events(&self) -> Vec<SpikeEvent> {
        let pos = self.write_pos.load(Ordering::Relaxed) as usize;
        let count = pos.min(self.buffer_size);

        if count == 0 {
            return Vec::new();
        }

        let start = if count >= self.buffer_size {
            count - self.buffer_size
        } else {
            0
        };

        let mut events = Vec::with_capacity(count - start);
        for i in start..count {
            let idx = i % self.buffer_size;
            if idx < self.event_buffer.len() {
                events.push(self.event_buffer[idx].clone());
            }
        }

        events
    }

    fn process_event(&self, event: &SpikeEvent) {
        let input = event.event_type.to_weight() * event.severity;

        let neuron_id = match event.event_type {
            SpikeEventType::PageFault
            | SpikeEventType::MemoryPressure
            | SpikeEventType::EptViolation => "memory",
            SpikeEventType::IoWait | SpikeEventType::NetworkLatency => "io",
            SpikeEventType::CpuStarvation => "cpu",
            SpikeEventType::CacheMiss => "global",
        };

        if let Some(mut neuron) = self.neurons.get_mut(neuron_id) {
            neuron.integrate(input, self.dt, self.tau);
            if neuron.fire() {
                self.spike_count.fetch_add(1, Ordering::Relaxed);
                debug!(
                    "Neuron {} fired for event {:?}",
                    neuron_id, event.event_type
                );
            }
        }
    }

    fn decide_action(&self) -> Option<SnnAction> {
        let spikes = self.spike_count.load(Ordering::Relaxed);
        let events = self.read_recent_events();

        if events.is_empty() {
            return None;
        }

        let last_event = events.last()?;

        let spike_rate = spikes as f32 / events.len() as f32;

        if spike_rate > 0.5 || spikes > self.spike_threshold.load(Ordering::Relaxed) {
            let action = match last_event.event_type {
                SpikeEventType::PageFault
                | SpikeEventType::MemoryPressure
                | SpikeEventType::EptViolation => {
                    if spike_rate > 0.8 {
                        SnnAction::FallbackToWine
                    } else {
                        SnnAction::Discard
                    }
                }
                SpikeEventType::IoWait | SpikeEventType::NetworkLatency => SnnAction::PageOut,
                SpikeEventType::CacheMiss => SnnAction::Prefetch,
                SpikeEventType::CpuStarvation => SnnAction::FallbackToWine,
            };

            self.spike_count.store(0, Ordering::Relaxed);
            return Some(action);
        }

        None
    }

    pub fn add_neuron(&self, id: &str, threshold: f32) {
        self.neurons
            .insert(id.to_string(), NeuronState::new(threshold));
        info!("Added neuron: {} with threshold {}", id, threshold);
    }

    pub fn set_threshold(&self, neuron_id: &str, threshold: f32) {
        if let Some(mut neuron) = self.neurons.get_mut(neuron_id) {
            neuron.threshold = threshold;
        }
    }

    pub fn reset_neurons(&self) {
        for mut entry in self.neurons.iter_mut() {
            entry.value_mut().reset();
        }
        self.spike_count.store(0, Ordering::Relaxed);
    }

    pub fn get_spike_count(&self) -> u64 {
        self.spike_count.load(Ordering::Relaxed)
    }

    pub fn get_action_count(&self) -> u64 {
        self.action_count.load(Ordering::Relaxed)
    }

    pub fn get_neuron_state(&self, id: &str) -> Option<(f32, f32)> {
        self.neurons.get(id).map(|n| (n.potential, n.threshold))
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        info!("SNN processor enabled: {}", enabled);
    }

    pub fn set_spike_threshold(&self, threshold: u64) {
        self.spike_threshold.store(threshold, Ordering::Relaxed);
    }

    pub fn get_buffer_size(&self) -> usize {
        self.buffer_size
    }

    pub fn clear_buffer(&mut self) {
        self.event_buffer.clear();
        self.write_pos.store(0, Ordering::Relaxed);
    }
}

impl Default for WindowsSnnProcessor {
    fn default() -> Self {
        Self::new(4096)
    }
}
