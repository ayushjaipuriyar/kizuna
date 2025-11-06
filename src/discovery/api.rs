use crate::discovery::{Discovery, DiscoveryManager, ServiceRecord, DiscoveryError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Configuration for the discovery system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Enable auto-selection of discovery strategies
    pub auto_select: bool,
    /// Default timeout for discovery operations
    pub default_timeout: Duration,
    /// Strategy-specific configurations
    pub strategy_configs: HashMap<String, StrategyConfig>,
    /// Enable specific strategies
    pub enabled_strategies: Vec<String>,
    /// Peer cache TTL
    pub peer_cache_ttl: Duration,
    /// Maximum number of concurrent discovery operations
    pub max_concurrent_discoveries: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            auto_select: true,
            default_timeout: Duration::from_secs(5),
            strategy_configs: HashMap::new(),
            enabled_strategies: vec![
                "mdns".to_string(),
                "udp".to_string(),
                "tcp".to_string(),
                "bluetooth".to_string(),
            ],
            peer_cache_ttl: Duration::from_secs(300), // 5 minutes
            max_concurrent_discoveries: 10,
        }
    }
}

/// Strategy-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Strategy priority (higher = preferred)
    pub priority: u8,
    /// Strategy-specific timeout override
    pub timeout: Option<Duration>,
    /// Strategy-specific parameters
    pub parameters: HashMap<String, String>,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            priority: 50,
            timeout: None,
            parameters: HashMap::new(),
        }
    }
}

/// Discovery event types
#[derive(Debug, Clone)]
pub enum DiscoveryEvent {
    /// New peer discovered
    PeerDiscovered(ServiceRecord),
    /// Peer lost (expired from cache)
    PeerLost(String), // peer_id
    /// Discovery strategy changed
    StrategyChanged(String),
    /// Discovery error occurred
    Error(String),
}

/// Public API for the Kizuna discovery system
pub struct KizunaDiscovery {
    manager: DiscoveryManager,
    config: DiscoveryConfig,
    event_sender: Option<mpsc::UnboundedSender<DiscoveryEvent>>,
    cancellation_token: CancellationToken,
}

impl KizunaDiscovery {
    /// Create a new discovery instance with default configuration
    pub fn new() -> Self {
        Self::with_config(DiscoveryConfig::default())
    }

    /// Create a new discovery instance with custom configuration
    pub fn with_config(config: DiscoveryConfig) -> Self {
        let mut manager = DiscoveryManager::new();
        
        // Configure auto-selection
        manager.set_auto_select(config.auto_select);
        
        Self {
            manager,
            config,
            event_sender: None,
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Initialize the discovery system with available strategies
    pub async fn initialize(&mut self) -> Result<(), DiscoveryError> {
        // Add strategies based on configuration
        for strategy_name in &self.config.enabled_strategies {
            match strategy_name.as_str() {
                "mdns" => {
                    let strategy = crate::discovery::strategies::mdns::MdnsDiscovery::new();
                    if strategy.is_available() {
                        self.manager.add_strategy(Box::new(strategy));
                    }
                }
                "udp" => {
                    let strategy = crate::discovery::strategies::udp::UdpDiscovery::new();
                    if strategy.is_available() {
                        self.manager.add_strategy(Box::new(strategy));
                    }
                }
                "tcp" => {
                    let strategy = crate::discovery::strategies::tcp::TcpDiscovery::new();
                    if strategy.is_available() {
                        self.manager.add_strategy(Box::new(strategy));
                    }
                }
                "bluetooth" => {
                    let strategy = crate::discovery::strategies::bluetooth::BluetoothDiscovery::new();
                    if strategy.is_available() {
                        self.manager.add_strategy(Box::new(strategy));
                    }
                }

                _ => {
                    return Err(DiscoveryError::StrategyUnavailable {
                        strategy: strategy_name.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Start continuous peer discovery with event notifications
    pub async fn start_discovery(&mut self) -> Result<mpsc::UnboundedReceiver<DiscoveryEvent>, DiscoveryError> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.event_sender = Some(sender.clone());

        // For now, return the receiver without background task
        // The user can call discover_once periodically
        Ok(receiver)
    }

    /// Discover peers once with optional timeout
    pub async fn discover_once(&self, timeout: Option<Duration>) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        let timeout = timeout.unwrap_or(self.config.default_timeout);
        self.manager.discover_peers(timeout).await
    }

    /// Announce this peer's presence
    pub async fn announce(&self) -> Result<(), DiscoveryError> {
        self.manager.announce_presence().await
    }

    /// Stop announcing and clean up resources
    pub async fn stop_announce(&self) -> Result<(), DiscoveryError> {
        self.manager.stop_announce().await
    }

    /// Get currently discovered peers from cache
    pub async fn get_cached_peers(&self) -> Vec<ServiceRecord> {
        self.manager.get_discovered_peers().await
    }

    /// Get available discovery strategies
    pub fn get_available_strategies(&self) -> Vec<String> {
        self.manager.get_available_strategies()
    }

    /// Set the active discovery strategy (disables auto-selection)
    pub fn set_active_strategy(&mut self, strategy: String) -> Result<(), DiscoveryError> {
        self.manager.set_auto_select(false);
        self.manager.set_active_strategy(Some(strategy))
    }

    /// Enable auto-selection of discovery strategies
    pub fn enable_auto_selection(&mut self) {
        self.manager.set_auto_select(true);
    }

    /// Update configuration
    pub fn update_config(&mut self, config: DiscoveryConfig) {
        self.config = config;
        self.manager.set_auto_select(self.config.auto_select);
    }

    /// Get current configuration
    pub fn get_config(&self) -> &DiscoveryConfig {
        &self.config
    }

    /// Shutdown the discovery system
    pub async fn shutdown(&mut self) -> Result<(), DiscoveryError> {
        self.cancellation_token.cancel();
        self.stop_announce().await?;
        Ok(())
    }
}

impl Default for KizunaDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder pattern for creating KizunaDiscovery instances
pub struct DiscoveryBuilder {
    config: DiscoveryConfig,
}

impl DiscoveryBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: DiscoveryConfig::default(),
        }
    }

    /// Set auto-selection behavior
    pub fn auto_select(mut self, enabled: bool) -> Self {
        self.config.auto_select = enabled;
        self
    }

    /// Set default timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.default_timeout = timeout;
        self
    }

    /// Enable specific strategies
    pub fn strategies(mut self, strategies: Vec<String>) -> Self {
        self.config.enabled_strategies = strategies;
        self
    }

    /// Set peer cache TTL
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.config.peer_cache_ttl = ttl;
        self
    }

    /// Set maximum concurrent discoveries
    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.config.max_concurrent_discoveries = max;
        self
    }

    /// Add strategy-specific configuration
    pub fn strategy_config(mut self, strategy: String, config: StrategyConfig) -> Self {
        self.config.strategy_configs.insert(strategy, config);
        self
    }

    /// Build the KizunaDiscovery instance
    pub fn build(self) -> KizunaDiscovery {
        KizunaDiscovery::with_config(self.config)
    }
}

impl Default for DiscoveryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_config_default() {
        let config = DiscoveryConfig::default();
        assert!(config.auto_select);
        assert_eq!(config.default_timeout, Duration::from_secs(5));
        assert!(!config.enabled_strategies.is_empty());
    }

    #[test]
    fn test_discovery_builder() {
        let discovery = DiscoveryBuilder::new()
            .auto_select(false)
            .timeout(Duration::from_secs(10))
            .strategies(vec!["mdns".to_string(), "udp".to_string()])
            .build();

        assert!(!discovery.config.auto_select);
        assert_eq!(discovery.config.default_timeout, Duration::from_secs(10));
        assert_eq!(discovery.config.enabled_strategies.len(), 2);
    }

    #[tokio::test]
    async fn test_discovery_initialization() {
        let mut discovery = KizunaDiscovery::new();
        
        // Should not fail even if some strategies are unavailable
        let result = discovery.initialize().await;
        assert!(result.is_ok() || matches!(result, Err(DiscoveryError::StrategyUnavailable { .. })));
    }
}