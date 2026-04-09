use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContainerError {
    #[error("Container not found: {0}")]
    NotFound(String),
    #[error("Container already exists: {0}")]
    AlreadyExists(String),
    #[error("Container operation failed: {0}")]
    OperationFailed(String),
    #[error("Container capacity exceeded (max 500)")]
    CapacityExceeded,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerState {
    Created,
    Running,
    Stopped,
    Frozen,
    Hibernated,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub state: ContainerState,
    pub cpu_percent: f32,
    pub memory_mb: u64,
}

pub struct AndroidContainerManager {
    containers: dashmap::DashMap<String, ContainerInfo>,
    running_containers: dashmap::DashSet<String>,
    container_counter: AtomicU64,
}

impl AndroidContainerManager {
    pub fn new() -> Result<Self, ContainerError> {
        Ok(Self {
            containers: dashmap::DashMap::new(),
            running_containers: dashmap::DashSet::new(),
            container_counter: AtomicU64::new(0),
        })
    }

    pub fn create_container(&self, name: &str) -> Result<String, ContainerError> {
        if self.containers.len() >= 500 {
            return Err(ContainerError::CapacityExceeded);
        }
        let id = format!(
            "android-ctr-{}",
            self.container_counter.fetch_add(1, Ordering::SeqCst)
        );
        let info = ContainerInfo {
            id: id.clone(),
            name: name.to_string(),
            state: ContainerState::Created,
            cpu_percent: 0.0,
            memory_mb: 0,
        };
        self.containers.insert(id.clone(), info);
        Ok(id)
    }

    pub fn start_container(&self, id: &str) -> Result<(), ContainerError> {
        if let Some(mut info) = self.containers.get_mut(id) {
            info.state = ContainerState::Running;
            self.running_containers.insert(id.to_string());
            Ok(())
        } else {
            Err(ContainerError::NotFound(id.to_string()))
        }
    }

    pub fn stop_container(&self, id: &str) -> Result<(), ContainerError> {
        if let Some(mut info) = self.containers.get_mut(id) {
            info.state = ContainerState::Stopped;
            self.running_containers.remove(id);
            Ok(())
        } else {
            Err(ContainerError::NotFound(id.to_string()))
        }
    }

    pub fn freeze_container(&self, id: &str) -> Result<(), ContainerError> {
        if let Some(mut info) = self.containers.get_mut(id) {
            info.state = ContainerState::Frozen;
            self.running_containers.remove(id);
            Ok(())
        } else {
            Err(ContainerError::NotFound(id.to_string()))
        }
    }

    pub fn hibernate_container(&self, id: &str) -> Result<(), ContainerError> {
        if let Some(mut info) = self.containers.get_mut(id) {
            info.state = ContainerState::Hibernated;
            self.running_containers.remove(id);
            Ok(())
        } else {
            Err(ContainerError::NotFound(id.to_string()))
        }
    }

    pub fn get_container(&self, id: &str) -> Option<ContainerInfo> {
        self.containers.get(id).map(|r| r.value().clone())
    }

    pub fn get_running_containers(&self) -> Vec<ContainerInfo> {
        self.running_containers
            .iter()
            .filter_map(|id| self.containers.get(&*id).map(|r| r.value().clone()))
            .collect()
    }

    pub fn remove_container(&self, id: &str) -> Result<(), ContainerError> {
        if self.containers.remove(id).is_some() {
            self.running_containers.remove(id);
            Ok(())
        } else {
            Err(ContainerError::NotFound(id.to_string()))
        }
    }

    pub fn container_count(&self) -> usize {
        self.containers.len()
    }

    pub fn running_container_count(&self) -> usize {
        self.running_containers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_container() -> anyhow::Result<()> {
        let manager = AndroidContainerManager::new()?;
        let id = manager.create_container("test-app")?;
        assert!(id.starts_with("android-ctr-"));
        assert_eq!(manager.container_count(), 1);
        Ok(())
    }

    #[test]
    fn test_container_lifecycle() -> anyhow::Result<()> {
        let manager = AndroidContainerManager::new()?;
        let id = manager.create_container("test-app")?;

        manager.start_container(&id)?;
        assert_eq!(
            manager
                .get_container(&id)
                .ok_or_else(|| anyhow::anyhow!("container not found"))?
                .state,
            ContainerState::Running
        );

        manager.freeze_container(&id)?;
        assert_eq!(
            manager
                .get_container(&id)
                .ok_or_else(|| anyhow::anyhow!("container not found"))?
                .state,
            ContainerState::Frozen
        );

        manager.hibernate_container(&id)?;
        assert_eq!(
            manager
                .get_container(&id)
                .ok_or_else(|| anyhow::anyhow!("container not found"))?
                .state,
            ContainerState::Hibernated
        );

        manager.stop_container(&id)?;
        assert_eq!(
            manager
                .get_container(&id)
                .ok_or_else(|| anyhow::anyhow!("container not found"))?
                .state,
            ContainerState::Stopped
        );
        Ok(())
    }

    #[test]
    fn test_remove_container() -> anyhow::Result<()> {
        let manager = AndroidContainerManager::new()?;
        let id = manager.create_container("test-app")?;
        manager.remove_container(&id)?;
        assert_eq!(manager.container_count(), 0);
        Ok(())
    }

    #[test]
    fn test_container_not_found() -> anyhow::Result<()> {
        let manager = AndroidContainerManager::new()?;
        assert!(manager.start_container("nonexistent").is_err());
        Ok(())
    }

    #[test]
    fn test_get_running_containers() -> anyhow::Result<()> {
        let manager = AndroidContainerManager::new()?;
        let id1 = manager.create_container("app1")?;
        let _id2 = manager.create_container("app2")?;
        manager.start_container(&id1)?;

        let running = manager.get_running_containers();
        assert_eq!(running.len(), 1);
        assert_eq!(running[0].id, id1);
        Ok(())
    }
}
