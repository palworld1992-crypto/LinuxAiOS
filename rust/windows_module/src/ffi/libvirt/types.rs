use libc::c_char;

#[derive(Clone, Debug)]
pub struct DomainInfo {
    pub id: Option<i32>,
    pub name: String,
    pub state: DomainState,
    pub cpu_time: u64,
    pub memory_bytes: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DomainState {
    Running,
    Paused,
    Shutdown,
    Crashed,
    Suspended,
    Unknown,
}

impl DomainState {
    pub fn from_vir_state(state: c_char) -> Self {
        match state {
            1 => DomainState::Running,
            2 => DomainState::Paused,
            3 => DomainState::Paused,
            4 => DomainState::Shutdown,
            5 => DomainState::Crashed,
            6 => DomainState::Suspended,
            _ => DomainState::Unknown,
        }
    }
}

impl From<super::ffi::VirDomainInfo> for DomainInfo {
    fn from(info: super::ffi::VirDomainInfo) -> Self {
        DomainInfo {
            id: None,
            name: "unknown".to_string(),
            state: DomainState::from_vir_state(info.state),
            cpu_time: info.cpu_time as u64,
            memory_bytes: info.memory as u64 * 1024,
        }
}