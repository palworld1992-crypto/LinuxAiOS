use libc::{c_char, c_int, c_uint};

use super::ffi::{VirConnect, VirDomain, VirDomainInfo, VirError};

extern "C" {
    pub fn virConnectOpen(uri: *const c_char) -> *mut VirConnect;
    pub fn virConnectClose(conn: *mut VirConnect) -> c_int;
    pub fn virConnectGetNumOfDomains(conn: *mut VirConnect) -> c_int;
    pub fn virDomainLookupByName(conn: *mut VirConnect, name: *const c_char) -> *mut VirDomain;
    pub fn virDomainCreate(domain: *mut VirDomain) -> c_int;
    pub fn virDomainDestroy(domain: *mut VirDomain) -> c_int;
    pub fn virDomainSuspend(domain: *mut VirDomain) -> c_int;
    pub fn virDomainResume(domain: *mut VirDomain) -> c_int;
    pub fn virDomainGetInfo(domain: *mut VirDomain, info: *mut VirDomainInfo) -> c_int;
    pub fn virDomainFree(domain: *mut VirDomain) -> c_int;
    pub fn virDomainSetMemory(domain: *mut VirDomain, memory: usize) -> c_int;
    pub fn virDomainSetVcpus(domain: *mut VirDomain, nvcpus: c_uint) -> c_int;
    pub fn virDomainMigrateToURI(
        domain: *mut VirDomain,
        duri: *const c_char,
        flags: usize,
        dname: *const c_char,
        bandwidth: usize,
    ) -> *mut VirDomain;
    pub fn virDomainSave(domain: *mut VirDomain, path: *const c_char) -> c_int;
    pub fn virDomainRestore(conn: *mut VirConnect, path: *const c_char) -> c_int;
    pub fn virGetLastError() -> *const VirError;
}