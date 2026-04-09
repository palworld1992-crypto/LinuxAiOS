//! Anti-Cheat Detector for Windows Module – queries SIH for anti-cheat types

use anyhow::Result;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AntiCheatError {
    #[error("Query failed: {0}")]
    QueryError(String),
    #[error("SIH not available")]
    SihUnavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum AntiCheatLevel {
    None,
    UserLevel,
    KernelLevel,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AntiCheatInfo {
    pub game_id: String,
    pub anti_cheat_type: AntiCheatLevel,
    pub risk_level: u8,
    pub recommendation: String,
}

pub struct WindowsAntiCheat {
    _sih_client: Arc<Option<scc::ConnectionManager>>,
    cache: DashMap<String, AntiCheatInfo>,
}

impl WindowsAntiCheat {
    pub fn new() -> Self {
        Self {
            _sih_client: Arc::new(None),
            cache: DashMap::new(),
        }
    }

    pub fn with_sih(sih_client: scc::ConnectionManager) -> Self {
        Self {
            _sih_client: Arc::new(Some(sih_client)),
            cache: DashMap::new(),
        }
    }

    pub fn query(
        &self,
        game_id: &str,
        executable_hash: &str,
    ) -> Result<AntiCheatInfo, AntiCheatError> {
        let cache_key = format!("{}:{}", game_id, executable_hash);

        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.value().clone());
        }

        let anti_cheat_info = self.query_sih(game_id, executable_hash)?;

        self.cache.insert(cache_key, anti_cheat_info.clone());

        Ok(anti_cheat_info)
    }

    fn query_sih(
        &self,
        game_id: &str,
        _executable_hash: &str,
    ) -> Result<AntiCheatInfo, AntiCheatError> {
        #[cfg(test)]
        {
            return Ok(AntiCheatInfo {
                game_id: game_id.to_string(),
                anti_cheat_type: AntiCheatLevel::None,
                risk_level: 0,
                recommendation: "Check SIH for more info".to_string(),
            });
        }
        #[cfg(not(test))]
        {
            let _ = self;
            let _ = game_id;
            let _ = _executable_hash;
            // TODO(Phase 6): Query SIH via SCC for anti-cheat info
            unimplemented!()
        }
    }

    pub fn should_use_kvm(&self, game_id: &str) -> bool {
        if let Ok(info) = self.query(game_id, "") {
            matches!(info.anti_cheat_type, AntiCheatLevel::KernelLevel)
        } else {
            false
        }
    }

    pub fn get_recommendation(&self, game_id: &str) -> String {
        if let Ok(info) = self.query(game_id, "") {
            info.recommendation
        } else {
            "Unknown".to_string()
        }
    }

    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

impl Default for WindowsAntiCheat {
    fn default() -> Self {
        Self::new()
    }
}
