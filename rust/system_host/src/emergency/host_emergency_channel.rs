//! Host Emergency Channel - Unix socket for emergency commands

use libc::{getsockopt, socklen_t, ucred, SOL_SOCKET, SO_PEERCRED};
use std::os::raw::c_void;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixListener;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmergencyCommand {
    RestartModule,
    Status,
    ForceFailover,
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct EmergencyRequest {
    pub command: EmergencyCommand,
    pub module_id: Option<String>,
    pub user_id: Option<u32>,
}

#[derive(Error, Debug)]
pub enum EmergencyError {
    #[error("Socket error: {0}")]
    SocketError(String),
    #[error("Command parse error: {0}")]
    CommandParseError(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct HostEmergencyChannel {
    socket_path: PathBuf,
    listener: Option<UnixListener>,
    allowed_users: Vec<u32>,
}

impl HostEmergencyChannel {
    pub fn new(socket_path: PathBuf) -> Self {
        Self {
            socket_path,
            listener: None,
            allowed_users: vec![0], // root
        }
    }

    pub fn with_allowed_users(mut self, users: Vec<u32>) -> Self {
        self.allowed_users = users;
        self
    }

    pub async fn start(&mut self) -> Result<(), EmergencyError> {
        if self.socket_path.exists() {
            std::fs::remove_file(&self.socket_path)?;
        }
        
        if let Some(parent) = self.socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        self.listener = Some(UnixListener::bind(&self.socket_path)?);
        
        Ok(())
    }

    pub async fn accept_connection(&self) -> Result<EmergencyRequest, EmergencyError> {
        if let Some(ref listener) = self.listener {
            let (socket, _addr) = listener.accept().await?;

            // Phase 7: Authenticate client using SO_PEERCRED
            self.authenticate_socket(&socket)?;

            let reader = BufReader::new(socket);
            let mut lines = reader.lines();
            
            if let Some(line) = lines.next_line().await? {
                self.parse_command(&line)
            } else {
                Err(EmergencyError::CommandParseError("Empty command".to_string()))
            }
        } else {
            Err(EmergencyError::SocketError("Listener not started".to_string()))
        }
    }

    // Phase 7: Authenticate socket using SO_PEERCRED (Linux only)
    fn authenticate_socket(&self, socket: &tokio::net::UnixStream) -> Result<(), EmergencyError> {
        #[cfg(target_os = "linux")]
        {
            let fd = socket.as_raw_fd();
            let mut cred: ucred = unsafe { std::mem::zeroed() };
            let mut len = std::mem::size_of::<ucred>() as socklen_t;

            let ret = unsafe {
                getsockopt(
                    fd,
                    SOL_SOCKET,
                    SO_PEERCRED,
                    &mut cred as *mut _ as *mut c_void,
                    &mut len,
                )
            };

            if ret == 0 {
                debug!("Client UID: {}", cred.uid);
                if !self.allowed_users.contains(&(cred.uid as u32)) {
                    return Err(EmergencyError::PermissionDenied);
                }
            } else {
                return Err(EmergencyError::SocketError(
                    "Failed to get peer credentials".to_string(),
                ));
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            // On non-Linux, fallback to allowed_users check if we can get UID via other means
            // For now, skip authentication on non-Linux but log warning
            debug!("SO_PEERCRED only available on Linux, skipping authentication");
        }

        Ok(())
    }

    pub fn parse_command(&self, line: &str) -> Result<EmergencyRequest, EmergencyError> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.is_empty() {
            return Err(EmergencyError::CommandParseError("Empty command".to_string()));
        }

        let command = match parts[0].to_lowercase().as_str() {
            "restart" => EmergencyCommand::RestartModule,
            "status" => EmergencyCommand::Status,
            "failover" => EmergencyCommand::ForceFailover,
            "shutdown" => EmergencyCommand::Shutdown,
            _ => {
                return Err(EmergencyError::CommandParseError(format!(
                    "Unknown command: {}",
                    parts[0]
                )));
            }
        };

        let module_id = if parts.len() > 1 {
            Some(parts[1].to_string())
        } else {
            None
        };

        Ok(EmergencyRequest {
            command,
            module_id,
            user_id: None,
        })
    }

    pub fn check_permission(&self, user_id: u32) -> bool {
        self.allowed_users.contains(&user_id)
    }

    pub fn get_socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    pub fn is_running(&self) -> bool {
        self.listener.is_some()
    }
}

impl Default for HostEmergencyChannel {
    fn default() -> Self {
        Self::new(PathBuf::from("/run/aios/emergency.sock"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emergency_channel_creation() -> anyhow::Result<()> {
        let channel = HostEmergencyChannel::default();
        assert_eq!(channel.get_socket_path(), &PathBuf::from("/run/aios/emergency.sock"));
        Ok(())
    }

    #[test]
    fn test_parse_command() -> anyhow::Result<()> {
        let channel = HostEmergencyChannel::default();
        
        let req = channel.parse_command("restart windows_module")?;
        assert!(matches!(req.command, EmergencyCommand::RestartModule));
        assert_eq!(req.module_id, Some("windows_module".to_string()));
        
        let req = channel.parse_command("status")?;
        assert!(matches!(req.command, EmergencyCommand::Status));
        assert_eq!(req.module_id, None);
        
        Ok(())
    }

    #[test]
    fn test_check_permission() -> anyhow::Result<()> {
        let channel = HostEmergencyChannel::default();
        
        assert!(channel.check_permission(0)); // root
        assert!(!channel.check_permission(1000)); // non-root
        
        let channel = channel.with_allowed_users(vec![0, 1000]);
        assert!(channel.check_permission(1000));
        
        Ok(())
    }
}
