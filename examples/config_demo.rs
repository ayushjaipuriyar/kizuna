// Configuration management system demonstration

use kizuna::cli::config::{
    load_or_create_config, save_config, load_config_with_overrides,
    ParsedArgs, ProfileManager, ConfigMerger, UnifiedConfigManager,
    TOMLConfigParser, DefaultPeerConfig, TransferSettingsConfig,
};
use kizuna::cli::types::{CLIConfig, ConfigProfile, OutputFormat, ColorMode};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Kizuna Configuration Management Demo ===\n");

    // Demo 1: Load or create default configuration
    println!("1. Loading or creating default configuration...");
    let config = load_or_create_config().await?;
    println!("   Output format: {:?}", config.output_format);
    println!("   Color mode: {:?}", config.color_mode);
    println!("   Compression: {}", config.transfer_settings.compression);
    println!("   Encryption: {}", config.transfer_settings.encryption);
    println!();

    // Demo 2: TOML parser
    println!("2. Demonstrating TOML parser...");
    let parser = TOMLConfigParser::new(None)?;
    let toml_str = parser.serialize_toml(&config)?;
    println!("   Configuration as TOML:");
    println!("{}", toml_str);
    println!();

    // Demo 3: Configuration validation
    println!("3. Validating configuration...");
    let validation = parser.validate(&config);
    println!("   Valid: {}", validation.is_valid());
    if !validation.errors.is_empty() {
        println!("   Errors:");
        for error in &validation.errors {
            println!("     - {}", error);
        }
    }
    if !validation.warnings.is_empty() {
        println!("   Warnings:");
        for warning in &validation.warnings {
            println!("     - {}", warning);
        }
    }
    println!();

    // Demo 4: Profile management
    println!("4. Demonstrating profile management...");
    let mut manager = ProfileManager::new(config.clone());
    
    // Create a work profile
    let mut work_settings = HashMap::new();
    work_settings.insert("compression".to_string(), serde_json::Value::Bool(false));
    work_settings.insert("encryption".to_string(), serde_json::Value::Bool(true));
    work_settings.insert("output_format".to_string(), serde_json::Value::String("json".to_string()));
    
    let work_profile = ConfigProfile {
        name: "work".to_string(),
        description: "Work environment settings".to_string(),
        settings: work_settings,
    };
    
    manager.add_profile(work_profile)?;
    println!("   Added 'work' profile");
    
    // Create a personal profile
    let mut personal_settings = HashMap::new();
    personal_settings.insert("compression".to_string(), serde_json::Value::Bool(true));
    personal_settings.insert("auto_accept_trusted".to_string(), serde_json::Value::Bool(true));
    personal_settings.insert("color_mode".to_string(), serde_json::Value::String("always".to_string()));
    
    let personal_profile = ConfigProfile {
        name: "personal".to_string(),
        description: "Personal use settings".to_string(),
        settings: personal_settings,
    };
    
    manager.add_profile(personal_profile)?;
    println!("   Added 'personal' profile");
    
    // List profiles
    let profiles = manager.list_profiles();
    println!("   Available profiles: {:?}", profiles);
    
    // Apply work profile
    let work_config = manager.apply_profile("work")?;
    println!("   Applied 'work' profile:");
    println!("     Compression: {}", work_config.transfer_settings.compression);
    println!("     Encryption: {}", work_config.transfer_settings.encryption);
    println!("     Output format: {:?}", work_config.output_format);
    println!();

    // Demo 5: Command-line overrides
    println!("5. Demonstrating command-line overrides...");
    let args = ParsedArgs {
        output_format: Some("minimal".to_string()),
        color_mode: Some("never".to_string()),
        config_file: None,
        profile: Some("work".to_string()),
        default_peer: Some("my-laptop".to_string()),
        compression: Some(true),
        encryption: None,
    };
    
    let merger = ConfigMerger::new(manager.config().clone());
    let merged = merger.merge_with_precedence(args)?;
    
    println!("   Merged configuration:");
    println!("     Output format: {:?}", merged.config.output_format);
    println!("     Color mode: {:?}", merged.config.color_mode);
    println!("     Default peer: {:?}", merged.config.default_peer);
    println!("     Compression: {}", merged.config.transfer_settings.compression);
    println!("   Overrides applied:");
    for override_msg in &merged.overrides {
        println!("     - {}", override_msg);
    }
    println!();

    // Demo 6: Specialized configuration managers
    println!("6. Demonstrating specialized configuration managers...");
    let unified = UnifiedConfigManager::new(config.clone());
    
    // Default peer config
    let peer_config = unified.default_peer();
    println!("   Default peer: {:?}", peer_config.get_default_peer());
    
    // Transfer settings config
    let transfer_config = unified.transfer_settings();
    println!("   Compression enabled: {}", transfer_config.is_compression_enabled());
    println!("   Encryption enabled: {}", transfer_config.is_encryption_enabled());
    println!("   Auto-accept trusted: {}", transfer_config.is_auto_accept_trusted());
    
    // Stream settings config
    let stream_config = unified.stream_settings();
    println!("   Default quality: {}", stream_config.get_default_quality());
    println!("   Auto-record: {}", stream_config.is_auto_record_enabled());
    
    // Output format config
    let output_config = unified.output_format();
    println!("   Output format: {:?}", output_config.get_output_format());
    println!("   Color mode: {:?}", output_config.get_color_mode());
    println!("   Should use colors: {}", output_config.should_use_colors());
    println!();

    // Demo 7: Profile inheritance
    println!("7. Demonstrating profile inheritance...");
    let mut parent_settings = HashMap::new();
    parent_settings.insert("compression".to_string(), serde_json::Value::Bool(true));
    parent_settings.insert("encryption".to_string(), serde_json::Value::Bool(true));
    
    let parent_profile = ConfigProfile {
        name: "base".to_string(),
        description: "Base settings".to_string(),
        settings: parent_settings,
    };
    
    manager.add_profile(parent_profile)?;
    
    let mut child_settings = HashMap::new();
    child_settings.insert("parent".to_string(), serde_json::Value::String("base".to_string()));
    child_settings.insert("output_format".to_string(), serde_json::Value::String("json".to_string()));
    
    let child_profile = ConfigProfile {
        name: "child".to_string(),
        description: "Child profile inheriting from base".to_string(),
        settings: child_settings,
    };
    
    manager.add_profile(child_profile)?;
    
    let inherited_config = manager.resolve_inheritance("child")?;
    println!("   Child profile with inheritance:");
    println!("     Compression (from parent): {}", inherited_config.transfer_settings.compression);
    println!("     Encryption (from parent): {}", inherited_config.transfer_settings.encryption);
    println!("     Output format (from child): {:?}", inherited_config.output_format);
    println!();

    // Demo 8: Conflict detection
    println!("8. Demonstrating conflict detection...");
    let conflicts = manager.detect_conflicts("work", "personal")?;
    if conflicts.is_empty() {
        println!("   No conflicts between 'work' and 'personal' profiles");
    } else {
        println!("   Conflicts found:");
        for conflict in &conflicts {
            println!("     - {}", conflict);
        }
    }
    println!();

    println!("=== Demo Complete ===");
    Ok(())
}
