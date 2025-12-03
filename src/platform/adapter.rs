// Platform adapter implementations

use async_trait::async_trait;
use crate::platform::{
    PlatformResult, PlatformError, PlatformAdapter, PlatformManager,
    PlatformInfo, PlatformCapabilities, Feature, SystemServices, UIFramework,
    NetworkConfig, SecurityConfig, PlatformConfig, GUIFramework,
    CapabilityManager, OperatingSystem,
};
use std::sync::Arc;

/// Default platform manager implementation
pub struct DefaultPlatformManager {
    platform_info: PlatformInfo,
    capability_manager: Arc<CapabilityManager>,
}

impl DefaultPlatformManager {
    /// Create a new platform manager
    pub fn new() -> PlatformResult<Self> {
        let platform_info = crate::platform::detection::detect_platform()?;
        let capability_manager = Arc::new(CapabilityManager::new(
            platform_info.capabilities.clone()
        ));
        
        Ok(Self {
            platform_info,
            capability_manager,
        })
    }
    
    /// Get capability manager
    pub fn capability_manager(&self) -> &CapabilityManager {
        &self.capability_manager
    }

    /// Get platform info
    pub fn platform_info(&self) -> &PlatformInfo {
        &self.platform_info
    }
}

impl Default for DefaultPlatformManager {
    fn default() -> Self {
        Self::new().expect("Failed to create platform manager")
    }
}

#[async_trait]
impl PlatformManager for DefaultPlatformManager {
    fn detect_platform(&self) -> PlatformResult<PlatformInfo> {
        Ok(self.platform_info.clone())
    }

    fn get_capabilities(&self) -> PlatformResult<PlatformCapabilities> {
        Ok(self.platform_info.capabilities.clone())
    }

    fn is_feature_available(&self, feature: Feature) -> bool {
        self.capability_manager.is_feature_available(feature)
    }

    fn get_platform_adapter(&self) -> PlatformResult<Box<dyn PlatformAdapter>> {
        match self.platform_info.os {
            #[cfg(target_os = "linux")]
            OperatingSystem::Linux => {
                Ok(Box::new(crate::platform::linux::LinuxAdapter::new()))
            }
            
            #[cfg(target_os = "macos")]
            OperatingSystem::MacOS => {
                Ok(Box::new(crate::platform::macos::MacOSAdapter::new()))
            }
            
            #[cfg(target_os = "windows")]
            OperatingSystem::Windows => {
                Ok(Box::new(crate::platform::windows::WindowsAdapter::new()))
            }
            
            #[cfg(target_os = "android")]
            OperatingSystem::Android => {
                Ok(Box::new(crate::platform::android::AndroidAdapter::new()))
            }
            
            #[cfg(target_os = "ios")]
            OperatingSystem::iOS => {
                Ok(Box::new(crate::platform::ios::IOSAdapter::new()))
            }
            
            #[cfg(target_arch = "wasm32")]
            OperatingSystem::WebBrowser => {
                Ok(Box::new(crate::platform::wasm::WasmAdapter::new()))
            }
            
            OperatingSystem::Container => {
                Ok(Box::new(GenericAdapter::new("container")))
            }
            
            _ => {
                Ok(Box::new(GenericAdapter::new("generic")))
            }
        }
    }

    fn optimize_for_platform(&self, config: &mut PlatformConfig) -> PlatformResult<()> {
        // Apply platform-specific optimizations
        match self.platform_info.os {
            OperatingSystem::Linux => {
                config.enable_hardware_acceleration = true;
                config.network.preferred_protocols = vec!["quic".to_string(), "tcp".to_string()];
            }
            OperatingSystem::MacOS => {
                config.enable_hardware_acceleration = true;
                config.security.use_keychain = true;
                config.security.use_hardware_crypto = true;
            }
            OperatingSystem::Windows => {
                config.enable_hardware_acceleration = true;
                config.network.preferred_protocols = vec!["quic".to_string(), "tcp".to_string()];
            }
            OperatingSystem::Android | OperatingSystem::iOS => {
                // Mobile optimizations
                config.enable_optimizations = true;
                config.network.max_connections = 50;
                config.network.timeout_ms = 10000;
            }
            OperatingSystem::WebBrowser => {
                // Browser limitations
                config.network.preferred_protocols = vec!["websocket".to_string(), "webrtc".to_string()];
                config.security.use_keychain = false;
            }
            OperatingSystem::Container => {
                // Container optimizations
                config.enable_hardware_acceleration = false;
                config.network.max_connections = 200;
            }
            _ => {}
        }
        
        Ok(())
    }
}

/// Generic platform adapter for unsupported platforms
pub struct GenericAdapter {
    name: String,
}

impl GenericAdapter {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait]
impl PlatformAdapter for GenericAdapter {
    async fn initialize_platform(&self) -> PlatformResult<()> {
        // Generic initialization - nothing to do
        Ok(())
    }

    async fn integrate_system_services(&self) -> PlatformResult<SystemServices> {
        Ok(SystemServices {
            notifications: false,
            system_tray: false,
            file_manager: false,
            network_manager: false,
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn setup_ui_framework(&self) -> PlatformResult<UIFramework> {
        Ok(UIFramework {
            framework_type: GUIFramework::None,
            version: "generic".to_string(),
            capabilities: vec![],
        })
    }

    async fn configure_networking(&self) -> PlatformResult<NetworkConfig> {
        Ok(NetworkConfig::default())
    }

    async fn setup_security_integration(&self) -> PlatformResult<SecurityConfig> {
        Ok(SecurityConfig::default())
    }

    fn platform_name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_manager_creation() {
        let manager = DefaultPlatformManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_detect_platform() {
        let manager = DefaultPlatformManager::new().unwrap();
        let info = manager.detect_platform();
        assert!(info.is_ok());
    }

    #[test]
    fn test_get_capabilities() {
        let manager = DefaultPlatformManager::new().unwrap();
        let caps = manager.get_capabilities();
        assert!(caps.is_ok());
    }

    #[test]
    fn test_optimize_for_platform() {
        let manager = DefaultPlatformManager::new().unwrap();
        let mut config = PlatformConfig::default();
        
        let result = manager.optimize_for_platform(&mut config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_generic_adapter() {
        let adapter = GenericAdapter::new("test");
        
        assert!(adapter.initialize_platform().await.is_ok());
        assert_eq!(adapter.platform_name(), "test");
        
        let services = adapter.integrate_system_services().await.unwrap();
        assert!(!services.notifications);
    }
}
