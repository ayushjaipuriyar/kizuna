use async_trait::async_trait;
use std::time::Duration;

pub mod error;
pub mod service_record;
pub mod manager;
pub mod strategies;
pub mod api;
pub mod cli;
pub mod config;

// Re-export legacy modules for backward compatibility
pub mod udp {
    pub use super::strategies::udp::*;
}
pub mod mdns {
    pub use super::strategies::mdns::*;
}

pub use error::DiscoveryError;
pub use service_record::ServiceRecord;
pub use manager::DiscoveryManager;
pub use api::{KizunaDiscovery, DiscoveryConfig, DiscoveryBuilder, DiscoveryEvent};
pub use cli::DiscoveryCli;
pub use config::{DiscoveryConfigFile, ConfigManager};

// Keep the legacy Peer struct for backward compatibility
#[derive(Debug, Clone)]
pub struct Peer {
    pub id: String,
    pub addr: String,
    pub port: u16,
}

/// Enhanced Discovery trait with comprehensive functionality
#[async_trait]
pub trait Discovery: Send + Sync {
    /// Discover peers using this strategy
    async fn discover(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError>;
    
    /// Announce this peer's presence
    async fn announce(&self) -> Result<(), DiscoveryError>;
    
    /// Stop announcing and clean up resources
    async fn stop_announce(&self) -> Result<(), DiscoveryError>;
    
    /// Get the strategy name for logging/debugging
    fn strategy_name(&self) -> &'static str;
    
    /// Check if this strategy is available on the current platform
    fn is_available(&self) -> bool;
    
    /// Get the priority of this strategy (higher = preferred)
    fn priority(&self) -> u8;

    // Legacy methods for backward compatibility
    async fn browse(&self) -> anyhow::Result<Vec<Peer>> {
        let timeout = Duration::from_secs(5);
        match self.discover(timeout).await {
            Ok(records) => Ok(records.into_iter().map(|r| r.into()).collect()),
            Err(e) => Err(anyhow::anyhow!("Discovery failed: {}", e)),
        }
    }
}

/// Legacy discovery selector function - now uses the enhanced DiscoveryManager
pub async fn discovery_selector() -> String {
    // For backward compatibility, return "mdns" as default
    // In practice, users should use DiscoveryManager for auto-selection
    "mdns".to_string()
}
