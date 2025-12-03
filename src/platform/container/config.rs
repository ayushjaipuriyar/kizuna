// Container configuration management
//
// Handles environment variables, configuration files, and container-specific settings.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use crate::platform::{PlatformResult, PlatformError};

/// Container configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    pub environment: ContainerEnvironment,
    pub resources: ResourceLimits,
    pub networking: NetworkingConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            environment: ContainerEnvironment::default(),
            resources: ResourceLimits::default(),
            networking: NetworkingConfig::default(),
            storage: StorageConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl ContainerConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> PlatformResult<Self> {
        let mut config = Self::default();

        // Load environment variables
        config.environment = ContainerEnvironment::from_env()?;

        // Load resource limits from environment
        if let Ok(cpu_limit) = std::env::var("KIZUNA_CPU_LIMIT") {
            if let Ok(limit) = cpu_limit.parse() {
                config.resources.cpu_limit = Some(limit);
            }
        }

        if let Ok(memory_limit) = std::env::var("KIZUNA_MEMORY_LIMIT_MB") {
            if let Ok(limit) = memory_limit.parse() {
                config.resources.memory_limit_mb = Some(limit);
            }
        }

        // Load networking config
        if let Ok(port) = std::env::var("KIZUNA_PORT") {
            if let Ok(p) = port.parse() {
                config.networking.listen_port = p;
            }
        }

        // Load logging config
        if let Ok(level) = std::env::var("KIZUNA_LOG_LEVEL") {
            config.logging.level = level;
        }

        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> PlatformResult<()> {
        // Validate resource limits
        if let Some(cpu) = self.resources.cpu_limit {
            if cpu <= 0.0 {
                return Err(PlatformError::ConfigurationError(
                    "CPU limit must be positive".to_string(),
                ));
            }
        }

        if let Some(memory) = self.resources.memory_limit_mb {
            if memory == 0 {
                return Err(PlatformError::ConfigurationError(
                    "Memory limit must be positive".to_string(),
                ));
            }
        }

        // Validate networking
        if self.networking.listen_port == 0 {
            return Err(PlatformError::ConfigurationError(
                "Listen port must be specified".to_string(),
            ));
        }

        Ok(())
    }

    /// Apply configuration to the running system
    pub fn apply(&self) -> PlatformResult<()> {
        // Set environment variables
        for (key, value) in &self.environment.variables {
            std::env::set_var(key, value);
        }

        // Configure logging
        self.logging.apply()?;

        Ok(())
    }
}

/// Container environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerEnvironment {
    pub variables: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
    pub config_files: Vec<ConfigFile>,
}

impl Default for ContainerEnvironment {
    fn default() -> Self {
        Self {
            variables: HashMap::new(),
            secrets: HashMap::new(),
            config_files: vec![],
        }
    }
}

impl ContainerEnvironment {
    /// Load environment from system environment variables
    pub fn from_env() -> PlatformResult<Self> {
        let mut env = Self::default();

        // Load all KIZUNA_* environment variables
        for (key, value) in std::env::vars() {
            if key.starts_with("KIZUNA_") {
                env.variables.insert(key, value);
            }
        }

        Ok(env)
    }

    /// Get environment variable
    pub fn get(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// Set environment variable
    pub fn set(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    /// Get secret (from secrets or environment)
    pub fn get_secret(&self, key: &str) -> Option<&String> {
        self.secrets.get(key)
    }

    /// Set secret
    pub fn set_secret(&mut self, key: String, value: String) {
        self.secrets.insert(key, value);
    }
}

/// Configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub path: PathBuf,
    pub content: String,
    pub format: ConfigFormat,
}

/// Configuration file format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigFormat {
    Json,
    Yaml,
    Toml,
    Env,
}

/// Resource limits for container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_limit: Option<f32>,           // CPU cores (e.g., 1.0, 0.5)
    pub memory_limit_mb: Option<u64>,     // Memory in MB
    pub disk_limit_mb: Option<u64>,       // Disk space in MB
    pub network_bandwidth_mbps: Option<u32>, // Network bandwidth in Mbps
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            cpu_limit: Some(1.0),
            memory_limit_mb: Some(512),
            disk_limit_mb: Some(1024),
            network_bandwidth_mbps: Some(100),
        }
    }
}

impl ResourceLimits {
    /// Check if resource limits are within acceptable ranges
    pub fn is_within_limits(&self, cpu_usage: f32, memory_mb: u64) -> bool {
        if let Some(cpu_limit) = self.cpu_limit {
            if cpu_usage > cpu_limit {
                return false;
            }
        }

        if let Some(memory_limit) = self.memory_limit_mb {
            if memory_mb > memory_limit {
                return false;
            }
        }

        true
    }
}

/// Networking configuration for container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkingConfig {
    pub listen_address: String,
    pub listen_port: u16,
    pub external_port: Option<u16>,
    pub network_mode: NetworkMode,
    pub dns_servers: Vec<String>,
}

impl Default for NetworkingConfig {
    fn default() -> Self {
        Self {
            listen_address: "0.0.0.0".to_string(),
            listen_port: 8080,
            external_port: None,
            network_mode: NetworkMode::Bridge,
            dns_servers: vec![],
        }
    }
}

/// Container network mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkMode {
    Bridge,
    Host,
    None,
    Custom,
}

/// Storage configuration for container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub volumes: Vec<VolumeMount>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("/app/data"),
            temp_dir: PathBuf::from("/tmp"),
            cache_dir: PathBuf::from("/app/cache"),
            volumes: vec![],
        }
    }
}

/// Volume mount configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMount {
    pub source: PathBuf,
    pub target: PathBuf,
    pub read_only: bool,
}

/// Logging configuration for container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: LogFormat,
    pub output: LogOutput,
    pub max_size_mb: u64,
    pub max_files: u32,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            output: LogOutput::Stdout,
            max_size_mb: 100,
            max_files: 5,
        }
    }
}

impl LoggingConfig {
    /// Apply logging configuration
    pub fn apply(&self) -> PlatformResult<()> {
        // Set log level environment variable
        std::env::set_var("RUST_LOG", &self.level);

        Ok(())
    }
}

/// Log format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Text,
    Structured,
}

/// Log output destination
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogOutput {
    Stdout,
    Stderr,
    File(PathBuf),
    Syslog,
}

/// Configuration builder for fluent API
pub struct ContainerConfigBuilder {
    config: ContainerConfig,
}

impl ContainerConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ContainerConfig::default(),
        }
    }

    pub fn with_cpu_limit(mut self, limit: f32) -> Self {
        self.config.resources.cpu_limit = Some(limit);
        self
    }

    pub fn with_memory_limit(mut self, limit_mb: u64) -> Self {
        self.config.resources.memory_limit_mb = Some(limit_mb);
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.config.networking.listen_port = port;
        self
    }

    pub fn with_log_level(mut self, level: String) -> Self {
        self.config.logging.level = level;
        self
    }

    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.config.environment.variables.insert(key, value);
        self
    }

    pub fn build(self) -> PlatformResult<ContainerConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for ContainerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ContainerConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = ContainerConfigBuilder::new()
            .with_cpu_limit(2.0)
            .with_memory_limit(1024)
            .with_port(9090)
            .with_log_level("debug".to_string())
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.resources.cpu_limit, Some(2.0));
        assert_eq!(config.resources.memory_limit_mb, Some(1024));
        assert_eq!(config.networking.listen_port, 9090);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_resource_limits_validation() {
        let limits = ResourceLimits {
            cpu_limit: Some(1.0),
            memory_limit_mb: Some(512),
            disk_limit_mb: None,
            network_bandwidth_mbps: None,
        };

        assert!(limits.is_within_limits(0.5, 256));
        assert!(!limits.is_within_limits(1.5, 256));
        assert!(!limits.is_within_limits(0.5, 1024));
    }

    #[test]
    fn test_environment_variables() {
        let mut env = ContainerEnvironment::default();
        env.set("TEST_VAR".to_string(), "test_value".to_string());
        
        assert_eq!(env.get("TEST_VAR"), Some(&"test_value".to_string()));
        assert_eq!(env.get("NONEXISTENT"), None);
    }

    #[test]
    fn test_invalid_config() {
        let mut config = ContainerConfig::default();
        config.resources.cpu_limit = Some(-1.0);
        
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_secrets_management() {
        let mut env = ContainerEnvironment::default();
        env.set_secret("API_KEY".to_string(), "secret123".to_string());
        
        assert_eq!(env.get_secret("API_KEY"), Some(&"secret123".to_string()));
    }
}
