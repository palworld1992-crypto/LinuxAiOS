use libc::{c_char, c_int, c_uint};

#[repr(C)]
pub struct VirConnect {
    _private: [u8; 0],
}

#[repr(C)]
pub struct VirDomain {
    _private: [u8; 0],
}

#[repr(C)]
pub struct VirError {
    pub code: c_int,
    pub domain: c_int,
    pub message: *mut c_char,
    pub level: c_int,
}

#[repr(C)]
pub struct VirDomainInfo {
    pub state: c_char,
    pub max_mem: usize,
    pub memory: usize,
    pub nr_virt_cpu: c_uint,
    pub cpu_time: usize,
}