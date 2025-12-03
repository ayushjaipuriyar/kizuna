// macOS platform adapter implementation

use async_trait::async_trait;
use crate::platform::{
    PlatformResult, PlatformAdapter, SystemServices, UIFramework,
    NetworkConfig, SecurityConfig, GUIFramework, PlatformError,
};
use std::collections::HashMap;

use super::{cocoa, keychain, notifications, system_tray};

/// macOS platform adapter
pub struct MacOSAdapter {
    initialized: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl MacOSAdapter {
    pub fn new() -> Self {
        Self {
            initialized: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Check if running on Apple Silicon
    pub fn is_apple_silicon() -> bool {
        #[cfg(target_arch = "aarch64")]
        {
            true
        }
        #[cfg(not(target_arch = "aarch64"))]
        {
            false
        }
    }

    /// Get macOS version
    pub fn get_macos_version() -> PlatformResult<String> {
        use std::process::Command;
        
        let output = Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .map_err(|e| PlatformError::DetectionError(format!("Failed to get macOS version: {}", e)))?;
        
        let version = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        
        Ok(version)
    }
}

impl Default for MacOSAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PlatformAdapter for MacOSAdapter {
    async fn initialize_platform(&self) -> PlatformResult<()> {
        // Check if already initialized
        if self.initialized.load(std::sync::atomic::Ordering::Relaxed) {
            return Ok(());
        }

        // Initialize Cocoa framework
        cocoa::initialize_cocoa()?;

        // Initialize notification center
        notifications::initialize_notification_center()?;

        // Initialize system tray if available
        if let Err(e) = system_tray::initialize_system_tray() {
            log::warn!("System tray initialization failed: {}", e);
            // Non-fatal, continue
        }

        // Mark as initialized
        self.initialized.store(true, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    async fn integrate_system_services(&self) -> PlatformResult<SystemServices> {
        let mut metadata = HashMap::new();
        
        // Add macOS version
        if let Ok(version) = Self::get_macos_version() {
            metadata.insert("macos_version".to_string(), version);
        }
        
        // Add architecture info
        metadata.insert(
            "architecture".to_string(),
            if Self::is_apple_silicon() {
                "apple_silicon".to_string()
            } else {
                "intel".to_string()
            }
        );

        // Check keychain availability
        let keychain_available = keychain::is_keychain_available();
        metadata.insert("keychain_available".to_string(), keychain_available.to_string());

        Ok(SystemServices {
            notifications: true,
            system_tray: true,
            file_manager: true,
            network_manager: true,
            metadata,
        })
    }

    async fn setup_ui_framework(&self) -> PlatformResult<UIFramework> {
        let version = Self::get_macos_version().unwrap_or_else(|_| "unknown".to_string());
        
        Ok(UIFramework {
            framework_type: GUIFramework::Native,
            version: format!("cocoa-{}", version),
            capabilities: vec![
                "appkit".to_string(),
                "metal".to_string(),
                "core_graphics".to_string(),
                "core_animation".to_string(),
                "notification_center".to_string(),
                "system_tray".to_string(),
            ],
        })
    }

    async fn configure_networking(&self) -> PlatformResult<NetworkConfig> {
        let mut config = NetworkConfig::default();
        config.preferred_protocols = vec!["quic".to_string(), "tcp".to_string()];
        config.max_connections = 200; // macOS can handle more connections
        Ok(config)
    }

    async fn setup_security_integration(&self) -> PlatformResult<SecurityConfig> {
        let mut config = SecurityConfig::default();
        config.use_keychain = keychain::is_keychain_available();
        config.use_hardware_crypto = Self::is_apple_silicon(); // Secure Enclave on Apple Silicon
        config.require_code_signing = true;
        config.sandbox_enabled = true; // macOS sandboxing
        Ok(config)
    }

    fn platform_name(&self) -> &str {
        "macos"
    }

    fn get_optimizations(&self) -> Vec<String> {
        let mut opts = vec![
            "grand_central_dispatch".to_string(),
            "metal_acceleration".to_string(),
            "core_graphics".to_string(),
        ];

        if Self::is_apple_silicon() {
            opts.push("arm_neon".to_string());
            opts.push("secure_enclave".to_string());
        }

        opts
    }
}
