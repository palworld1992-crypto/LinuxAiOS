//! Trust Scoring Engine - ML-based trust scoring with learning

use crate::TrustScoringError;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct TrustScoringEngine {
    source_scores: Arc<DashMap<String, SourceTrustScore>>,
    model_enabled: bool,
    // ML model weights: [sig_weight, hist_weight, pop_weight, content_weight, bias]
    model_weights: Arc<DashMap<usize, f32>>,
    // Learning rate for feedback
    learning_rate: f32,
    // Feature statistics for normalization
    feature_stats: Arc<DashMap<(), FeatureStats>>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FeatureStats {
    pub sig_mean: f32,
    pub hist_mean: f32,
    pub pop_mean: f32,
    pub sig_std: f32,
    pub hist_std: f32,
    pub pop_std: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceTrustScore {
    pub source_id: String,
    pub trust_score: f32,
    pub historical_accuracy: f32,
    pub signature_valid: bool,
    pub popularity: f32,
    pub last_updated: i64,
    // Additional features
    pub content_quality: f32,
    pub response_time_ms: f32,
    pub uptime_percentage: f32,
    pub failure_count: u32,
    pub success_count: u32,
}

impl Default for TrustScoringEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TrustScoringEngine {
    pub fn new() -> Self {
        let weights_map = DashMap::new();
        weights_map.insert(0, 0.30);
        weights_map.insert(1, 0.35);
        weights_map.insert(2, 0.20);
        weights_map.insert(3, 0.10);
        weights_map.insert(4, 0.05);

        Self {
            source_scores: Arc::new(DashMap::new()),
            model_enabled: false,
            model_weights: Arc::new(weights_map),
            learning_rate: 0.05,
            feature_stats: {
                let map = DashMap::new();
                map.insert((), FeatureStats::default());
                Arc::new(map)
            },
        }
    }

    pub fn enable_model(&mut self, model_path: &str) -> Result<(), TrustScoringError> {
        info!("Trust scoring ML model enabled from: {}", model_path);

        // Initialize model with different weights based on path
        let weights = if model_path.contains("conservative") {
            [0.40, 0.30, 0.15, 0.10, 0.05] // More weight on signature
        } else if model_path.contains("aggressive") {
            [0.20, 0.40, 0.25, 0.10, 0.05] // More weight on historical
        } else {
            [0.30, 0.35, 0.20, 0.10, 0.05] // Balanced
        };

        for (idx, &val) in weights.iter().enumerate() {
            self.model_weights.insert(idx, val);
        }
        self.model_enabled = true;

        info!("ML model weights initialized: {:?}", weights);
        Ok(())
    }

    pub fn calculate_score(&self, context: &TrustContext) -> Result<f32, TrustScoringError> {
        // Collect weights
        let w0 = *self
            .model_weights
            .get(&0)
            .ok_or_else(|| TrustScoringError::Internal("weight 0 missing".to_string()))?;
        let w1 = *self
            .model_weights
            .get(&1)
            .ok_or_else(|| TrustScoringError::Internal("weight 1 missing".to_string()))?;
        let w2 = *self
            .model_weights
            .get(&2)
            .ok_or_else(|| TrustScoringError::Internal("weight 2 missing".to_string()))?;
        let w3 = *self
            .model_weights
            .get(&3)
            .ok_or_else(|| TrustScoringError::Internal("weight 3 missing".to_string()))?;
        let w4 = *self
            .model_weights
            .get(&4)
            .ok_or_else(|| TrustScoringError::Internal("weight 4 missing".to_string()))?;

        let stats_guard = self
            .feature_stats
            .get(&())
            .ok_or_else(|| TrustScoringError::Internal("stats missing".to_string()))?;
        let stats = &*stats_guard;

        // Extract features
        let sig_feature = if context.signature_valid { 1.0 } else { 0.0 };
        let hist_feature = context.historical_accuracy;
        let pop_feature = context.popularity;

        // Normalize features using stats
        let sig_normalized = (sig_feature - stats.sig_mean) / (stats.sig_std + 0.001);
        let hist_normalized = (hist_feature - stats.hist_mean) / (stats.hist_std + 0.001);
        let pop_normalized = (pop_feature - stats.pop_mean) / (stats.pop_std + 0.001);

        // Content hash quality (derived from hash characteristics)
        let content_feature = self.derive_content_quality(&context.content_hash);

        // Weighted sum with normalized features
        let mut score = w0 * sig_normalized
            + w1 * hist_normalized
            + w2 * pop_normalized
            + w3 * content_feature
            + w4;

        // Apply sigmoid activation to map to [0, 1]
        score = 1.0 / (1.0 + (-score * 4.0).exp());

        // Clamp to valid range
        let score = score.clamp(0.0, 1.0);

        debug!(
            "Trust score calculated: {:.3} for source: {}",
            score, context.source_id
        );
        Ok(score)
    }

    fn derive_content_quality(&self, content_hash: &str) -> f32 {
        // Compute based on hash entropy - higher entropy = better quality
        if content_hash.is_empty() {
            return 0.5;
        }

        let bytes: Vec<u8> = content_hash.bytes().take(32).collect();
        if bytes.is_empty() {
            return 0.5;
        }

        let mean: f32 = bytes.iter().map(|&b| b as f32).sum::<f32>() / bytes.len() as f32;
        let variance: f32 = bytes
            .iter()
            .map(|&b| (b as f32 - mean).powi(2))
            .sum::<f32>()
            / bytes.len() as f32;

        // Higher variance = more entropy = higher quality score
        (variance / 6500.0).min(1.0)
    }

    pub fn update_source_score(
        &self,
        source_id: &str,
        score: f32,
    ) -> Result<(), TrustScoringError> {
        let now = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            Ok(d) => d.as_secs() as i64,
            Err(e) => {
                warn!("System clock before UNIX_EPOCH: {}", e);
                0
            }
        };

        let mut entry = self
            .source_scores
            .entry(source_id.to_string())
            .or_insert_with(|| SourceTrustScore {
                source_id: source_id.to_string(),
                trust_score: 0.5,
                historical_accuracy: 0.5,
                signature_valid: false,
                popularity: 0.5,
                last_updated: now,
                content_quality: 0.5,
                response_time_ms: 100.0,
                uptime_percentage: 0.95,
                failure_count: 0,
                success_count: 0,
            });

        // Update with exponential moving average
        let alpha = 0.2;
        entry.trust_score = alpha * score + (1.0 - alpha) * entry.trust_score;
        entry.last_updated = now;

        // Update success/failure counts
        if score > 0.5 {
            entry.success_count += 1;
            if entry.success_count + entry.failure_count > 0 {
                entry.historical_accuracy =
                    entry.success_count as f32 / (entry.success_count + entry.failure_count) as f32;
            }
        } else {
            entry.failure_count += 1;
        }

        debug!(
            "Updated source score for {}: {:.3}",
            source_id, entry.trust_score
        );
        Ok(())
    }

    /// Learn from feedback - adjust model weights based on outcome
    pub fn learn_from_feedback(
        &self,
        context: &TrustContext,
        predicted_score: f32,
        actual_outcome: f32, // 0.0 = failed, 1.0 = success
    ) -> Result<(), TrustScoringError> {
        if !self.model_enabled {
            return Ok(());
        }

        let error = actual_outcome - predicted_score;
        let lr = self.learning_rate;

        // Update weights based on error - use get_mut for each index
        if let Some(mut w0) = self.model_weights.get_mut(&0) {
            *w0 += lr * error * if context.signature_valid { 1.0 } else { -0.5 };
        }
        if let Some(mut w1) = self.model_weights.get_mut(&1) {
            *w1 += lr * error * context.historical_accuracy;
        }
        if let Some(mut w2) = self.model_weights.get_mut(&2) {
            *w2 += lr * error * context.popularity;
        }
        if let Some(mut w3) = self.model_weights.get_mut(&3) {
            *w3 += lr * error;
        }

        // Normalize weights to sum to ~1.0
        let mut sum: f32 = 0.0;
        for entry in self.model_weights.iter() {
            let idx = *entry.key();
            if idx < 4 {
                sum += entry.value();
            }
        }
        if sum > 0.0 {
            for idx in 0..4 {
                if let Some(mut w) = self.model_weights.get_mut(&(idx as usize)) {
                    *w /= sum;
                }
            }
        }

        debug!(
            "Learned from feedback: error={:.3}, new_weights={:?}",
            error,
            {
                let mut vals = Vec::new();
                for i in 0..4 {
                    if let Some(w) = self.model_weights.get(&i) {
                        vals.push(*w);
                    } else {
                        vals.push(0.0);
                    }
                }
                vals
            }
        );
        Ok(())
    }

    /// Update feature statistics for normalization
    pub fn update_feature_stats(&self, context: &TrustContext) {
        if let Some(mut stats) = self.feature_stats.get_mut(&()) {
            // Exponential moving average of feature means
            let alpha = 0.1;
            stats.sig_mean = alpha * if context.signature_valid { 1.0 } else { 0.0 }
                + (1.0 - alpha) * stats.sig_mean;
            stats.hist_mean = alpha * context.historical_accuracy + (1.0 - alpha) * stats.hist_mean;
            stats.pop_mean = alpha * context.popularity + (1.0 - alpha) * stats.pop_mean;

            // Simple std estimation
            stats.sig_std = (stats.sig_std * 0.95 + 0.05).max(0.1);
            stats.hist_std = (stats.hist_std * 0.95 + 0.05).max(0.1);
            stats.pop_std = (stats.pop_std * 0.95 + 0.05).max(0.1);
        }
    }

    pub fn get_source_score(&self, source_id: &str) -> Option<f32> {
        self.source_scores.get(source_id).map(|r| r.trust_score)
    }

    pub fn get_source_details(&self, source_id: &str) -> Option<SourceTrustScore> {
        self.source_scores.get(source_id).map(|r| r.clone())
    }

    pub fn get_all_scores(&self) -> Vec<SourceTrustScore> {
        self.source_scores.iter().map(|r| r.clone()).collect()
    }

    pub fn is_model_enabled(&self) -> bool {
        self.model_enabled
    }
}

#[derive(Clone, Debug)]
pub struct TrustContext {
    pub source_id: String,
    pub signature_valid: bool,
    pub historical_accuracy: f32,
    pub popularity: f32,
    pub content_hash: String,
}
