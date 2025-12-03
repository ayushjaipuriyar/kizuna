// Platform detection and capability demonstration

use kizuna::platform::{
    DefaultPlatformManager, PlatformManager, Feature,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Kizuna Platform Detection Demo ===\n");
    
    // Create platform manager
    let manager = DefaultPlatformManager::new()?;
    
    // Detect platform
    let info = manager.detect_platform()?;
    println!("Operating System: {:?}", info.os);
    println!("Architecture: {:?}", info.architecture);
    println!("Version: {}", info.version);
    if let Some(variant) = &info.variant {
        println!("Variant: {}", variant);
    }
    
    // Get capabilities
    println!("\n=== Platform Capabilities ===");
    let caps = manager.get_capabilities()?;
    println!("GUI Framework: {:?}", caps.gui_framework);
    println!("System Tray: {}", caps.system_tray);
    println!("Notifications: {}", caps.notifications);
    println!("File Associations: {}", caps.file_associations);
    println!("Auto Start: {}", caps.auto_start);
    
    // Check feature availability
    println!("\n=== Feature Availability ===");
    let features = [
        Feature::Clipboard,
        Feature::FileTransfer,
        Feature::Streaming,
        Feature::CommandExecution,
        Feature::Discovery,
        Feature::SystemTray,
        Feature::Notifications,
        Feature::AutoStart,
        Feature::FileAssociations,
    ];
    
    for feature in features {
        let available = manager.is_feature_available(feature);
        println!("{:?}: {}", feature, if available { "✓" } else { "✗" });
    }
    
    // Get platform adapter
    println!("\n=== Platform Adapter ===");
    let adapter = manager.get_platform_adapter()?;
    println!("Platform: {}", adapter.platform_name());
    
    // Initialize platform
    adapter.initialize_platform().await?;
    println!("Platform initialized successfully");
    
    // Get system services
    let services = adapter.integrate_system_services().await?;
    println!("\nSystem Services:");
    println!("  Notifications: {}", services.notifications);
    println!("  System Tray: {}", services.system_tray);
    println!("  File Manager: {}", services.file_manager);
    println!("  Network Manager: {}", services.network_manager);
    
    // Get UI framework
    let ui = adapter.setup_ui_framework().await?;
    println!("\nUI Framework:");
    println!("  Type: {:?}", ui.framework_type);
    println!("  Version: {}", ui.version);
    println!("  Capabilities: {:?}", ui.capabilities);
    
    // Get network config
    let network = adapter.configure_networking().await?;
    println!("\nNetwork Configuration:");
    println!("  Preferred Protocols: {:?}", network.preferred_protocols);
    println!("  Fallback Enabled: {}", network.fallback_enabled);
    println!("  Timeout: {}ms", network.timeout_ms);
    println!("  Max Connections: {}", network.max_connections);
    
    // Get security config
    let security = adapter.setup_security_integration().await?;
    println!("\nSecurity Configuration:");
    println!("  Use Keychain: {}", security.use_keychain);
    println!("  Use Hardware Crypto: {}", security.use_hardware_crypto);
    println!("  Require Code Signing: {}", security.require_code_signing);
    println!("  Sandbox Enabled: {}", security.sandbox_enabled);
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}
