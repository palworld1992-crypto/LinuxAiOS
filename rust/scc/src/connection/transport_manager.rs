use std::collections::HashMap;
use parking_lot::RwLock;
use transport_tunnel::{TransportTunnel, PeerId};
use anyhow::Result;

pub struct TransportConnectionManager {
    tunnel: TransportTunnel,
    my_id: PeerId,
    peers: RwLock<HashMap<PeerId, String>>, // peer id -> address (for handshake)
}

impl TransportConnectionManager {
    pub fn new(tunnel: TransportTunnel, my_id: PeerId) -> Self {
        Self {
            tunnel,
            my_id,
            peers: RwLock::new(HashMap::new()),
        }
    }

    pub fn register_peer(&self, peer_id: PeerId, kyber_pub: [u8; 1568], dilithium_pub: [u8; 1952]) {
        self.tunnel.register_peer(peer_id, kyber_pub, dilithium_pub);
    }

    pub fn initiate_handshake(&self, peer_id: PeerId) -> Result<Vec<u8>> {
        self.tunnel.initiate_handshake(peer_id)
    }

    pub fn accept_handshake(&self, peer_id: PeerId, msg: &[u8]) -> Result<()> {
        self.tunnel.accept_handshake(peer_id, msg)
    }

    pub fn send(&self, peer_id: PeerId, payload: &[u8]) -> Result<Vec<u8>> {
        self.tunnel.encapsulate(peer_id, payload, b"aios")
    }

    pub fn receive(&self, peer_id: PeerId, data: &[u8]) -> Result<Vec<u8>> {
        self.tunnel.decapsulate(peer_id, data, b"aios")
    }
}