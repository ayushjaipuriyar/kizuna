// Platform abstraction traits

use async_trait::async_trait;
use crate::platform::{
    PlatformResult, PlatformInfo, PlatformCapabilities, Feature,
    SystemServices, UIFramework, NetworkConfig, SecurityConfig, PlatformConfig,
};

/// Platform manager trait for runtime platform detection and management
#[async_trait]
pub trait PlatformManager: Send + Sync {
    /// Detect the current platform
    fn detect_platform(&self) -> PlatformResult<PlatformInfo>;

    /// Get platform capabilities
    fn get_capabilities(&self) -> PlatformResult<PlatformCapabilities>;

    /// Check if a feature is available on this platform
    fn is_feature_available(&self, feature: Feature) -> bool;

    /// Get the platform adapter for this platform
    fn get_platform_adapter(&self) -> PlatformResult<Box<dyn PlatformAdapter>>;

    /// Optimize configuration for the current platform
    fn optimize_for_platform(&self, config: &mut PlatformConfig) -> PlatformResult<()>;
}

/// Platform adapter trait for platform-specific implementations
#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    /// Initialize platform-specific components
    async fn initialize_platform(&self) -> PlatformResult<()>;

    /// Integrate with system services
    async fn integrate_system_services(&self) -> PlatformResult<SystemServices>;

    /// Setup UI framework
    async fn setup_ui_framework(&self) -> PlatformResult<UIFramework>;

    /// Configure networking for this platform
    async fn configure_networking(&self) -> PlatformResult<NetworkConfig>;

    /// Setup security integration
    async fn setup_security_integration(&self) -> PlatformResult<SecurityConfig>;

    /// Get platform name
    fn platform_name(&self) -> &str;

    /// Check if running in a container
    fn is_containerized(&self) -> bool {
        false
    }

    /// Get platform-specific optimizations
    fn get_optimizations(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Build system trait for cross-compilation
#[async_trait]
pub trait BuildSystem: Send + Sync {
    /// Configure build target
    async fn configure_target(&self, target: crate::platform::BuildTarget) 
        -> PlatformResult<crate::platform::BuildConfig>;

    /// Get supported targets
    fn supported_targets(&self) -> Vec<crate::platform::BuildTarget>;

    /// Validate build configuration
    fn validate_config(&self, config: &crate::platform::BuildConfig) -> PlatformResult<()>;
}

/// Deployment manager trait for packaging and distribution
#[async_trait]
pub trait DeploymentManager: Send + Sync {
    /// Create platform-specific package
    async fn create_package(&self, config: &crate::platform::BuildConfig) 
        -> PlatformResult<Vec<u8>>;

    /// Get package format name
    fn package_format(&self) -> &str;

    /// Validate package
    fn validate_package(&self, package: &[u8]) -> PlatformResult<()>;
}

/// Resource manager trait for platform-specific resource handling
#[async_trait]
pub trait ResourceManager: Send + Sync {
    /// Get available memory
    fn available_memory(&self) -> PlatformResult<u64>;

    /// Get CPU count
    fn cpu_count(&self) -> PlatformResult<usize>;

    /// Check battery status (for mobile platforms)
    fn battery_level(&self) -> PlatformResult<Option<f32>>;

    /// Check if on battery power
    fn is_on_battery(&self) -> PlatformResult<bool>;

    /// Get network status
    fn network_status(&self) -> PlatformResult<NetworkStatus>;
}

/// Network status information
#[derive(Debug, Clone)]
pub struct NetworkStatus {
    pub connected: bool,
    pub connection_type: ConnectionType,
    pub metered: bool,
}

/// Connection type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    Ethernet,
    WiFi,
    Cellular,
    Unknown,
}
