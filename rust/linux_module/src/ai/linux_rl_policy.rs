//! Reinforcement Learning policy for Linux Module.
//! Uses a small neural network (ONNX/GGUF) loaded from Tensor Pool to propose actions.

use anyhow::{anyhow, Result};
use candle_core::{Device, Tensor};
use candle_nn::{Module, Sequential};
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlAction {
    PageOut(u8),
    Prefetch(u32),
    ActivateModule(u32),
    HibernateModule(u32),
}

impl RlAction {
    pub fn encode(&self) -> u32 {
        match self {
            RlAction::PageOut(level) => 0x1000 | (*level as u32),
            RlAction::Prefetch(idx) => 0x2000 | idx,
            RlAction::ActivateModule(id) => 0x3000 | id,
            RlAction::HibernateModule(id) => 0x4000 | id,
        }
    }

    pub fn decode(value: u32) -> Option<Self> {
        match value >> 12 {
            0x1 => Some(RlAction::PageOut((value & 0xFFF) as u8)),
            0x2 => Some(RlAction::Prefetch(value & 0xFFF)),
            0x3 => Some(RlAction::ActivateModule(value & 0xFFF)),
            0x4 => Some(RlAction::HibernateModule(value & 0xFFF)),
            _ => None,
        }
    }
}

pub struct LinuxRlPolicy {
    model: Option<Sequential>,
    device: Device,
    state_dim: usize,
    action_dim: usize,
    confidence_threshold: f32,
}

impl LinuxRlPolicy {
    pub fn new(model_path: Option<&str>, state_dim: usize, action_dim: usize) -> Result<Self> {
        let device = Device::Cpu;
        let model = if let Some(path) = model_path {
            Self::load_onnx_model(path)?
        } else {
            None
        };
        Ok(Self {
            model,
            device,
            state_dim,
            action_dim,
            confidence_threshold: 0.7,
        })
    }

    fn load_onnx_model(_path: &str) -> Result<Option<Sequential>> {
        // ONNX loading not fully configured; fallback to rule-based policy.
        Ok(None)
    }

    pub fn load_from_buffer(&mut self, _buffer: &[u8]) -> Result<()> {
        self.model = None;
        debug!("RL model loading from buffer is not implemented yet");
        Ok(())
    }

    pub fn recommend(&self, state: &[f32]) -> Result<(RlAction, f32)> {
        if state.len() != self.state_dim {
            anyhow::bail!(
                "State dimension mismatch: expected {}, got {}",
                self.state_dim,
                state.len()
            );
        }

        let (action, confidence) = if let Some(ref model) = self.model {
            // Run inference
            let input = Tensor::new(state, &self.device)?.reshape(&[1, self.state_dim])?;
            let output = model.forward(&input)?;
            let logits: Vec<f32> = output.to_vec1()?;

            // Find best action
            let mut best_idx = 0;
            let mut best_score = f32::NEG_INFINITY;
            for (i, &score) in logits.iter().enumerate() {
                if score > best_score {
                    best_score = score;
                    best_idx = i;
                }
            }

            // Normalize confidence
            let max_val = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let min_val = logits.iter().cloned().fold(f32::INFINITY, f32::min);
            let confidence = if max_val > min_val {
                (best_score - min_val) / (max_val - min_val)
            } else {
                0.5
            };

            // Decode action
            let action =
                RlAction::decode(best_idx as u32).ok_or_else(|| anyhow!("Invalid action index"))?;
            (action, confidence)
        } else {
            // Rule-based fallback
            let action = self
                .rule_based_action(state)
                .ok_or_else(|| anyhow!("No action from rule-based policy"))?;
            (action, 1.0)
        };

        Ok((action, confidence))
    }

    fn rule_based_action(&self, state: &[f32]) -> Option<RlAction> {
        if state.len() >= 3 {
            let cpu = state[0];
            let memory = state[1];
            let io_wait = state[2];

            if cpu > 0.9 || memory > 0.9 {
                return Some(RlAction::HibernateModule(1));
            }
            if cpu < 0.3 && memory < 0.3 {
                return Some(RlAction::ActivateModule(1));
            }
            if io_wait > 0.7 {
                return Some(RlAction::Prefetch(0));
            }
        }
        Some(RlAction::PageOut(1))
    }

    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
    }

    pub fn get_confidence_threshold(&self) -> f32 {
        self.confidence_threshold
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rl_action_encode_decode() {
        let actions = [
            RlAction::PageOut(5),
            RlAction::Prefetch(10),
            RlAction::ActivateModule(3),
            RlAction::HibernateModule(7),
        ];

        for action in actions.iter() {
            let encoded = action.encode();
            let decoded = match RlAction::decode(encoded) {
                Ok(d) => d,
                Err(e) => panic!("decode failed: {:?}", e),
            };
            assert_eq!(action, &decoded);
        }
    }
}
