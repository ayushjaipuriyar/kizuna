// macOS platform support demonstration

#[cfg(target_os = "macos")]
use kizuna::platform::macos::{
    MacOSAdapter, is_code_signed, check_gatekeeper_status,
    AppBundle, get_architecture, is_apple_silicon,
};

#[cfg(target_os = "macos")]
use kizuna::platform::PlatformAdapter;

#[cfg(target_os = "macos")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== macOS Platform Support Demo ===\n");

    // Platform detection
    println!("Architecture: {}", get_architecture());
    println!("Apple Silicon: {}", is_apple_silicon());
    println!();

    // Initialize macOS adapter
    println!("Initializing macOS adapter...");
    let adapter = MacOSAdapter::new();
    adapter.initialize_platform().await?;
    println!("✓ macOS adapter initialized\n");

    // System services integration
    println!("Integrating system services...");
    let services = adapter.integrate_system_services().await?;
    println!("✓ Notifications: {}", services.notifications);
    println!("✓ System Tray: {}", services.system_tray);
    println!("✓ File Manager: {}", services.file_manager);
    println!("✓ Network Manager: {}", services.network_manager);
    
    if let Some(version) = services.metadata.get("macos_version") {
        println!("✓ macOS Version: {}", version);
    }
    if let Some(arch) = services.metadata.get("architecture") {
        println!("✓ Architecture: {}", arch);
    }
    println!();

    // UI Framework setup
    println!("Setting up UI framework...");
    let ui_framework = adapter.setup_ui_framework().await?;
    println!("✓ Framework: {:?}", ui_framework.framework_type);
    println!("✓ Version: {}", ui_framework.version);
    println!("✓ Capabilities: {:?}", ui_framework.capabilities);
    println!();

    // Security integration
    println!("Setting up security integration...");
    let security_config = adapter.setup_security_integration().await?;
    println!("✓ Keychain: {}", security_config.use_keychain);
    println!("✓ Hardware Crypto: {}", security_config.use_hardware_crypto);
    println!("✓ Code Signing Required: {}", security_config.require_code_signing);
    println!("✓ Sandbox Enabled: {}", security_config.sandbox_enabled);
    println!();

    // Check code signing status
    println!("Checking code signing status...");
    match is_code_signed() {
        Ok(signed) => println!("✓ Application is code signed: {}", signed),
        Err(e) => println!("⚠ Could not check code signing: {}", e),
    }
    println!();

    // Check Gatekeeper status
    println!("Checking Gatekeeper status...");
    match check_gatekeeper_status() {
        Ok(enabled) => println!("✓ Gatekeeper is enabled: {}", enabled),
        Err(e) => println!("⚠ Could not check Gatekeeper: {}", e),
    }
    println!();

    // Platform optimizations
    println!("Available optimizations:");
    for opt in adapter.get_optimizations() {
        println!("  • {}", opt);
    }
    println!();

    // App bundle example
    println!("App Bundle Configuration Example:");
    let bundle = AppBundle::new(
        "Kizuna".to_string(),
        "com.example.kizuna".to_string(),
        "0.1.0".to_string(),
    );
    println!("✓ Bundle Name: {}", bundle.name);
    println!("✓ Identifier: {}", bundle.identifier);
    println!("✓ Version: {}", bundle.version);
    println!("✓ Executable: {}", bundle.executable_name);
    println!();

    println!("=== Demo Complete ===");

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("This demo is only available on macOS");
}
