//! Scheduler - Lên lịch thu thập dữ liệu

use crate::web_scraper::WebScraper;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::debug;

pub struct Scheduler {
    active: Arc<AtomicBool>,
    min_potential: f32,
    check_interval_secs: u64,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            active: Arc::new(AtomicBool::new(false)),
            min_potential: 0.6,
            check_interval_secs: 10,
        }
    }

    pub fn start(&self, scraper: WebScraper) {
        self.active.store(true, Ordering::SeqCst);
        
        let active = self.active.clone();
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                if !active.load(Ordering::SeqCst) {
                    break;
                }
                
                let potential = Self::get_system_potential();
                
                if potential >= 0.6 && scraper.is_running() {
                    debug!("System potential {:.2} - running collection", potential);
                } else if potential < 0.6 {
                    debug!("System potential too low: {:.2}", potential);
                }
            }
        });
    }

    pub fn stop(&self) {
        self.active.store(false, Ordering::SeqCst);
    }

    fn get_system_potential() -> f32 {
        0.8
    }

    pub fn set_min_potential(&mut self, potential: f32) {
        self.min_potential = potential;
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}