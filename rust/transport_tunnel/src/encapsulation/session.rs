use super::handshake::SessionKey;
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub type PeerId = u64;

#[derive(Clone)]
pub struct SessionManager {
    cache: Arc<DashMap<PeerId, SessionKey>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
        }
    }

    pub fn get(&self, peer: PeerId) -> Result<SessionKey> {
        self.cache.get(&peer).map_or_else(
            || Err(anyhow!("No session key found for peer {}", peer)),
            |key| {
                if key.is_expired() {
                    tracing::debug!("Session key expired for peer {}", peer);
                    Err(anyhow!("Session key expired for peer {}", peer))
                } else {
                    Ok(key.value().clone())
                }
            },
        )
    }

    pub fn insert(&self, peer: PeerId, key: SessionKey) {
        self.cache.insert(peer, key);
    }

    pub fn cleanup(&self) {
        self.cache.retain(|_, key| !key.is_expired());
    }
}
