// Platform type definitions

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Operating system identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperatingSystem {
    Linux,
    MacOS,
    Windows,
    Android,
    iOS,
    WebBrowser,
    Container,
    Unknown,
}

/// CPU architecture identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Architecture {
    X86_64,
    ARM64,
    ARM32,
    WASM32,
    Unknown,
}

/// Platform information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: OperatingSystem,
    pub architecture: Architecture,
    pub version: String,
    pub variant: Option<String>,
    pub capabilities: PlatformCapabilities,
}

/// GUI framework types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GUIFramework {
    Native,
    Web,
    CrossPlatform,
    None,
}

/// Hardware acceleration features
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HardwareFeature {
    SIMD,
    GPU,
    VideoCodec,
    AudioCodec,
    Crypto,
}

/// Network capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkCapabilities {
    pub tcp: bool,
    pub udp: bool,
    pub quic: bool,
    pub webrtc: bool,
    pub websocket: bool,
    pub mdns: bool,
    pub bluetooth: bool,
}

impl Default for NetworkCapabilities {
    fn default() -> Self {
        Self {
            tcp: true,
            udp: true,
            quic: true,
            webrtc: true,
            websocket: true,
            mdns: true,
            bluetooth: false,
        }
    }
}

/// Security capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCapabilities {
    pub keychain: bool,
    pub secure_enclave: bool,
    pub hardware_crypto: bool,
    pub sandboxing: bool,
    pub code_signing: bool,
}

impl Default for SecurityCapabilities {
    fn default() -> Self {
        Self {
            keychain: false,
            secure_enclave: false,
            hardware_crypto: false,
            sandboxing: false,
            code_signing: false,
        }
    }
}

/// Platform capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformCapabilities {
    pub gui_framework: Option<GUIFramework>,
    pub system_tray: bool,
    pub notifications: bool,
    pub file_associations: bool,
    pub auto_start: bool,
    pub hardware_acceleration: HashSet<HardwareFeature>,
    pub network_features: NetworkCapabilities,
    pub security_features: SecurityCapabilities,
}

impl Default for PlatformCapabilities {
    fn default() -> Self {
        Self {
            gui_framework: Some(GUIFramework::Native),
            system_tray: false,
            notifications: false,
            file_associations: false,
            auto_start: false,
            hardware_acceleration: HashSet::new(),
            network_features: NetworkCapabilities::default(),
            security_features: SecurityCapabilities::default(),
        }
    }
}

/// Feature identification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Feature {
    Clipboard,
    FileTransfer,
    Streaming,
    CommandExecution,
    Discovery,
    SystemTray,
    Notifications,
    AutoStart,
    FileAssociations,
}

/// Build target information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildTarget {
    pub platform: OperatingSystem,
    pub architecture: Architecture,
    pub environment: BuildEnvironment,
}

/// Build environment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildEnvironment {
    Native,
    Container,
    CrossCompile,
}

/// Optimization level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationLevel {
    Debug,
    Release,
    ReleaseWithDebug,
    MinSize,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    pub target: BuildTarget,
    pub optimization_level: OptimizationLevel,
    pub features: HashSet<String>,
    pub dependencies: Vec<String>,
    pub compiler_flags: Vec<String>,
    pub linker_flags: Vec<String>,
}

/// System services interface
#[derive(Debug)]
pub struct SystemServices {
    pub notifications: bool,
    pub system_tray: bool,
    pub file_manager: bool,
    pub network_manager: bool,
    pub metadata: HashMap<String, String>,
}

/// UI framework interface
#[derive(Debug)]
pub struct UIFramework {
    pub framework_type: GUIFramework,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub preferred_protocols: Vec<String>,
    pub fallback_enabled: bool,
    pub timeout_ms: u64,
    pub max_connections: usize,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            preferred_protocols: vec!["quic".to_string(), "tcp".to_string()],
            fallback_enabled: true,
            timeout_ms: 5000,
            max_connections: 100,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub use_keychain: bool,
    pub use_hardware_crypto: bool,
    pub require_code_signing: bool,
    pub sandbox_enabled: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            use_keychain: false,
            use_hardware_crypto: false,
            require_code_signing: false,
            sandbox_enabled: false,
        }
    }
}

/// Platform configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub enable_optimizations: bool,
    pub enable_hardware_acceleration: bool,
    pub network: NetworkConfig,
    pub security: SecurityConfig,
    pub custom_settings: HashMap<String, String>,
}

impl Default for PlatformConfig {
    fn default() -> Self {
        Self {
            enable_optimizations: true,
            enable_hardware_acceleration: true,
            network: NetworkConfig::default(),
            security: SecurityConfig::default(),
            custom_settings: HashMap::new(),
        }
    }
}
