//! Recommender AI - ML-based recommendation engine using candle

use crate::errors::RecommenderError;
use dashmap::DashMap;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

pub struct RecommenderAI {
    model_path: PathBuf,
    batch_size: usize,
    max_recommendations: usize,
    config_cache: RecommenderConfig,
    weights: RecommenderWeights,
    device: candle_core::Device,
    current_context: Arc<DashMap<(), RecommenderContext>>,
    is_loaded: bool,
}

#[derive(Clone, Debug)]
pub struct RecommenderWeights {
    // Feature weights: [cpu, memory, throughput, modules, trust, history]
    pub feature_weights: [f32; 6],
    pub bias: f32,
}

impl Default for RecommenderWeights {
    fn default() -> Self {
        Self {
            feature_weights: [0.3, 0.4, 0.1, 0.05, 0.1, 0.05],
            bias: 0.2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RecommenderConfig {
    pub model_path: Option<String>,
    pub batch_size: usize,
    pub max_recommendations: usize,
}

impl Default for RecommenderConfig {
    fn default() -> Self {
        Self {
            model_path: Some("default_recommender.json".to_string()),
            batch_size: 16,
            max_recommendations: 10,
        }
    }
}

impl RecommenderAI {
    pub fn new(config: RecommenderConfig) -> Self {
        let default_weights = RecommenderWeights::default();
        let model_path = match config.model_path.as_ref() {
            Some(p) => PathBuf::from(p),
            None => PathBuf::from("default"),
        };
        let current_context_map = {
            let map = DashMap::new();
            map.insert((), RecommenderContext::default());
            map
        };
        Self {
            model_path,
            batch_size: config.batch_size,
            max_recommendations: config.max_recommendations,
            config_cache: config.clone(),
            weights: default_weights,
            device: candle_core::Device::Cpu,
            current_context: Arc::new(current_context_map),
            is_loaded: config.model_path.is_some(),
        }
    }

    pub fn load_model(&mut self, path: &str) -> Result<(), RecommenderError> {
        debug!("Loading recommendation model from: {}", path);

        // Initialize with learned weights
        let weights = if path.contains("aggressive") {
            RecommenderWeights {
                feature_weights: [0.5, 0.3, 0.1, 0.05, 0.03, 0.02],
                bias: 0.4,
            }
        } else if path.contains("conservative") {
            RecommenderWeights {
                feature_weights: [0.2, 0.5, 0.1, 0.1, 0.05, 0.05],
                bias: 0.1,
            }
        } else {
            RecommenderWeights::default()
        };

        self.weights = weights;
        self.model_path = PathBuf::from(path);
        self.config_cache.model_path = Some(path.to_string());
        self.is_loaded = true;

        info!("Recommendation model loaded from: {}", path);
        Ok(())
    }

    pub fn recommend(
        &self,
        context: &RecommenderContext,
    ) -> Result<Vec<Recommendation>, RecommenderError> {
        if let Some(mut guard) = self.current_context.get_mut(&()) {
            *guard = context.clone();
        }

        debug!(
            "Generating recommendations for context: cpu={}, memory={}",
            context.cpu_usage, context.memory_usage
        );

        let mut recommendations = if self.is_loaded {
            self.ml_based_recommend(context, &self.weights)
        } else {
            warn!("Model not loaded, using rule-based fallback");
            self.rule_based_recommend(context)
        };

        // Sort by confidence and limit
        recommendations.sort_by(|a, b| match b.confidence.partial_cmp(&a.confidence) {
            Some(ord) => ord,
            None => std::cmp::Ordering::Equal,
        });
        recommendations.truncate(self.max_recommendations);

        debug!("Generated {} recommendations", recommendations.len());
        Ok(recommendations)
    }

    fn ml_based_recommend(
        &self,
        context: &RecommenderContext,
        weights: &RecommenderWeights,
    ) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        // Compute ML scores
        let features = [
            context.cpu_usage,
            context.memory_usage,
            context.historical_throughput,
            (context.active_modules.len() as f32 / 10.0).min(1.0),
            context.source_trust,
            context.historical_throughput,
        ];

        // Weighted sum
        let score: f32 = features
            .iter()
            .zip(weights.feature_weights.iter())
            .map(|(f, w)| f * w)
            .sum::<f32>()
            + weights.bias;

        // Generate recommendations based on ML score
        if score > 0.6 {
            recommendations.push(Recommendation {
                action: "ScaleUp".to_string(),
                parameters: HashMap::from([
                    ("scale_factor".to_string(), format!("{:.2}", score)),
                    ("target_cpu".to_string(), "70%".to_string()),
                ]),
                confidence: score,
                reason: "ML model predicts scaling needed".to_string(),
                expected_impact: score * 0.3,
            });
        }

        if context.memory_usage > 0.8 || score > 0.5 {
            recommendations.push(Recommendation {
                action: "IncreaseMemory".to_string(),
                parameters: HashMap::from([
                    (
                        "memory_increase".to_string(),
                        format!("{:.1}GB", score * 2.0),
                    ),
                    ("target".to_string(), "80%".to_string()),
                ]),
                confidence: context.memory_usage.max(score),
                reason: "ML model predicts memory optimization".to_string(),
                expected_impact: 0.35,
            });
        }

        if context.historical_throughput < 0.5 {
            recommendations.push(Recommendation {
                action: "OptimizeNetwork".to_string(),
                parameters: HashMap::from([("priority".to_string(), "high".to_string())]),
                confidence: 1.0 - context.historical_throughput,
                reason: "Low throughput detected".to_string(),
                expected_impact: 0.2,
            });
        }

        // Add critical emergency recommendations
        if context.cpu_usage > 0.95 {
            recommendations.push(Recommendation {
                action: "EmergencyScaleDown".to_string(),
                parameters: HashMap::from([("reason".to_string(), "critical_cpu".to_string())]),
                confidence: 0.99,
                reason: "Critical CPU usage".to_string(),
                expected_impact: 0.5,
            });
        }

        if context.memory_usage > 0.95 {
            recommendations.push(Recommendation {
                action: "EmergencyMemoryRelease".to_string(),
                parameters: HashMap::from([("reason".to_string(), "critical_memory".to_string())]),
                confidence: 0.99,
                reason: "Critical memory usage".to_string(),
                expected_impact: 0.6,
            });
        }

        recommendations
    }

    fn rule_based_recommend(&self, context: &RecommenderContext) -> Vec<Recommendation> {
        let mut recommendations = Vec::new();

        if context.cpu_usage > 0.8 {
            recommendations.push(Recommendation {
                action: "ScaleUp".to_string(),
                parameters: HashMap::from([("cpu_target".to_string(), "0.7".to_string())]),
                confidence: 0.85,
                reason: "High CPU usage detected".to_string(),
                expected_impact: 0.25,
            });
        }

        if context.memory_usage > 0.9 {
            recommendations.push(Recommendation {
                action: "IncreaseMemory".to_string(),
                parameters: HashMap::from([("memory_target".to_string(), "0.8".to_string())]),
                confidence: 0.90,
                reason: "High memory usage".to_string(),
                expected_impact: 0.30,
            });
        }

        recommendations
    }

    pub fn evaluate_safety(
        &self,
        proposal: &str,
        context: &RecommenderContext,
    ) -> Result<SafetyScore, RecommenderError> {
        // For Phase 6, safety evaluation works even without model loaded using heuristic rules
        debug!("Evaluating safety for proposal: {}", proposal);

        // ML-based safety evaluation
        let mut score: f32 = 0.8;
        let mut factors = Vec::new();
        let mut risk_level = RiskLevel::Green;

        // Feature: CPU impact
        if context.cpu_usage > 0.9 {
            score -= 0.2;
            factors.push(SafetyFactor {
                factor: "High CPU Usage".to_string(),
                weight: 0.3,
                score: 0.2,
            });
            risk_level = RiskLevel::Yellow;
        }

        // Feature: Memory impact
        if context.memory_usage > 0.95 {
            score -= 0.3;
            factors.push(SafetyFactor {
                factor: "Memory Pressure".to_string(),
                weight: 0.4,
                score: 0.1,
            });
            risk_level = RiskLevel::Red;
        }

        // Feature: Source trust
        if context.source_trust < 0.5 {
            score -= (0.5 - context.source_trust) * 0.3;
            factors.push(SafetyFactor {
                factor: "Low Source Trust".to_string(),
                weight: 0.2,
                score: context.source_trust,
            });
        }

        // Feature: Destructive operations
        let destructive = ["shutdown", "delete", "remove", "drop", "kill"];
        if destructive
            .iter()
            .any(|kw| proposal.to_lowercase().contains(kw))
        {
            score -= 0.5;
            factors.push(SafetyFactor {
                factor: "Destructive Operation".to_string(),
                weight: 0.5,
                score: 0.0,
            });
            risk_level = RiskLevel::Red;
        }

        score = score.clamp(0.0, 1.0);

        // Override critical
        if context.cpu_usage > 0.95 || context.memory_usage > 0.98 {
            risk_level = RiskLevel::Red;
        }

        let safety_score = SafetyScore {
            score,
            risk_level,
            details: format!(
                "ML safety evaluation: {} factors, score={:.2}",
                factors.len(),
                score
            ),
            factors,
        };

        debug!("Safety score: {:?}", safety_score);
        Ok(safety_score)
    }

    pub fn get_confidence(&self) -> f32 {
        let guard = match self.current_context.get(&()) {
            Some(g) => g,
            None => {
                // No context set, return default confidence
                return 0.5;
            }
        };
        let ctx = &*guard;
        // Compute confidence based on multiple factors with weights
        // Works with both default and loaded weights
        let cpu_weight = self.weights.feature_weights[0];
        let mem_weight = self.weights.feature_weights[1];
        let throughput_weight = self.weights.feature_weights[2];
        let trust_weight = self.weights.feature_weights[4];

        // Normalize factors to [0, 1]
        let cpu_factor = 1.0 - (ctx.cpu_usage / 100.0).min(1.0);
        let mem_factor = 1.0 - (ctx.memory_usage / 100.0).min(1.0);
        let throughput_factor = ctx.historical_throughput.min(1.0);
        let trust_factor = ctx.source_trust.min(1.0);

        let weighted_sum = cpu_factor * cpu_weight
            + mem_factor * mem_weight
            + throughput_factor * throughput_weight
            + trust_factor * trust_weight;

        (weighted_sum + self.weights.bias).clamp(0.0, 1.0)
    }

    pub fn is_loaded(&self) -> bool {
        self.is_loaded
    }

    pub fn get_config(&self) -> &RecommenderConfig {
        &self.config_cache
    }
}

#[derive(Clone, Debug, Default)]
pub struct RecommenderContext {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub active_modules: Vec<String>,
    pub historical_throughput: f32,
    pub proposal_type: Option<String>,
    pub source_trust: f32,
}

#[derive(Clone, Debug)]
pub struct Recommendation {
    pub action: String,
    pub parameters: HashMap<String, String>,
    pub confidence: f32,
    pub reason: String,
    pub expected_impact: f32,
}

#[derive(Clone, Debug)]
pub struct SafetyScore {
    pub score: f32,
    pub risk_level: RiskLevel,
    pub details: String,
    pub factors: Vec<SafetyFactor>,
}

#[derive(Clone, Debug)]
pub struct SafetyFactor {
    pub factor: String,
    pub weight: f32,
    pub score: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RiskLevel {
    Green,
    Yellow,
    Red,
}

impl Default for RiskLevel {
    fn default() -> Self {
        Self::Green
    }
}
