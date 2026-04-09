use common::ring_buffer::RingBuffer;
use dashmap::DashMap;
use std::sync::OnceLock;
use tokio::sync::{mpsc, oneshot};

pub struct ConnectionManager {
    peers: DashMap<String, mpsc::UnboundedSender<Vec<u8>>>,
    _inbound: std::sync::Arc<RingBuffer<Vec<u8>>>,
    handlers: DashMap<String, mpsc::UnboundedSender<IncomingMessage>>,
    peer_id: OnceLock<String>,
}

#[derive(Debug)]
pub struct IncomingMessage {
    pub from: String,
    pub data: Vec<u8>,
    pub response_tx: Option<oneshot::Sender<Vec<u8>>>,
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            peers: DashMap::new(),
            _inbound: std::sync::Arc::new(RingBuffer::new(1024)),
            handlers: DashMap::new(),
            peer_id: OnceLock::from(String::new()),
        }
    }

    pub fn with_peer_id(mut self, peer_id: &str) -> Self {
        self.peer_id = OnceLock::from(peer_id.to_string());
        self
    }

    pub fn set_peer_id(&self, peer_id: &str) {
        let _ = self.peer_id.set(peer_id.to_string());
    }

    pub fn register_peer(&self, id: String, tx: mpsc::UnboundedSender<Vec<u8>>) {
        self.peers.insert(id, tx);
    }

    pub fn register_handler(&self, id: &str, tx: mpsc::UnboundedSender<IncomingMessage>) {
        self.handlers.insert(id.to_string(), tx);
    }

    pub fn send(&self, target: &str, data: Vec<u8>) -> Result<(), &'static str> {
        if let Some(tx) = self.peers.get(target) {
            tx.send(data).map_err(|_| "send failed")
        } else {
            Err("peer not found")
        }
    }

    pub fn send_with_reply(
        &self,
        target: &str,
        data: Vec<u8>,
    ) -> Result<oneshot::Receiver<Vec<u8>>, &'static str> {
        let (response_tx, response_rx) = oneshot::channel();

        let from_id = match self.peer_id.get() {
            Some(id) => id.clone(),
            None => {
                tracing::warn!("peer_id not set, using empty string");
                String::new()
            }
        };

        let msg = IncomingMessage {
            from: from_id,
            data,
            response_tx: Some(response_tx),
        };

        if let Some(handler) = self.handlers.get(target) {
            if handler.send(msg).is_err() {
                return Err("handler not available");
            }
        } else {
            return Err("target handler not found");
        }

        Ok(response_rx)
    }

    pub fn broadcast(&self, data: Vec<u8>) {
        for entry in self.peers.iter() {
            let _ = entry.value().send(data.clone());
        }
    }

    pub fn broadcast_to_handlers(&self, data: Vec<u8>) {
        let peer_id = match self.peer_id.get() {
            Some(id) => id.clone(),
            None => {
                tracing::warn!("peer_id not set, using empty string");
                String::new()
            }
        };
        for entry in self.handlers.iter() {
            let msg = IncomingMessage {
                from: peer_id.clone(),
                data: data.clone(),
                response_tx: None,
            };
            let _ = entry.value().send(msg);
        }
    }
}
