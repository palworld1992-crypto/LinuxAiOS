mod handshake;
mod parallel;
mod session;
mod single;

pub use handshake::{client_handshake, server_handshake, SessionKey};
pub use parallel::ParallelEncapsulator;
pub use session::{PeerId, SessionManager};
pub use single::SingleEncapsulator;

use anyhow::{anyhow, Result};
use dashmap::DashMap;
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
        // SAFETY: Calling FFI function zig_init_ipc_router with valid CStr pointer
        let fd = std::panic::catch_unwind(|| unsafe { zig_init_ipc_router(prog_path.as_ptr()) })
            .map_err(|e| anyhow::anyhow!("FFI panic in zig_init_ipc_router: {:?}", e))?;
        if fd < 0 {
            anyhow::bail!("Failed to init IPC router: {}", fd);
        }
        Ok(fd)
    }

    pub fn update_route(map_fd: i32, src_peer: u64, dst_sock: u32) -> anyhow::Result<()> {
        // SAFETY: Calling FFI function zig_update_route with valid parameters
        let ret =
            std::panic::catch_unwind(|| unsafe { zig_update_route(map_fd, src_peer, dst_sock) })
                .map_err(|e| anyhow::anyhow!("FFI panic in zig_update_route: {:?}", e))?;
        if ret < 0 {
            anyhow::bail!("Failed to update route: {}", ret);
        }
        Ok(())
    }

    pub fn remove_route(map_fd: i32, src_peer: u64) -> anyhow::Result<()> {
        // SAFETY: Calling FFI function zig_remove_route with valid parameters
        let ret = std::panic::catch_unwind(|| unsafe { zig_remove_route(map_fd, src_peer) })
            .map_err(|e| anyhow::anyhow!("FFI panic in zig_remove_route: {:?}", e))?;
        if ret < 0 {
            anyhow::bail!("Failed to remove route: {}", ret);
        }
        Ok(())
    }

    pub fn set_sockmap_prog(map_fd: i32, prog_fd: i32) -> anyhow::Result<()> {
        // SAFETY: Calling FFI function zig_set_sockmap_prog with valid file descriptors
        let ret = std::panic::catch_unwind(|| unsafe { zig_set_sockmap_prog(map_fd, prog_fd) })
            .map_err(|e| anyhow::anyhow!("FFI panic in zig_set_sockmap_prog: {:?}", e))?;
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
    peer_kyber_pub_cache: Arc<DashMap<PeerId, [u8; 1568]>>,
    peer_dilithium_pub_cache: Arc<DashMap<PeerId, [u8; 1952]>>,
    counters: Arc<DashMap<PeerId, AtomicU64>>,
    ebpf_map_fd: Arc<DashMap<u64, i32>>,
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
            peer_kyber_pub_cache: Arc::new(DashMap::new()),
            peer_dilithium_pub_cache: Arc::new(DashMap::new()),
            counters: Arc::new(DashMap::new()),
            ebpf_map_fd: Arc::new(DashMap::new()),
        }
    }

    /// Attach eBPF program to a pre‑existing map (legacy compatibility)
    pub fn attach_ebpf(&self, prog_fd: i32, map_fd: i32) -> Result<()> {
        zig_bindings::set_sockmap_prog(map_fd, prog_fd)
    }

    /// Initialize the eBPF IPC router: load sockmap program, create map, attach program.
    pub fn init_ebpf_router(&self, prog_path: &std::ffi::CStr) -> Result<i32> {
        let map_fd = zig_bindings::init_ipc_router(prog_path)?;
        self.ebpf_map_fd.insert(0, map_fd);
        Ok(map_fd)
    }

    /// Update a routing entry in the eBPF map: key = source peer ID, value = destination socket fd.
    pub fn update_route(&self, src_peer: PeerId, dst_sock: i32) -> Result<()> {
        let map_fd = self
            .ebpf_map_fd
            .get(&0)
            .map(|r| *r.value())
            .ok_or_else(|| anyhow!("eBPF router not initialized"))?;
        zig_bindings::update_route(map_fd, src_peer, dst_sock as u32)
    }

    /// Remove a routing entry from the eBPF map.
    pub fn remove_route(&self, src_peer: PeerId) -> Result<()> {
        let map_fd = self
            .ebpf_map_fd
            .get(&0)
            .map(|r| *r.value())
            .ok_or_else(|| anyhow!("eBPF router not initialized"))?;
        zig_bindings::remove_route(map_fd, src_peer)
    }

    pub fn register_peer(&self, peer_id: PeerId, kyber_pub: [u8; 1568], dilithium_pub: [u8; 1952]) {
        self.peer_kyber_pub_cache.insert(peer_id, kyber_pub);
        self.peer_dilithium_pub_cache.insert(peer_id, dilithium_pub);
    }

    pub fn encapsulate(&self, peer_id: PeerId, payload: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        let session = self.session_mgr.get(peer_id)?;

        let counter = self
            .counters
            .entry(peer_id)
            .or_insert_with(|| AtomicU64::new(0));

        SingleEncapsulator::encapsulate(
            &session.key,
            &session.nonce_base,
            counter.value(),
            payload,
            aad,
        )
    }

    pub fn decapsulate(&self, peer_id: PeerId, data: &[u8], aad: &[u8]) -> Result<Vec<u8>> {
        let session = self.session_mgr.get(peer_id)?;
        SingleEncapsulator::decapsulate(&session.key, data, aad)
    }

    pub fn initiate_handshake(&self, peer_id: PeerId) -> Result<Vec<u8>> {
        let peer_kyber = self
            .peer_kyber_pub_cache
            .get(&peer_id)
            .map(|r| *r.value())
            .ok_or_else(|| anyhow!("No Kyber public key for peer {}", peer_id))?;

        let (session, handshake_msg) =
            handshake::client_handshake(&peer_kyber, &self.my_dilithium_priv)?;

        self.session_mgr.insert(peer_id, session);
        self.counters.insert(peer_id, AtomicU64::new(0));
        Ok(handshake_msg)
    }

    pub fn accept_handshake(&self, peer_id: PeerId, handshake_msg: &[u8]) -> Result<()> {
        let peer_dilithium = self
            .peer_dilithium_pub_cache
            .get(&peer_id)
            .map(|r| *r.value())
            .ok_or_else(|| anyhow!("No Dilithium public key for peer {}", peer_id))?;

        let session =
            handshake::server_handshake(&self.my_kyber_priv, &peer_dilithium, handshake_msg)?;

        self.session_mgr.insert(peer_id, session);
        self.counters.insert(peer_id, AtomicU64::new(0));
        Ok(())
    }

    /// Kiểm tra xem đã có session với peer chưa (dùng cho test)
    pub fn has_session(&self, peer_id: PeerId) -> bool {
        self.session_mgr.get(peer_id).is_ok()
    }
}
