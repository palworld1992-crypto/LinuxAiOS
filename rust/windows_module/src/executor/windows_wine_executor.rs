//! Wine Executor for Windows Module

use dashmap::DashMap;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

#[derive(Error, Debug)]
pub enum WineError {
    #[error("Failed to start Wine: {0}")]
    StartError(String),
    #[error("Wine process error: {0}")]
    ProcessError(String),
    #[error("Wine not found")]
    NotFound,
}

#[derive(Clone, Debug)]
pub struct WineConfig {
    pub prefix: Option<String>,
    pub program: Option<String>,
    pub arch: String,
    pub server_timeout: u32,
    pub preload_libs: Vec<String>,
}

impl Default for WineConfig {
    fn default() -> Self {
        Self {
            prefix: None,
            program: None,
            arch: "win64".to_string(),
            server_timeout: 60,
            preload_libs: Vec::new(),
        }
    }
}

pub struct WineExecutor {
    config: Arc<WineConfig>,
    child: DashMap<u64, Option<Child>>,
    running: AtomicBool,
    pid: AtomicU32,
}

impl WineExecutor {
    pub fn new() -> Self {
        Self {
            config: Arc::new(WineConfig::default()),
            child: DashMap::new(),
            running: AtomicBool::new(false),
            pid: AtomicU32::new(0),
        }
    }

    pub fn with_config(config: WineConfig) -> Self {
        Self {
            config: Arc::new(config),
            child: DashMap::new(),
            running: AtomicBool::new(false),
            pid: AtomicU32::new(0),
        }
    }

    pub fn set_config(&mut self, config: WineConfig) {
        self.config = Arc::new(config);
    }

    pub fn find_wine() -> Result<String, WineError> {
        let wine_paths = [
            "/usr/bin/wine",
            "/usr/local/bin/wine",
            "/opt/wine-stable/bin/wine",
            "/opt/wine-development/bin/wine",
            "/usr/bin/wine64",
        ];

        for path in wine_paths {
            if std::path::Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }

        Err(WineError::NotFound)
    }

    pub fn is_wine_available(&self) -> bool {
        Self::find_wine().is_ok()
    }

    pub fn start(&self) -> Result<u32, WineError> {
        let wine_path = Self::find_wine()?;

        let mut cmd = Command::new(&wine_path);
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        if let Some(ref prefix) = self.config.prefix {
            cmd.env("WINEPREFIX", prefix);
        }

        cmd.env("WINARCH", &self.config.arch);
        cmd.env("WINESERVER", format!("-t {}", self.config.server_timeout));

        for lib in &self.config.preload_libs {
            cmd.env("LD_PRELOAD", lib);
        }

        if let Some(ref program) = self.config.program {
            cmd.arg(program);
        }

        let child = cmd
            .spawn()
            .map_err(|e| WineError::StartError(e.to_string()))?;
        let pid = child.id();

        self.child.insert(0, Some(child));

        self.running.store(true, Ordering::Relaxed);
        self.pid.store(pid, Ordering::Relaxed);

        info!("Wine started with PID {}", pid);
        Ok(pid)
    }

    pub fn stop(&self) -> Result<(), WineError> {
        let pid = self.pid.load(Ordering::Relaxed);
        if pid != 0 {
            let _ = std::process::Command::new("kill")
                .arg("-9")
                .arg(pid.to_string())
                .output();
        }

        if let Some((_, Some(mut child))) = self.child.remove(&0) {
            let _ = child.kill();
            let _ = child.wait();
        }

        self.running.store(false, Ordering::Relaxed);
        self.pid.store(0, Ordering::Relaxed);
        info!("Wine stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn get_pid(&self) -> u32 {
        self.pid.load(Ordering::Relaxed)
    }

    pub fn get_config(&self) -> WineConfig {
        self.config.as_ref().clone()
    }
}

impl Default for WineExecutor {
    fn default() -> Self {
        Self::new()
    }
}
