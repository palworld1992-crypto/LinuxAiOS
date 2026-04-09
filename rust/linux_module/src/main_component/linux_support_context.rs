//! SupportContext cho Linux Main

#[derive(Debug, Clone, Default)]
pub struct LinuxSupportContext {
    pub memory_tiering: bool,
    pub health_check: bool,
    pub cgroups: bool,
    pub snn_processor: bool,
    pub rl_policy: bool,
    pub hardware_collector: bool,
    pub micro_scheduler: bool,
}
