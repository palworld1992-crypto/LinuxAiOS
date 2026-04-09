use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("Invalid policy: {0}")]
    InvalidPolicy(String),
    #[error("Failed to apply policy: {0}")]
    ApplyError(String),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerPolicy {
    pub max_cpu_percent: u32,
    pub max_memory_mb: u64,
    pub max_io_mbps: u32,
    pub max_containers: u32,
}

pub struct AndroidPolicyEngine {
    policies: HashMap<String, ContainerPolicy>,
}

impl Default for AndroidPolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidPolicyEngine {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    pub fn load_policy(
        &mut self,
        container_id: &str,
        policy_json: &str,
    ) -> Result<(), PolicyError> {
        let policy: ContainerPolicy = serde_json::from_str(policy_json)
            .map_err(|e| PolicyError::InvalidPolicy(e.to_string()))?;
        self.policies.insert(container_id.to_string(), policy);
        Ok(())
    }

    pub fn get_policy(&self, container_id: &str) -> Option<&ContainerPolicy> {
        self.policies.get(container_id)
    }

    pub fn remove_policy(&mut self, container_id: &str) {
        self.policies.remove(container_id);
    }

    pub fn get_all_policies(&self) -> &HashMap<String, ContainerPolicy> {
        &self.policies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_and_get_policy() -> anyhow::Result<()> {
        let mut engine = AndroidPolicyEngine::new();
        let policy_json =
            r#"{"max_cpu_percent":80,"max_memory_mb":512,"max_io_mbps":100,"max_containers":5}"#;
        engine.load_policy("container-1", policy_json)?;
        assert!(engine.get_policy("container-1").is_some());
        Ok(())
    }

    #[test]
    fn test_invalid_policy() -> anyhow::Result<()> {
        let mut engine = AndroidPolicyEngine::new();
        let result = engine.load_policy("container-1", "invalid json");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_remove_policy() -> anyhow::Result<()> {
        let mut engine = AndroidPolicyEngine::new();
        let policy_json =
            r#"{"max_cpu_percent":80,"max_memory_mb":512,"max_io_mbps":100,"max_containers":5}"#;
        engine.load_policy("container-1", policy_json)?;
        engine.remove_policy("container-1");
        assert!(engine.get_policy("container-1").is_none());
        Ok(())
    }
}
