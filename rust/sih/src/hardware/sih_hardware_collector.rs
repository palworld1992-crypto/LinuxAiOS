use super::collector::{HardwareCollector, HardwareMetrics};
use crate::errors::HardwareCollectorError;
use dashmap::DashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct SihHardwareCollector {
    collector: Option<HardwareCollector>,
    metrics_cache: Arc<DashMap<String, HardwareMetrics>>,
    running: Arc<AtomicBool>,
    collection_thread: Option<thread::JoinHandle<()>>,
}

impl SihHardwareCollector {
    pub fn new(buffer_capacity: usize) -> Self {
        Self {
            collector: Some(HardwareCollector::new(buffer_capacity)),
            metrics_cache: Arc::new(DashMap::new()),
            running: Arc::new(AtomicBool::new(false)),
            collection_thread: None,
        }
    }

    pub fn start(&mut self) {
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(true, Ordering::SeqCst);

        if let Some(ref mut collector) = self.collector {
            let _ = collector.start();
        }

        let running = self.running.clone();
        let metrics_cache = self.metrics_cache.clone();
        let collector = self.collector.take();

        let thread = thread::spawn(move || {
            let mut coll = collector;
            while running.load(Ordering::SeqCst) {
                if let Some(ref mut collector) = coll {
                    match collector.collect() {
                        Ok(metrics) => {
                            metrics_cache.insert("latest".to_string(), metrics);
                        }
                        Err(e) => {
                            tracing::warn!("Collection error: {}", e);
                        }
                    }
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        self.collection_thread = Some(thread);
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(ref mut collector) = self.collector {
            collector.stop();
        }
        if let Some(thread) = self.collection_thread.take() {
            let _ = thread.join();
        }
    }

    pub fn collect_once(&mut self) -> Result<HardwareMetrics, HardwareCollectorError> {
        if !self.running.load(Ordering::SeqCst) {
            return Err(HardwareCollectorError::NotRunning);
        }

        if let Some(ref mut collector) = self.collector {
            collector.collect().map_err(HardwareCollectorError::from)
        } else {
            Err(HardwareCollectorError::NotRunning)
        }
    }

    pub fn get_last_metrics(&self) -> Option<HardwareMetrics> {
        self.metrics_cache.get("latest").map(|r| r.value().clone())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn health_check(&mut self) -> Result<(), HardwareCollectorError> {
        if !self.is_running() {
            return Err(HardwareCollectorError::NotRunning);
        }

        let _ = self.collect_once()?;
        Ok(())
    }
}

impl Default for SihHardwareCollector {
    fn default() -> Self {
        Self::new(3600)
    }
}
