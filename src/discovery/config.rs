use crate::discovery::api::{DiscoveryConfig, StrategyConfig as ApiStrategyConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// Configuration file format for discovery settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfigFile {
    /// Global discovery settings
    pub discovery: GlobalDiscoveryConfig,
    /// Strategy-specific configurations
    pub strategies: HashMap<String, StrategyConfigFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalDiscoveryConfig {
    /// Enable auto-selection of discovery strategies
    #[serde(default = "default_auto_select")]
    pub auto_select: bool,
    /// Default timeout for discovery operations (in seconds)
    #[serde(default = "default_timeout_secs")]
    pub default_timeout_secs: u64,
    /// Peer cache TTL (in seconds)
    #[serde(default = "default_cache_ttl_secs")]
    pub peer_cache_ttl_secs: u64,
    /// Maximum number of concurrent discovery operations
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_discoveries: usize,
    /// List of enabled strategies
    #[serde(default = "default_enabled_strategies")]
    pub enabled_strategies: Vec<String>,
    /// Enable concurrent discovery across multiple strategies
    #[serde(default)]
    pub concurrent_discovery: bool,
    /// Device name for announcements
    pub device_name: Option<String>,
    /// Default port for services
    pub default_port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfigFile {
    /// Enable this strategy
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Strategy priority (higher = preferred)
    #[serde(default = "default_priority")]
    pub priority: u8,
    /// Strategy-specific timeout override (in seconds)
    pub timeout_secs: Option<u64>,
    /// Strategy-specific parameters
    #[serde(default)]
    pub parameters: HashMap<String, String>,
}

// Default value functions
fn default_auto_select() -> bool { true }
fn default_timeout_secs() -> u64 { 5 }
fn default_cache_ttl_secs() -> u64 { 300 }
fn default_max_concurrent() -> usize { 10 }
fn default_enabled_strategies() -> Vec<String> {
    vec![
        "mdns".to_string(),
        "udp".to_string(),
        "tcp".to_string(),
        "bluetooth".to_string(),
    ]
}
fn default_true() -> bool { true }
fn default_priority() -> u8 { 50 }

impl Default for DiscoveryConfigFile {
    fn default() -> Self {
        let mut strategies = HashMap::new();
        
        // Default mDNS configuration
        strategies.insert("mdns".to_string(), StrategyConfigFile {
            enabled: true,
            priority: 80,
            timeout_secs: None,
            parameters: HashMap::new(),
        });
        
        // Default UDP configuration
        strategies.insert("udp".to_string(), StrategyConfigFile {
            enabled: true,
            priority: 70,
            timeout_secs: None,
            parameters: {
                let mut params = HashMap::new();
                params.insert("broadcast_interval_secs".to_string(), "30".to_string());
                params
            },
        });
        
        // Default TCP configuration
        strategies.insert("tcp".to_string(), StrategyConfigFile {
            enabled: true,
            priority: 60,
            timeout_secs: Some(10),
            parameters: {
                let mut params = HashMap::new();
                params.insert("port_range_start".to_string(), "8000".to_string());
                params.insert("port_range_end".to_string(), "8100".to_string());
                params.insert("max_concurrent_scans".to_string(), "10".to_string());
                params
            },
        });
        
        // Default Bluetooth configuration
        strategies.insert("bluetooth".to_string(), StrategyConfigFile {
            enabled: true,
            priority: 50,
            timeout_secs: Some(15),
            parameters: {
                let mut params = HashMap::new();
                params.insert("service_uuid".to_string(), "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string());
                params
            },
        });

        Self {
            discovery: GlobalDiscoveryConfig {
                auto_select: true,
                default_timeout_secs: 5,
                peer_cache_ttl_secs: 300,
                max_concurrent_discoveries: 10,
                enabled_strategies: default_enabled_strategies(),
                concurrent_discovery: false,
                device_name: None,
                default_port: Some(8080),
            },
            strategies,
        }
    }
}

impl DiscoveryConfigFile {
    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: DiscoveryConfigFile = toml::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Load configuration from JSON file
    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path)?;
        let config: DiscoveryConfigFile = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to JSON file
    pub fn save_to_json<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Convert to the runtime DiscoveryConfig format
    pub fn to_discovery_config(&self) -> DiscoveryConfig {
        let mut strategy_configs = HashMap::new();
        
        for (name, strategy_config) in &self.strategies {
            if strategy_config.enabled {
                let api_config = ApiStrategyConfig {
                    priority: strategy_config.priority,
                    timeout: strategy_config.timeout_secs.map(Duration::from_secs),
                    parameters: strategy_config.parameters.clone(),
                };
                strategy_configs.insert(name.clone(), api_config);
            }
        }

        DiscoveryConfig {
            auto_select: self.discovery.auto_select,
            default_timeout: Duration::from_secs(self.discovery.default_timeout_secs),
            strategy_configs,
            enabled_strategies: self.discovery.enabled_strategies.clone(),
            peer_cache_ttl: Duration::from_secs(self.discovery.peer_cache_ttl_secs),
            max_concurrent_discoveries: self.discovery.max_concurrent_discoveries,
        }
    }

    /// Create from runtime DiscoveryConfig
    pub fn from_discovery_config(config: &DiscoveryConfig) -> Self {
        let mut strategies = HashMap::new();
        
        for (name, strategy_config) in &config.strategy_configs {
            let file_config = StrategyConfigFile {
                enabled: true,
                priority: strategy_config.priority,
                timeout_secs: strategy_config.timeout.map(|d| d.as_secs()),
                parameters: strategy_config.parameters.clone(),
            };
            strategies.insert(name.clone(), file_config);
        }

        Self {
            discovery: GlobalDiscoveryConfig {
                auto_select: config.auto_select,
                default_timeout_secs: config.default_timeout.as_secs(),
                peer_cache_ttl_secs: config.peer_cache_ttl.as_secs(),
                max_concurrent_discoveries: config.max_concurrent_discoveries,
                enabled_strategies: config.enabled_strategies.clone(),
                concurrent_discovery: false, // This would need to be tracked separately
                device_name: None,
                default_port: None,
            },
            strategies,
        }
    }

    /// Generate a sample configuration file content
    pub fn generate_sample_config() -> String {
        let sample = Self::default();
        toml::to_string_pretty(&sample).unwrap_or_else(|_| {
            "# Failed to generate sample configuration".to_string()
        })
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate global settings
        if self.discovery.default_timeout_secs == 0 {
            errors.push("Default timeout must be greater than 0".to_string());
        }

        if self.discovery.peer_cache_ttl_secs == 0 {
            errors.push("Peer cache TTL must be greater than 0".to_string());
        }

        if self.discovery.max_concurrent_discoveries == 0 {
            errors.push("Max concurrent discoveries must be greater than 0".to_string());
        }

        if self.discovery.enabled_strategies.is_empty() {
            errors.push("At least one strategy must be enabled".to_string());
        }

        // Validate strategy configurations
        for (name, strategy) in &self.strategies {
            if strategy.enabled && !self.discovery.enabled_strategies.contains(name) {
                errors.push(format!("Strategy '{}' is enabled but not in enabled_strategies list", name));
            }

            if let Some(timeout) = strategy.timeout_secs {
                if timeout == 0 {
                    errors.push(format!("Strategy '{}' timeout must be greater than 0", name));
                }
            }
        }

        // Validate strategy-specific parameters
        for (name, strategy) in &self.strategies {
            match name.as_str() {
                "udp" => {
                    if let Some(interval) = strategy.parameters.get("broadcast_interval_secs") {
                        if interval.parse::<u64>().is_err() {
                            errors.push(format!("UDP broadcast_interval_secs must be a valid number"));
                        }
                    }
                }
                "tcp" => {
                    if let Some(start) = strategy.parameters.get("port_range_start") {
                        if start.parse::<u16>().is_err() {
                            errors.push(format!("TCP port_range_start must be a valid port number"));
                        }
                    }
                    if let Some(end) = strategy.parameters.get("port_range_end") {
                        if end.parse::<u16>().is_err() {
                            errors.push(format!("TCP port_range_end must be a valid port number"));
                        }
                    }
                }
                "bluetooth" => {
                    if let Some(uuid) = strategy.parameters.get("service_uuid") {
                        if uuid::Uuid::parse_str(uuid).is_err() {
                            errors.push(format!("Bluetooth service_uuid must be a valid UUID"));
                        }
                    }
                }
                _ => {} // Other strategies don't have specific validation yet
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Merge with another configuration (other takes precedence)
    pub fn merge(&mut self, other: DiscoveryConfigFile) {
        // Merge global settings
        self.discovery = other.discovery;
        
        // Merge strategy configurations
        for (name, strategy) in other.strategies {
            self.strategies.insert(name, strategy);
        }
    }

    /// Get the default configuration file paths
    pub fn default_config_paths() -> Vec<std::path::PathBuf> {
        let mut paths = Vec::new();
        
        // Current directory
        paths.push("kizuna-discovery.toml".into());
        paths.push("discovery.toml".into());
        
        // User config directory
        if let Some(config_dir) = dirs::config_dir() {
            paths.push(config_dir.join("kizuna").join("discovery.toml"));
        }
        
        // System config directory
        paths.push("/etc/kizuna/discovery.toml".into());
        
        paths
    }

    /// Try to load configuration from default locations
    pub fn load_from_default_locations() -> Result<Self, Box<dyn std::error::Error>> {
        for path in Self::default_config_paths() {
            if path.exists() {
                println!("Loading discovery configuration from: {}", path.display());
                return Self::load_from_file(&path);
            }
        }
        
        // No configuration file found, use defaults
        println!("No discovery configuration file found, using defaults");
        Ok(Self::default())
    }
}

/// Configuration management utilities
pub struct ConfigManager;

impl ConfigManager {
    /// Initialize configuration in the default location
    pub fn init_config(force: bool) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = std::path::Path::new("kizuna-discovery.toml");
        
        if config_path.exists() && !force {
            return Err("Configuration file already exists. Use --force to overwrite.".into());
        }
        
        let default_config = DiscoveryConfigFile::default();
        default_config.save_to_file(config_path)?;
        
        println!("Created default discovery configuration at: {}", config_path.display());
        Ok(())
    }

    /// Validate configuration file
    pub fn validate_config<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
        let config = DiscoveryConfigFile::load_from_file(path)?;
        
        match config.validate() {
            Ok(()) => {
                println!("Configuration is valid");
                Ok(())
            }
            Err(errors) => {
                eprintln!("Configuration validation failed:");
                for error in errors {
                    eprintln!("  - {}", error);
                }
                Err("Configuration validation failed".into())
            }
        }
    }

    /// Show current configuration
    pub fn show_config<P: AsRef<Path>>(path: P) -> Result<(), Box<dyn std::error::Error>> {
        let config = DiscoveryConfigFile::load_from_file(path)?;
        let content = toml::to_string_pretty(&config)?;
        println!("{}", content);
        Ok(())
    }

    /// Generate sample configuration
    pub fn generate_sample() -> String {
        DiscoveryConfigFile::generate_sample_config()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_default_config() {
        let config = DiscoveryConfigFile::default();
        assert!(config.discovery.auto_select);
        assert_eq!(config.discovery.default_timeout_secs, 5);
        assert!(!config.strategies.is_empty());
        
        // Validate default config
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_serialization() {
        let config = DiscoveryConfigFile::default();
        
        // Test TOML serialization
        let toml_str = toml::to_string(&config).unwrap();
        let parsed_config: DiscoveryConfigFile = toml::from_str(&toml_str).unwrap();
        assert_eq!(config.discovery.auto_select, parsed_config.discovery.auto_select);
        
        // Test JSON serialization
        let json_str = serde_json::to_string(&config).unwrap();
        let parsed_config: DiscoveryConfigFile = serde_json::from_str(&json_str).unwrap();
        assert_eq!(config.discovery.auto_select, parsed_config.discovery.auto_select);
    }

    #[test]
    fn test_config_file_operations() {
        let config = DiscoveryConfigFile::default();
        
        // Test TOML file operations
        let toml_file = NamedTempFile::new().unwrap();
        config.save_to_file(toml_file.path()).unwrap();
        let loaded_config = DiscoveryConfigFile::load_from_file(toml_file.path()).unwrap();
        assert_eq!(config.discovery.auto_select, loaded_config.discovery.auto_select);
        
        // Test JSON file operations
        let json_file = NamedTempFile::new().unwrap();
        config.save_to_json(json_file.path()).unwrap();
        let loaded_config = DiscoveryConfigFile::load_from_json(json_file.path()).unwrap();
        assert_eq!(config.discovery.auto_select, loaded_config.discovery.auto_select);
    }

    #[test]
    fn test_config_conversion() {
        let file_config = DiscoveryConfigFile::default();
        let runtime_config = file_config.to_discovery_config();
        let converted_back = DiscoveryConfigFile::from_discovery_config(&runtime_config);
        
        assert_eq!(file_config.discovery.auto_select, converted_back.discovery.auto_select);
        assert_eq!(file_config.discovery.default_timeout_secs, converted_back.discovery.default_timeout_secs);
    }

    #[test]
    fn test_config_validation() {
        let mut config = DiscoveryConfigFile::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid timeout should fail
        config.discovery.default_timeout_secs = 0;
        assert!(config.validate().is_err());
        
        // Fix timeout and test empty strategies
        config.discovery.default_timeout_secs = 5;
        config.discovery.enabled_strategies.clear();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_merge() {
        let mut config1 = DiscoveryConfigFile::default();
        let mut config2 = DiscoveryConfigFile::default();
        
        config2.discovery.auto_select = false;
        config2.discovery.default_timeout_secs = 10;
        
        config1.merge(config2);
        
        assert!(!config1.discovery.auto_select);
        assert_eq!(config1.discovery.default_timeout_secs, 10);
    }

    #[test]
    fn test_sample_config_generation() {
        let sample = DiscoveryConfigFile::generate_sample_config();
        assert!(!sample.is_empty());
        assert!(sample.contains("[discovery]"));
        assert!(sample.contains("[strategies"));
    }
}