use anyhow::Result;
use dashmap::DashMap;
use transport_tunnel::{PeerId, TransportTunnel};

pub struct TransportConnectionManager {
    tunnel: TransportTunnel,
    my_id: PeerId,
    peers: DashMap<PeerId, String>,
}

impl TransportConnectionManager {
    pub fn new(tunnel: TransportTunnel, my_id: PeerId) -> Self {
        Self {
            tunnel,
            my_id,
            peers: DashMap::new(),
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
