// Linux systemd integration

use crate::platform::{PlatformResult, PlatformError};
use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use std::env;

/// Systemd service configuration
#[derive(Debug, Clone)]
pub struct SystemdServiceConfig {
    pub service_name: String,
    pub description: String,
    pub exec_start: String,
    pub working_directory: Option<String>,
    pub user: Option<String>,
    pub restart: RestartPolicy,
    pub wanted_by: Vec<String>,
    pub after: Vec<String>,
    pub environment: Vec<(String, String)>,
}

/// Systemd restart policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestartPolicy {
    No,
    Always,
    OnFailure,
    OnAbnormal,
    OnAbort,
    OnWatchdog,
}

impl RestartPolicy {
    fn as_str(&self) -> &str {
        match self {
            RestartPolicy::No => "no",
            RestartPolicy::Always => "always",
            RestartPolicy::OnFailure => "on-failure",
            RestartPolicy::OnAbnormal => "on-abnormal",
            RestartPolicy::OnAbort => "on-abort",
            RestartPolicy::OnWatchdog => "on-watchdog",
        }
    }
}

impl Default for SystemdServiceConfig {
    fn default() -> Self {
        Self {
            service_name: "kizuna".to_string(),
            description: "Kizuna cross-platform connectivity service".to_string(),
            exec_start: "/usr/bin/kizuna".to_string(),
            working_directory: None,
            user: None,
            restart: RestartPolicy::OnFailure,
            wanted_by: vec!["multi-user.target".to_string()],
            after: vec!["network.target".to_string()],
            environment: Vec::new(),
        }
    }
}

/// Systemd service manager
pub struct SystemdManager {
    config: SystemdServiceConfig,
}

impl SystemdManager {
    pub fn new(config: SystemdServiceConfig) -> Self {
        Self {
            config,
        }
    }

    /// Generate systemd service unit file content
    fn generate_service_unit(&self) -> String {
        let mut unit = String::new();

        // [Unit] section
        unit.push_str("[Unit]\n");
        unit.push_str(&format!("Description={}\n", self.config.description));
        
        if !self.config.after.is_empty() {
            unit.push_str(&format!("After={}\n", self.config.after.join(" ")));
        }
        unit.push_str("\n");

        // [Service] section
        unit.push_str("[Service]\n");
        unit.push_str("Type=simple\n");
        unit.push_str(&format!("ExecStart={}\n", self.config.exec_start));
        
        if let Some(ref working_dir) = self.config.working_directory {
            unit.push_str(&format!("WorkingDirectory={}\n", working_dir));
        }
        
        if let Some(ref user) = self.config.user {
            unit.push_str(&format!("User={}\n", user));
        }
        
        unit.push_str(&format!("Restart={}\n", self.config.restart.as_str()));
        unit.push_str("RestartSec=5\n");
        
        // Environment variables
        for (key, value) in &self.config.environment {
            unit.push_str(&format!("Environment=\"{}={}\"\n", key, value));
        }
        
        unit.push_str("\n");

        // [Install] section
        unit.push_str("[Install]\n");
        if !self.config.wanted_by.is_empty() {
            unit.push_str(&format!("WantedBy={}\n", self.config.wanted_by.join(" ")));
        }

        unit
    }

    /// Get systemd service file path for user service
    fn get_user_service_path(&self) -> PlatformResult<PathBuf> {
        let home = env::var("HOME")
            .map_err(|_| PlatformError::IntegrationError("HOME not set".to_string()))?;
        
        let service_dir = Path::new(&home).join(".config/systemd/user");
        Ok(service_dir.join(format!("{}.service", self.config.service_name)))
    }

    /// Get systemd service file path for system service
    fn get_system_service_path(&self) -> PathBuf {
        PathBuf::from("/etc/systemd/system")
            .join(format!("{}.service", self.config.service_name))
    }

    /// Install service as user service
    pub fn install_user_service(&self) -> PlatformResult<PathBuf> {
        let service_path = self.get_user_service_path()?;
        
        // Create directory if it doesn't exist
        if let Some(parent) = service_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Write service file
        let service_content = self.generate_service_unit();
        let mut service_file = fs::File::create(&service_path)?;
        service_file.write_all(service_content.as_bytes())?;

        Ok(service_path)
    }

    /// Install service as system service (requires root)
    pub fn install_system_service(&self) -> PlatformResult<PathBuf> {
        let service_path = self.get_system_service_path();
        
        // Write service file
        let service_content = self.generate_service_unit();
        let mut service_file = fs::File::create(&service_path)?;
        service_file.write_all(service_content.as_bytes())?;

        Ok(service_path)
    }

    /// Enable the service (user or system)
    pub fn enable_service(&self, user: bool) -> PlatformResult<()> {
        let mut cmd = std::process::Command::new("systemctl");
        
        if user {
            cmd.arg("--user");
        }
        
        cmd.arg("enable");
        cmd.arg(&self.config.service_name);

        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PlatformError::IntegrationError(
                format!("Failed to enable service: {}", stderr)
            ));
        }

        Ok(())
    }

    /// Start the service
    pub fn start_service(&self, user: bool) -> PlatformResult<()> {
        let mut cmd = std::process::Command::new("systemctl");
        
        if user {
            cmd.arg("--user");
        }
        
        cmd.arg("start");
        cmd.arg(&self.config.service_name);

        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PlatformError::IntegrationError(
                format!("Failed to start service: {}", stderr)
            ));
        }

        Ok(())
    }

    /// Stop the service
    pub fn stop_service(&self, user: bool) -> PlatformResult<()> {
        let mut cmd = std::process::Command::new("systemctl");
        
        if user {
            cmd.arg("--user");
        }
        
        cmd.arg("stop");
        cmd.arg(&self.config.service_name);

        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PlatformError::IntegrationError(
                format!("Failed to stop service: {}", stderr)
            ));
        }

        Ok(())
    }

    /// Check service status
    pub fn service_status(&self, user: bool) -> PlatformResult<ServiceStatus> {
        let mut cmd = std::process::Command::new("systemctl");
        
        if user {
            cmd.arg("--user");
        }
        
        cmd.arg("is-active");
        cmd.arg(&self.config.service_name);

        let output = cmd.output()?;
        let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();

        let status = match status_str.as_str() {
            "active" => ServiceStatus::Active,
            "inactive" => ServiceStatus::Inactive,
            "failed" => ServiceStatus::Failed,
            "activating" => ServiceStatus::Activating,
            "deactivating" => ServiceStatus::Deactivating,
            _ => ServiceStatus::Unknown,
        };

        Ok(status)
    }

    /// Reload systemd daemon
    pub fn reload_daemon(&self, user: bool) -> PlatformResult<()> {
        let mut cmd = std::process::Command::new("systemctl");
        
        if user {
            cmd.arg("--user");
        }
        
        cmd.arg("daemon-reload");

        let output = cmd.output()?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(PlatformError::IntegrationError(
                format!("Failed to reload daemon: {}", stderr)
            ));
        }

        Ok(())
    }
}

/// Service status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    Active,
    Inactive,
    Failed,
    Activating,
    Deactivating,
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_service_unit() {
        let config = SystemdServiceConfig::default();
        let manager = SystemdManager::new(config);
        let unit = manager.generate_service_unit();

        assert!(unit.contains("[Unit]"));
        assert!(unit.contains("[Service]"));
        assert!(unit.contains("[Install]"));
        assert!(unit.contains("Description="));
        assert!(unit.contains("ExecStart="));
    }

    #[test]
    fn test_service_unit_with_environment() {
        let mut config = SystemdServiceConfig::default();
        config.environment.push(("RUST_LOG".to_string(), "info".to_string()));
        
        let manager = SystemdManager::new(config);
        let unit = manager.generate_service_unit();

        assert!(unit.contains("Environment=\"RUST_LOG=info\""));
    }
}
