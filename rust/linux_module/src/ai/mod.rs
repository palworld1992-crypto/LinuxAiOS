//! AI components for Linux Module.
//! Includes LNN (Liquid Time-Constant Network), SNN (Spiking Neural Network),
//! RL policy, GPU monitor, and the main Linux Assistant orchestrator.

mod linux_assistant;
mod linux_gpu_monitor;
mod linux_lnn_predictor;
pub mod linux_rl_policy;
mod linux_snn_processor;

pub use linux_assistant::{AssistantConfig, LinuxAssistant, RlState};
pub use linux_gpu_monitor::{GpuBackend, GpuEvent, GpuEventType, GpuMonitor, GpuMonitorConfig};
pub use linux_lnn_predictor::LinuxLnnPredictor;
pub use linux_rl_policy::{LinuxRlPolicy, RlAction};
pub use linux_snn_processor::{GpuSpikeEvent, GpuSpikeType, LinuxSnnProcessor, SnnAction, SpikeEvent};
