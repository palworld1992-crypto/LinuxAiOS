mod handshake;
mod parallel;
mod session;
mod single;

pub use handshake::{client_handshake, server_handshake, SessionKey};
pub use parallel::ParallelEncapsulator;
pub use session::{PeerId, SessionManager};
pub use single::SingleEncapsulator;

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

// ========== eBPF router FFI (cross‑platform) ==========
#[cfg(all(target_os = "linux", not(tarpaulin)))]
mod ebpf_ffi {
    use std::ffi::CStr;
    use std::os::raw::c_char;

    extern "C" {
        pub fn zig_init_ipc_router(prog_path: *const c_char) -> i32;
        pub fn zig_update_route(map_fd: i32, src_peer: u64, dst_sock: u32) -> i32;
        pub fn zig_remove_route(map_fd: i32, src_peer: u64) -> i32;
        pub fn zig_set_sockmap_prog(map_fd: i32, prog_fd: i32) -> i32;
    }

    pub fn init_ipc_router(prog_path: &CStr) -> anyhow::Result<i32> {
        let fd = unsafe { zig_init_ipc_router(prog_path.as_ptr()) };
        if fd < 0 {
            anyhow::bail!("Failed to init IPC router: {}", fd);
        }
        Ok(fd)
    }

    pub fn update_route(map_fd: i32, src_peer: u64, dst_sock: u32) -> anyhow::Result<()> {
        let ret = unsafe { zig_update_route(map_fd, src_peer, dst_sock) };
        if ret < 0 {
            anyhow::bail!("Failed to update route: {}", ret);
        }
        Ok(())
    }

    pub fn remove_route(map_fd: i32, src_peer: u64) -> anyhow::Result<()> {
        let ret = unsafe { zig_remove_route(map_fd, src_peer) };
        if ret < 0 {
            anyhow::bail!("Failed to remove route: {}", ret);
        }
        Ok(())
    }

    pub fn set_sockmap_prog(map_fd: i32, prog_fd: i32) -> anyhow::Result<()> {
        let ret = unsafe { zig_set_sockmap_prog(map_fd, prog_fd) };
        if ret < 0 {
            anyhow::bail!("Failed to set sockmap prog: {}", ret);
        }
        Ok(())
    }
}

#[cfg(any(not(target_os = "linux"), tarpaulin))]
mod ebpf_ffi {
    use anyhow::anyhow;
    use std::ffi::CStr;

    pub fn init_ipc_router(_prog_path: &CStr) -> anyhow::Result<i32> {
        Err(anyhow!("eBPF not supported on this OS"))
    }

    pub fn update_route(_map_fd: i32, _src_peer: u64, _dst_sock: u32) -> anyhow::Result<()> {
        Err(anyhow!("eBPF not supported on this OS"))
    }

    pub fn remove_route(_map_fd: i32, _src_peer: u64) -> anyhow::Result<()> {
        Err(anyhow!("eBPF not supported on this OS"))
    }

    pub fn set_sockmap_prog(_map_fd: i32, _prog_fd: i32) -> anyhow::Result<()> {
        Err(anyhow!("eBPF not supported on this OS"))
    }
}

use ebpf_ffi as zig_bindings;
// ===========================================

pub struct TransportTunnel {
    session_mgr: SessionManager,
    my_kyber_priv: [u8; 2400],
    my_dilithium_priv: [u8; 4032],
    _my_dilithium_pub: [u8; 1952],
    peer_kyber_pub_cache: Arc<RwLock<HashMap<PeerId, [u8; 1568]>>>,
    peer_dilithium_pub_cache: Arc<RwLock<HashMap<PeerId, [u8; 1952]>>>,
    counters: Arc<RwLock<HashMap<PeerId, AtomicU64>>>,
    ebpf_map_fd: RwLock<Option<i32>>,
}

impl TransportTunnel {
    pub fn new(
        my_kyber_priv: [u8; 2400],
        my_dilithium_priv: [u8; 4032],
        my_dilithium_pub: [u8; 1952],
    ) -> Self {
        Self {
            session_mgr: SessionManager::new(),
            my_kyber_priv,
            my_dilithium_priv,
            _my_dilithium_pub: my_dilithium_pub,
            peer_kyber_pub_cache: Arc::new(RwLock::new(HashMap::new())),
            peer_dilithium_pub_cache: Arc::new(RwLock::new(HashMap::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            ebpf_map_fd: RwLock::new(None),
        }
    }

    /// Attach eBPF program to a pre‑existing map (legacy compatibility)
    pub fn attach_ebpf(&self, prog_fd: i32, map_fd: i32) -> Result<()> {
        zig_bindings::set_sockmap_prog(map_fd, prog_fd)
    }

    /// Initialize the eBPF IPC router: load sockmap program, create map, attach program.
    pub fn init_ebpf_router(&self, prog_path: &std::ffi::CStr) -> Result<i32> {
        let map_fd = zig_bindings::init_ipc_router(prog_path)?;
        *self.ebpf_map_fd.write() = Some(map_fd);
        Ok(map_fd)
    }

    /// Update a routing entry in the eBPF map: key = source peer ID, value = destination socket fd.
    pub fn update_route(&self, src_peer: PeerId, dst_sock: i32) -> Result<()> {
        let map_fd = *self
            .ebpf_map_fd
            .read()
            .as_ref()
            .ok_or_else(|| anyhow!("eBPF router not initialized"))?;
        zig_bindings::update_route(map_fd, src_peer, dst_sock as u32)
    }

    /// Remove a routing entry from the eBPF map.
    pub fn remove_route(&self, src_peer: PeerId) -> Result<()> {
        let map_fd = *self
            .ebpf_map_fd
            .read()
            .as_ref()
            .ok_or_else(|| anyhow!("eBPF router not initialized"))?;
        zig_bindings::remove_route(map_fd, src_peer)
    }

    pub fn register_peer(&self, peer_id: PeerId, kyber_pub: [u8; 1568], dilithium_pub: [u8; 1952]) {
        let mut k_cache = self.peer_kyber_pub_cache.write();
        let mut d_cache = self.peer_dilithium_pub_cache.write();
        k_cache.insert(peer_id, kyber_pub);
        d_cache.insert(peer_id, dilithium_pub);
    }

    pub fn encapsulate(&self, peer_id: PeerId, payload: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        let session = self
            .session_mgr
            .get(peer_id)
            .ok_or_else(|| anyhow!("No valid session key for peer {}", peer_id))?;

        let mut counters = self.counters.write();
        let counter = counters.entry(peer_id).or_insert_with(|| AtomicU64::new(0));

        SingleEncapsulator::encapsulate(&session.key, &session.nonce_base, counter, payload, aad)
    }

    pub fn decapsulate(&self, peer_id: PeerId, data: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        let session = self
            .session_mgr
            .get(peer_id)
            .ok_or_else(|| anyhow!("No session key for peer {}", peer_id))?;

        SingleEncapsulator::decapsulate(&session.key, data, aad)
            .ok_or_else(|| anyhow!("Decryption failed or data integrity compromised"))
    }

    pub fn initiate_handshake(&self, peer_id: PeerId) -> Result<Vec<u8>> {
        let peer_kyber = self
            .peer_kyber_pub_cache
            .read()
            .get(&peer_id)
            .cloned()
            .ok_or_else(|| anyhow!("No Kyber public key for peer {}", peer_id))?;

        let (session, handshake_msg) =
            handshake::client_handshake(&peer_kyber, &self.my_dilithium_priv)?;

        self.session_mgr.insert(peer_id, session);
        self.counters.write().insert(peer_id, AtomicU64::new(0));
        Ok(handshake_msg)
    }

    pub fn accept_handshake(&self, peer_id: PeerId, handshake_msg: &[u8]) -> Result<()> {
        let peer_dilithium = self
            .peer_dilithium_pub_cache
            .read()
            .get(&peer_id)
            .cloned()
            .ok_or_else(|| anyhow!("No Dilithium public key for peer {}", peer_id))?;

        let session =
            handshake::server_handshake(&self.my_kyber_priv, &peer_dilithium, handshake_msg)?;

        self.session_mgr.insert(peer_id, session);
        self.counters.write().insert(peer_id, AtomicU64::new(0));
        Ok(())
    }

    /// Kiểm tra xem đã có session với peer chưa (dùng cho test)
    pub fn has_session(&self, peer_id: PeerId) -> bool {
        self.session_mgr.get(peer_id).is_some()
    }
}
