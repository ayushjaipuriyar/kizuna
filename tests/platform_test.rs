// Integration tests for platform module

use kizuna::platform::{
    PlatformManager, DefaultPlatformManager, OperatingSystem, Architecture,
};

#[test]
fn test_platform_detection() {
    let manager = DefaultPlatformManager::new().expect("Failed to create platform manager");
    let info = manager.detect_platform().expect("Failed to detect platform");
    
    // Verify we detected a valid platform
    assert_ne!(info.os, OperatingSystem::Unknown);
    assert_ne!(info.architecture, Architecture::Unknown);
    
    println!("Detected platform: {:?} on {:?}", info.os, info.architecture);
}

#[test]
fn test_platform_capabilities() {
    let manager = DefaultPlatformManager::new().expect("Failed to create platform manager");
    let caps = manager.get_capabilities().expect("Failed to get capabilities");
    
    // File transfer should always be available
    assert!(manager.is_feature_available(kizuna::platform::Feature::FileTransfer));
    
    println!("Platform capabilities: {:?}", caps);
}

#[test]
fn test_platform_optimization() {
    let manager = DefaultPlatformManager::new().expect("Failed to create platform manager");
    let mut config = kizuna::platform::PlatformConfig::default();
    
    let result = manager.optimize_for_platform(&mut config);
    assert!(result.is_ok());
    
    println!("Optimized config: {:?}", config);
}

#[tokio::test]
async fn test_platform_adapter() {
    let manager = DefaultPlatformManager::new().expect("Failed to create platform manager");
    let adapter = manager.get_platform_adapter().expect("Failed to get adapter");
    
    // Test adapter initialization
    let result = adapter.initialize_platform().await;
    assert!(result.is_ok());
    
    println!("Platform adapter: {}", adapter.platform_name());
}
