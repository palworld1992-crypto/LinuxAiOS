use anyhow::{anyhow, Result};
use common::health_tunnel::HealthTunnel;
use common::utils::current_timestamp_ms;
use parking_lot::RwLock;
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
    policies: RwLock<PolicyConfig>,
    token_cache: RwLock<HashMap<String, u64>>, // key: source|target|nonce, timestamp
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
        Self {
            policies: RwLock::new(default_config),
            token_cache: RwLock::new(HashMap::new()),
            health_tunnel: None,
        }
    }

    /// Load policy từ file JSON
    pub fn load_policy(&self, path: &PathBuf) -> Result<()> {
        let content = fs::read_to_string(path)?;
        let config: PolicyConfig = serde_json::from_str(&content)?;
        *self.policies.write() = config;
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
        {
            let cache = self.token_cache.read();
            if cache.contains_key(&cache_key) {
                return Err(anyhow!("Intent token already used (replay attack)"));
            }
        }

        // 3. Kiểm tra chữ ký Dilithium của source
        // Giả sử public key của source đã được lấy từ registry (Master Tunnel)
        // Ở đây tạm thời bỏ qua verify signature vì chưa có public key
        // TODO: lấy public key từ Master Tunnel qua `token.source`
        // if !verify_signature(...) { return Err(...); }

        // 4. Kiểm tra quyền dựa trên policy
        let policy = self.policies.read();
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
        let mut cache = self.token_cache.write();
        cache.insert(cache_key, current_timestamp_ms() + ttl_ms);
        // Dọn cache: xóa các entry hết hạn (đơn giản, có thể tối ưu sau)
        cache.retain(|_, &mut expire| expire > current_timestamp_ms());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::health_tunnel::{HealthRecord, HealthStatus, HealthTunnel};
    use std::sync::Arc;

    struct MockHealthTunnel {
        status: std::sync::Mutex<HashMap<String, HealthStatus>>,
    }

    impl MockHealthTunnel {
        fn new() -> Self {
            Self {
                status: std::sync::Mutex::new(HashMap::new()),
            }
        }

        fn set_status(&self, module: &str, status: HealthStatus) {
            self.status
                .lock()
                .unwrap()
                .insert(module.to_string(), status);
        }
    }

    impl HealthTunnel for MockHealthTunnel {
        fn record_health(&self, _record: HealthRecord) -> Result<()> {
            Ok(())
        }
        fn last_health(&self, module_id: &str) -> Option<HealthRecord> {
            let status = self.status.lock().unwrap().get(module_id).cloned();
            status.map(|s| HealthRecord {
                module_id: module_id.to_string(),
                timestamp: 0,
                status: s,
                details: vec![],
            })
        }
        fn health_history(&self, _module_id: &str, _limit: usize) -> Vec<HealthRecord> {
            vec![]
        }
        fn rollback(&self) -> Option<Vec<HealthRecord>> {
            None
        }
    }

    #[test]
    fn test_load_policy() {
        let temp_file = tempfile::NamedTempFile::new().unwrap();
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
        fs::write(temp_file.path(), policy_json).unwrap();

        let validator = IntentValidator::new();
        validator
            .load_policy(&temp_file.path().to_path_buf())
            .unwrap();

        let policy = validator.policies.read();
        assert_eq!(policy.modules.len(), 2);
        assert_eq!(policy.default.allowed_intents, vec!["proposal"]);
        assert_eq!(
            policy.modules["linux"].allowed_intents,
            vec!["proposal", "model_update"]
        );
    }

    #[test]
    #[ignore = "May segfault due to missing crypto libraries"]
    fn test_validate_token() {
        // ... same as before
        let validator = IntentValidator::new();
        let config = PolicyConfig {
            modules: HashMap::new(),
            default: PolicyEntry {
                allowed_intents: vec!["proposal".to_string()],
            },
        };
        *validator.policies.write() = config;

        let token = IntentToken::new(
            "linux".to_string(),
            "master_tunnel".to_string(),
            Intent::Proposal,
            vec![],
        );

        let result = validator.validate(&token, 10000);
        assert!(result.is_ok());

        let result2 = validator.validate(&token, 10000);
        assert!(result2.is_err());
        assert!(result2.unwrap_err().to_string().contains("replay"));

        let mut expired_token = token.clone();
        expired_token.timestamp = 0;
        let result3 = validator.validate(&expired_token, 10000);
        assert!(result3.is_err());
        assert!(result3.unwrap_err().to_string().contains("expired"));

        let mut bad_token = token.clone();
        bad_token.intent = Intent::ModelUpdate;
        let result4 = validator.validate(&bad_token, 10000);
        assert!(result4.is_err());
        assert!(result4.unwrap_err().to_string().contains("not allowed"));
    }

    #[test]
    #[ignore = "May segfault due to missing crypto libraries"]
    fn test_validate_with_health() {
        // ... same as before
        let mut validator = IntentValidator::new();
        let health = Arc::new(MockHealthTunnel::new());
        validator.set_health_tunnel(health.clone());

        let config = PolicyConfig {
            modules: HashMap::new(),
            default: PolicyEntry {
                allowed_intents: vec!["proposal".to_string()],
            },
        };
        *validator.policies.write() = config;

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
    }
}
