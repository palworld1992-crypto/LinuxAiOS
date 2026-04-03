//! Wine Executor for Windows Module

use parking_lot::RwLock;
use std::process::{Child, Command, Stdio};
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
    config: RwLock<WineConfig>,
    child: RwLock<Option<Child>>,
    running: RwLock<bool>,
}

impl WineExecutor {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(WineConfig::default()),
            child: RwLock::new(None),
            running: RwLock::new(false),
        }
    }

    pub fn with_config(config: WineConfig) -> Self {
        Self {
            config: RwLock::new(config),
            child: RwLock::new(None),
            running: RwLock::new(false),
        }
    }

    pub fn set_config(&self, config: WineConfig) {
        *self.config.write() = config;
    }

    pub fn find_wine() -> Result<String, WineError> {
        let wine_paths = [
            "/usr/bin/wine",
            "/usr/local/bin/wine",
            "/opt/wine-stable/bin/wine",
            "/opt/wine-devel/bin/wine",
            "/opt/wine-staging/bin/wine",
        ];

        for path in wine_paths {
            if std::path::Path::new(path).exists() {
                return Ok(path.to_string());
            }
        }

        Err(WineError::NotFound)
    }

    pub fn start(&self, program: Option<&str>) -> Result<u32, WineError> {
        if *self.running.read() {
            if let Some(ref child) = *self.child.read() {
                return Ok(child.id());
            }
        }

        let wine_path = Self::find_wine()?;
        let cfg = self.config.read().clone();

        let mut cmd = Command::new(&wine_path);
        cmd.arg("server");

        if let Some(ref prefix) = cfg.prefix {
            cmd.env("WINEPREFIX", prefix);
        }

        cmd.arg("-timeout").arg(cfg.server_timeout.to_string());

        let _server = cmd
            .spawn()
            .map_err(|e| WineError::StartError(e.to_string()))?;

        let mut cmd = Command::new(wine_path);
        if let Some(ref prefix) = cfg.prefix {
            cmd.env("WINEPREFIX", prefix);
        }
        cmd.env("WINEDEBUG", "-all");
        cmd.env("WINESERVER", format!("-timeout {}", cfg.server_timeout));

        if !cfg.preload_libs.is_empty() {
            let ld_preload = cfg.preload_libs.join(":");
            cmd.env("LD_PRELOAD", ld_preload);
        }

        if let Some(prog) = program {
            cmd.arg(prog);
        } else if let Some(ref prog) = cfg.program {
            cmd.arg(prog);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        let child = cmd
            .spawn()
            .map_err(|e| WineError::StartError(e.to_string()))?;
        let pid = child.id();

        *self.child.write() = Some(child);
        *self.running.write() = true;

        info!("Wine started with PID {}", pid);
        Ok(pid)
    }

    pub fn stop(&self) -> Result<(), WineError> {
        if let Some(ref mut child) = self.child.write().take() {
            child
                .kill()
                .map_err(|e| WineError::ProcessError(e.to_string()))?;
            let _ = child.wait();
        }
        *self.running.write() = false;
        info!("Wine stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        *self.running.read()
    }

    pub fn get_pid(&self) -> Option<u32> {
        self.child.read().as_ref().map(|c| c.id())
    }

    pub fn execute_command(&self, args: &[&str]) -> Result<std::process::Output, WineError> {
        let wine_path = Self::find_wine()?;
        let cfg = self.config.read().clone();

        let mut cmd = Command::new(wine_path);
        if let Some(ref prefix) = cfg.prefix {
            cmd.env("WINEPREFIX", prefix);
        }
        cmd.args(args);

        cmd.output()
            .map_err(|e| WineError::ProcessError(e.to_string()))
    }

    pub fn get_version(&self) -> Result<String, WineError> {
        let output = self.execute_command(&["--version"])?;
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub fn create_prefix(&self, prefix_path: &str) -> Result<(), WineError> {
        let wine_path = Self::find_wine()?;
        let mut cmd = Command::new(wine_path);
        cmd.arg("boot");
        cmd.env("WINEPREFIX", prefix_path);

        cmd.output()
            .map_err(|e| WineError::ProcessError(e.to_string()))?;
        info!("Created Wine prefix at {}", prefix_path);
        Ok(())
    }
}

impl Default for WineExecutor {
    fn default() -> Self {
        Self::new()
    }
}
