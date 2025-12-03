// Windows platform adapter

use async_trait::async_trait;
use crate::platform::{
    PlatformResult, PlatformAdapter, SystemServices, UIFramework,
    NetworkConfig, SecurityConfig, GUIFramework, PlatformError,
};
use std::collections::HashMap;

pub mod windows;

#[cfg(windows)]
use windows::{win32, registry, networking};

/// Windows platform adapter
pub struct WindowsAdapter {
    #[cfg(windows)]
    registry: registry::RegistryManager,
    #[cfg(windows)]
    networking: networking::WindowsNetworking,
    #[cfg(windows)]
    architecture: architecture::ArchitectureOptimizer,
    #[cfg(windows)]
    notifications: notifications::NotificationManager,
    #[cfg(windows)]
    performance: performance::PerformanceOptimizer,
}

impl WindowsAdapter {
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            registry: registry::RegistryManager::new(),
            #[cfg(windows)]
            networking: networking::WindowsNetworking::new(),
            #[cfg(windows)]
            architecture: architecture::ArchitectureOptimizer::default(),
            #[cfg(windows)]
            notifications: notifications::NotificationManager::new("Kizuna".to_string()),
            #[cfg(windows)]
            performance: performance::PerformanceOptimizer::new(),
        }
    }
    
    /// Get architecture information
    #[cfg(windows)]
    pub fn get_architecture(&self) -> &architecture::WindowsArchitecture {
        self.architecture.architecture()
    }
    
    /// Get notification manager
    #[cfg(windows)]
    pub fn notifications(&self) -> &notifications::NotificationManager {
        &self.notifications
    }
    
    /// Get performance optimizer
    #[cfg(windows)]
    pub fn performance(&self) -> &performance::PerformanceOptimizer {
        &self.performance
    }
}

    #[cfg(windows)]
    async fn initialize_win32(&self) -> PlatformResult<()> {
        win32::initialize_com()?;
        win32::initialize_winsock()?;
        Ok(())
    }

    #[cfg(windows)]
    async fn setup_windows_services(&self) -> PlatformResult<SystemServices> {
        let mut metadata = HashMap::new();
        
        // Get Windows version information
        if let Ok(version) = win32::get_windows_version() {
            metadata.insert("windows_version".to_string(), version);
        }
        
        // Get architecture information
        metadata.insert("architecture".to_string(), self.architecture.architecture().as_str().to_string());
        
        // Check for Windows features
        metadata.insert("action_center".to_string(), "available".to_string());
        metadata.insert("windows_security".to_string(), "available".to_string());
        
        Ok(SystemServices {
            notifications: true,
            system_tray: true,
            file_manager: true,
            network_manager: true,
            metadata,
        })
    }

    #[cfg(windows)]
    async fn configure_windows_networking(&self) -> PlatformResult<NetworkConfig> {
        let mut config = NetworkConfig::default();
        config.preferred_protocols = vec!["quic".to_string(), "tcp".to_string()];
        
        // Get Windows firewall status
        if let Ok(firewall_enabled) = self.networking.is_firewall_enabled() {
            config.metadata.insert("firewall_enabled".to_string(), firewall_enabled.to_string());
        }
        
        // Get network adapter information
        if let Ok(adapters) = self.networking.get_network_adapters() {
            config.metadata.insert("adapter_count".to_string(), adapters.len().to_string());
        }
        
        Ok(config)
    }

    #[cfg(windows)]
    async fn setup_windows_security(&self) -> PlatformResult<SecurityConfig> {
        let mut config = SecurityConfig::default();
        config.require_code_signing = true;
        
        // Check Windows Defender status
        if let Ok(defender_enabled) = win32::is_windows_defender_enabled() {
            config.metadata.insert("defender_enabled".to_string(), defender_enabled.to_string());
        }
        
        Ok(config)
    }
}

#[async_trait]
impl PlatformAdapter for WindowsAdapter {
    async fn initialize_platform(&self) -> PlatformResult<()> {
        #[cfg(windows)]
        {
            self.initialize_win32().await?;
            self.registry.initialize()?;
        }
        Ok(())
    }

    async fn integrate_system_services(&self) -> PlatformResult<SystemServices> {
        #[cfg(windows)]
        {
            return self.setup_windows_services().await;
        }
        
        #[cfg(not(windows))]
        {
            Ok(SystemServices {
                notifications: false,
                system_tray: false,
                file_manager: false,
                network_manager: false,
                metadata: HashMap::new(),
            })
        }
    }

    async fn setup_ui_framework(&self) -> PlatformResult<UIFramework> {
        Ok(UIFramework {
            framework_type: GUIFramework::Native,
            version: "win32".to_string(),
            capabilities: vec![
                "win32".to_string(),
                "winrt".to_string(),
                "action_center".to_string(),
            ],
        })
    }

    async fn configure_networking(&self) -> PlatformResult<NetworkConfig> {
        #[cfg(windows)]
        {
            return self.configure_windows_networking().await;
        }
        
        #[cfg(not(windows))]
        {
            let mut config = NetworkConfig::default();
            config.preferred_protocols = vec!["quic".to_string(), "tcp".to_string()];
            Ok(config)
        }
    }

    async fn setup_security_integration(&self) -> PlatformResult<SecurityConfig> {
        #[cfg(windows)]
        {
            return self.setup_windows_security().await;
        }
        
        #[cfg(not(windows))]
        {
            let mut config = SecurityConfig::default();
            config.require_code_signing = true;
            Ok(config)
        }
    }

    fn platform_name(&self) -> &str {
        "windows"
    }
}
