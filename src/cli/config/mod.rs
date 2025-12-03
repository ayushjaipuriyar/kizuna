// Configuration management module

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::types::{CLIConfig, ConfigProfile, OutputFormat, ColorMode};
use async_trait::async_trait;
use std::path::PathBuf;

/// Configuration manager trait
#[async_trait]
pub trait ConfigurationManager {
    /// Load configuration from file
    async fn load_config(&self, path: Option<PathBuf>) -> CLIResult<CLIConfig>;

    /// Save configuration to file
    async fn save_config(&self, config: CLIConfig, path: Option<PathBuf>) -> CLIResult<()>;

    /// Validate configuration
    async fn validate_config(&self, config: &CLIConfig) -> CLIResult<ValidationResult>;

    /// Get a specific profile
    async fn get_profile(&self, name: String) -> CLIResult<ConfigProfile>;

    /// Merge command-line arguments with configuration
    async fn merge_args_with_config(
        &self,
        args: ParsedArgs,
        config: CLIConfig,
    ) -> CLIResult<MergedConfig>;
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

impl ValidationResult {
    /// Create a new validation result
    pub fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: impl Into<String>) {
        self.errors.push(error.into());
        self.valid = false;
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Add a suggestion
    pub fn add_suggestion(&mut self, suggestion: impl Into<String>) {
        self.suggestions.push(suggestion.into());
    }

    /// Check if validation passed
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Check if there are any issues (errors or warnings)
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Parsed command-line arguments
#[derive(Debug, Clone, Default)]
pub struct ParsedArgs {
    pub output_format: Option<String>,
    pub color_mode: Option<String>,
    pub config_file: Option<PathBuf>,
    pub profile: Option<String>,
    pub default_peer: Option<String>,
    pub compression: Option<bool>,
    pub encryption: Option<bool>,
}

/// Merged configuration
#[derive(Debug, Clone)]
pub struct MergedConfig {
    pub config: CLIConfig,
    pub overrides: Vec<String>,
}

/// TOML configuration parser
pub struct TOMLConfigParser {
    config_path: PathBuf,
}

impl TOMLConfigParser {
    /// Create a new TOML configuration parser
    pub fn new(config_path: Option<PathBuf>) -> CLIResult<Self> {
        let path = config_path.unwrap_or_else(|| default_config_path().unwrap());
        Ok(Self { config_path: path })
    }

    /// Parse configuration from TOML string
    pub fn parse_toml(&self, content: &str) -> CLIResult<CLIConfig> {
        toml::from_str(content)
            .map_err(|e| CLIError::config(format!("Failed to parse TOML: {}", e)))
    }

    /// Serialize configuration to TOML string
    pub fn serialize_toml(&self, config: &CLIConfig) -> CLIResult<String> {
        toml::to_string_pretty(config)
            .map_err(|e| CLIError::config(format!("Failed to serialize to TOML: {}", e)))
    }

    /// Load configuration from file
    pub async fn load(&self) -> CLIResult<CLIConfig> {
        if !self.config_path.exists() {
            return Err(CLIError::config(format!(
                "Configuration file not found: {}",
                self.config_path.display()
            )));
        }

        let content = tokio::fs::read_to_string(&self.config_path)
            .await
            .map_err(|e| CLIError::config(format!("Failed to read config file: {}", e)))?;

        self.parse_toml(&content)
    }

    /// Save configuration to file
    pub async fn save(&self, config: &CLIConfig) -> CLIResult<()> {
        // Ensure directory exists
        if let Some(parent) = self.config_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| CLIError::config(format!("Failed to create config directory: {}", e)))?;
        }

        let content = self.serialize_toml(config)?;
        
        tokio::fs::write(&self.config_path, content)
            .await
            .map_err(|e| CLIError::config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self, config: &CLIConfig) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Validate output format
        match config.output_format {
            OutputFormat::Table | OutputFormat::JSON | OutputFormat::CSV | OutputFormat::Minimal => {}
        }

        // Validate color mode
        match config.color_mode {
            ColorMode::Auto | ColorMode::Always | ColorMode::Never => {}
        }

        // Validate transfer settings
        if let Some(ref path) = config.transfer_settings.default_download_path {
            if !path.exists() {
                result.add_warning(format!(
                    "Default download path does not exist: {}",
                    path.display()
                ));
                result.add_suggestion(format!(
                    "Create the directory or update the path in your configuration"
                ));
            }
        }

        // Validate stream settings
        let valid_qualities = ["low", "medium", "high", "ultra"];
        if !valid_qualities.contains(&config.stream_settings.default_quality.as_str()) {
            result.add_error(format!(
                "Invalid stream quality '{}'. Valid options: {}",
                config.stream_settings.default_quality,
                valid_qualities.join(", ")
            ));
            result.add_suggestion("Set default_quality to one of: low, medium, high, ultra".to_string());
        }

        if let Some(ref path) = config.stream_settings.recording_path {
            if !path.exists() {
                result.add_warning(format!(
                    "Recording path does not exist: {}",
                    path.display()
                ));
                result.add_suggestion("Create the directory or update the path in your configuration".to_string());
            }
        }

        // Validate profiles
        for (name, profile) in &config.profiles {
            if profile.name != *name {
                result.add_warning(format!(
                    "Profile key '{}' does not match profile name '{}'",
                    name, profile.name
                ));
            }
        }

        result
    }

    /// Create default configuration
    pub fn create_default() -> CLIConfig {
        CLIConfig::default()
    }

    /// Generate default configuration file with comments
    pub fn generate_default_with_comments() -> String {
        r#"# Kizuna CLI Configuration
# This file configures the behavior of the Kizuna command-line interface

# Default peer to connect to (optional)
# default_peer = "my-laptop"

# Output format for command results
# Options: table, json, csv, minimal
output_format = "table"

# Color mode for terminal output
# Options: auto, always, never
color_mode = "auto"

# File transfer settings
[transfer_settings]
# Enable compression for file transfers
compression = true

# Enable encryption for file transfers
encryption = true

# Default download directory (optional)
# default_download_path = "/home/user/Downloads"

# Auto-accept transfers from trusted peers
auto_accept_trusted = false

# Streaming settings
[stream_settings]
# Default streaming quality
# Options: low, medium, high, ultra
default_quality = "medium"

# Automatically record streams
auto_record = false

# Directory for stream recordings (optional)
# recording_path = "/home/user/Videos/kizuna"

# Configuration profiles
# Profiles allow you to define different configurations for different use cases
# [profiles.work]
# name = "work"
# description = "Work environment settings"
# settings = { compression = false, encryption = true }
"#.to_string()
    }

    /// Migrate old configuration to new format
    pub async fn migrate(&self, old_config: CLIConfig) -> CLIResult<CLIConfig> {
        // For now, just return the config as-is
        // In the future, this could handle version migrations
        Ok(old_config)
    }
}

/// Get default configuration file path
pub fn default_config_path() -> CLIResult<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| CLIError::config("Could not determine config directory"))?;
    
    Ok(config_dir.join("kizuna").join("config.toml"))
}

/// Ensure configuration directory exists
pub fn ensure_config_dir() -> CLIResult<PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| CLIError::config("Could not determine config directory"))?
        .join("kizuna");
    
    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .map_err(|e| CLIError::config(format!("Failed to create config directory: {}", e)))?;
    }
    
    Ok(config_dir)
}

/// Load configuration from default location or create default
pub async fn load_or_create_config() -> CLIResult<CLIConfig> {
    let parser = TOMLConfigParser::new(None)?;
    
    if parser.config_path.exists() {
        let config = parser.load().await?;
        
        // Validate the loaded configuration
        let validation = parser.validate(&config);
        if !validation.is_valid() {
            return Err(CLIError::config(format!(
                "Configuration validation failed:\n{}",
                validation.errors.join("\n")
            )));
        }
        
        // Print warnings if any
        if !validation.warnings.is_empty() {
            eprintln!("Configuration warnings:");
            for warning in &validation.warnings {
                eprintln!("  - {}", warning);
            }
        }
        
        Ok(config)
    } else {
        // Create default configuration
        let config = TOMLConfigParser::create_default();
        
        // Ensure directory exists
        ensure_config_dir()?;
        
        // Save default config with comments
        let content = TOMLConfigParser::generate_default_with_comments();
        tokio::fs::write(&parser.config_path, content)
            .await
            .map_err(|e| CLIError::config(format!("Failed to write config file: {}", e)))?;
        
        Ok(config)
    }
}

/// Load configuration from specific path
pub async fn load_config_from_path(path: PathBuf) -> CLIResult<CLIConfig> {
    let parser = TOMLConfigParser::new(Some(path))?;
    let config = parser.load().await?;
    
    // Validate the loaded configuration
    let validation = parser.validate(&config);
    if !validation.is_valid() {
        return Err(CLIError::config(format!(
            "Configuration validation failed:\n{}",
            validation.errors.join("\n")
        )));
    }
    
    Ok(config)
}

/// Save configuration to default location
pub async fn save_config(config: &CLIConfig) -> CLIResult<()> {
    let parser = TOMLConfigParser::new(None)?;
    
    // Validate before saving
    let validation = parser.validate(config);
    if !validation.is_valid() {
        return Err(CLIError::config(format!(
            "Cannot save invalid configuration:\n{}",
            validation.errors.join("\n")
        )));
    }
    
    parser.save(config).await
}

/// Save configuration to specific path
pub async fn save_config_to_path(config: &CLIConfig, path: PathBuf) -> CLIResult<()> {
    let parser = TOMLConfigParser::new(Some(path))?;
    
    // Validate before saving
    let validation = parser.validate(config);
    if !validation.is_valid() {
        return Err(CLIError::config(format!(
            "Cannot save invalid configuration:\n{}",
            validation.errors.join("\n")
        )));
    }
    
    parser.save(config).await
}

/// Profile manager for handling multiple configuration profiles
pub struct ProfileManager {
    config: CLIConfig,
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new(config: CLIConfig) -> Self {
        Self { config }
    }

    /// Get a profile by name
    pub fn get_profile(&self, name: &str) -> CLIResult<&ConfigProfile> {
        self.config
            .profiles
            .get(name)
            .ok_or_else(|| CLIError::config(format!("Profile '{}' not found", name)))
    }

    /// Add a new profile
    pub fn add_profile(&mut self, profile: ConfigProfile) -> CLIResult<()> {
        let name = profile.name.clone();
        
        if self.config.profiles.contains_key(&name) {
            return Err(CLIError::config(format!(
                "Profile '{}' already exists",
                name
            )));
        }
        
        self.config.profiles.insert(name, profile);
        Ok(())
    }

    /// Update an existing profile
    pub fn update_profile(&mut self, profile: ConfigProfile) -> CLIResult<()> {
        let name = profile.name.clone();
        
        if !self.config.profiles.contains_key(&name) {
            return Err(CLIError::config(format!(
                "Profile '{}' does not exist",
                name
            )));
        }
        
        self.config.profiles.insert(name, profile);
        Ok(())
    }

    /// Remove a profile
    pub fn remove_profile(&mut self, name: &str) -> CLIResult<ConfigProfile> {
        self.config
            .profiles
            .remove(name)
            .ok_or_else(|| CLIError::config(format!("Profile '{}' not found", name)))
    }

    /// List all profile names
    pub fn list_profiles(&self) -> Vec<String> {
        self.config.profiles.keys().cloned().collect()
    }

    /// Apply a profile to the base configuration
    pub fn apply_profile(&self, profile_name: &str) -> CLIResult<CLIConfig> {
        let profile = self.get_profile(profile_name)?;
        let mut config = self.config.clone();
        
        // Apply profile settings to config
        for (key, value) in &profile.settings {
            match key.as_str() {
                "output_format" => {
                    if let Some(format_str) = value.as_str() {
                        config.output_format = parse_output_format(format_str)?;
                    }
                }
                "color_mode" => {
                    if let Some(mode_str) = value.as_str() {
                        config.color_mode = parse_color_mode(mode_str)?;
                    }
                }
                "default_peer" => {
                    config.default_peer = value.as_str().map(|s| s.to_string());
                }
                "compression" => {
                    if let Some(val) = value.as_bool() {
                        config.transfer_settings.compression = val;
                    }
                }
                "encryption" => {
                    if let Some(val) = value.as_bool() {
                        config.transfer_settings.encryption = val;
                    }
                }
                "auto_accept_trusted" => {
                    if let Some(val) = value.as_bool() {
                        config.transfer_settings.auto_accept_trusted = val;
                    }
                }
                "default_quality" => {
                    if let Some(quality) = value.as_str() {
                        config.stream_settings.default_quality = quality.to_string();
                    }
                }
                "auto_record" => {
                    if let Some(val) = value.as_bool() {
                        config.stream_settings.auto_record = val;
                    }
                }
                _ => {
                    // Unknown setting, ignore
                }
            }
        }
        
        Ok(config)
    }

    /// Validate a profile
    pub fn validate_profile(&self, profile: &ConfigProfile) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // Check profile name is not empty
        if profile.name.is_empty() {
            result.add_error("Profile name cannot be empty");
        }
        
        // Validate settings
        for (key, value) in &profile.settings {
            match key.as_str() {
                "output_format" => {
                    if let Some(format_str) = value.as_str() {
                        if parse_output_format(format_str).is_err() {
                            result.add_error(format!(
                                "Invalid output_format '{}'. Valid options: table, json, csv, minimal",
                                format_str
                            ));
                        }
                    } else {
                        result.add_error("output_format must be a string");
                    }
                }
                "color_mode" => {
                    if let Some(mode_str) = value.as_str() {
                        if parse_color_mode(mode_str).is_err() {
                            result.add_error(format!(
                                "Invalid color_mode '{}'. Valid options: auto, always, never",
                                mode_str
                            ));
                        }
                    } else {
                        result.add_error("color_mode must be a string");
                    }
                }
                "default_peer" => {
                    if !value.is_string() {
                        result.add_error("default_peer must be a string");
                    }
                }
                "compression" | "encryption" | "auto_accept_trusted" | "auto_record" => {
                    if !value.is_boolean() {
                        result.add_error(format!("{} must be a boolean", key));
                    }
                }
                "default_quality" => {
                    if let Some(quality) = value.as_str() {
                        let valid_qualities = ["low", "medium", "high", "ultra"];
                        if !valid_qualities.contains(&quality) {
                            result.add_error(format!(
                                "Invalid default_quality '{}'. Valid options: {}",
                                quality,
                                valid_qualities.join(", ")
                            ));
                        }
                    } else {
                        result.add_error("default_quality must be a string");
                    }
                }
                _ => {
                    result.add_warning(format!("Unknown setting '{}'", key));
                }
            }
        }
        
        result
    }

    /// Create a profile from current configuration
    pub fn create_profile_from_config(
        &self,
        name: String,
        description: String,
    ) -> ConfigProfile {
        let mut settings = std::collections::HashMap::new();
        
        // Add current settings to profile
        settings.insert(
            "output_format".to_string(),
            serde_json::Value::String(format!("{:?}", self.config.output_format).to_lowercase()),
        );
        settings.insert(
            "color_mode".to_string(),
            serde_json::Value::String(format!("{:?}", self.config.color_mode).to_lowercase()),
        );
        
        if let Some(ref peer) = self.config.default_peer {
            settings.insert(
                "default_peer".to_string(),
                serde_json::Value::String(peer.clone()),
            );
        }
        
        settings.insert(
            "compression".to_string(),
            serde_json::Value::Bool(self.config.transfer_settings.compression),
        );
        settings.insert(
            "encryption".to_string(),
            serde_json::Value::Bool(self.config.transfer_settings.encryption),
        );
        settings.insert(
            "auto_accept_trusted".to_string(),
            serde_json::Value::Bool(self.config.transfer_settings.auto_accept_trusted),
        );
        
        settings.insert(
            "default_quality".to_string(),
            serde_json::Value::String(self.config.stream_settings.default_quality.clone()),
        );
        settings.insert(
            "auto_record".to_string(),
            serde_json::Value::Bool(self.config.stream_settings.auto_record),
        );
        
        ConfigProfile {
            name,
            description,
            settings,
        }
    }

    /// Get the underlying configuration
    pub fn config(&self) -> &CLIConfig {
        &self.config
    }

    /// Get a mutable reference to the underlying configuration
    pub fn config_mut(&mut self) -> &mut CLIConfig {
        &mut self.config
    }

    /// Resolve profile inheritance (if a profile inherits from another)
    pub fn resolve_inheritance(&self, profile_name: &str) -> CLIResult<CLIConfig> {
        let profile = self.get_profile(profile_name)?;
        
        // Check if profile has a "parent" setting
        if let Some(parent_value) = profile.settings.get("parent") {
            if let Some(parent_name) = parent_value.as_str() {
                // Apply parent profile first
                let mut config = self.apply_profile(parent_name)?;
                
                // Then apply current profile settings on top
                for (key, value) in &profile.settings {
                    if key == "parent" {
                        continue; // Skip parent setting
                    }
                    
                    // Apply setting (same logic as apply_profile)
                    match key.as_str() {
                        "output_format" => {
                            if let Some(format_str) = value.as_str() {
                                config.output_format = parse_output_format(format_str)?;
                            }
                        }
                        "color_mode" => {
                            if let Some(mode_str) = value.as_str() {
                                config.color_mode = parse_color_mode(mode_str)?;
                            }
                        }
                        "default_peer" => {
                            config.default_peer = value.as_str().map(|s| s.to_string());
                        }
                        "compression" => {
                            if let Some(val) = value.as_bool() {
                                config.transfer_settings.compression = val;
                            }
                        }
                        "encryption" => {
                            if let Some(val) = value.as_bool() {
                                config.transfer_settings.encryption = val;
                            }
                        }
                        "auto_accept_trusted" => {
                            if let Some(val) = value.as_bool() {
                                config.transfer_settings.auto_accept_trusted = val;
                            }
                        }
                        "default_quality" => {
                            if let Some(quality) = value.as_str() {
                                config.stream_settings.default_quality = quality.to_string();
                            }
                        }
                        "auto_record" => {
                            if let Some(val) = value.as_bool() {
                                config.stream_settings.auto_record = val;
                            }
                        }
                        _ => {}
                    }
                }
                
                return Ok(config);
            }
        }
        
        // No inheritance, just apply the profile
        self.apply_profile(profile_name)
    }

    /// Detect conflicts between profiles
    pub fn detect_conflicts(&self, profile1: &str, profile2: &str) -> CLIResult<Vec<String>> {
        let p1 = self.get_profile(profile1)?;
        let p2 = self.get_profile(profile2)?;
        
        let mut conflicts = Vec::new();
        
        for (key, value1) in &p1.settings {
            if let Some(value2) = p2.settings.get(key) {
                if value1 != value2 {
                    conflicts.push(format!(
                        "Setting '{}' differs: {} vs {}",
                        key, value1, value2
                    ));
                }
            }
        }
        
        Ok(conflicts)
    }
}

/// Parse output format from string
fn parse_output_format(s: &str) -> CLIResult<OutputFormat> {
    match s.to_lowercase().as_str() {
        "table" => Ok(OutputFormat::Table),
        "json" => Ok(OutputFormat::JSON),
        "csv" => Ok(OutputFormat::CSV),
        "minimal" => Ok(OutputFormat::Minimal),
        _ => Err(CLIError::config(format!(
            "Invalid output format '{}'. Valid options: table, json, csv, minimal",
            s
        ))),
    }
}

/// Parse color mode from string
fn parse_color_mode(s: &str) -> CLIResult<ColorMode> {
    match s.to_lowercase().as_str() {
        "auto" => Ok(ColorMode::Auto),
        "always" => Ok(ColorMode::Always),
        "never" => Ok(ColorMode::Never),
        _ => Err(CLIError::config(format!(
            "Invalid color mode '{}'. Valid options: auto, always, never",
            s
        ))),
    }
}

/// Configuration merger for combining config file with command-line overrides
pub struct ConfigMerger {
    base_config: CLIConfig,
}

impl ConfigMerger {
    /// Create a new configuration merger
    pub fn new(base_config: CLIConfig) -> Self {
        Self { base_config }
    }

    /// Merge command-line arguments with base configuration
    pub fn merge(&self, args: ParsedArgs) -> CLIResult<MergedConfig> {
        let mut config = self.base_config.clone();
        let mut overrides = Vec::new();

        // Apply profile if specified
        if let Some(ref profile_name) = args.profile {
            let manager = ProfileManager::new(config.clone());
            config = manager.resolve_inheritance(profile_name)?;
            overrides.push(format!("Applied profile: {}", profile_name));
        }

        // Override output format
        if let Some(ref format_str) = args.output_format {
            config.output_format = parse_output_format(format_str)?;
            overrides.push(format!("output_format = {}", format_str));
        }

        // Override color mode
        if let Some(ref mode_str) = args.color_mode {
            config.color_mode = parse_color_mode(mode_str)?;
            overrides.push(format!("color_mode = {}", mode_str));
        }

        // Override default peer
        if let Some(ref peer) = args.default_peer {
            config.default_peer = Some(peer.clone());
            overrides.push(format!("default_peer = {}", peer));
        }

        // Override compression
        if let Some(compression) = args.compression {
            config.transfer_settings.compression = compression;
            overrides.push(format!("compression = {}", compression));
        }

        // Override encryption
        if let Some(encryption) = args.encryption {
            config.transfer_settings.encryption = encryption;
            overrides.push(format!("encryption = {}", encryption));
        }

        // Validate merged configuration
        let parser = TOMLConfigParser::new(None)?;
        let validation = parser.validate(&config);
        
        if !validation.is_valid() {
            return Err(CLIError::config(format!(
                "Merged configuration is invalid:\n{}",
                validation.errors.join("\n")
            )));
        }

        Ok(MergedConfig { config, overrides })
    }

    /// Merge with precedence rules
    /// Precedence (highest to lowest):
    /// 1. Command-line arguments
    /// 2. Profile settings
    /// 3. Configuration file
    /// 4. Default values
    pub fn merge_with_precedence(&self, args: ParsedArgs) -> CLIResult<MergedConfig> {
        // Start with base config (from file or defaults)
        let mut config = self.base_config.clone();
        let mut overrides = Vec::new();

        // Apply profile (if specified) - overrides base config
        if let Some(ref profile_name) = args.profile {
            let manager = ProfileManager::new(config.clone());
            config = manager.resolve_inheritance(profile_name)?;
            overrides.push(format!("Profile '{}' applied", profile_name));
        }

        // Apply command-line overrides - highest precedence
        if let Some(ref format_str) = args.output_format {
            config.output_format = parse_output_format(format_str)?;
            overrides.push(format!("CLI override: output_format = {}", format_str));
        }

        if let Some(ref mode_str) = args.color_mode {
            config.color_mode = parse_color_mode(mode_str)?;
            overrides.push(format!("CLI override: color_mode = {}", mode_str));
        }

        if let Some(ref peer) = args.default_peer {
            config.default_peer = Some(peer.clone());
            overrides.push(format!("CLI override: default_peer = {}", peer));
        }

        if let Some(compression) = args.compression {
            config.transfer_settings.compression = compression;
            overrides.push(format!("CLI override: compression = {}", compression));
        }

        if let Some(encryption) = args.encryption {
            config.transfer_settings.encryption = encryption;
            overrides.push(format!("CLI override: encryption = {}", encryption));
        }

        // Validate final configuration
        self.validate_merged_config(&config)?;

        Ok(MergedConfig { config, overrides })
    }

    /// Validate merged configuration
    fn validate_merged_config(&self, config: &CLIConfig) -> CLIResult<()> {
        let parser = TOMLConfigParser::new(None)?;
        let validation = parser.validate(config);
        
        if !validation.is_valid() {
            return Err(CLIError::config(format!(
                "Merged configuration is invalid:\n{}",
                validation.errors.join("\n")
            )));
        }

        // Print warnings if any
        if !validation.warnings.is_empty() {
            eprintln!("Configuration warnings:");
            for warning in &validation.warnings {
                eprintln!("  - {}", warning);
            }
        }

        Ok(())
    }

    /// Get the base configuration
    pub fn base_config(&self) -> &CLIConfig {
        &self.base_config
    }
}

/// Load configuration with command-line overrides
pub async fn load_config_with_overrides(args: ParsedArgs) -> CLIResult<MergedConfig> {
    // Load base configuration from file or use custom path
    let base_config = if let Some(ref config_path) = args.config_file {
        load_config_from_path(config_path.clone()).await?
    } else {
        load_or_create_config().await?
    };

    // Merge with command-line arguments
    let merger = ConfigMerger::new(base_config);
    merger.merge_with_precedence(args)
}

/// Runtime configuration validator
pub struct RuntimeConfigValidator;

impl RuntimeConfigValidator {
    /// Validate configuration at runtime
    pub fn validate_runtime(config: &CLIConfig) -> CLIResult<()> {
        let parser = TOMLConfigParser::new(None)?;
        let validation = parser.validate(config);
        
        if !validation.is_valid() {
            return Err(CLIError::config(format!(
                "Runtime configuration validation failed:\n{}",
                validation.errors.join("\n")
            )));
        }

        Ok(())
    }

    /// Validate and suggest fixes for configuration errors
    pub fn validate_with_suggestions(config: &CLIConfig) -> ValidationResult {
        let parser = TOMLConfigParser::new(None).unwrap();
        parser.validate(config)
    }

    /// Check if configuration needs migration
    pub fn needs_migration(_config: &CLIConfig) -> bool {
        // For now, no migration needed
        // In the future, check version and determine if migration is needed
        false
    }
}

/// Default peer configuration manager
pub struct DefaultPeerConfig {
    config: CLIConfig,
}

impl DefaultPeerConfig {
    /// Create a new default peer configuration manager
    pub fn new(config: CLIConfig) -> Self {
        Self { config }
    }

    /// Get the default peer
    pub fn get_default_peer(&self) -> Option<&String> {
        self.config.default_peer.as_ref()
    }

    /// Set the default peer
    pub fn set_default_peer(&mut self, peer: Option<String>) {
        self.config.default_peer = peer;
    }

    /// Check if a default peer is configured
    pub fn has_default_peer(&self) -> bool {
        self.config.default_peer.is_some()
    }

    /// Clear the default peer
    pub fn clear_default_peer(&mut self) {
        self.config.default_peer = None;
    }

    /// Get the underlying configuration
    pub fn config(&self) -> &CLIConfig {
        &self.config
    }

    /// Get a mutable reference to the underlying configuration
    pub fn config_mut(&mut self) -> &mut CLIConfig {
        &mut self.config
    }
}

/// Transfer settings configuration manager
pub struct TransferSettingsConfig {
    config: CLIConfig,
}

impl TransferSettingsConfig {
    /// Create a new transfer settings configuration manager
    pub fn new(config: CLIConfig) -> Self {
        Self { config }
    }

    /// Get compression setting
    pub fn is_compression_enabled(&self) -> bool {
        self.config.transfer_settings.compression
    }

    /// Set compression setting
    pub fn set_compression(&mut self, enabled: bool) {
        self.config.transfer_settings.compression = enabled;
    }

    /// Get encryption setting
    pub fn is_encryption_enabled(&self) -> bool {
        self.config.transfer_settings.encryption
    }

    /// Set encryption setting
    pub fn set_encryption(&mut self, enabled: bool) {
        self.config.transfer_settings.encryption = enabled;
    }

    /// Get default download path
    pub fn get_default_download_path(&self) -> Option<&PathBuf> {
        self.config.transfer_settings.default_download_path.as_ref()
    }

    /// Set default download path
    pub fn set_default_download_path(&mut self, path: Option<PathBuf>) -> CLIResult<()> {
        // Validate path exists if provided
        if let Some(ref p) = path {
            if !p.exists() {
                return Err(CLIError::config(format!(
                    "Download path does not exist: {}",
                    p.display()
                )));
            }
            if !p.is_dir() {
                return Err(CLIError::config(format!(
                    "Download path is not a directory: {}",
                    p.display()
                )));
            }
        }
        
        self.config.transfer_settings.default_download_path = path;
        Ok(())
    }

    /// Get auto-accept trusted setting
    pub fn is_auto_accept_trusted(&self) -> bool {
        self.config.transfer_settings.auto_accept_trusted
    }

    /// Set auto-accept trusted setting
    pub fn set_auto_accept_trusted(&mut self, enabled: bool) {
        self.config.transfer_settings.auto_accept_trusted = enabled;
    }

    /// Get all transfer settings
    pub fn get_transfer_settings(&self) -> &crate::cli::types::TransferSettings {
        &self.config.transfer_settings
    }

    /// Update transfer settings
    pub fn update_transfer_settings(
        &mut self,
        settings: crate::cli::types::TransferSettings,
    ) -> CLIResult<()> {
        // Validate settings
        if let Some(ref path) = settings.default_download_path {
            if !path.exists() {
                return Err(CLIError::config(format!(
                    "Download path does not exist: {}",
                    path.display()
                )));
            }
        }
        
        self.config.transfer_settings = settings;
        Ok(())
    }

    /// Get the underlying configuration
    pub fn config(&self) -> &CLIConfig {
        &self.config
    }

    /// Get a mutable reference to the underlying configuration
    pub fn config_mut(&mut self) -> &mut CLIConfig {
        &mut self.config
    }
}

/// Stream settings configuration manager
pub struct StreamSettingsConfig {
    config: CLIConfig,
}

impl StreamSettingsConfig {
    /// Create a new stream settings configuration manager
    pub fn new(config: CLIConfig) -> Self {
        Self { config }
    }

    /// Get default quality setting
    pub fn get_default_quality(&self) -> &str {
        &self.config.stream_settings.default_quality
    }

    /// Set default quality setting
    pub fn set_default_quality(&mut self, quality: String) -> CLIResult<()> {
        let valid_qualities = ["low", "medium", "high", "ultra"];
        if !valid_qualities.contains(&quality.as_str()) {
            return Err(CLIError::config(format!(
                "Invalid quality '{}'. Valid options: {}",
                quality,
                valid_qualities.join(", ")
            )));
        }
        
        self.config.stream_settings.default_quality = quality;
        Ok(())
    }

    /// Get auto-record setting
    pub fn is_auto_record_enabled(&self) -> bool {
        self.config.stream_settings.auto_record
    }

    /// Set auto-record setting
    pub fn set_auto_record(&mut self, enabled: bool) {
        self.config.stream_settings.auto_record = enabled;
    }

    /// Get recording path
    pub fn get_recording_path(&self) -> Option<&PathBuf> {
        self.config.stream_settings.recording_path.as_ref()
    }

    /// Set recording path
    pub fn set_recording_path(&mut self, path: Option<PathBuf>) -> CLIResult<()> {
        // Validate path exists if provided
        if let Some(ref p) = path {
            if !p.exists() {
                return Err(CLIError::config(format!(
                    "Recording path does not exist: {}",
                    p.display()
                )));
            }
            if !p.is_dir() {
                return Err(CLIError::config(format!(
                    "Recording path is not a directory: {}",
                    p.display()
                )));
            }
        }
        
        self.config.stream_settings.recording_path = path;
        Ok(())
    }

    /// Get all stream settings
    pub fn get_stream_settings(&self) -> &crate::cli::types::StreamSettings {
        &self.config.stream_settings
    }

    /// Update stream settings
    pub fn update_stream_settings(
        &mut self,
        settings: crate::cli::types::StreamSettings,
    ) -> CLIResult<()> {
        // Validate settings
        let valid_qualities = ["low", "medium", "high", "ultra"];
        if !valid_qualities.contains(&settings.default_quality.as_str()) {
            return Err(CLIError::config(format!(
                "Invalid quality '{}'. Valid options: {}",
                settings.default_quality,
                valid_qualities.join(", ")
            )));
        }
        
        if let Some(ref path) = settings.recording_path {
            if !path.exists() {
                return Err(CLIError::config(format!(
                    "Recording path does not exist: {}",
                    path.display()
                )));
            }
        }
        
        self.config.stream_settings = settings;
        Ok(())
    }

    /// Get the underlying configuration
    pub fn config(&self) -> &CLIConfig {
        &self.config
    }

    /// Get a mutable reference to the underlying configuration
    pub fn config_mut(&mut self) -> &mut CLIConfig {
        &mut self.config
    }
}

/// Output format configuration manager
pub struct OutputFormatConfig {
    config: CLIConfig,
}

impl OutputFormatConfig {
    /// Create a new output format configuration manager
    pub fn new(config: CLIConfig) -> Self {
        Self { config }
    }

    /// Get output format
    pub fn get_output_format(&self) -> OutputFormat {
        self.config.output_format
    }

    /// Set output format
    pub fn set_output_format(&mut self, format: OutputFormat) {
        self.config.output_format = format;
    }

    /// Get color mode
    pub fn get_color_mode(&self) -> ColorMode {
        self.config.color_mode
    }

    /// Set color mode
    pub fn set_color_mode(&mut self, mode: ColorMode) {
        self.config.color_mode = mode;
    }

    /// Check if colors should be used based on mode and terminal capabilities
    pub fn should_use_colors(&self) -> bool {
        match self.config.color_mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => {
                // Check if terminal supports colors
                atty::is(atty::Stream::Stdout)
            }
        }
    }

    /// Get the underlying configuration
    pub fn config(&self) -> &CLIConfig {
        &self.config
    }

    /// Get a mutable reference to the underlying configuration
    pub fn config_mut(&mut self) -> &mut CLIConfig {
        &mut self.config
    }
}

/// Unified configuration manager that provides access to all configuration aspects
pub struct UnifiedConfigManager {
    config: CLIConfig,
}

impl UnifiedConfigManager {
    /// Create a new unified configuration manager
    pub fn new(config: CLIConfig) -> Self {
        Self { config }
    }

    /// Get default peer configuration manager
    pub fn default_peer(&self) -> DefaultPeerConfig {
        DefaultPeerConfig::new(self.config.clone())
    }

    /// Get transfer settings configuration manager
    pub fn transfer_settings(&self) -> TransferSettingsConfig {
        TransferSettingsConfig::new(self.config.clone())
    }

    /// Get stream settings configuration manager
    pub fn stream_settings(&self) -> StreamSettingsConfig {
        StreamSettingsConfig::new(self.config.clone())
    }

    /// Get output format configuration manager
    pub fn output_format(&self) -> OutputFormatConfig {
        OutputFormatConfig::new(self.config.clone())
    }

    /// Get profile manager
    pub fn profiles(&self) -> ProfileManager {
        ProfileManager::new(self.config.clone())
    }

    /// Get the underlying configuration
    pub fn config(&self) -> &CLIConfig {
        &self.config
    }

    /// Get a mutable reference to the underlying configuration
    pub fn config_mut(&mut self) -> &mut CLIConfig {
        &mut self.config
    }

    /// Save the current configuration
    pub async fn save(&self) -> CLIResult<()> {
        save_config(&self.config).await
    }

    /// Reload configuration from disk
    pub async fn reload(&mut self) -> CLIResult<()> {
        self.config = load_or_create_config().await?;
        Ok(())
    }
}
