//! API Profiler for Windows Module – tracks API call frequency and latency

use crossbeam::channel;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
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

impl Default for ApiStats {
    fn default() -> Self {
        Self::new()
    }
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
    stats: DashMap<String, ApiStats>,
    calls: DashMap<usize, ApiCall>,
    call_counter: AtomicU64,
    tx: DashMap<String, channel::Sender<ApiCall>>,
    enabled: AtomicBool,
    _call_window_seconds: u64,
}

impl ApiProfiler {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            stats: DashMap::new(),
            calls: DashMap::new(),
            call_counter: AtomicU64::new(0),
            tx: DashMap::new(),
            enabled: AtomicBool::new(true),
            _call_window_seconds: 60,
        }
    }

    pub fn with_channel(&self, api_name: &str, tx: channel::Sender<ApiCall>) {
        self.tx.insert(api_name.to_string(), tx);
    }

    pub fn record(&self, api_name: &str, latency_ms: f64, context: Option<String>) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let call = ApiCall {
            api_name: api_name.to_string(),
            timestamp: Self::current_timestamp(),
            latency_ms,
            context,
        };

        let idx = self.call_counter.fetch_add(1, Ordering::Relaxed) as usize;
        self.calls.insert(idx, call.clone());

        if let Some(ref sender) = self.tx.get(api_name) {
            let _ = sender.send(call);
        }

        self.update_stats(api_name, latency_ms);
    }

    fn update_stats(&self, api_name: &str, latency_ms: f64) {
        let stats = self
            .stats
            .entry(api_name.to_string())
            .or_insert_with(|| ApiStats::new());

        stats.total_calls.fetch_add(1, Ordering::Relaxed);

        let current_min = stats.min_latency_ms.load(Ordering::Relaxed);
        if (latency_ms as u64) < current_min {
            stats
                .min_latency_ms
                .store(latency_ms as u64, Ordering::Relaxed);
        }

        let current_max = stats.max_latency_ms.load(Ordering::Relaxed);
        if (latency_ms as u64) > current_max {
            stats
                .max_latency_ms
                .store(latency_ms as u64, Ordering::Relaxed);
        }
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64)
    }

    pub fn get_stats(&self, api_name: &str) -> Option<(u64, u64, u64, u64, u64)> {
        let stats = self.stats.get(api_name)?;

        Some((
            stats.total_calls.load(Ordering::Relaxed),
            stats.calls_per_second.load(Ordering::Relaxed),
            stats.avg_latency_ms.load(Ordering::Relaxed),
            stats.min_latency_ms.load(Ordering::Relaxed),
            stats.max_latency_ms.load(Ordering::Relaxed),
        ))
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn get_total_calls(&self) -> u64 {
        self.stats
            .iter()
            .map(|r| r.value().total_calls.load(Ordering::Relaxed))
            .sum()
    }
}
