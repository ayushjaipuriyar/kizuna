// iOS platform adapter
//
// Provides iOS-specific implementations for UI, system services,
// networking, security, file management, App Store compliance,
// form factor support, accessibility, and internationalization.

pub mod ui;
pub mod services;
pub mod networking;
pub mod security;
pub mod file_management;
pub mod app_store;
pub mod form_factor;
pub mod accessibility;
pub mod internationalization;

use async_trait::async_trait;
use crate::platform::{
    PlatformResult, PlatformError, PlatformAdapter, SystemServices, UIFramework,
    NetworkConfig, SecurityConfig, GUIFramework,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// iOS platform adapter
pub struct IOSAdapter {
    ui_manager: Arc<RwLock<ui::IOSUIManager>>,
    service_manager: Arc<services::IOSServiceManager>,
    network_manager: Arc<networking::IOSNetworkManager>,
    security_manager: Arc<security::IOSSecurityManager>,
    file_manager: Arc<file_management::IOSFileManager>,
}

impl IOSAdapter {
    /// Create a new iOS adapter
    pub fn new() -> Self {
        Self {
            ui_manager: Arc::new(RwLock::new(ui::IOSUIManager::new())),
            service_manager: Arc::new(services::IOSServiceManager::new()),
            network_manager: Arc::new(networking::IOSNetworkManager::new()),
            security_manager: Arc::new(security::IOSSecurityManager::new()),
            file_manager: Arc::new(file_management::IOSFileManager::new()),
        }
    }

    /// Get UI manager
    pub fn ui_manager(&self) -> Arc<RwLock<ui::IOSUIManager>> {
        Arc::clone(&self.ui_manager)
    }

    /// Get service manager
    pub fn service_manager(&self) -> Arc<services::IOSServiceManager> {
        Arc::clone(&self.service_manager)
    }

    /// Get network manager
    pub fn network_manager(&self) -> Arc<networking::IOSNetworkManager> {
        Arc::clone(&self.network_manager)
    }

    /// Get security manager
    pub fn security_manager(&self) -> Arc<security::IOSSecurityManager> {
        Arc::clone(&self.security_manager)
    }

    /// Get file manager
    pub fn file_manager(&self) -> Arc<file_management::IOSFileManager> {
        Arc::clone(&self.file_manager)
    }
}

impl Default for IOSAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformAdapter for IOSAdapter {
    async fn initialize_platform(&self) -> PlatformResult<()> {
        // Initialize iOS-specific components
        self.ui_manager.write().await.initialize().await?;
        self.service_manager.initialize().await?;
        self.network_manager.initialize().await?;
        self.security_manager.initialize().await?;
        self.file_manager.initialize().await?;
        
        Ok(())
    }

    async fn integrate_system_services(&self) -> PlatformResult<SystemServices> {
        let services = self.service_manager.get_system_services().await?;
        
        Ok(SystemServices {
            notifications: services.notifications_available,
            system_tray: false, // iOS doesn't have system tray
            file_manager: services.file_access_available,
            network_manager: services.network_available,
            metadata: services.metadata,
        })
    }

    async fn setup_ui_framework(&self) -> PlatformResult<UIFramework> {
        let ui_info = self.ui_manager.read().await.get_framework_info().await?;
        
        Ok(UIFramework {
            framework_type: GUIFramework::Native,
            version: ui_info.version,
            capabilities: ui_info.capabilities,
        })
    }

    async fn configure_networking(&self) -> PlatformResult<NetworkConfig> {
        self.network_manager.get_network_config().await
    }

    async fn setup_security_integration(&self) -> PlatformResult<SecurityConfig> {
        let mut config = SecurityConfig::default();
        config.sandbox_enabled = true;
        
        // Check if keychain is available
        if self.security_manager.is_keychain_available().await {
            config.use_keychain = true;
        }
        
        // Check if Secure Enclave is available
        if self.security_manager.is_secure_enclave_available().await {
            config.use_hardware_crypto = true;
        }
        
        Ok(config)
    }

    fn platform_name(&self) -> &str {
        "ios"
    }

    fn get_optimizations(&self) -> Vec<String> {
        vec![
            "mobile_battery_optimization".to_string(),
            "background_processing_limits".to_string(),
            "mobile_network_optimization".to_string(),
            "reduced_memory_footprint".to_string(),
            "secure_enclave_crypto".to_string(),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ios_adapter_creation() {
        let adapter = IOSAdapter::new();
        assert_eq!(adapter.platform_name(), "ios");
    }

    #[tokio::test]
    async fn test_ios_adapter_initialization() {
        let adapter = IOSAdapter::new();
        let result = adapter.initialize_platform().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_system_services_integration() {
        let adapter = IOSAdapter::new();
        adapter.initialize_platform().await.unwrap();
        
        let services = adapter.integrate_system_services().await.unwrap();
        assert!(services.notifications);
        assert!(!services.system_tray); // iOS doesn't have system tray
        assert!(services.file_manager);
    }

    #[tokio::test]
    async fn test_ui_framework_setup() {
        let adapter = IOSAdapter::new();
        adapter.initialize_platform().await.unwrap();
        
        let ui = adapter.setup_ui_framework().await.unwrap();
        assert_eq!(ui.framework_type, GUIFramework::Native);
        assert!(!ui.capabilities.is_empty());
    }

    #[tokio::test]
    async fn test_networking_configuration() {
        let adapter = IOSAdapter::new();
        adapter.initialize_platform().await.unwrap();
        
        let config = adapter.configure_networking().await.unwrap();
        assert!(config.max_connections <= 100); // Mobile should have reasonable limits
        assert!(config.timeout_ms >= 10000); // Mobile networks need longer timeouts
    }

    #[tokio::test]
    async fn test_security_integration() {
        let adapter = IOSAdapter::new();
        adapter.initialize_platform().await.unwrap();
        
        let config = adapter.setup_security_integration().await.unwrap();
        assert!(config.sandbox_enabled); // iOS always sandboxes apps
        assert!(config.use_keychain); // iOS has Keychain
        assert!(config.use_hardware_crypto); // iOS has Secure Enclave
    }

    #[test]
    fn test_platform_optimizations() {
        let adapter = IOSAdapter::new();
        let optimizations = adapter.get_optimizations();
        
        assert!(!optimizations.is_empty());
        assert!(optimizations.iter().any(|o| o.contains("battery")));
        assert!(optimizations.iter().any(|o| o.contains("mobile")));
        assert!(optimizations.iter().any(|o| o.contains("secure_enclave")));
    }
}

