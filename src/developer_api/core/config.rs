/// Configuration types for the Developer API
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Main configuration for Kizuna API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KizunaConfig {
    /// Identity configuration
    pub identity: Option<IdentityConfig>,
    
    /// Discovery configuration
    pub discovery: DiscoveryConfig,
    
    /// Security configuration
    pub security: SecurityConfig,
    
    /// Networking configuration
    pub networking: NetworkConfig,
    
    /// Plugin configurations
    pub plugins: Vec<PluginConfig>,
    
    /// Number of runtime worker threads (None = default based on CPU cores)
    pub runtime_threads: Option<usize>,
    
    // === System Integration Configuration ===
    
    /// Enable discovery system
    pub enable_discovery: bool,
    
    /// Enable transport system
    pub enable_transport: bool,
    
    /// Enable security system (always recommended)
    pub enable_security: bool,
    
    /// Enable file transfer system
    pub enable_file_transfer: bool,
    
    /// Enable streaming system
    pub enable_streaming: bool,
    
    /// Enable clipboard system
    pub enable_clipboard: bool,
    
    /// Enable command execution system
    pub enable_command_execution: bool,
    
    /// Discovery strategies to enable
    pub discovery_strategies: Vec<String>,
    
    /// Transport protocols to enable
    pub transport_protocols: Vec<String>,
    
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    
    /// Security session timeout in seconds
    pub security_session_timeout_secs: u64,
    
    /// File transfer session directory
    pub file_transfer_session_dir: PathBuf,
}

impl Default for KizunaConfig {
    fn default() -> Self {
        Self {
            identity: None,
            discovery: DiscoveryConfig::default(),
            security: SecurityConfig::default(),
            networking: NetworkConfig::default(),
            plugins: Vec::new(),
            runtime_threads: None,
            enable_discovery: true,
            enable_transport: true,
            enable_security: true,
            enable_file_transfer: true,
            enable_streaming: true,
            enable_clipboard: true,
            enable_command_execution: true,
            discovery_strategies: vec![
                "mdns".to_string(),
                "udp".to_string(),
                "tcp".to_string(),
            ],
            transport_protocols: vec![
                "tcp".to_string(),
                "quic".to_string(),
                "webrtc".to_string(),
                "websocket".to_string(),
            ],
            connection_timeout_secs: 30,
            security_session_timeout_secs: 3600,
            file_transfer_session_dir: {
                let mut path = std::env::temp_dir();
                path.push("kizuna");
                path.push("file_transfer");
                path
            },
        }
    }
}

/// Identity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Device name
    pub device_name: String,
    
    /// User name
    pub user_name: Option<String>,
    
    /// Identity file path
    pub identity_path: Option<PathBuf>,
}

/// Discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Enable mDNS discovery
    pub enable_mdns: bool,
    
    /// Enable UDP broadcast discovery
    pub enable_udp: bool,
    
    /// Enable Bluetooth discovery
    pub enable_bluetooth: bool,
    
    /// Discovery interval in seconds
    pub interval_secs: u64,
    
    /// Discovery timeout in seconds
    pub timeout_secs: u64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_mdns: true,
            enable_udp: true,
            enable_bluetooth: false,
            interval_secs: 5,
            timeout_secs: 30,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable encryption
    pub enable_encryption: bool,
    
    /// Require authentication
    pub require_authentication: bool,
    
    /// Trust mode
    pub trust_mode: TrustMode,
    
    /// Key storage path
    pub key_storage_path: Option<PathBuf>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_encryption: true,
            require_authentication: true,
            trust_mode: TrustMode::Manual,
            key_storage_path: None,
        }
    }
}

/// Trust mode for peer connections
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TrustMode {
    /// Trust all peers automatically
    TrustAll,
    
    /// Require manual approval for each peer
    Manual,
    
    /// Trust only allowlisted peers
    AllowlistOnly,
}

/// Networking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen port
    pub listen_port: Option<u16>,
    
    /// Enable IPv6
    pub enable_ipv6: bool,
    
    /// Enable QUIC transport
    pub enable_quic: bool,
    
    /// Enable WebRTC transport
    pub enable_webrtc: bool,
    
    /// Enable WebSocket transport
    pub enable_websocket: bool,
    
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_port: None,
            enable_ipv6: true,
            enable_quic: true,
            enable_webrtc: true,
            enable_websocket: true,
            connection_timeout_secs: 30,
        }
    }
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Plugin name
    pub name: String,
    
    /// Plugin library path
    pub path: PathBuf,
    
    /// Whether the plugin is enabled
    pub enabled: bool,
    
    /// Plugin-specific configuration
    pub config: HashMap<String, serde_json::Value>,
}

impl KizunaConfig {
    /// Creates a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Sets the identity configuration
    pub fn with_identity(mut self, identity: IdentityConfig) -> Self {
        self.identity = Some(identity);
        self
    }
    
    /// Sets the discovery configuration
    pub fn with_discovery(mut self, discovery: DiscoveryConfig) -> Self {
        self.discovery = discovery;
        self
    }
    
    /// Sets the security configuration
    pub fn with_security(mut self, security: SecurityConfig) -> Self {
        self.security = security;
        self
    }
    
    /// Sets the networking configuration
    pub fn with_networking(mut self, networking: NetworkConfig) -> Self {
        self.networking = networking;
        self
    }
    
    /// Adds a plugin configuration
    pub fn with_plugin(mut self, plugin: PluginConfig) -> Self {
        self.plugins.push(plugin);
        self
    }
    
    /// Validates the configuration
    pub fn validate(&self) -> Result<(), String> {
        // Validate discovery configuration
        if !self.discovery.enable_mdns 
            && !self.discovery.enable_udp 
            && !self.discovery.enable_bluetooth {
            return Err("At least one discovery method must be enabled".to_string());
        }
        
        // Validate networking configuration
        if !self.networking.enable_quic 
            && !self.networking.enable_webrtc 
            && !self.networking.enable_websocket {
            return Err("At least one transport protocol must be enabled".to_string());
        }
        
        Ok(())
    }
}
