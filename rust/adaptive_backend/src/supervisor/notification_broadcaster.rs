//! Notification broadcaster – đẩy thông báo qua WebSocket

use tokio::sync::broadcast;

pub struct NotificationBroadcaster {
    tx: broadcast::Sender<Vec<u8>>,
}

impl NotificationBroadcaster {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self { tx }
    }

    pub fn broadcast(&self, message: Vec<u8>) {
        let _ = self.tx.send(message);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Vec<u8>> {
        self.tx.subscribe()
    }
}
