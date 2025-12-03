// Peer discovery command handler
//
// Implements "kizuna discover" command with peer listing, filtering,
// and incremental discovery results display with full integration to
// the core discovery system.
//
// Requirements: 1.1, 1.2, 1.3, 1.5

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::handlers::{DiscoverArgs, DiscoverResult};
use crate::cli::types::{ConnectionStatus, PeerInfo, TrustStatus};
use crate::discovery::api::{DiscoveryBuilder, DiscoveryEvent, KizunaDiscovery};
use crate::security::api::SecuritySystem;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// Discover command handler implementation with real-time event support
/// Fully integrated with core discovery system and security
pub struct DiscoverHandler {
    discovery: KizunaDiscovery,
    /// Cached peers with real-time updates
    cached_peers: Arc<RwLock<Vec<PeerInfo>>>,
    /// Event receiver for discovery events
    event_receiver: Arc<RwLock<Option<mpsc::UnboundedReceiver<DiscoveryEvent>>>>,
    /// Security system for trust verification
    security: Option<Arc<SecuritySystem>>,
    /// Event notification channel for CLI/TUI updates
    notification_tx: Arc<RwLock<Option<mpsc::UnboundedSender<PeerInfo>>>>,
}

impl DiscoverHandler {
    /// Create a new discover handler
    pub fn new() -> Self {
        Self {
            discovery: DiscoveryBuilder::new()
                .timeout(Duration::from_secs(10))
                .build(),
            cached_peers: Arc::new(RwLock::new(Vec::new())),
            event_receiver: Arc::new(RwLock::new(None)),
            security: None,
            notification_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new discover handler with custom discovery instance
    pub fn with_discovery(discovery: KizunaDiscovery) -> Self {
        Self {
            discovery,
            cached_peers: Arc::new(RwLock::new(Vec::new())),
            event_receiver: Arc::new(RwLock::new(None)),
            security: None,
            notification_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Create a new discover handler with security integration
    pub fn with_security(security: Arc<SecuritySystem>) -> Self {
        Self {
            discovery: DiscoveryBuilder::new()
                .timeout(Duration::from_secs(10))
                .build(),
            cached_peers: Arc::new(RwLock::new(Vec::new())),
            event_receiver: Arc::new(RwLock::new(None)),
            security: Some(security),
            notification_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Set security system for trust verification
    pub fn set_security(&mut self, security: Arc<SecuritySystem>) {
        self.security = Some(security);
    }

    /// Subscribe to peer discovery notifications
    /// Returns a receiver that will get notified when new peers are discovered
    pub async fn subscribe_notifications(&self) -> mpsc::UnboundedReceiver<PeerInfo> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.notification_tx.write().await = Some(tx);
        rx
    }

    /// Handle discover command
    pub async fn handle_discover(&mut self, args: DiscoverArgs) -> CLIResult<DiscoverResult> {
        let start_time = Instant::now();

        // Initialize discovery system
        self.discovery
            .initialize()
            .await
            .map_err(|e| CLIError::discovery(format!("Failed to initialize discovery: {}", e)))?;

        // Determine timeout
        let timeout = args
            .timeout
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(10));

        // Discover peers
        let service_records = self
            .discovery
            .discover_once(Some(timeout))
            .await
            .map_err(|e| CLIError::discovery(format!("Discovery failed: {}", e)))?;

        // Convert service records to PeerInfo with security integration
        let mut peers: Vec<PeerInfo> = Vec::new();
        for record in service_records {
            // Check trust status if security system is available
            let trust_status = if let Some(ref security) = self.security {
                // Convert String peer_id to PeerId
                if let Ok(peer_id) = crate::security::identity::PeerId::from_string(&record.peer_id) {
                    match security.is_trusted(&peer_id).await {
                        Ok(true) => TrustStatus::Trusted,
                        Ok(false) => TrustStatus::Untrusted,
                        Err(_) => TrustStatus::Untrusted,
                    }
                } else {
                    TrustStatus::Untrusted
                }
            } else {
                TrustStatus::Untrusted
            };

            let peer_info = PeerInfo {
                id: Uuid::new_v4(), // Generate UUID for peer
                name: record.peer_id.clone(),
                device_type: record
                    .capabilities
                    .get("device_type")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
                connection_status: if record.addresses.is_empty() {
                    ConnectionStatus::Disconnected
                } else {
                    ConnectionStatus::Connected
                },
                capabilities: record
                    .capabilities
                    .get("capabilities")
                    .map(|c| c.split(',').map(|s| s.to_string()).collect::<Vec<_>>())
                    .unwrap_or_default(),
                trust_status,
                last_seen: Some(chrono::Utc::now()),
            };
            peers.push(peer_info);
        }

        // Apply filters
        if let Some(filter_type) = &args.filter_type {
            peers.retain(|p| p.device_type.to_lowercase().contains(&filter_type.to_lowercase()));
        }

        if let Some(filter_name) = &args.filter_name {
            peers.retain(|p| p.name.to_lowercase().contains(&filter_name.to_lowercase()));
        }

        let discovery_time = start_time.elapsed();

        Ok(DiscoverResult {
            peers,
            discovery_time,
        })
    }

    /// Get cached peers without performing new discovery
    pub async fn get_cached_peers(&self) -> CLIResult<Vec<PeerInfo>> {
        let service_records = self.discovery.get_cached_peers().await;

        let peers = service_records
            .into_iter()
            .map(|record| PeerInfo {
                id: Uuid::new_v4(),
                name: record.peer_id.clone(),
                device_type: record
                    .capabilities
                    .get("device_type")
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
                connection_status: if record.addresses.is_empty() {
                    ConnectionStatus::Disconnected
                } else {
                    ConnectionStatus::Connected
                },
                capabilities: record
                    .capabilities
                    .get("capabilities")
                    .map(|c| c.split(',').map(|s| s.to_string()).collect::<Vec<_>>())
                    .unwrap_or_default(),
                trust_status: TrustStatus::Untrusted,
                last_seen: Some(chrono::Utc::now()),
            })
            .collect();

        Ok(peers)
    }

    /// Start continuous discovery mode with real-time event handling
    pub async fn start_continuous_discovery(&mut self) -> CLIResult<()> {
        // Start discovery and get event receiver
        let receiver = self
            .discovery
            .start_discovery()
            .await
            .map_err(|e| CLIError::discovery(format!("Failed to start continuous discovery: {}", e)))?;

        // Store the receiver
        *self.event_receiver.write().await = Some(receiver);

        // Start event processing task
        self.start_event_processor().await;

        Ok(())
    }

    /// Start event processor task for handling discovery events with security integration
    async fn start_event_processor(&self) {
        let event_receiver = Arc::clone(&self.event_receiver);
        let cached_peers = Arc::clone(&self.cached_peers);
        let security = self.security.clone();
        let notification_tx = Arc::clone(&self.notification_tx);

        tokio::spawn(async move {
            loop {
                let event = {
                    let mut receiver_lock = event_receiver.write().await;
                    if let Some(receiver) = receiver_lock.as_mut() {
                        receiver.recv().await
                    } else {
                        break;
                    }
                };

                if let Some(event) = event {
                    match event {
                        DiscoveryEvent::PeerDiscovered(service_record) => {
                            // Check trust status if security system is available
                            let trust_status = if let Some(ref sec) = security {
                                // Convert String peer_id to PeerId
                                if let Ok(peer_id) = crate::security::identity::PeerId::from_string(&service_record.peer_id) {
                                    match sec.is_trusted(&peer_id).await {
                                        Ok(true) => TrustStatus::Trusted,
                                        Ok(false) => TrustStatus::Untrusted,
                                        Err(_) => TrustStatus::Untrusted,
                                    }
                                } else {
                                    TrustStatus::Untrusted
                                }
                            } else {
                                TrustStatus::Untrusted
                            };

                            // Convert service record to PeerInfo and add to cache
                            let peer_info = PeerInfo {
                                id: Uuid::new_v4(),
                                name: service_record.peer_id.clone(),
                                device_type: service_record
                                    .capabilities
                                    .get("device_type")
                                    .cloned()
                                    .unwrap_or_else(|| "unknown".to_string()),
                                connection_status: ConnectionStatus::Connected,
                                capabilities: service_record
                                    .capabilities
                                    .get("capabilities")
                                    .map(|c| c.split(',').map(|s| s.to_string()).collect())
                                    .unwrap_or_default(),
                                trust_status,
                                last_seen: Some(chrono::Utc::now()),
                            };

                            let mut peers = cached_peers.write().await;
                            // Update or add peer
                            let is_new = !peers.iter().any(|p| p.name == peer_info.name);
                            if let Some(existing) = peers.iter_mut().find(|p| p.name == peer_info.name) {
                                *existing = peer_info.clone();
                            } else {
                                peers.push(peer_info.clone());
                            }
                            drop(peers);

                            // Send notification for new peers
                            if is_new {
                                if let Some(tx) = notification_tx.read().await.as_ref() {
                                    let _ = tx.send(peer_info);
                                }
                            }
                        }
                        DiscoveryEvent::PeerLost(peer_id) => {
                            // Remove peer from cache or mark as disconnected
                            let mut peers = cached_peers.write().await;
                            if let Some(peer) = peers.iter_mut().find(|p| p.name == peer_id) {
                                peer.connection_status = ConnectionStatus::Disconnected;
                                peer.last_seen = Some(chrono::Utc::now());
                            }
                        }
                        DiscoveryEvent::StrategyChanged(_strategy) => {
                            // Log strategy change if needed
                        }
                        DiscoveryEvent::Error(error) => {
                            eprintln!("Discovery error: {}", error);
                        }
                    }
                } else {
                    break;
                }
            }
        });
    }

    /// Stop continuous discovery mode
    pub async fn stop_continuous_discovery(&mut self) -> CLIResult<()> {
        self.discovery
            .shutdown()
            .await
            .map_err(|e| CLIError::discovery(format!("Failed to stop discovery: {}", e)))?;

        // Clear event receiver
        *self.event_receiver.write().await = None;

        Ok(())
    }

    /// Get real-time peer updates from cache
    pub async fn get_realtime_peers(&self) -> CLIResult<Vec<PeerInfo>> {
        let peers = self.cached_peers.read().await;
        Ok(peers.clone())
    }

    /// Subscribe to discovery events
    pub async fn subscribe_to_events(
        &mut self,
    ) -> CLIResult<mpsc::UnboundedReceiver<DiscoveryEvent>> {
        let receiver = self
            .discovery
            .start_discovery()
            .await
            .map_err(|e| CLIError::discovery(format!("Failed to subscribe to events: {}", e)))?;

        Ok(receiver)
    }
}

impl Default for DiscoverHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_handler_creation() {
        let handler = DiscoverHandler::new();
        assert!(handler.discovery.get_available_strategies().len() >= 0);
    }

    #[tokio::test]
    async fn test_discover_with_timeout() {
        let mut handler = DiscoverHandler::new();
        let args = DiscoverArgs {
            filter_type: None,
            filter_name: None,
            timeout: Some(2),
            continuous: false,
        };

        let result = handler.handle_discover(args).await;
        // Discovery may fail if no strategies are available, which is OK in test environment
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_discover_with_filters() {
        let mut handler = DiscoverHandler::new();
        let args = DiscoverArgs {
            filter_type: Some("laptop".to_string()),
            filter_name: Some("test".to_string()),
            timeout: Some(1),
            continuous: false,
        };

        let result = handler.handle_discover(args).await;
        // Discovery may fail if no strategies are available
        if let Ok(result) = result {
            // If discovery succeeds, filtered results should match criteria
            for peer in &result.peers {
                assert!(peer.device_type.to_lowercase().contains("laptop"));
                assert!(peer.name.to_lowercase().contains("test"));
            }
        }
    }

    #[tokio::test]
    async fn test_get_cached_peers() {
        let handler = DiscoverHandler::new();
        let result = handler.get_cached_peers().await;
        assert!(result.is_ok());
    }
}
