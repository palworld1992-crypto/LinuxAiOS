//! Quality Assessor - Đánh giá chất lượng câu trả lời

use crate::web_scraper::response_parser::ExtractedData;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering;
use tracing::debug;

pub struct QualityAssessor {
    min_length: AtomicU32,
    min_code_blocks: AtomicU32,
    default_threshold: AtomicU32,
}

impl QualityAssessor {
    pub fn new(min_length: u32, min_code: u32, threshold: f32) -> Self {
        Self {
            min_length: AtomicU32::new(min_length),
            min_code_blocks: AtomicU32::new(min_code),
            default_threshold: AtomicU32::new((threshold * 1000.0) as u32),
        }
    }

    pub fn assess(&self, data: &ExtractedData) -> f32 {
        let len_clamped = data.text.len().min(2000) as f32 / 2000.0;
        let num_code = data.code_snippets.len() as f32;
        let num_code_normalized = (num_code / 5.0).min(1.0);

        let score = 0.4 * len_clamped + 0.3 * num_code_normalized + 0.3 * data.trust_hint;

        debug!("Quality score: {:.2}", score);
        score
    }

    pub fn is_acceptable(&self, data: &ExtractedData) -> bool {
        let threshold_raw = self.default_threshold.load(Ordering::Relaxed) as f32 / 1000.0;
        self.assess(data) >= threshold_raw
    }

    pub fn set_threshold(&self, threshold: f32) {
        self.default_threshold
            .store((threshold * 1000.0) as u32, Ordering::Relaxed);
    }
}

impl Default for QualityAssessor {
    fn default() -> Self {
        Self::new(200, 0, 0.7)
    }
}
