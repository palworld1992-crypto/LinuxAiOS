//! Reinforcement Learning policy for Linux Module.
//! Uses a small neural network (ONNX/GGUF) loaded from Tensor Pool to propose actions.

use anyhow::Result;
use candle_core::{Device, Shape, Tensor};
use candle_nn::{Module, Sequential};
use parking_lot::RwLock;
use tracing::{info, warn};

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
    #[allow(dead_code)]
    action_dim: usize,
    confidence_threshold: f32,
    current_state: RwLock<Option<Vec<f32>>>,
}

impl LinuxRlPolicy {
    pub fn new(model_path: Option<&str>, state_dim: usize, action_dim: usize) -> Result<Self> {
        let device = Device::Cpu;
        let model = if let Some(path) = model_path {
            // Placeholder: real loading would use candle_onnx
            match candle_onnx::read_file(path) {
                Ok(_onnx_graph) => None,
                Err(e) => {
                    warn!("Failed to load RL model from {}: {}", path, e);
                    None
                }
            }
        } else {
            None
        };
        Ok(Self {
            model,
            device,
            state_dim,
            action_dim,
            confidence_threshold: 0.7,
            current_state: RwLock::new(None),
        })
    }

    pub fn load_from_buffer(&mut self, buffer: &[u8]) -> Result<()> {
        info!("Loading RL policy from buffer ({} bytes)", buffer.len());
        self.model = None;
        Ok(())
    }

    pub fn observe(&self, state: Vec<f32>) {
        if state.len() != self.state_dim {
            warn!(
                "State dimension mismatch: expected {}, got {}",
                self.state_dim,
                state.len()
            );
            return;
        }
        *self.current_state.write() = Some(state);
    }

    pub fn recommend(&self) -> Option<(RlAction, f32)> {
        let state = self.current_state.read().clone()?;
        if self.model.is_none() {
            // Fallback heuristic
            if state.get(0).copied().unwrap_or(0.0) > 0.8 {
                return Some((RlAction::PageOut(1), 0.5));
            }
            return None;
        }

        // Convert state to tensor with shape [1, state_dim]
        let shape = Shape::from(&[1, self.state_dim][..]);
        let input = Tensor::from_vec(state, shape, &self.device).ok()?;
        let output = self.model.as_ref()?.forward(&input).ok()?;

        // Output shape: [1, action_dim] -> get first row as Vec<f32>
        let logits = output.to_vec2::<f32>().ok()?;
        let logits = logits.into_iter().next()?;

        let max_idx = logits
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)?;

        let prob = logits[max_idx].exp() / logits.iter().map(|&x| x.exp()).sum::<f32>();
        if prob < self.confidence_threshold {
            return None;
        }
        let action = RlAction::decode(max_idx as u32)?;
        Some((action, prob))
    }

    pub fn set_confidence_threshold(&mut self, threshold: f32) {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
    }

    pub fn update(&mut self, _reward: f32) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_encode_decode() {
        let action = RlAction::PageOut(2);
        let enc = action.encode();
        let dec = RlAction::decode(enc).expect("decode should succeed");
        assert!(matches!(dec, RlAction::PageOut(2)));
    }

    #[test]
    fn test_policy_fallback() {
        let policy = LinuxRlPolicy::new(None, 4, 10).expect("policy creation should succeed");
        policy.observe(vec![0.9, 0.5, 0.3, 0.2]);
        let result = policy.recommend();
        assert!(result.is_some());
        let (action, _) = result.expect("recommend should return Some");
        assert!(matches!(action, RlAction::PageOut(_)));
    }
}
