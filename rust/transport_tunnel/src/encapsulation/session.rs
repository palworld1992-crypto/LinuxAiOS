use super::handshake::SessionKey;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub type PeerId = u64;

#[derive(Clone)]
pub struct SessionManager {
    cache: Arc<RwLock<HashMap<PeerId, SessionKey>>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get(&self, peer: PeerId) -> Option<SessionKey> {
        let cache = self.cache.read();
        cache.get(&peer).and_then(|key| {
            if key.is_expired() {
                None
            } else {
                Some(key.clone())
            }
        })
    }

    pub fn insert(&self, peer: PeerId, key: SessionKey) {
        let mut cache = self.cache.write();
        cache.insert(peer, key);
    }

    pub fn cleanup(&self) {
        let mut cache = self.cache.write();
        cache.retain(|_, key| !key.is_expired());
    }
}
