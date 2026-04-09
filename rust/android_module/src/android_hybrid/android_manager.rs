use dashmap::DashMap;
use std::process::Command;
use thiserror::Error;
use tracing::warn;

fn get_current_timestamp() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(d) => d.as_secs(),
        Err(e) => {
            warn!("SystemTime before UNIX_EPOCH: {}, using 0", e);
            0
        }
    }
}

#[derive(Error, Debug)]
pub enum HybridLibraryError {
    #[error("Failed to load library: {0}")]
    LoadError(String),
    #[error("Library not found: {0}")]
    NotFound(String),
    #[error("Signature verification failed: {0}")]
    SignatureError(String),
    #[error("Failed to spawn process: {0}")]
    SpawnError(String),
    #[error("Seccomp filter error: {0}")]
    SeccompError(String),
}

#[derive(Debug, Clone)]
pub struct HybridLibraryInfo {
    pub name: String,
    pub path: String,
    pub version: String,
    pub expires_at: u64,
    pub is_loaded: bool,
}

pub struct AndroidHybridLibraryManager {
    libraries: DashMap<String, HybridLibraryInfo>,
    processes: DashMap<String, u32>,
    seccomp_filter: crate::android_hybrid::android_seccomp_filter::AndroidSeccompFilter,
}

impl Default for AndroidHybridLibraryManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AndroidHybridLibraryManager {
    pub fn new() -> Self {
        Self {
            libraries: DashMap::new(),
            processes: DashMap::new(),
            seccomp_filter:
                crate::android_hybrid::android_seccomp_filter::AndroidSeccompFilter::new(),
        }
    }

    pub fn register_library(
        &self,
        name: &str,
        path: &str,
        version: &str,
    ) -> Result<(), HybridLibraryError> {
        if self.libraries.contains_key(name) {
            return Err(HybridLibraryError::LoadError(format!(
                "Library {} already registered",
                name
            )));
        }

        let now = get_current_timestamp();

        let info = HybridLibraryInfo {
            name: name.to_string(),
            path: path.to_string(),
            version: version.to_string(),
            expires_at: now + 30 * 24 * 3600,
            is_loaded: false,
        };

        self.libraries.insert(name.to_string(), info);
        Ok(())
    }

    pub fn load_library(&self, name: &str) -> Result<(), HybridLibraryError> {
        let mut info = self
            .libraries
            .get_mut(name)
            .ok_or_else(|| HybridLibraryError::NotFound(name.to_string()))?;
        info.value_mut().is_loaded = true;
        Ok(())
    }

    pub fn spawn_library_process(
        &self,
        name: &str,
        args: &[&str],
    ) -> Result<u32, HybridLibraryError> {
        let lib_info = self
            .libraries
            .get(name)
            .ok_or_else(|| HybridLibraryError::NotFound(name.to_string()))?
            .value()
            .clone();

        let mut cmd = Command::new(&lib_info.path);
        cmd.args(args);

        let child = cmd.spawn().map_err(|e| {
            HybridLibraryError::SpawnError(format!("Failed to spawn {}: {}", name, e))
        })?;

        let pid = child.id();

        self.processes.insert(name.to_string(), pid);

        drop(child);

        self.seccomp_filter
            .apply_to_pid(pid as i32)
            .map_err(|e| HybridLibraryError::SeccompError(e.to_string()))?;

        Ok(pid)
    }

    pub fn kill_library_process(&self, name: &str) -> Result<(), HybridLibraryError> {
        if let Some((_, pid)) = self.processes.remove(name) {
            std::process::Command::new("kill")
                .arg("-9")
                .arg(pid.to_string())
                .spawn()
                .map_err(|e| {
                    HybridLibraryError::SpawnError(format!("Failed to kill process: {}", e))
                })?;
        }

        Ok(())
    }

    pub fn get_process_pid(&self, name: &str) -> Option<u32> {
        self.processes.get(name).map(|e| *e.value())
    }

    pub fn unload_library(&self, name: &str) -> Result<(), HybridLibraryError> {
        let _ = self.kill_library_process(name);

        let mut info = self
            .libraries
            .get_mut(name)
            .ok_or_else(|| HybridLibraryError::NotFound(name.to_string()))?;
        info.value_mut().is_loaded = false;
        Ok(())
    }

    pub fn get_library(&self, name: &str) -> Option<HybridLibraryInfo> {
        self.libraries.get(name).map(|e| e.value().clone())
    }

    pub fn list_libraries(&self) -> Vec<HybridLibraryInfo> {
        self.libraries.iter().map(|e| e.value().clone()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_unload_library() -> anyhow::Result<()> {
        let manager = AndroidHybridLibraryManager::new();
        manager.register_library("libbinder", "/system/lib/libbinder.so", "1.0")?;
        manager.load_library("libbinder")?;
        assert!(
            manager
                .get_library("libbinder")
                .ok_or_else(|| anyhow::anyhow!("library not found"))?
                .is_loaded
        );
        manager.unload_library("libbinder")?;
        assert!(
            !manager
                .get_library("libbinder")
                .ok_or_else(|| anyhow::anyhow!("library not found"))?
                .is_loaded
        );
        Ok(())
    }

    #[test]
    fn test_duplicate_registration() -> anyhow::Result<()> {
        let manager = AndroidHybridLibraryManager::new();
        manager.register_library("libbinder", "/system/lib/libbinder.so", "1.0")?;
        let result = manager.register_library("libbinder", "/system/lib/libbinder.so", "1.0");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_list_libraries() -> anyhow::Result<()> {
        let manager = AndroidHybridLibraryManager::new();
        manager.register_library("libbinder", "/system/lib/libbinder.so", "1.0")?;
        manager.register_library("libcutils", "/system/lib/libcutils.so", "1.0")?;
        assert_eq!(manager.list_libraries().len(), 2);
        Ok(())
    }
}
