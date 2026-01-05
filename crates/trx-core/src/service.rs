//! Service management for trx-api
//!
//! Provides start/stop/status functionality for the trx-api daemon.

use crate::Result;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use sysinfo::{Pid, System};

/// Service manager for trx-api
pub struct ServiceManager {
    state_dir: PathBuf,
}

/// Service status
#[derive(Debug, Clone)]
pub enum ServiceStatus {
    Running { pid: u32, port: Option<u16> },
    Stopped,
    Dead, // PID file exists but process not running
}

impl ServiceManager {
    /// Create a new service manager
    pub fn new() -> Result<Self> {
        let state_dir = get_state_dir()?;
        std::fs::create_dir_all(&state_dir)?;
        Ok(Self { state_dir })
    }

    /// Path to the PID file
    pub fn pid_file(&self) -> PathBuf {
        self.state_dir.join("trx-api.pid")
    }

    /// Path to the port file
    pub fn port_file(&self) -> PathBuf {
        self.state_dir.join("trx-api.port")
    }

    /// Check if the service is running
    pub fn is_running(&self) -> bool {
        if let Ok(pid) = self.read_pid() {
            process_exists(pid)
        } else {
            false
        }
    }

    /// Read the PID from the PID file
    pub fn read_pid(&self) -> Result<u32> {
        let content = std::fs::read_to_string(self.pid_file())?;
        content
            .trim()
            .parse()
            .map_err(|e| crate::Error::Service(format!("Invalid PID: {e}")))
    }

    /// Read the port from the port file
    pub fn read_port(&self) -> Result<u16> {
        let content = std::fs::read_to_string(self.port_file())?;
        content
            .trim()
            .parse()
            .map_err(|e| crate::Error::Service(format!("Invalid port: {e}")))
    }

    /// Start the service
    ///
    /// If `foreground` is true, runs in foreground (blocking).
    /// Otherwise, spawns as a background daemon.
    pub fn start(&self, foreground: bool, workdir: Option<&PathBuf>) -> Result<()> {
        if self.is_running() {
            return Err(crate::Error::Service("Service already running".into()));
        }

        let exe = std::env::current_exe()?;
        let service_exe = exe
            .parent()
            .ok_or_else(|| crate::Error::Service("Cannot find service binary".into()))?
            .join("trx-api");

        if !service_exe.exists() {
            return Err(crate::Error::Service(
                "trx-api binary not found. Please install it first.".into(),
            ));
        }

        let mut cmd = Command::new(&service_exe);

        // Pass workdir if specified
        if let Some(dir) = workdir {
            cmd.current_dir(dir);
        }

        if foreground {
            let status = cmd.status()?;
            if !status.success() {
                return Err(crate::Error::Service("Service failed to start".into()));
            }
        } else {
            // Start in background
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                cmd.stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .process_group(0) // Create new process group
                    .spawn()?;
            }

            #[cfg(not(unix))]
            {
                cmd.stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()?;
            }

            // Wait for service to start
            let mut attempts = 0;
            while attempts < 20 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                if self.is_running() {
                    break;
                }
                attempts += 1;
            }

            if !self.is_running() {
                return Err(crate::Error::Service("Service failed to start".into()));
            }
        }

        Ok(())
    }

    /// Stop the service
    pub fn stop(&self) -> Result<()> {
        let pid = self.read_pid()?;

        if !process_exists(pid) {
            // Cleanup stale PID file
            std::fs::remove_file(self.pid_file()).ok();
            return Err(crate::Error::Service("Service not running".into()));
        }

        // Send termination signal
        #[cfg(unix)]
        {
            Command::new("kill").arg(pid.to_string()).status()?;
        }

        #[cfg(windows)]
        {
            Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .status()?;
        }

        // Wait for process to exit
        for _ in 0..50 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            if !process_exists(pid) {
                break;
            }
        }

        // Cleanup files
        std::fs::remove_file(self.pid_file()).ok();
        std::fs::remove_file(self.port_file()).ok();

        Ok(())
    }

    /// Restart the service
    pub fn restart(&self, workdir: Option<&PathBuf>) -> Result<()> {
        if self.is_running() {
            self.stop()?;
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        self.start(false, workdir)
    }

    /// Get the service status
    pub fn status(&self) -> ServiceStatus {
        if let Ok(pid) = self.read_pid() {
            if process_exists(pid) {
                let port = self.read_port().ok();
                ServiceStatus::Running { pid, port }
            } else {
                ServiceStatus::Dead
            }
        } else {
            ServiceStatus::Stopped
        }
    }

    /// Write PID file (called by trx-api on startup)
    pub fn write_pid(&self, pid: u32) -> Result<()> {
        std::fs::write(self.pid_file(), pid.to_string())?;
        Ok(())
    }

    /// Write port file (called by trx-api on startup)
    pub fn write_port(&self, port: u16) -> Result<()> {
        std::fs::write(self.port_file(), port.to_string())?;
        Ok(())
    }

    /// Cleanup PID and port files (called by trx-api on shutdown)
    pub fn cleanup(&self) {
        std::fs::remove_file(self.pid_file()).ok();
        std::fs::remove_file(self.port_file()).ok();
    }
}

fn process_exists(pid: u32) -> bool {
    let mut sys = System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All, false);
    sys.process(Pid::from_u32(pid)).is_some()
}

fn get_state_dir() -> Result<PathBuf> {
    let base = std::env::var("XDG_STATE_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|home| home.join(".local").join("state")))
        .ok_or_else(|| crate::Error::Service("Could not determine state directory".into()))?;

    Ok(base.join("trx"))
}
