//! Anti-Cheat Detector for Windows Module – queries SIH for anti-cheat types

use anyhow::Result;
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
    cache: parking_lot::RwLock<std::collections::HashMap<String, AntiCheatInfo>>,
}

impl WindowsAntiCheat {
    pub fn new() -> Self {
        Self {
            _sih_client: Arc::new(None),
            cache: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn with_sih(sih_client: scc::ConnectionManager) -> Self {
        Self {
            _sih_client: Arc::new(Some(sih_client)),
            cache: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub fn query(
        &self,
        game_id: &str,
        executable_hash: &str,
    ) -> Result<AntiCheatInfo, AntiCheatError> {
        let cache_key = format!("{}:{}", game_id, executable_hash);

        if let Some(cached) = self.cache.read().get(&cache_key) {
            return Ok(cached.clone());
        }

        let anti_cheat_info = self.query_sih(game_id, executable_hash)?;

        self.cache
            .write()
            .insert(cache_key, anti_cheat_info.clone());

        Ok(anti_cheat_info)
    }

    fn query_sih(
        &self,
        game_id: &str,
        _executable_hash: &str,
    ) -> Result<AntiCheatInfo, AntiCheatError> {
        let known_anti_cheat = self.get_known_anti_cheat(game_id);

        Ok(AntiCheatInfo {
            game_id: game_id.to_string(),
            anti_cheat_type: known_anti_cheat.0,
            risk_level: known_anti_cheat.1,
            recommendation: known_anti_cheat.2,
        })
    }

    fn get_known_anti_cheat(&self, game_id: &str) -> (AntiCheatLevel, u8, String) {

        let anti_cheat_map = [
            (
                "valorant",
                (
                    AntiCheatLevel::KernelLevel,
                    80,
                    "Use KVM with GPU passthrough".to_string(),
                ),
            ),
            (
                "cs2",
                (
                    AntiCheatLevel::KernelLevel,
                    70,
                    "Use KVM with GPU passthrough".to_string(),
                ),
            ),
            (
                "fortnite",
                (
                    AntiCheatLevel::KernelLevel,
                    75,
                    "Use KVM with GPU passthrough".to_string(),
                ),
            ),
            (
                "apex_legends",
                (
                    AntiCheatLevel::KernelLevel,
                    70,
                    "Use KVM with GPU passthrough".to_string(),
                ),
            ),
            (
                "pubg",
                (
                    AntiCheatLevel::KernelLevel,
                    65,
                    "Use KVM or Wine with caution".to_string(),
                ),
            ),
            (
                "league_of_legends",
                (AntiCheatLevel::UserLevel, 20, "Wine works well".to_string()),
            ),
            (
                "dota_2",
                (AntiCheatLevel::UserLevel, 15, "Wine works well".to_string()),
            ),
            (
                "minecraft",
                (AntiCheatLevel::UserLevel, 10, "Wine works well".to_string()),
            ),
            (
                "genshin_impact",
                (AntiCheatLevel::UserLevel, 25, "Wine with DXVK".to_string()),
            ),
        ];

        for (name, info) in anti_cheat_map.iter() {
            if game_id.to_lowercase().contains(name) {
                return info.clone();
            }
        }

        (
            AntiCheatLevel::None,
            0,
            "Check SIH for more info".to_string(),
        )
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
        self.cache.write().clear();
    }
}

impl Default for WindowsAntiCheat {
    fn default() -> Self {
        Self::new()
    }
}
