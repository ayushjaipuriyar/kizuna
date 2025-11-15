use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::discovery::{DiscoveryManager, ServiceRecord, Discovery, DiscoveryError};
use crate::transport::{
    KizunaTransport, KizunaTransportConfig, ConnectionHandle, ConnectionCallback, 
    ConnectionEvent, PeerAddress, TransportCapabilities, PeerId, TransportError
};

/// Configuration for transport-discovery integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportDiscoveryConfig {
    /// Enable automatic connection establishment when peers are discovered
    pub auto_connect: bool,
    /// Maximum number of automatic connections per peer
    pub max_auto_connections: usize,
    /// Timeout for automatic connection attempts
    pub auto_connect_timeout: Duration,
    /// Protocols to advertise during discovery
    pub advertised_protocols: Vec<String>,
    /// Transport capabilities to advertise
    pub advertised_capabilities: TransportCapabilities,
    /// Enable transport capability exchange during discovery
    pub enable_capability_exchange: bool,
    /// Retry failed automatic connections
    pub retry_failed_connections: bool,
    /// Maximum retry attempts for automatic connections
    pub max_retry_attempts: u32,
    /// Delay between retry attempts
    pub retry_delay: Duration,
}

impl Default for TransportDiscoveryConfig {
    fn default() -> Self {
        Self {
            auto_connect: true,
            max_auto_connections: 3,
            auto_connect_timeout: Duration::from_secs(30),
            advertised_protocols: vec![
                "tcp".to_string(),
                "quic".to_string(),
                "webrtc".to_string(),
                "websocket".to_string(),
            ],
            advertised_capabilities: TransportCapabilities {
                reliable: true,
                ordered: true,
                multiplexed: true,
                resumable: true,
                nat_traversal: true,
                max_message_size: None,
            },
            enable_capability_exchange: true,
            retry_failed_connections: true,
            max_retry_attempts: 3,
            retry_delay: Duration::from_secs(5),
        }
    }
}

/// Events related to transport-discovery integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransportDiscoveryEvent {
    /// Peer discovered through discovery layer
    PeerDiscovered {
        peer_id: PeerId,
        service_record: ServiceRecord,
        discovery_method: String,
    },
    /// Automatic connection attempt started
    AutoConnectStarted {
        peer_id: PeerId,
        protocol: String,
        attempt: u32,
    },
    /// Automatic connection succeeded
    AutoConnectSucceeded {
        peer_id: PeerId,
        protocol: String,
        connection_info: String,
    },
    /// Automatic connection failed
    AutoConnectFailed {
        peer_id: PeerId,
        protocol: String,
        error: String,
        will_retry: bool,
    },
    /// Transport capabilities exchanged
    CapabilitiesExchanged {
        peer_id: PeerId,
        local_capabilities: TransportCapabilities,
        remote_capabilities: TransportCapabilities,
    },
    /// Peer lost (no longer discoverable)
    PeerLost {
        peer_id: PeerId,
        reason: String,
    },
}

/// Callback trait for transport-discovery integration events
#[async_trait]
pub trait TransportDiscoveryCallback: Send + Sync {
    /// Called when a transport-discovery event occurs
    async fn on_transport_discovery_event(&self, event: TransportDiscoveryEvent);
    
    /// Called when a peer is discovered and before automatic connection
    /// Return false to prevent automatic connection
    async fn should_auto_connect(&self, peer_id: &PeerId, service_record: &ServiceRecord) -> bool;
    
    /// Called to select which protocol to use for automatic connection
    /// Return None to use default protocol negotiation
    async fn select_protocol(&self, peer_id: &PeerId, available_protocols: &[String]) -> Option<String>;
}

/// Integrated transport and discovery system
pub struct TransportDiscoveryBridge {
    transport: Arc<KizunaTransport>,
    discovery: Arc<RwLock<DiscoveryManager>>,
    config: TransportDiscoveryConfig,
    active_connections: Arc<RwLock<HashMap<PeerId, Vec<ConnectionHandle>>>>,
    discovered_peers: Arc<RwLock<HashMap<PeerId, ServiceRecord>>>,
    event_sender: mpsc::UnboundedSender<TransportDiscoveryEvent>,
    event_receiver: Arc<RwLock<mpsc::UnboundedReceiver<TransportDiscoveryEvent>>>,
    callbacks: Arc<RwLock<Vec<Arc<dyn TransportDiscoveryCallback>>>>,
    is_running: Arc<RwLock<bool>>,
}

impl TransportDiscoveryBridge {
    /// Create a new transport-discovery bridge
    pub async fn new(
        transport_config: KizunaTransportConfig,
        integration_config: TransportDiscoveryConfig,
    ) -> Result<Self, TransportError> {
        let transport = Arc::new(KizunaTransport::with_config(transport_config).await?);
        let discovery = Arc::new(RwLock::new(DiscoveryManager::new()));
        
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        Ok(Self {
            transport,
            discovery,
            config: integration_config,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            discovered_peers: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            event_receiver: Arc::new(RwLock::new(event_receiver)),
            callbacks: Arc::new(RwLock::new(Vec::new())),
            is_running: Arc::new(RwLock::new(false)),
        })
    }
    
    /// Register a callback for transport-discovery events
    pub async fn register_callback(&self, callback: Arc<dyn TransportDiscoveryCallback>) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(callback);
    }
    
    /// Start the integrated system
    pub async fn start(&self, bind_address: SocketAddr) -> Result<(), TransportError> {
        {
            let mut is_running = self.is_running.write().await;
            if *is_running {
                return Err(TransportError::Configuration("Bridge already running".to_string()));
            }
            *is_running = true;
        }
        
        // Start transport system
        self.transport.start_listening(bind_address).await?;
        
        // Register transport callback to handle connection events
        let bridge_callback = Arc::new(BridgeConnectionCallback {
            event_sender: self.event_sender.clone(),
        });
        self.transport.register_callback(bridge_callback).await;
        
        // Start discovery system with all available strategies
        self.discovery.write().await.register_all_strategies().await
            .map_err(|e| TransportError::Configuration(format!("Failed to register discovery strategies: {}", e)))?;
        
        // Start discovery loop
        self.start_discovery_loop().await;
        
        // Start event processing
        self.start_event_processing().await;
        
        // Start peer monitoring
        self.start_peer_monitoring().await;
        
        Ok(())
    }
    
    /// Stop the integrated system
    pub async fn stop(&self) -> Result<(), TransportError> {
        {
            let mut is_running = self.is_running.write().await;
            if !*is_running {
                return Ok(());
            }
            *is_running = false;
        }
        
        // Stop transport system
        self.transport.stop_listening().await?;
        
        // Disconnect all peers
        self.transport.disconnect_all().await?;
        
        // Stop discovery announcements
        self.discovery.write().await.stop_announce().await
            .map_err(|e| TransportError::Configuration(format!("Failed to stop discovery: {}", e)))?;
        
        Ok(())
    }
    
    /// Manually connect to a discovered peer
    pub async fn connect_to_peer(&self, peer_id: &PeerId) -> Result<ConnectionHandle, TransportError> {
        let peer_address = {
            let peers = self.discovered_peers.read().await;
            let service_record = peers.get(peer_id)
                .ok_or_else(|| TransportError::Configuration(format!("Peer {} not discovered", peer_id)))?;
            
            self.service_record_to_peer_address(service_record)
        };
        
        let handle = self.transport.connect_to_peer(&peer_address).await?;
        
        // Store connection
        {
            let mut connections = self.active_connections.write().await;
            connections.entry(peer_id.clone())
                .or_insert_with(Vec::new)
                .push(handle.clone());
        }
        
        Ok(handle)
    }
    
    /// Get all discovered peers
    pub async fn get_discovered_peers(&self) -> Vec<ServiceRecord> {
        let peers = self.discovered_peers.read().await;
        peers.values().cloned().collect()
    }
    
    /// Get active connections for a peer
    pub async fn get_connections(&self, peer_id: &PeerId) -> Vec<ConnectionHandle> {
        let connections = self.active_connections.read().await;
        connections.get(peer_id).cloned().unwrap_or_default()
    }
    
    /// Get all active peer IDs
    pub async fn get_active_peers(&self) -> Vec<PeerId> {
        let connections = self.active_connections.read().await;
        connections.keys().cloned().collect()
    }
    
    /// Announce this peer's presence with transport capabilities
    pub async fn announce_presence(&self, device_name: String, port: u16) -> Result<(), TransportError> {
        // Create service record with transport capabilities
        let mut service_record = ServiceRecord::new(
            format!("kizuna-{}", uuid::Uuid::new_v4()),
            device_name,
            port,
        );
        
        // Add transport capabilities
        for protocol in &self.config.advertised_protocols {
            service_record.add_capability("transport_protocol".to_string(), protocol.clone());
        }
        
        // Add capability details
        if self.config.advertised_capabilities.reliable {
            service_record.add_capability("reliable".to_string(), "true".to_string());
        }
        if self.config.advertised_capabilities.ordered {
            service_record.add_capability("ordered".to_string(), "true".to_string());
        }
        if self.config.advertised_capabilities.multiplexed {
            service_record.add_capability("multiplexed".to_string(), "true".to_string());
        }
        if self.config.advertised_capabilities.resumable {
            service_record.add_capability("resumable".to_string(), "true".to_string());
        }
        if self.config.advertised_capabilities.nat_traversal {
            service_record.add_capability("nat_traversal".to_string(), "true".to_string());
        }
        
        // Start discovery announcements
        self.discovery.write().await.announce_presence().await
            .map_err(|e| TransportError::Configuration(format!("Failed to announce presence: {}", e)))?;
        
        Ok(())
    }
    
    /// Start continuous discovery loop
    async fn start_discovery_loop(&self) {
        let discovery = self.discovery.clone();
        let event_sender = self.event_sender.clone();
        let discovered_peers = Arc::clone(&self.discovered_peers);
        let is_running = Arc::clone(&self.is_running);
        let config = self.config.clone();
        let callbacks = Arc::clone(&self.callbacks);
        let active_connections = Arc::clone(&self.active_connections);
        let transport = self.transport.clone();
        
        tokio::spawn(async move {
            let mut discovery_interval = tokio::time::interval(Duration::from_secs(30));
            
            while *is_running.read().await {
                discovery_interval.tick().await;
                
                // Discover peers
                match discovery.write().await.discover_peers(Duration::from_secs(10)).await {
                    Ok(peers) => {
                        for service_record in peers {
                            let peer_id = service_record.peer_id.clone();
                            
                            // Check if this is a new peer
                            let is_new_peer = {
                                let mut peers_map = discovered_peers.write().await;
                                let is_new = !peers_map.contains_key(&peer_id);
                                peers_map.insert(peer_id.clone(), service_record.clone());
                                is_new
                            };
                            
                            if is_new_peer {
                                // Send discovery event
                                let _ = event_sender.send(TransportDiscoveryEvent::PeerDiscovered {
                                    peer_id: peer_id.clone(),
                                    service_record: service_record.clone(),
                                    discovery_method: service_record.discovery_method.clone(),
                                });
                                
                                // Check if we should auto-connect
                                if config.auto_connect {
                                    let should_connect = {
                                        let callbacks_guard = callbacks.read().await;
                                        if callbacks_guard.is_empty() {
                                            true // Default to auto-connect if no callbacks
                                        } else {
                                            // Check with all callbacks
                                            let mut should_connect = true;
                                            for callback in callbacks_guard.iter() {
                                                if !callback.should_auto_connect(&peer_id, &service_record).await {
                                                    should_connect = false;
                                                    break;
                                                }
                                            }
                                            should_connect
                                        }
                                    };
                                    
                                    if should_connect {
                                        // Start automatic connection
                                        Self::start_auto_connect(
                                            transport.clone(),
                                            active_connections.clone(),
                                            event_sender.clone(),
                                            callbacks.clone(),
                                            config.clone(),
                                            service_record,
                                        ).await;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Discovery failed: {}", e);
                    }
                }
            }
        });
    }
    
    /// Start automatic connection to a discovered peer
    async fn start_auto_connect(
        transport: Arc<KizunaTransport>,
        active_connections: Arc<RwLock<HashMap<PeerId, Vec<ConnectionHandle>>>>,
        event_sender: mpsc::UnboundedSender<TransportDiscoveryEvent>,
        callbacks: Arc<RwLock<Vec<Arc<dyn TransportDiscoveryCallback>>>>,
        config: TransportDiscoveryConfig,
        service_record: ServiceRecord,
    ) {
        let peer_id = service_record.peer_id.clone();
        
        tokio::spawn(async move {
            // Check if we already have enough connections to this peer
            {
                let connections = active_connections.read().await;
                if let Some(peer_connections) = connections.get(&peer_id) {
                    if peer_connections.len() >= config.max_auto_connections {
                        return; // Already have enough connections
                    }
                }
            }
            
            // Convert service record to peer address
            let peer_address = Self::service_record_to_peer_address_static(&service_record);
            
            // Select protocol
            let selected_protocol = {
                let callbacks_guard = callbacks.read().await;
                let mut selected = None;
                
                for callback in callbacks_guard.iter() {
                    if let Some(protocol) = callback.select_protocol(&peer_id, &config.advertised_protocols).await {
                        selected = Some(protocol);
                        break;
                    }
                }
                
                selected
            };
            
            // Attempt connection with retries
            for attempt in 1..=config.max_retry_attempts {
                let protocol = selected_protocol.as_deref().unwrap_or("auto");
                
                // Send auto-connect started event
                let _ = event_sender.send(TransportDiscoveryEvent::AutoConnectStarted {
                    peer_id: peer_id.clone(),
                    protocol: protocol.to_string(),
                    attempt,
                });
                
                // Attempt connection
                let connection_result = if let Some(specific_protocol) = &selected_protocol {
                    transport.connect_with_protocol(&peer_address, specific_protocol).await
                } else {
                    transport.connect_to_peer(&peer_address).await
                };
                
                match connection_result {
                    Ok(handle) => {
                        // Store connection
                        {
                            let mut connections = active_connections.write().await;
                            connections.entry(peer_id.clone())
                                .or_insert_with(Vec::new)
                                .push(handle.clone());
                        }
                        
                        // Send success event
                        let connection_info = handle.info().await;
                        let _ = event_sender.send(TransportDiscoveryEvent::AutoConnectSucceeded {
                            peer_id: peer_id.clone(),
                            protocol: connection_info.protocol.clone(),
                            connection_info: format!("{}:{}", connection_info.remote_addr.ip(), connection_info.remote_addr.port()),
                        });
                        
                        return; // Success, exit retry loop
                    }
                    Err(e) => {
                        let will_retry = attempt < config.max_retry_attempts;
                        
                        // Send failure event
                        let _ = event_sender.send(TransportDiscoveryEvent::AutoConnectFailed {
                            peer_id: peer_id.clone(),
                            protocol: protocol.to_string(),
                            error: e.to_string(),
                            will_retry,
                        });
                        
                        if will_retry {
                            tokio::time::sleep(config.retry_delay).await;
                        }
                    }
                }
            }
        });
    }
    
    /// Start event processing task
    /// Note: This method is currently disabled due to lifetime issues with spawning tasks
    async fn start_event_processing(&self) {
        // TODO: Implement event processing without lifetime issues
        // See transport/api.rs for details on the issue and possible solutions
    }
    
    /// Start peer monitoring task to detect lost peers
    async fn start_peer_monitoring(&self) {
        let discovered_peers = Arc::clone(&self.discovered_peers);
        let active_connections = Arc::clone(&self.active_connections);
        let event_sender = self.event_sender.clone();
        let is_running = Arc::clone(&self.is_running);
        
        tokio::spawn(async move {
            let mut monitor_interval = tokio::time::interval(Duration::from_secs(60));
            
            while *is_running.read().await {
                monitor_interval.tick().await;
                
                // Check for expired peers
                let expired_peers = {
                    let mut peers = discovered_peers.write().await;
                    let mut expired = Vec::new();
                    
                    peers.retain(|peer_id, service_record| {
                        if service_record.is_expired(Duration::from_secs(300)) { // 5 minute TTL
                            expired.push(peer_id.clone());
                            false
                        } else {
                            true
                        }
                    });
                    
                    expired
                };
                
                // Handle expired peers
                for peer_id in expired_peers {
                    // Send peer lost event
                    let _ = event_sender.send(TransportDiscoveryEvent::PeerLost {
                        peer_id: peer_id.clone(),
                        reason: "Discovery timeout".to_string(),
                    });
                    
                    // Close connections to lost peers
                    if let Some(connections) = active_connections.write().await.remove(&peer_id) {
                        for handle in connections {
                            let _ = handle.close().await;
                        }
                    }
                }
            }
        });
    }
    
    /// Convert ServiceRecord to PeerAddress
    fn service_record_to_peer_address(&self, service_record: &ServiceRecord) -> PeerAddress {
        Self::service_record_to_peer_address_static(service_record)
    }
    
    /// Static version of service_record_to_peer_address for use in async tasks
    fn service_record_to_peer_address_static(service_record: &ServiceRecord) -> PeerAddress {
        let mut transport_hints = Vec::new();
        let mut capabilities = TransportCapabilities::default();
        
        // Extract transport protocols from capabilities
        for (key, value) in &service_record.capabilities {
            if key == "transport_protocol" {
                transport_hints.push(value.clone());
            } else if key == "reliable" && value == "true" {
                capabilities.reliable = true;
            } else if key == "ordered" && value == "true" {
                capabilities.ordered = true;
            } else if key == "multiplexed" && value == "true" {
                capabilities.multiplexed = true;
            } else if key == "resumable" && value == "true" {
                capabilities.resumable = true;
            } else if key == "nat_traversal" && value == "true" {
                capabilities.nat_traversal = true;
            }
        }
        
        PeerAddress::new(
            service_record.peer_id.clone(),
            service_record.addresses.clone(),
            transport_hints,
            capabilities,
        )
    }
    
    /// Get integration statistics
    pub async fn get_integration_stats(&self) -> IntegrationStats {
        let discovered_peers = self.discovered_peers.read().await;
        let active_connections = self.active_connections.read().await;
        
        let total_discovered = discovered_peers.len();
        let total_connections: usize = active_connections.values().map(|v| v.len()).sum();
        let connected_peers = active_connections.len();
        
        // Calculate connection success rate
        let connection_success_rate = if total_discovered > 0 {
            connected_peers as f64 / total_discovered as f64
        } else {
            0.0
        };
        
        IntegrationStats {
            total_discovered_peers: total_discovered,
            connected_peers,
            total_connections,
            connection_success_rate,
            auto_connect_enabled: self.config.auto_connect,
        }
    }
}
// Note: KizunaTransport Clone implementation removed as it requires access to private fields
// If cloning is needed, it should be implemented in the api module where the struct is defined

impl Clone for DiscoveryManager {
    fn clone(&self) -> Self {
        // This is a simplified clone - in a real implementation, 
        // this would need proper cloning of the discovery manager state
        DiscoveryManager::new()
    }
}

/// Bridge connection callback to forward transport events
struct BridgeConnectionCallback {
    event_sender: mpsc::UnboundedSender<TransportDiscoveryEvent>,
}

#[async_trait]
impl ConnectionCallback for BridgeConnectionCallback {
    async fn on_connection_event(&self, event: ConnectionEvent) {
        // Convert transport events to bridge events if needed
        // For now, we just log them
        match event {
            ConnectionEvent::Connected { peer_id, protocol, connection_info } => {
                println!("Bridge: Connection established to {} via {}", peer_id, protocol);
            }
            ConnectionEvent::Disconnected { peer_id, reason } => {
                println!("Bridge: Connection to {} lost: {}", peer_id, reason);
            }
            _ => {}
        }
    }
    
    async fn on_connection_quality_change(&self, peer_id: PeerId, quality: crate::transport::ConnectionQuality) {
        println!("Bridge: Connection quality changed for {}: {:?}", peer_id, quality.quality_class);
    }
    
    async fn on_error(&self, error: TransportError, context: String) {
        eprintln!("Bridge: Transport error in {}: {}", context, error);
    }
}

/// Integration statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationStats {
    /// Total number of discovered peers
    pub total_discovered_peers: usize,
    /// Number of peers with active connections
    pub connected_peers: usize,
    /// Total number of active connections
    pub total_connections: usize,
    /// Connection success rate (connected_peers / total_discovered_peers)
    pub connection_success_rate: f64,
    /// Whether auto-connect is enabled
    pub auto_connect_enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_bridge_creation() {
        let transport_config = KizunaTransportConfig::default();
        let integration_config = TransportDiscoveryConfig::default();
        
        let bridge = TransportDiscoveryBridge::new(transport_config, integration_config).await;
        assert!(bridge.is_ok());
    }

    #[tokio::test]
    async fn test_service_record_conversion() {
        let mut service_record = ServiceRecord::new(
            "test-peer".to_string(),
            "Test Device".to_string(),
            8080,
        );
        service_record.add_address(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));
        service_record.add_capability("transport_protocol".to_string(), "tcp".to_string());
        service_record.add_capability("reliable".to_string(), "true".to_string());
        
        let peer_address = TransportDiscoveryBridge::service_record_to_peer_address_static(&service_record);
        
        assert_eq!(peer_address.peer_id, "test-peer");
        assert_eq!(peer_address.addresses.len(), 1);
        assert!(peer_address.supports_protocol("tcp"));
        assert!(peer_address.capabilities.reliable);
    }

    // Mock callback for testing
    struct TestTransportDiscoveryCallback {
        should_auto_connect: bool,
        preferred_protocol: Option<String>,
    }

    impl TestTransportDiscoveryCallback {
        fn new(should_auto_connect: bool, preferred_protocol: Option<String>) -> Self {
            Self {
                should_auto_connect,
                preferred_protocol,
            }
        }
    }

    #[async_trait]
    impl TransportDiscoveryCallback for TestTransportDiscoveryCallback {
        async fn on_transport_discovery_event(&self, _event: TransportDiscoveryEvent) {
            // Test implementation
        }

        async fn should_auto_connect(&self, _peer_id: &PeerId, _service_record: &ServiceRecord) -> bool {
            self.should_auto_connect
        }

        async fn select_protocol(&self, _peer_id: &PeerId, _available_protocols: &[String]) -> Option<String> {
            self.preferred_protocol.clone()
        }
    }

    #[tokio::test]
    async fn test_callback_registration() {
        let transport_config = KizunaTransportConfig::default();
        let integration_config = TransportDiscoveryConfig::default();
        
        let bridge = TransportDiscoveryBridge::new(transport_config, integration_config).await.unwrap();
        let callback = Arc::new(TestTransportDiscoveryCallback::new(true, Some("tcp".to_string())));
        
        bridge.register_callback(callback).await;
        
        let callbacks = bridge.callbacks.read().await;
        assert_eq!(callbacks.len(), 1);
    }

    #[tokio::test]
    async fn test_integration_stats() {
        let transport_config = KizunaTransportConfig::default();
        let integration_config = TransportDiscoveryConfig::default();
        
        let bridge = TransportDiscoveryBridge::new(transport_config, integration_config).await.unwrap();
        let stats = bridge.get_integration_stats().await;
        
        assert_eq!(stats.total_discovered_peers, 0);
        assert_eq!(stats.connected_peers, 0);
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.connection_success_rate, 0.0);
        assert!(stats.auto_connect_enabled);
    }
}