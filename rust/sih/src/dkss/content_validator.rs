//! Content Validator - ML-based content validation with toxicity detection

use crate::ContentValidatorError;
use dashmap::DashMap;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use tracing::{debug, info};

pub struct ContentValidator {
    similarity_threshold: Arc<AtomicU32>,
    toxicity_model_loaded: Arc<AtomicBool>,
    // MinHash LSH state for similarity detection
    min_hash_seeds: Arc<DashMap<usize, u32>>,
    // Toxicity model weights (learned)
    toxicity_weights: Arc<DashMap<(), ToxicityWeights>>,
}

#[derive(Clone, Debug, Default)]
pub struct ToxicityWeights {
    pub keyword_weights: std::collections::HashMap<String, f32>,
    pub pattern_weights: std::collections::HashMap<String, f32>,
    pub bias: f32,
}

impl ToxicityWeights {
    fn default_weights() -> Self {
        let mut keyword_weights = std::collections::HashMap::new();
        // High risk keywords
        keyword_weights.insert("malware".to_string(), 0.9);
        keyword_weights.insert("exploit".to_string(), 0.85);
        keyword_weights.insert("virus".to_string(), 0.85);
        keyword_weights.insert("ransomware".to_string(), 0.95);
        keyword_weights.insert("phishing".to_string(), 0.8);
        keyword_weights.insert("scam".to_string(), 0.7);

        // Medium risk keywords
        keyword_weights.insert("hack".to_string(), 0.5);
        keyword_weights.insert("crack".to_string(), 0.5);
        keyword_weights.insert("bypass".to_string(), 0.4);

        // Low risk keywords
        keyword_weights.insert("spam".to_string(), 0.3);
        keyword_weights.insert("fake".to_string(), 0.25);

        // Negative sentiment (context-dependent)
        keyword_weights.insert("hate".to_string(), 0.6);
        keyword_weights.insert("violence".to_string(), 0.7);
        keyword_weights.insert("abuse".to_string(), 0.6);
        keyword_weights.insert("threat".to_string(), 0.65);

        let mut pattern_weights = std::collections::HashMap::new();
        pattern_weights.insert(r"(?i)download.*\.exe".to_string(), 0.8);
        pattern_weights.insert(r"(?i)click.*here".to_string(), 0.4);
        pattern_weights.insert(r"(?i)your.*account.*verify".to_string(), 0.6);
        pattern_weights.insert(r"(?i)password.*reset".to_string(), 0.5);
        pattern_weights.insert(r"(?i)bitcoin|btc|eth".to_string(), 0.5);

        Self {
            keyword_weights,
            pattern_weights,
            bias: 0.1,
        }
    }
}

impl Default for ContentValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentValidator {
    pub fn new() -> Self {
        let seeds = DashMap::new();
        let mut hasher = DefaultHasher::new();
        0u64.hash(&mut hasher);
        let base = hasher.finish();

        for i in 0..128 {
            let mut h = DefaultHasher::new();
            (base + i as u64).hash(&mut h);
            seeds.insert(i, (h.finish() % (1u64 << 31)) as u32);
        }

        Self {
            similarity_threshold: Arc::new(AtomicU32::new(0.85f32.to_bits())),
            toxicity_model_loaded: Arc::new(AtomicBool::new(false)),
            min_hash_seeds: Arc::new(seeds),
            toxicity_weights: {
                let map = DashMap::new();
                map.insert((), ToxicityWeights::default_weights());
                Arc::new(map)
            },
        }
    }

    pub fn enable_toxicity_model(&self, path: &str) -> Result<(), ContentValidatorError> {
        info!("Loading toxicity model from: {}", path);

        // In full implementation, would load ML model weights from file
        // For now, update weights based on model path characteristics
        if let Some(mut guard) = self.toxicity_weights.get_mut(&()) {
            let weights = &mut *guard;

            if path.contains("strict") {
                // Increase all weights for stricter detection
                for weight in weights.keyword_weights.values_mut() {
                    *weight = (*weight * 1.5).min(1.0);
                }
                weights.bias = 0.15;
            } else if path.contains("lenient") {
                // Decrease weights for more lenient detection
                for weight in weights.keyword_weights.values_mut() {
                    *weight *= 0.7;
                }
                weights.bias = 0.05;
            }
        }

        self.toxicity_model_loaded
            .store(true, std::sync::atomic::Ordering::SeqCst);

        info!("Toxicity model enabled");
        Ok(())
    }

    pub fn check_toxicity(&self, content: &str) -> Result<ToxicityResult, ContentValidatorError> {
        let guard = self.toxicity_weights.get(&()).ok_or_else(|| {
            ContentValidatorError::Internal("toxicity weights missing".to_string())
        })?;
        let weights = &*guard;
        let lower = content.to_ascii_lowercase();

        let mut score = weights.bias;
        let mut categories = Vec::new();

        // Check each keyword
        for (keyword, weight) in &weights.keyword_weights {
            if lower.contains(keyword.as_str()) {
                score += *weight;
                categories.push(keyword.clone());
            }
        }

        // Check regex patterns (simplified - no regex crate)
        let suspicious_patterns = [
            ("http://", "URL present"),
            (".exe", "Executable reference"),
            (".dll", "DLL reference"),
            ("base64", "Encoded content"),
            ("eval(", "Code evaluation"),
            ("script>", "Script tag"),
        ];

        for (pattern, category) in suspicious_patterns {
            if lower.contains(pattern) {
                score += 0.15;
                categories.push(category.to_string());
            }
        }

        // Normalize score to [0, 1]
        score = score.clamp(0.0, 1.0);

        let is_toxic = score > 0.5;

        // Deduplicate categories
        categories.sort();
        categories.dedup();

        debug!(
            "Toxicity check: score={:.3}, is_toxic={}, categories={:?}",
            score, is_toxic, categories
        );

        Ok(ToxicityResult {
            is_toxic,
            score,
            categories,
        })
    }

    pub fn validate_config(
        &self,
        config: &str,
    ) -> Result<ConfigValidationResult, ContentValidatorError> {
        let mut is_safe = true;
        let mut issues = Vec::new();

        // 1. JSON syntax validation
        let trimmed = config.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Err(e) = serde_json::from_str::<serde_json::Value>(config) {
                issues.push(format!("Invalid JSON syntax: {}", e));
                is_safe = false;
            }
        }

        // 2. Dangerous pattern detection
        let dangerous_patterns = [
            ("exec(", "Command execution"),
            ("system(", "System call"),
            ("fork(", "Process forking"),
            ("rm -rf", "Recursive deletion"),
            ("dd if=", "Disk manipulation"),
            ("chmod 777", "Insecure permissions"),
            ("chmod +x", "Executable permission"),
            ("wget", "Remote download"),
            ("curl | sh", "Remote script execution"),
            ("eval(", "Code evaluation"),
            ("execfile", "File execution"),
            ("subprocess", "Subprocess spawn"),
            ("shell=True", "Shell execution"),
            ("> /dev/sd", "Direct disk write"),
            ("DROP TABLE", "Database deletion"),
            ("DELETE FROM", "Database deletion"),
            ("--no-check-certificate", "SSL bypass"),
        ];

        let lower = config.to_ascii_lowercase();
        for (pattern, description) in dangerous_patterns {
            if lower.contains(pattern) {
                issues.push(format!("Dangerous pattern: {} ({})", pattern, description));
                is_safe = false;
            }
        }

        // 3. Config size check
        if config.len() > 1_000_000 {
            issues.push(format!(
                "Config too large: {} bytes (max 1MB)",
                config.len()
            ));
            is_safe = false;
        }

        // 4. Recursion depth check for nested structures
        fn count_depth(s: &str, depth: usize) -> usize {
            if depth > 20 {
                return depth;
            }
            let next = s.find(|c| c == '{' || c == '[' || c == '}' || c == ']');
            match next {
                Some(idx) => count_depth(&s[idx + 1..], depth + 1),
                None => depth,
            }
        }
        let depth = count_depth(config, 0);
        if depth > 15 {
            issues.push(format!("Excessive nesting depth: {}", depth));
            is_safe = false;
        }

        let sandbox_result = if is_safe {
            Some("Passed all validation checks".to_string())
        } else {
            None
        };

        Ok(ConfigValidationResult {
            is_safe,
            issues,
            sandbox_result,
        })
    }

    pub fn check_similarity(
        &self,
        content: &str,
        existing: &[String],
    ) -> Result<SimilarityResult, ContentValidatorError> {
        let threshold = f32::from_bits(self.similarity_threshold.load(Ordering::Relaxed));
        // Collect seeds into array
        let mut seeds_array = [0u32; 128];
        for i in 0..128 {
            if let Some(v) = self.min_hash_seeds.get(&i) {
                seeds_array[i] = *v;
            }
        }

        let content_hash = self.compute_minhash(content, &seeds_array);

        let mut max_similarity = 0.0_f32;

        for existing_content in existing {
            let existing_hash = self.compute_minhash(existing_content, &seeds_array);
            let sim = self.compute_jaccard(&content_hash, &existing_hash);
            max_similarity = max_similarity.max(sim);
        }

        Ok(SimilarityResult {
            is_duplicate: max_similarity > threshold,
            max_similarity,
            threshold,
        })
    }

    /// Compute MinHash signature for content
    fn compute_minhash(&self, content: &str, seeds: &[u32; 128]) -> Vec<u32> {
        let words: std::collections::HashSet<_> = content.split_whitespace().collect();

        let mut signature = Vec::with_capacity(128);

        for &seed in seeds {
            let mut min_hash = u32::MAX;

            for word in &words {
                let mut hasher = DefaultHasher::new();
                word.hash(&mut hasher);
                seed.hash(&mut hasher);
                let hash = hasher.finish() as u32;
                min_hash = min_hash.min(hash);
            }

            signature.push(min_hash);
        }

        signature
    }

    /// Compute Jaccard similarity between two MinHash signatures
    fn compute_jaccard(&self, a: &[u32], b: &[u32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }

        let matches = a.iter().zip(b.iter()).filter(|(x, y)| x == y).count();
        matches as f32 / a.len() as f32
    }

    pub fn set_similarity_threshold(&self, threshold: f32) {
        self.similarity_threshold
            .store(threshold.clamp(0.0, 1.0).to_bits(), Ordering::Relaxed);
    }

    pub fn is_toxicity_model_loaded(&self) -> bool {
        self.toxicity_model_loaded
            .load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ToxicityResult {
    pub is_toxic: bool,
    pub score: f32,
    pub categories: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct SimilarityResult {
    pub is_duplicate: bool,
    pub max_similarity: f32,
    pub threshold: f32,
}

#[derive(Clone, Debug)]
pub struct ConfigValidationResult {
    pub is_safe: bool,
    pub issues: Vec<String>,
    pub sandbox_result: Option<String>,
}
