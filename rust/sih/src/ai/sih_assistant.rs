use crate::ai::{EmbeddingEngine, RecommenderAI, SihLnnPredictor, SihModelManager, SihRlPolicy};
use crate::errors::SihAssistantError;
use child_tunnel::ChildTunnel;
use std::sync::Arc;
use tracing::{info, warn};

pub struct SihAssistant {
    recommender: RecommenderAI,
    embedding_engine: EmbeddingEngine,
    model_manager: SihModelManager,
    lnn_predictor: SihLnnPredictor,
    rl_policy: SihRlPolicy,
    child_tunnel: Arc<ChildTunnel>,
}

impl SihAssistant {
    pub fn new(child_tunnel: Arc<ChildTunnel>) -> Self {
        // Register SIH Assistant with Child Tunnel
        let component_id = "sih_assistant".to_string();
        if let Err(e) = child_tunnel.update_state(component_id.clone(), vec![], true) {
            warn!("Failed to register SIH Assistant with Child Tunnel: {}", e);
        } else {
            info!("SIH Assistant registered with Child Tunnel");
        }

        Self {
            recommender: RecommenderAI::new(Default::default()),
            embedding_engine: EmbeddingEngine::new(768, 1024),
            model_manager: SihModelManager::new(std::path::PathBuf::from("/tmp/sih_models")),
            lnn_predictor: SihLnnPredictor::new(1024, 10),
            rl_policy: SihRlPolicy::new(),
            child_tunnel,
        }
    }

    pub fn initialize(&mut self) -> Result<(), SihAssistantError> {
        use std::fs;

        let models_dir = "/tmp/sih_models";

        if !std::path::Path::new(models_dir).exists() {
            fs::create_dir_all(models_dir).map_err(|e| {
                SihAssistantError::InitFailed(format!("Failed to create models dir: {}", e))
            })?;
        }

        let entries = match fs::read_dir(models_dir) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(SihAssistantError::InitFailed(format!(
                    "Failed to read models dir: {}",
                    e
                )));
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    tracing::warn!("Failed to read directory entry: {}", e);
                    continue;
                }
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext_opt = path.extension().and_then(|s| s.to_str());
            let name = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => "unknown".to_string(),
            };

            if let Err(e) = self.model_manager.load_model(&name, &path) {
                tracing::warn!("Failed to register model {}: {}", name, e);
            }

            let ext = match ext_opt {
                Some(s) => s,
                None => "",
            };
            match ext {
                "onnx" | "gguf" => {
                    if name.contains("embedding") || name == "distilbert" || name.contains("bert") {
                        let path_str = match path.to_str() {
                            Some(s) => s,
                            None => {
                                tracing::warn!("Invalid path for embedding model: {}", name);
                                continue;
                            }
                        };
                        if let Err(e) = self.embedding_engine.load_model(path_str) {
                            tracing::warn!("Failed to load embedding model {}: {}", name, e);
                        } else {
                            tracing::info!("Loaded embedding model: {}", name);
                        }
                    } else if name.contains("recommender") || name == "recommender" {
                        let path_str = match path.to_str() {
                            Some(s) => s,
                            None => {
                                tracing::warn!("Invalid path for recommender model: {}", name);
                                continue;
                            }
                        };
                        if let Err(e) = self.recommender.load_model(path_str) {
                            tracing::warn!("Failed to load recommender model {}: {}", name, e);
                        } else {
                            tracing::info!("Loaded recommender model: {}", name);
                        }
                    } else if name.contains("lnn") || name.contains("predictor") {
                        let path_str = match path.to_str() {
                            Some(s) => s,
                            None => {
                                tracing::warn!("Invalid path for LNN predictor: {}", name);
                                continue;
                            }
                        };
                        if let Err(e) = self.lnn_predictor.load_model(path_str) {
                            tracing::warn!("Failed to load LNN predictor model {}: {}", name, e);
                        } else {
                            tracing::info!("Loaded LNN predictor model: {}", name);
                        }
                    } else if name.contains("rl") || name.contains("policy") {
                        let path_str = match path.to_str() {
                            Some(s) => s,
                            None => {
                                tracing::warn!("Invalid path for RL policy: {}", name);
                                continue;
                            }
                        };
                        if let Err(e) = self.rl_policy.load_policy(path_str) {
                            tracing::warn!("Failed to load RL policy model {}: {}", name, e);
                        } else {
                            tracing::info!("Loaded RL policy model: {}", name);
                        }
                    }
                }
                _ => {}
            }
        }

        tracing::info!("SIH Assistant initialization complete");
        Ok(())
    }

    pub fn get_recommender(&self) -> &RecommenderAI {
        &self.recommender
    }

    pub fn get_embedding_engine(&self) -> &EmbeddingEngine {
        &self.embedding_engine
    }

    pub fn get_model_manager(&self) -> &SihModelManager {
        &self.model_manager
    }

    pub fn set_lnn_predictor(&mut self, predictor: SihLnnPredictor) {
        self.lnn_predictor = predictor;
    }

    pub fn set_rl_policy(&mut self, policy: SihRlPolicy) {
        self.rl_policy = policy;
    }

    pub fn predict_query_trend(&self) -> Vec<String> {
        self.lnn_predictor
            .predict_next()
            .iter()
            .map(|q| q.query.clone())
            .collect()
    }

    pub fn get_trust_policy(&self) -> String {
        self.rl_policy.get_policy_action()
    }

    pub fn get_signal_strength(&self) -> f32 {
        self.recommender.get_confidence().clamp(0.0, 1.0)
    }
}

impl Default for SihAssistant {
    fn default() -> Self {
        let child_tunnel = Arc::new(ChildTunnel::default());
        Self::new(child_tunnel)
    }
}
