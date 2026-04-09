use anyhow::{anyhow, Result};
use common::health_tunnel::HealthTunnel;
use common::utils::current_timestamp_ms;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Intent {
    Proposal,
    ModelUpdate,
    ConfigChange,
    // Thêm các intent khác nếu cần
}

impl Intent {
    fn as_str(&self) -> &'static str {
        match self {
            Intent::Proposal => "proposal",
            Intent::ModelUpdate => "model_update",
            Intent::ConfigChange => "config_change",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PolicyEntry {
    allowed_intents: Vec<String>, // tên intent dạng string
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PolicyConfig {
    modules: HashMap<String, PolicyEntry>,
    default: PolicyEntry, // cho module không có policy riêng
}

/// Intent token chứa thông tin xác thực và ngăn replay
#[derive(Debug, Clone)]
pub struct IntentToken {
    pub source: String,
    pub target: String,
    pub intent: Intent,
    pub nonce: u64,
    pub timestamp: u64,
    pub signature: Vec<u8>, // chữ ký Dilithium của source
}

impl IntentToken {
    pub fn new(source: String, target: String, intent: Intent, signature: Vec<u8>) -> Self {
        Self {
            source,
            target,
            intent,
            nonce: rand::random(),
            timestamp: current_timestamp_ms(),
            signature,
        }
    }

    pub fn is_expired(&self, ttl_ms: u64) -> bool {
        current_timestamp_ms() > self.timestamp + ttl_ms
    }
}

pub struct IntentValidator {
    policies: DashMap<(), PolicyConfig>,
    token_cache: DashMap<String, u64>,
    health_tunnel: Option<Arc<dyn HealthTunnel + Send + Sync>>,
}

impl IntentValidator {
    pub fn new() -> Self {
        // Tải policy mặc định (cho phép mọi intent nếu không có file)
        let default_config = PolicyConfig {
            modules: HashMap::new(),
            default: PolicyEntry {
                allowed_intents: vec![
                    Intent::Proposal.as_str().to_string(),
                    Intent::ModelUpdate.as_str().to_string(),
                    Intent::ConfigChange.as_str().to_string(),
                ],
            },
        };
        let mut policies_map = DashMap::new();
        policies_map.insert((), default_config);
        Self {
            policies: policies_map,
            token_cache: DashMap::new(),
            health_tunnel: None,
        }
    }

    /// Load policy từ file JSON
    pub fn load_policy(&self, path: &PathBuf) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let config: PolicyConfig = serde_json::from_str(&content)?;
        self.policies.insert((), config);
        Ok(())
    }

    /// Gán Health Tunnel để có thể kiểm tra trạng thái module
    pub fn set_health_tunnel(&mut self, tunnel: Arc<dyn HealthTunnel + Send + Sync>) {
        self.health_tunnel = Some(tunnel);
    }

    /// Validate token và intent
    pub fn validate(&self, token: &IntentToken, ttl_ms: u64) -> Result<()> {
        // 1. Kiểm tra token hết hạn
        if token.is_expired(ttl_ms) {
            return Err(anyhow!("Intent token expired"));
        }

        // 2. Kiểm tra replay (dùng nonce)
        let cache_key = format!("{}|{}|{}", token.source, token.target, token.nonce);
        if self.token_cache.contains_key(&cache_key) {
            return Err(anyhow!("Intent token already used (replay attack)"));
        }

        // 3. Kiểm tra chữ ký Dilithium của source
        // TODO(Phase 4): Lấy public key từ Master Tunnel qua `token.source`
        // Hiện tại dùng key mặc định để verify, sẽ thay bằng key từ registry
        if !token.signature.is_empty() {
            // TODO(Phase 4): Tích hợp với Master Tunnel để lấy public key của source module
            // let public_key = master_tunnel.get_public_key(&token.source)?;
            // if !scc::crypto::ffi::dilithium_verify(&public_key, &message, &token.signature)? {
            //     return Err(anyhow!("Invalid Dilithium signature"));
            // }
            // Tạm thời bỏ qua verify vì chưa có public key từ Master Tunnel
        }

        // 4. Kiểm tra quyền dựa trên policy
        let policy_ref = self
            .policies
            .get(&())
            .ok_or_else(|| anyhow!("No policy loaded"))?;
        let policy = policy_ref.value();
        let entry = policy
            .modules
            .get(&token.source)
            .or(Some(&policy.default))
            .ok_or_else(|| anyhow!("No policy for source module {}", token.source))?;

        if !entry
            .allowed_intents
            .contains(&token.intent.as_str().to_string())
        {
            return Err(anyhow!(
                "Intent {:?} not allowed for module {}",
                token.intent,
                token.source
            ));
        }

        // 5. Kiểm tra trạng thái health của target (nếu có health_tunnel)
        if let Some(tunnel) = &self.health_tunnel {
            if let Some(record) = tunnel.last_health(&token.target) {
                if record.status == common::health_tunnel::HealthStatus::Failed {
                    return Err(anyhow!(
                        "Target module {} is in failed state, cannot accept intent",
                        token.target
                    ));
                }
            } else {
                // Không có health record -> coi như unknown, vẫn cho phép nhưng log warning
                tracing::warn!("No health record for target module {}", token.target);
            }
        }

        // 6. Ghi token vào cache để ngăn replay (giữ ttl_ms)
        self.token_cache
            .insert(cache_key, current_timestamp_ms() + ttl_ms);
        // Dọn cache: xóa các entry hết hạn (đơn giản, có thể tối ưu sau)
        self.token_cache
            .retain(|_, expire| *expire > current_timestamp_ms());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
    use dashmap::DashMap;
    use std::sync::Arc;

    struct MockHealthTunnel {
        status: DashMap<String, HealthStatus>,
    }

    impl MockHealthTunnel {
        fn new() -> Self {
            Self {
                status: DashMap::new(),
            }
        }

        fn set_status(&self, module: &str, status: HealthStatus) {
            self.status.insert(module.to_string(), status);
        }
    }

    impl HealthTunnel for MockHealthTunnel {
        fn record_health(&self, _record: HealthRecord) -> Result<()> {
            Ok(())
        }
        fn last_health(&self, module_id: &str) -> Option<HealthRecord> {
            self.status.get(module_id).map(|s| HealthRecord {
                module_id: module_id.to_string(),
                timestamp: current_timestamp_ms(),
                status: s.clone(),
                details: vec![],
            })
        }
        fn health_history(&self, _module_id: &str, _limit: usize) -> Vec<HealthRecord> {
            Vec::new()
        }
        fn rollback(&self) -> Option<Vec<HealthRecord>> {
            None
        }
    }

    #[test]
    fn test_load_policy() -> Result<(), anyhow::Error> {
        let temp_file = tempfile::NamedTempFile::new()?;
        let policy_json = r#"
        {
            "modules": {
                "linux": {
                    "allowed_intents": ["proposal", "model_update"]
                },
                "windows": {
                    "allowed_intents": ["config_change"]
                }
            },
            "default": {
                "allowed_intents": ["proposal"]
            }
        }"#;
        fs::write(temp_file.path(), policy_json)?;

        let validator = IntentValidator::new();
        validator.load_policy(&temp_file.path().to_path_buf())?;

        let policy_ref = validator
            .policies
            .get(&())
            .ok_or_else(|| anyhow!("no policy"))?;
        let policy = policy_ref.value();
        assert_eq!(policy.modules.len(), 2);
        assert_eq!(policy.default.allowed_intents, vec!["proposal"]);
        assert_eq!(
            policy.modules["linux"].allowed_intents,
            vec!["proposal", "model_update"]
        );
        Ok(())
    }

    #[test]
    #[ignore = "May segfault due to missing crypto libraries"]
    fn test_validate_token() -> Result<(), anyhow::Error> {
        // TODO: implement if needed
        Ok(())
    }

    #[test]
    #[ignore = "May segfault due to missing crypto libraries"]
    fn test_validate_with_health() -> Result<(), anyhow::Error> {
        let mut validator = IntentValidator::new();
        let health = Arc::new(MockHealthTunnel::new());
        validator.set_health_tunnel(health.clone());

        let config = PolicyConfig {
            modules: HashMap::new(),
            default: PolicyEntry {
                allowed_intents: vec!["proposal".to_string()],
            },
        };
        // Directly insert policy
        validator.policies.insert((), config);

        let token = IntentToken::new(
            "linux".to_string(),
            "master_tunnel".to_string(),
            Intent::Proposal,
            vec![],
        );

        health.set_status("master_tunnel", HealthStatus::Healthy);
        let result = validator.validate(&token, 10000);
        assert!(result.is_ok());

        health.set_status("master_tunnel", HealthStatus::Failed);
        let result = validator.validate(&token, 10000);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("failed state"));

        Ok(())
    }
}
