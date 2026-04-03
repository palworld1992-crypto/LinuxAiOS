use crate::token::IntentToken;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

const URGENCY_THRESHOLD_LOW: u8 = 20;
const URGENCY_THRESHOLD_DROP: u8 = 10;
const SIGNAL_TYPE_NOISE: u8 = 255;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct RouteEntry {
    pub src: [u8; 32],
    pub dst: [u8; 32],
    pub ring_buffer_fd: i32,
    pub weight: u8,
    pub urgency: u8,
}

#[derive(Error, Debug)]
pub enum BridgeError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Route not found")]
    RouteNotFound,
    #[error("FFI error: {0}")]
    FfiError(String),
    #[error("Signal dropped: low priority")]
    SignalDropped,
}

#[derive(Debug, Clone, Default)]
pub struct BridgeStats {
    pub total_signals: u64,
    pub dropped_low_urgency: u64,
    pub dropped_noise: u64,
    pub forwarded: u64,
}

pub struct TransportBridge {
    routes: DashMap<String, RouteEntry>,
    total_signals: AtomicU64,
    dropped_low_urgency: AtomicU64,
    dropped_noise: AtomicU64,
    forwarded: AtomicU64,
}

impl TransportBridge {
    pub fn new() -> Self {
        Self {
            routes: DashMap::new(),
            total_signals: AtomicU64::new(0),
            dropped_low_urgency: AtomicU64::new(0),
            dropped_noise: AtomicU64::new(0),
            forwarded: AtomicU64::new(0),
        }
    }

    pub fn add_route(&self, key: String, entry: RouteEntry) {
        self.routes.insert(key, entry);
    }

    pub fn find_route(&self, src: &[u8; 32], dst: &[u8; 32]) -> Option<RouteEntry> {
        let key = format!("{:02x}{:02x}:{:02x}{:02x}", src[0], src[1], dst[0], dst[1]);
        self.routes.get(&key).map(|r| *r.value())
    }

    pub fn remove_route(&self, key: &str) {
        self.routes.remove(key);
    }

    pub fn send_message(&self, token: &IntentToken, payload: &[u8]) -> Result<(), BridgeError> {
        self.total_signals.fetch_add(1, Ordering::Relaxed);

        if token.signal_type == SIGNAL_TYPE_NOISE {
            self.dropped_noise.fetch_add(1, Ordering::Relaxed);
            return Err(BridgeError::SignalDropped);
        }

        if token.urgency <= URGENCY_THRESHOLD_DROP {
            self.dropped_low_urgency.fetch_add(1, Ordering::Relaxed);
            return Err(BridgeError::SignalDropped);
        }

        let src = token.module_id;
        let dst = [0u8; 32];

        if let Some(route) = self.find_route(&src, &dst) {
            if token.urgency >= 200 && route.urgency >= 200 {
                self.forwarded.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }

            if token.urgency < URGENCY_THRESHOLD_LOW {
                self.dropped_low_urgency.fetch_add(1, Ordering::Relaxed);
                return Err(BridgeError::SignalDropped);
            }

            if route.weight > 128 && token.signal_type == 1 {
                if payload.len() <= 64 {
                    self.dropped_low_urgency.fetch_add(1, Ordering::Relaxed);
                    return Err(BridgeError::SignalDropped);
                }
                self.forwarded.fetch_add(1, Ordering::Relaxed);
                return Ok(());
            }
        }

        self.forwarded.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.routes.len()
    }

    pub fn get_stats(&self) -> BridgeStats {
        BridgeStats {
            total_signals: self.total_signals.load(Ordering::Acquire),
            dropped_low_urgency: self.dropped_low_urgency.load(Ordering::Acquire),
            dropped_noise: self.dropped_noise.load(Ordering::Acquire),
            forwarded: self.forwarded.load(Ordering::Acquire),
        }
    }
}

impl Default for TransportBridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge() {
        let bridge = TransportBridge::new();
        let entry = RouteEntry {
            src: [0u8; 32],
            dst: [1u8; 32],
            ring_buffer_fd: -1,
            weight: 128,
            urgency: 200,
        };
        bridge.add_route("0001:0002".to_string(), entry);
        assert_eq!(bridge.len(), 1);
    }

    #[test]
    fn test_find_route() {
        let bridge = TransportBridge::new();

        let entry = RouteEntry {
            src: [
                0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
                0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66,
                0x77, 0x88, 0x99, 0xAA,
            ],
            dst: [
                0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE,
                0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC,
                0xDD, 0xEE, 0xFF, 0x00,
            ],
            ring_buffer_fd: 5,
            weight: 200,
            urgency: 250,
        };

        let src: [u8; 32] = entry.src;
        let dst: [u8; 32] = entry.dst;

        bridge.add_route("aabb:1122".to_string(), entry);

        let found = bridge.find_route(&src, &dst);
        assert!(found.is_some());
    }

    #[test]
    fn test_drop_low_urgency() {
        let bridge = TransportBridge::new();

        let token = IntentToken {
            module_id: [0u8; 32],
            signal_type: 1,
            urgency: 5,
            timestamp: 0,
            signature: vec![],
        };

        let result = bridge.send_message(&token, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_drop_noise_signal() {
        let bridge = TransportBridge::new();

        let token = IntentToken {
            module_id: [0u8; 32],
            signal_type: 255,
            urgency: 100,
            timestamp: 0,
            signature: vec![],
        };

        let result = bridge.send_message(&token, &[]);
        assert!(result.is_err());
    }
}
