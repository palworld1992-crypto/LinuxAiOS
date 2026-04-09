//! WebSocket Handler - Real-time notifications with lock-free ring buffer

use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::broadcast;
use tokio_tungstenite::tungstenite::Message;

pub struct WebSocketManager {
    clients: DashMap<String, broadcast::Sender<String>>,
    message_store: Arc<DashMap<u64, String>>,
    message_head: AtomicU64,
    ring_capacity: usize,
}

impl WebSocketManager {
    pub fn new(ring_capacity: usize) -> Self {
        Self {
            clients: DashMap::new(),
            message_store: Arc::new(DashMap::new()),
            message_head: AtomicU64::new(0),
            ring_capacity,
        }
    }

    pub fn add_client(&self, client_id: String) -> broadcast::Receiver<String> {
        let (tx, rx) = broadcast::channel(256);
        self.clients.insert(client_id, tx);
        rx
    }

    pub fn remove_client(&self, client_id: &str) {
        self.clients.remove(client_id);
    }

    pub fn broadcast_message(&self, message: &str) {
        let seq = self.message_head.fetch_add(1, Ordering::Relaxed);
        self.message_store.insert(seq, message.to_string());

        // Maintain bounded size
        while self.message_store.len() > self.ring_capacity {
            if let Some(min_entry) = self.message_store.iter().min_by_key(|e| *e.key()) {
                self.message_store.remove(min_entry.key());
            } else {
                break;
            }
        }

        for entry in self.clients.iter() {
            let _ = entry.value().send(message.to_string());
        }
    }

    pub fn get_recent_messages(&self) -> Vec<String> {
        let head = self.message_head.load(Ordering::Relaxed);
        let cutoff = head.saturating_sub(self.ring_capacity as u64);
        let mut entries: Vec<_> = self
            .message_store
            .iter()
            .filter(|e| *e.key() >= cutoff)
            .map(|e| (e.key().clone(), e.value().clone()))
            .collect();
        entries.sort_by_key(|(seq, _)| *seq);
        entries.into_iter().map(|(_, msg)| msg).collect()
    }

    pub fn get_client_count(&self) -> usize {
        self.clients.len()
    }

    pub async fn handle_connection(
        &self,
        client_id: String,
        stream: TcpStream,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let ws_stream = tokio_tungstenite::accept_async(stream).await?;
        let (mut write, mut read) = ws_stream.split();

        let mut rx = self.add_client(client_id.clone());

        loop {
            tokio::select! {
                Ok(msg) = rx.recv() => {
                    if write.send(Message::Text(msg)).await.is_err() {
                        break;
                    }
                }
                Some(Ok(msg)) = read.next() => {
                    if msg.is_close() {
                        break;
                    }
                }
                else => break,
            }
        }

        self.remove_client(&client_id);
        Ok(())
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new(4096)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_manager_creation() -> anyhow::Result<()> {
        let manager = WebSocketManager::default();
        assert_eq!(manager.get_client_count(), 0);
        assert_eq!(manager.get_recent_messages().len(), 0);
        Ok(())
    }

    #[test]
    fn test_add_remove_client() -> anyhow::Result<()> {
        let manager = WebSocketManager::default();

        let _rx = manager.add_client("client1".to_string());
        assert_eq!(manager.get_client_count(), 1);

        manager.remove_client("client1");
        assert_eq!(manager.get_client_count(), 0);

        Ok(())
    }

    #[test]
    fn test_broadcast_message() -> anyhow::Result<()> {
        let manager = WebSocketManager::default();

        manager.broadcast_message("test message");
        let messages = manager.get_recent_messages();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0], "test message");

        Ok(())
    }

    #[test]
    fn test_ring_buffer_capacity() -> anyhow::Result<()> {
        let manager = WebSocketManager::new(3);

        manager.broadcast_message("msg1");
        manager.broadcast_message("msg2");
        manager.broadcast_message("msg3");
        manager.broadcast_message("msg4");

        let messages = manager.get_recent_messages();
        assert_eq!(messages.len(), 3);
        assert_eq!(messages[0], "msg2");
        assert_eq!(messages[2], "msg4");

        Ok(())
    }
}
