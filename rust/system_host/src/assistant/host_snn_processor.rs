//! Host SNN Processor - Spiking Neural Network for hardware interrupt handling

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct InterruptEvent {
    pub event_type: String,
    pub timestamp: Instant,
    pub urgency: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifState {
    Resting,
    Firing,
    Refractory,
}

pub struct HostSnnProcessor {
    ring_buffer: Arc<DashMap<u64, InterruptEvent>>,
    next_index: Arc<AtomicU64>,
    threshold: f32,
    membrane_potential: f32,
    decay: f32,
    refractory_period: u32,
    state: LifState,
}

impl HostSnnProcessor {
    pub fn new(capacity: usize, threshold: f32, decay: f32) -> Self {
        Self {
            ring_buffer: Arc::new(DashMap::with_capacity(capacity)),
            next_index: Arc::new(AtomicU64::new(0)),
            threshold,
            membrane_potential: 0.0,
            decay,
            refractory_period: 0,
            state: LifState::Resting,
        }
    }

    pub fn push_interrupt(&self, event: InterruptEvent) {
        let index = self.next_index.fetch_add(1, Ordering::Relaxed);
        let oldest_index = index.saturating_sub(4096);
        self.ring_buffer.remove(&oldest_index);
        self.ring_buffer.insert(index, event);
    }

    pub fn process_next(&mut self) -> Option<String> {
        if self.state == LifState::Refractory {
            self.refractory_period = self.refractory_period.saturating_sub(1);
            if self.refractory_period == 0 {
                self.state = LifState::Resting;
            }
            return None;
        }

        let next_index = self.next_index.load(Ordering::Relaxed);
        if next_index == 0 {
            return None;
        }

        let event = self.ring_buffer.remove(&(next_index - 1)).map(|r| r.1);

        if let Some(event) = event {
            let input = match event.urgency {
                0..=100 => 0.3,
                101..=200 => 0.6,
                _ => 0.9,
            };

            self.membrane_potential = self.membrane_potential * self.decay + input;

            if self.membrane_potential >= self.threshold {
                self.state = LifState::Refractory;
                self.membrane_potential = 0.0;
                self.refractory_period = 5;

                return Some(match event.event_type.as_str() {
                    "timer" => "PinCurrentThread".to_string(),
                    "network" => "MigrateThread".to_string(),
                    "io" => "IncreasePriority".to_string(),
                    _ => "DefaultAction".to_string(),
                });
            }
        }

        None
    }

    pub fn get_state(&self) -> LifState {
        self.state
    }

    pub fn get_membrane_potential(&self) -> f32 {
        self.membrane_potential
    }

    pub fn get_pending_count(&self) -> usize {
        self.ring_buffer.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snn_creation() -> anyhow::Result<()> {
        let snn = HostSnnProcessor::new(4096, 0.7, 0.9);
        assert_eq!(snn.get_state(), LifState::Resting);
        Ok(())
    }

    #[test]
    fn test_push_interrupt() -> anyhow::Result<()> {
        let snn = HostSnnProcessor::new(4096, 0.7, 0.9);

        let event = InterruptEvent {
            event_type: "timer".to_string(),
            timestamp: Instant::now(),
            urgency: 150,
        };

        snn.push_interrupt(event);
        assert_eq!(snn.get_pending_count(), 1);

        Ok(())
    }

    #[test]
    fn test_process_spike() -> anyhow::Result<()> {
        let snn = HostSnnProcessor::new(4096, 0.5, 0.9);

        let event = InterruptEvent {
            event_type: "timer".to_string(),
            timestamp: Instant::now(),
            urgency: 200,
        };

        snn.push_interrupt(event);

        let mut snn = snn;
        let action = snn.process_next();

        assert_eq!(action, Some("PinCurrentThread".to_string()));
        assert_eq!(snn.get_state(), LifState::Refractory);

        Ok(())
    }
}
