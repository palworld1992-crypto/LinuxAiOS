//! SupportContext cho Linux Main

#[derive(Debug, Clone, Default)]
pub struct LinuxSupportContext {
    pub memory_tiering: bool,
    pub health_check: bool,
    pub cgroups: bool,
}
