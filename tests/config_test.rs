// Configuration management system tests

use kizuna::cli::config::{
    TOMLConfigParser, ProfileManager, ConfigMerger, ParsedArgs,
    load_or_create_config, ValidationResult,
};
use kizuna::cli::types::{CLIConfig, ConfigProfile, OutputFormat, ColorMode};
use std::collections::HashMap;

#[tokio::test]
async fn test_toml_parser_serialize_deserialize() {
    let config = CLIConfig::default();
    let parser = TOMLConfigParser::new(None).unwrap();
    
    // Serialize to TOML
    let toml_str = parser.serialize_toml(&config).unwrap();
    assert!(!toml_str.is_empty());
    
    // Deserialize back
    let parsed_config = parser.parse_toml(&toml_str).unwrap();
    assert_eq!(parsed_config.output_format, config.output_format);
    assert_eq!(parsed_config.color_mode, config.color_mode);
}

#[tokio::test]
async fn test_config_validation() {
    let mut config = CLIConfig::default();
    let parser = TOMLConfigParser::new(None).unwrap();
    
    // Valid configuration
    let validation = parser.validate(&config);
    assert!(validation.is_valid());
    assert!(validation.errors.is_empty());
    
    // Invalid stream quality
    config.stream_settings.default_quality = "invalid".to_string();
    let validation = parser.validate(&config);
    assert!(!validation.is_valid());
    assert!(!validation.errors.is_empty());
}

#[tokio::test]
async fn test_profile_management() {
    let config = CLIConfig::default();
    let mut manager = ProfileManager::new(config);
    
    // Create a profile
    let mut settings = HashMap::new();
    settings.insert("compression".to_string(), serde_json::Value::Bool(false));
    settings.insert("output_format".to_string(), serde_json::Value::String("json".to_string()));
    
    let profile = ConfigProfile {
        name: "test".to_string(),
        description: "Test profile".to_string(),
        settings,
    };
    
    // Add profile
    manager.add_profile(profile.clone()).unwrap();
    
    // List profiles
    let profiles = manager.list_profiles();
    assert!(profiles.contains(&"test".to_string()));
    
    // Get profile
    let retrieved = manager.get_profile("test").unwrap();
    assert_eq!(retrieved.name, "test");
    
    // Apply profile
    let applied_config = manager.apply_profile("test").unwrap();
    assert_eq!(applied_config.output_format, OutputFormat::JSON);
    assert!(!applied_config.transfer_settings.compression);
}

#[tokio::test]
async fn test_profile_inheritance() {
    let config = CLIConfig::default();
    let mut manager = ProfileManager::new(config);
    
    // Create parent profile
    let mut parent_settings = HashMap::new();
    parent_settings.insert("compression".to_string(), serde_json::Value::Bool(true));
    parent_settings.insert("encryption".to_string(), serde_json::Value::Bool(true));
    
    let parent_profile = ConfigProfile {
        name: "base".to_string(),
        description: "Base profile".to_string(),
        settings: parent_settings,
    };
    
    manager.add_profile(parent_profile).unwrap();
    
    // Create child profile that inherits from parent
    let mut child_settings = HashMap::new();
    child_settings.insert("parent".to_string(), serde_json::Value::String("base".to_string()));
    child_settings.insert("output_format".to_string(), serde_json::Value::String("json".to_string()));
    
    let child_profile = ConfigProfile {
        name: "child".to_string(),
        description: "Child profile".to_string(),
        settings: child_settings,
    };
    
    manager.add_profile(child_profile).unwrap();
    
    // Resolve inheritance
    let inherited_config = manager.resolve_inheritance("child").unwrap();
    
    // Should have parent's settings
    assert!(inherited_config.transfer_settings.compression);
    assert!(inherited_config.transfer_settings.encryption);
    
    // And child's settings
    assert_eq!(inherited_config.output_format, OutputFormat::JSON);
}

#[tokio::test]
async fn test_config_merger() {
    let mut config = CLIConfig::default();
    config.output_format = OutputFormat::Table;
    config.color_mode = ColorMode::Auto;
    
    let merger = ConfigMerger::new(config);
    
    // Create command-line arguments
    let args = ParsedArgs {
        output_format: Some("json".to_string()),
        color_mode: Some("never".to_string()),
        config_file: None,
        profile: None,
        default_peer: Some("my-laptop".to_string()),
        compression: Some(false),
        encryption: None,
    };
    
    // Merge
    let merged = merger.merge_with_precedence(args).unwrap();
    
    // Check overrides were applied
    assert_eq!(merged.config.output_format, OutputFormat::JSON);
    assert_eq!(merged.config.color_mode, ColorMode::Never);
    assert_eq!(merged.config.default_peer, Some("my-laptop".to_string()));
    assert!(!merged.config.transfer_settings.compression);
    
    // Check overrides were recorded
    assert!(!merged.overrides.is_empty());
}

#[tokio::test]
async fn test_profile_validation() {
    let config = CLIConfig::default();
    let manager = ProfileManager::new(config);
    
    // Valid profile
    let mut valid_settings = HashMap::new();
    valid_settings.insert("compression".to_string(), serde_json::Value::Bool(true));
    
    let valid_profile = ConfigProfile {
        name: "valid".to_string(),
        description: "Valid profile".to_string(),
        settings: valid_settings,
    };
    
    let validation = manager.validate_profile(&valid_profile);
    assert!(validation.is_valid());
    
    // Invalid profile - wrong type for compression
    let mut invalid_settings = HashMap::new();
    invalid_settings.insert("compression".to_string(), serde_json::Value::String("yes".to_string()));
    
    let invalid_profile = ConfigProfile {
        name: "invalid".to_string(),
        description: "Invalid profile".to_string(),
        settings: invalid_settings,
    };
    
    let validation = manager.validate_profile(&invalid_profile);
    assert!(!validation.is_valid());
}

#[tokio::test]
async fn test_conflict_detection() {
    let config = CLIConfig::default();
    let mut manager = ProfileManager::new(config);
    
    // Create two profiles with conflicting settings
    let mut profile1_settings = HashMap::new();
    profile1_settings.insert("compression".to_string(), serde_json::Value::Bool(true));
    profile1_settings.insert("output_format".to_string(), serde_json::Value::String("json".to_string()));
    
    let profile1 = ConfigProfile {
        name: "profile1".to_string(),
        description: "Profile 1".to_string(),
        settings: profile1_settings,
    };
    
    let mut profile2_settings = HashMap::new();
    profile2_settings.insert("compression".to_string(), serde_json::Value::Bool(false));
    profile2_settings.insert("output_format".to_string(), serde_json::Value::String("table".to_string()));
    
    let profile2 = ConfigProfile {
        name: "profile2".to_string(),
        description: "Profile 2".to_string(),
        settings: profile2_settings,
    };
    
    manager.add_profile(profile1).unwrap();
    manager.add_profile(profile2).unwrap();
    
    // Detect conflicts
    let conflicts = manager.detect_conflicts("profile1", "profile2").unwrap();
    assert!(!conflicts.is_empty());
    assert_eq!(conflicts.len(), 2); // compression and output_format differ
}
