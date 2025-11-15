/// Plugin hook system for extensibility
use super::PluginContext;
use crate::developer_api::core::KizunaError;
use async_trait::async_trait;

/// Types of hooks available in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HookType {
    /// Discovery hook for custom peer discovery
    Discovery,
    
    /// Connection hook for custom connection handling
    Connection,
    
    /// Transfer hook for custom file transfer handling
    Transfer,
    
    /// Stream hook for custom media streaming
    Stream,
    
    /// Command hook for custom command execution
    Command,
}

/// Plugin hook trait for implementing extension points
#[async_trait]
pub trait PluginHook<T>: Send + Sync {
    /// Executes the hook with the given context and data
    async fn execute(&self, context: &PluginContext, data: T) -> Result<T, KizunaError>;
}

/// Discovery plugin trait for custom discovery strategies
#[async_trait]
pub trait DiscoveryPlugin: Send + Sync {
    /// Discovers peers using the custom strategy
    async fn discover(&self) -> Result<Vec<crate::developer_api::core::events::PeerInfo>, KizunaError>;
    
    /// Checks if the plugin supports the given network type
    fn supports_network(&self, network_type: &str) -> bool;
}
