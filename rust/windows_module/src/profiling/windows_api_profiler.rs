//! API Profiler for Windows Module – tracks API call frequency and latency

use crossbeam::channel;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProfilerError {
    #[error("Channel error: {0}")]
    ChannelError(String),
}

#[derive(Clone, Debug)]
pub struct ApiCall {
    pub api_name: String,
    pub timestamp: u64,
    pub latency_ms: f64,
    pub context: Option<String>,
}

#[derive(Debug)]
pub struct ApiStats {
    pub total_calls: AtomicU64,
    pub calls_per_second: AtomicU64,
    pub avg_latency_ms: AtomicU64,
    pub min_latency_ms: AtomicU64,
    pub max_latency_ms: AtomicU64,
}

impl ApiStats {
    pub fn new() -> Self {
        Self {
            total_calls: AtomicU64::new(0),
            calls_per_second: AtomicU64::new(0),
            avg_latency_ms: AtomicU64::new(0),
            min_latency_ms: AtomicU64::new(u64::MAX),
            max_latency_ms: AtomicU64::new(0),
        }
    }
}

pub struct ApiProfiler {
    stats: RwLock<std::collections::HashMap<String, ApiStats>>,
    buffer: RwLock<VecDeque<ApiCall>>,
    tx: RwLock<Option<channel::Sender<ApiCall>>>,
    enabled: RwLock<bool>,
    _call_window_seconds: u64,
}

impl ApiProfiler {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            stats: RwLock::new(std::collections::HashMap::new()),
            buffer: RwLock::new(VecDeque::with_capacity(buffer_size)),
            tx: RwLock::new(None),
            enabled: RwLock::new(true),
            _call_window_seconds: 60,
        }
    }

    pub fn with_channel(&self, tx: channel::Sender<ApiCall>) {
        *self.tx.write() = Some(tx);
    }

    pub fn record(&self, api_name: &str, latency_ms: f64, context: Option<String>) {
        if !*self.enabled.read() {
            return;
        }

        let call = ApiCall {
            api_name: api_name.to_string(),
            timestamp: Self::current_timestamp(),
            latency_ms,
            context,
        };

        {
            let mut buffer = self.buffer.write();
            if buffer.len() >= buffer.capacity() {
                let _ = buffer.pop_front();
            }
            buffer.push_back(call.clone());
        }

        if let Some(ref tx) = *self.tx.read() {
            let _ = tx.send(call);
        }

        self.update_stats(api_name, latency_ms);
    }

    fn update_stats(&self, api_name: &str, latency_ms: f64) {
        let mut stats_map = self.stats.write();

        let stats = stats_map
            .entry(api_name.to_string())
            .or_insert_with(ApiStats::new);

        let total = stats.total_calls.fetch_add(1, Ordering::Relaxed) + 1;

        let avg_latency = stats.avg_latency_ms.load(Ordering::Relaxed);
        let new_avg = if avg_latency == 0 {
            latency_ms as u64
        } else {
            ((avg_latency * (total - 1)) as f64 + latency_ms) as u64 / total
        };
        stats.avg_latency_ms.store(new_avg, Ordering::Relaxed);

        let min_lat = stats.min_latency_ms.load(Ordering::Relaxed);
        if (latency_ms as u64) < min_lat {
            stats
                .min_latency_ms
                .store(latency_ms as u64, Ordering::Relaxed);
        }

        let max_lat = stats.max_latency_ms.load(Ordering::Relaxed);
        if (latency_ms as u64) > max_lat {
            stats
                .max_latency_ms
                .store(latency_ms as u64, Ordering::Relaxed);
        }
    }

    pub fn drain(&self) -> Vec<ApiCall> {
        let mut buffer = self.buffer.write();
        let mut calls = Vec::new();
        while let Some(call) = buffer.pop_front() {
            calls.push(call);
        }
        calls
    }

    pub fn get_stats(&self, api_name: &str) -> Option<ApiStatsSnapshot> {
        let stats_map = self.stats.read();
        stats_map.get(api_name).map(|s| ApiStatsSnapshot {
            total_calls: s.total_calls.load(Ordering::Relaxed),
            calls_per_second: s.calls_per_second.load(Ordering::Relaxed),
            avg_latency_ms: s.avg_latency_ms.load(Ordering::Relaxed),
            min_latency_ms: s.min_latency_ms.load(Ordering::Relaxed),
            max_latency_ms: s.max_latency_ms.load(Ordering::Relaxed),
        })
    }

    pub fn get_all_stats(&self) -> std::collections::HashMap<String, ApiStatsSnapshot> {
        let stats_map = self.stats.read();
        stats_map
            .iter()
            .map(|(k, s)| {
                (
                    k.clone(),
                    ApiStatsSnapshot {
                        total_calls: s.total_calls.load(Ordering::Relaxed),
                        calls_per_second: s.calls_per_second.load(Ordering::Relaxed),
                        avg_latency_ms: s.avg_latency_ms.load(Ordering::Relaxed),
                        min_latency_ms: s.min_latency_ms.load(Ordering::Relaxed),
                        max_latency_ms: s.max_latency_ms.load(Ordering::Relaxed),
                    },
                )
            })
            .collect()
    }

    pub fn get_hot_apis(&self, threshold: u64) -> Vec<String> {
        let stats_map = self.stats.read();
        stats_map
            .iter()
            .filter(|(_, s)| s.total_calls.load(Ordering::Relaxed) > threshold)
            .map(|(k, _)| k.clone())
            .collect()
    }

    pub fn enable(&self) {
        *self.enabled.write() = true;
    }

    pub fn disable(&self) {
        *self.enabled.write() = false;
    }

    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug)]
pub struct ApiStatsSnapshot {
    pub total_calls: u64,
    pub calls_per_second: u64,
    pub avg_latency_ms: u64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = ApiProfiler::new(1000);
        assert!(profiler.is_enabled());
    }

    #[test]
    fn test_record_api_call() {
        let profiler = ApiProfiler::new(1000);
        profiler.record("test_api", 1.5, None);
        let stats = profiler.get_stats("test_api");
        assert!(stats.is_some());
        assert_eq!(stats.expect("stats should be present for recorded API").total_calls, 1);
    }
}
