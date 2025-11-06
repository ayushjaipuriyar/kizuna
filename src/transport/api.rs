use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use tokio::sync::{RwLock, mpsc};
use serde::{Deserialize, Serialize};

use crate::transport::{
    ConnectionManager, Connection, ConnectionInfo, TransportError, PeerAddress, 
    TransportCapabilities, PeerId, IntegratedTransportSystem, IntegratedSystemConfig,
    SystemState, SystemHealthReport, PerformanceMonitor, ErrorHandler
};

/// Configuration for the Kizuna Transport API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KizunaTransportConfig {
    /// Maximum number of concurrent connections per peer
    pub max_connections_per_peer: usize,
    /// Connection timeout for new connections
    pub connection_timeout: Duration,
    /// Keep-alive interval for established connections
    pub keep_alive_interval: Duration,
    /// Enable automatic connection retry on failure
    pub auto_retry: bool,
    /// Maximum retry attempts for failed connections
    pub max_retry_attempts: u32,
    /// Enable connection pooling and reuse
    pub enable_connection_pooling: bool,
    /// Enable performance monitoring
    pub enable_performance_monitoring: bool,
    /// Enable detailed logging
    pub enable_detailed_logging: bool,
    /// Protocols to enable (tcp, quic, webrtc, websocket)
    pub enabled_protocols: Vec<String>,
    /// NAT traversal configuration
    pub nat_traversal_config: Option<NatTraversalConfig>,
    /// Relay configuration for fallback connections
    pub relay_config: Option<RelayConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NatTraversalConfig {
    /// STUN servers for NAT type detection
    pub stun_servers: Vec<String>,
    /// Enable UDP hole punching
    pub enable_hole_punching: bool,
    /// Hole punching timeout
    pub hole_punch_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayConfig {
    /// List of relay server URLs
    pub relay_servers: Vec<String>,
    /// Enable automatic relay fallback
    pub enable_auto_fallback: bool,
    /// Relay connection timeout
    pub relay_timeout: Duration,
}

impl Default for KizunaTransportConfig {
    fn default() -> Self {
        Self {
            max_connections_per_peer: 5,
            connection_timeout: Duration::from_secs(30),
            keep_alive_interval: Duration::from_secs(60),
            auto_retry: true,
            max_retry_attempts: 3,
            enable_connection_pooling: true,
            enable_performance_monitoring: true,
            enable_detailed_logging: false,
            enabled_protocols: vec![
                "tcp".to_string(),
                "quic".to_string(),
                "webrtc".to_string(),
                "websocket".to_string(),
            ],
            nat_traversal_config: Some(NatTraversalConfig {
                stun_servers: vec![
                    "stun:stun.l.google.com:19302".to_string(),
                    "stun:stun1.l.google.com:19302".to_string(),
                ],
                enable_hole_punching: true,
                hole_punch_timeout: Duration::from_secs(10),
            }),
            relay_config: Some(RelayConfig {
                relay_servers: vec![],
                enable_auto_fallback: true,
                relay_timeout: Duration::from_secs(15),
            }),
        }
    }
}

/// Connection lifecycle events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionEvent {
    /// Connection attempt started
    Connecting {
        peer_id: PeerId,
        protocol: String,
        attempt: u32,
    },
    /// Connection established successfully
    Connected {
        peer_id: PeerId,
        protocol: String,
        connection_info: ConnectionInfo,
    },
    /// Connection failed
    ConnectionFailed {
        peer_id: PeerId,
        protocol: String,
        error: String,
        attempt: u32,
    },
    /// Connection lost/disconnected
    Disconnected {
        peer_id: PeerId,
        reason: String,
    },
    /// Data received on connection
    DataReceived {
        peer_id: PeerId,
        bytes: usize,
    },
    /// Data sent on connection
    DataSent {
        peer_id: PeerId,
        bytes: usize,
    },
    /// Protocol negotiation completed
    ProtocolNegotiated {
        peer_id: PeerId,
        selected_protocol: String,
        available_protocols: Vec<String>,
    },
    /// NAT traversal attempt
    NatTraversalAttempt {
        peer_id: PeerId,
        method: String,
    },
    /// NAT traversal successful
    NatTraversalSuccess {
        peer_id: PeerId,
        method: String,
    },
    /// Relay connection established
    RelayConnected {
        peer_id: PeerId,
        relay_address: String,
    },
}

/// Connection lifecycle callback trait
#[async_trait]
pub trait ConnectionCallback: Send + Sync {
    /// Called when a connection event occurs
    async fn on_connection_event(&self, event: ConnectionEvent);
    
    /// Called when connection quality changes
    async fn on_connection_quality_change(&self, peer_id: PeerId, quality: ConnectionQuality);
    
    /// Called when an error occurs that may require user attention
    async fn on_error(&self, error: TransportError, context: String);
}

/// Connection quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionQuality {
    /// Round-trip time in milliseconds
    pub rtt_ms: Option<u64>,
    /// Bandwidth estimate in bytes per second
    pub bandwidth_bps: Option<u64>,
    /// Packet loss rate (0.0 to 1.0)
    pub packet_loss_rate: f64,
    /// Connection stability score (0.0 to 1.0)
    pub stability_score: f64,
    /// Quality classification
    pub quality_class: QualityClass,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityClass {
    Excellent,
    Good,
    Fair,
    Poor,
    Unusable,
}

/// Connection handle for managing individual connections
pub struct ConnectionHandle {
    peer_id: PeerId,
    connection: Arc<RwLock<Box<dyn Connection>>>,
    event_sender: mpsc::UnboundedSender<ConnectionEvent>,
    quality: Arc<RwLock<ConnectionQuality>>,
}

impl ConnectionHandle {
    /// Get the peer ID for this connection
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
    
    /// Read data from the connection
    pub async fn read(&self, buffer: &mut [u8]) -> Result<usize, TransportError> {
        let mut conn = self.connection.write().await;
        let bytes_read = conn.read(buffer).await?;
        
        // Send data received event
        let _ = self.event_sender.send(ConnectionEvent::DataReceived {
            peer_id: self.peer_id.clone(),
            bytes: bytes_read,
        });
        
        Ok(bytes_read)
    }
    
    /// Write data to the connection
    pub async fn write(&self, data: &[u8]) -> Result<usize, TransportError> {
        let mut conn = self.connection.write().await;
        let bytes_written = conn.write(data).await?;
        
        // Send data sent event
        let _ = self.event_sender.send(ConnectionEvent::DataSent {
            peer_id: self.peer_id.clone(),
            bytes: bytes_written,
        });
        
        Ok(bytes_written)
    }
    
    /// Flush any buffered data
    pub async fn flush(&self) -> Result<(), TransportError> {
        let mut conn = self.connection.write().await;
        conn.flush().await
    }
    
    /// Close the connection gracefully
    pub async fn close(&self) -> Result<(), TransportError> {
        let mut conn = self.connection.write().await;
        let result = conn.close().await;
        
        // Send disconnected event
        let _ = self.event_sender.send(ConnectionEvent::Disconnected {
            peer_id: self.peer_id.clone(),
            reason: "User requested close".to_string(),
        });
        
        result
    }
    
    /// Get connection information and statistics
    pub async fn info(&self) -> ConnectionInfo {
        let conn = self.connection.read().await;
        conn.info()
    }
    
    /// Check if the connection is still active
    pub async fn is_connected(&self) -> bool {
        let conn = self.connection.read().await;
        conn.is_connected()
    }
    
    /// Get current connection quality metrics
    pub async fn quality(&self) -> ConnectionQuality {
        let quality = self.quality.read().await;
        quality.clone()
    }
}

/// Main Kizuna Transport API
pub struct KizunaTransport {
    config: KizunaTransportConfig,
    transport_system: IntegratedTransportSystem,
    active_connections: Arc<RwLock<HashMap<PeerId, Vec<ConnectionHandle>>>>,
    event_sender: mpsc::UnboundedSender<ConnectionEvent>,
    event_receiver: Arc<RwLock<mpsc::UnboundedReceiver<ConnectionEvent>>>,
    callbacks: Arc<RwLock<Vec<Arc<dyn ConnectionCallback>>>>,
    is_listening: Arc<RwLock<bool>>,
}

impl KizunaTransport {
    /// Create a new Kizuna Transport instance with default configuration
    pub async fn new() -> Result<Self, TransportError> {
        Self::with_config(KizunaTransportConfig::default()).await
    }
    
    /// Create a new Kizuna Transport instance with custom configuration
    pub async fn with_config(config: KizunaTransportConfig) -> Result<Self, TransportError> {
        // Convert config to IntegratedSystemConfig
        let system_config = IntegratedSystemConfig {
            connection_timeout: config.connection_timeout,
            keep_alive_interval: config.keep_alive_interval,
            max_connections_per_peer: config.max_connections_per_peer,
            enable_performance_monitoring: config.enable_performance_monitoring,
            enable_detailed_logging: config.enable_detailed_logging,
            enabled_protocols: config.enabled_protocols.clone(),
            auto_retry: config.auto_retry,
            max_retry_attempts: config.max_retry_attempts,
            enable_connection_pooling: config.enable_connection_pooling,
        };
        
        let transport_system = IntegratedTransportSystem::new(system_config).await
            .map_err(|e| TransportError::Configuration(format!("Failed to initialize transport system: {}", e)))?;
        
        let (event_sender, event_receiver) = mpsc::unbounded_channel();
        
        Ok(Self {
            config,
            transport_system,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            event_receiver: Arc::new(RwLock::new(event_receiver)),
            callbacks: Arc::new(RwLock::new(Vec::new())),
            is_listening: Arc::new(RwLock::new(false)),
        })
    }
    
    /// Register a connection lifecycle callback
    pub async fn register_callback(&self, callback: Arc<dyn ConnectionCallback>) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(callback);
    }
    
    /// Start listening for incoming connections
    pub async fn start_listening(&self, bind_address: SocketAddr) -> Result<(), TransportError> {
        {
            let mut is_listening = self.is_listening.write().await;
            if *is_listening {
                return Err(TransportError::Configuration("Already listening".to_string()));
            }
            *is_listening = true;
        }
        
        // Start the transport system listener
        self.transport_system.start_listening(bind_address).await?;
        
        // Start event processing task
        self.start_event_processing().await;
        
        Ok(())
    }
    
    /// Stop listening for incoming connections
    pub async fn stop_listening(&self) -> Result<(), TransportError> {
        {
            let mut is_listening = self.is_listening.write().await;
            if !*is_listening {
                return Ok(());
            }
            *is_listening = false;
        }
        
        self.transport_system.stop_listening().await?;
        Ok(())
    }
    
    /// Connect to a peer with automatic protocol negotiation
    pub async fn connect_to_peer(&self, peer_address: &PeerAddress) -> Result<ConnectionHandle, TransportError> {
        // Send connecting event
        let _ = self.event_sender.send(ConnectionEvent::Connecting {
            peer_id: peer_address.peer_id.clone(),
            protocol: "auto".to_string(),
            attempt: 1,
        });
        
        // Attempt connection through transport system
        let connection = self.transport_system.connect_to_peer(peer_address).await?;
        let connection_info = connection.info();
        
        // Create connection handle
        let handle = ConnectionHandle {
            peer_id: peer_address.peer_id.clone(),
            connection: Arc::new(RwLock::new(connection)),
            event_sender: self.event_sender.clone(),
            quality: Arc::new(RwLock::new(ConnectionQuality {
                rtt_ms: None,
                bandwidth_bps: None,
                packet_loss_rate: 0.0,
                stability_score: 1.0,
                quality_class: QualityClass::Good,
            })),
        };
        
        // Store connection
        {
            let mut connections = self.active_connections.write().await;
            connections.entry(peer_address.peer_id.clone())
                .or_insert_with(Vec::new)
                .push(handle);
        }
        
        // Send connected event
        let _ = self.event_sender.send(ConnectionEvent::Connected {
            peer_id: peer_address.peer_id.clone(),
            protocol: connection_info.protocol.clone(),
            connection_info,
        });
        
        // Return the last added handle
        let connections = self.active_connections.read().await;
        let peer_connections = connections.get(&peer_address.peer_id).unwrap();
        Ok(peer_connections.last().unwrap().clone())
    }
    
    /// Connect to a peer using a specific protocol
    pub async fn connect_with_protocol(&self, peer_address: &PeerAddress, protocol: &str) -> Result<ConnectionHandle, TransportError> {
        // Send connecting event
        let _ = self.event_sender.send(ConnectionEvent::Connecting {
            peer_id: peer_address.peer_id.clone(),
            protocol: protocol.to_string(),
            attempt: 1,
        });
        
        // Attempt connection with specific protocol
        let connection = self.transport_system.connect_with_protocol(peer_address, protocol).await?;
        let connection_info = connection.info();
        
        // Create connection handle
        let handle = ConnectionHandle {
            peer_id: peer_address.peer_id.clone(),
            connection: Arc::new(RwLock::new(connection)),
            event_sender: self.event_sender.clone(),
            quality: Arc::new(RwLock::new(ConnectionQuality {
                rtt_ms: None,
                bandwidth_bps: None,
                packet_loss_rate: 0.0,
                stability_score: 1.0,
                quality_class: QualityClass::Good,
            })),
        };
        
        // Store connection
        {
            let mut connections = self.active_connections.write().await;
            connections.entry(peer_address.peer_id.clone())
                .or_insert_with(Vec::new)
                .push(handle);
        }
        
        // Send connected event
        let _ = self.event_sender.send(ConnectionEvent::Connected {
            peer_id: peer_address.peer_id.clone(),
            protocol: connection_info.protocol.clone(),
            connection_info,
        });
        
        // Return the last added handle
        let connections = self.active_connections.read().await;
        let peer_connections = connections.get(&peer_address.peer_id).unwrap();
        Ok(peer_connections.last().unwrap().clone())
    }
    
    /// Get all active connections for a peer
    pub async fn get_connections(&self, peer_id: &PeerId) -> Vec<ConnectionHandle> {
        let connections = self.active_connections.read().await;
        connections.get(peer_id).cloned().unwrap_or_default()
    }
    
    /// Get all active peer IDs
    pub async fn get_active_peers(&self) -> Vec<PeerId> {
        let connections = self.active_connections.read().await;
        connections.keys().cloned().collect()
    }
    
    /// Disconnect from a specific peer (closes all connections)
    pub async fn disconnect_peer(&self, peer_id: &PeerId) -> Result<(), TransportError> {
        let mut connections = self.active_connections.write().await;
        if let Some(peer_connections) = connections.remove(peer_id) {
            for handle in peer_connections {
                let _ = handle.close().await;
            }
        }
        Ok(())
    }
    
    /// Disconnect all peers and close all connections
    pub async fn disconnect_all(&self) -> Result<(), TransportError> {
        let mut connections = self.active_connections.write().await;
        for (_, peer_connections) in connections.drain() {
            for handle in peer_connections {
                let _ = handle.close().await;
            }
        }
        Ok(())
    }
    
    /// Get transport system health report
    pub async fn get_health_report(&self) -> SystemHealthReport {
        self.transport_system.get_health_report().await
    }
    
    /// Get transport system state
    pub async fn get_system_state(&self) -> SystemState {
        self.transport_system.get_system_state().await
    }
    
    /// Get supported transport protocols
    pub fn get_supported_protocols(&self) -> Vec<String> {
        self.config.enabled_protocols.clone()
    }
    
    /// Check if a specific protocol is supported
    pub fn supports_protocol(&self, protocol: &str) -> bool {
        self.config.enabled_protocols.contains(&protocol.to_string())
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> &KizunaTransportConfig {
        &self.config
    }
    
    /// Update configuration (requires restart for some changes)
    pub async fn update_config(&mut self, new_config: KizunaTransportConfig) -> Result<(), TransportError> {
        // Check if critical settings changed that require restart
        let needs_restart = self.config.enabled_protocols != new_config.enabled_protocols ||
                           self.config.max_connections_per_peer != new_config.max_connections_per_peer;
        
        if needs_restart {
            return Err(TransportError::Configuration(
                "Configuration changes require transport system restart".to_string()
            ));
        }
        
        self.config = new_config;
        Ok(())
    }
    
    /// Get connection statistics
    pub async fn get_connection_stats(&self) -> ConnectionStats {
        let connections = self.active_connections.read().await;
        let total_connections: usize = connections.values().map(|v| v.len()).sum();
        let active_peers = connections.len();
        
        ConnectionStats {
            total_connections,
            active_peers,
            connections_by_protocol: self.get_connections_by_protocol().await,
            average_connection_quality: self.calculate_average_quality().await,
        }
    }
    
    /// Enable or disable automatic retry on connection failures
    pub fn set_auto_retry(&mut self, enabled: bool) {
        self.config.auto_retry = enabled;
    }
    
    /// Set maximum retry attempts
    pub fn set_max_retry_attempts(&mut self, attempts: u32) {
        self.config.max_retry_attempts = attempts;
    }
    
    /// Start event processing task
    async fn start_event_processing(&self) {
        let callbacks = Arc::clone(&self.callbacks);
        let mut receiver = self.event_receiver.write().await;
        
        tokio::spawn(async move {
            while let Some(event) = receiver.recv().await {
                let callbacks_guard = callbacks.read().await;
                for callback in callbacks_guard.iter() {
                    callback.on_connection_event(event.clone()).await;
                }
            }
        });
    }
    
    /// Get connections grouped by protocol
    async fn get_connections_by_protocol(&self) -> HashMap<String, usize> {
        let mut protocol_counts = HashMap::new();
        let connections = self.active_connections.read().await;
        
        for peer_connections in connections.values() {
            for handle in peer_connections {
                let info = handle.info().await;
                *protocol_counts.entry(info.protocol).or_insert(0) += 1;
            }
        }
        
        protocol_counts
    }
    
    /// Calculate average connection quality across all connections
    async fn calculate_average_quality(&self) -> f64 {
        let connections = self.active_connections.read().await;
        let mut total_quality = 0.0;
        let mut count = 0;
        
        for peer_connections in connections.values() {
            for handle in peer_connections {
                let quality = handle.quality().await;
                total_quality += quality.stability_score;
                count += 1;
            }
        }
        
        if count > 0 {
            total_quality / count as f64
        } else {
            0.0
        }
    }
}

impl Clone for ConnectionHandle {
    fn clone(&self) -> Self {
        Self {
            peer_id: self.peer_id.clone(),
            connection: Arc::clone(&self.connection),
            event_sender: self.event_sender.clone(),
            quality: Arc::clone(&self.quality),
        }
    }
}

/// Connection statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    /// Total number of active connections
    pub total_connections: usize,
    /// Number of unique peers with active connections
    pub active_peers: usize,
    /// Number of connections by protocol
    pub connections_by_protocol: HashMap<String, usize>,
    /// Average connection quality score
    pub average_connection_quality: f64,
}

/// Builder for creating KizunaTransport with fluent API
pub struct KizunaTransportBuilder {
    config: KizunaTransportConfig,
}

impl KizunaTransportBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: KizunaTransportConfig::default(),
        }
    }
    
    /// Set connection timeout
    pub fn connection_timeout(mut self, timeout: Duration) -> Self {
        self.config.connection_timeout = timeout;
        self
    }
    
    /// Set keep-alive interval
    pub fn keep_alive_interval(mut self, interval: Duration) -> Self {
        self.config.keep_alive_interval = interval;
        self
    }
    
    /// Enable or disable auto-retry
    pub fn auto_retry(mut self, enabled: bool) -> Self {
        self.config.auto_retry = enabled;
        self
    }
    
    /// Set maximum retry attempts
    pub fn max_retry_attempts(mut self, attempts: u32) -> Self {
        self.config.max_retry_attempts = attempts;
        self
    }
    
    /// Enable specific protocols
    pub fn enable_protocols(mut self, protocols: Vec<String>) -> Self {
        self.config.enabled_protocols = protocols;
        self
    }
    
    /// Enable performance monitoring
    pub fn performance_monitoring(mut self, enabled: bool) -> Self {
        self.config.enable_performance_monitoring = enabled;
        self
    }
    
    /// Enable detailed logging
    pub fn detailed_logging(mut self, enabled: bool) -> Self {
        self.config.enable_detailed_logging = enabled;
        self
    }
    
    /// Set NAT traversal configuration
    pub fn nat_traversal_config(mut self, config: NatTraversalConfig) -> Self {
        self.config.nat_traversal_config = Some(config);
        self
    }
    
    /// Set relay configuration
    pub fn relay_config(mut self, config: RelayConfig) -> Self {
        self.config.relay_config = Some(config);
        self
    }
    
    /// Build the KizunaTransport instance
    pub async fn build(self) -> Result<KizunaTransport, TransportError> {
        KizunaTransport::with_config(self.config).await
    }
}

impl Default for KizunaTransportBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_kizuna_transport_creation() {
        let transport = KizunaTransport::new().await;
        assert!(transport.is_ok());
        
        let transport = transport.unwrap();
        assert_eq!(transport.get_supported_protocols().len(), 4);
        assert!(transport.supports_protocol("tcp"));
        assert!(transport.supports_protocol("quic"));
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        let transport = KizunaTransportBuilder::new()
            .connection_timeout(Duration::from_secs(10))
            .auto_retry(false)
            .enable_protocols(vec!["tcp".to_string(), "quic".to_string()])
            .build()
            .await;
        
        assert!(transport.is_ok());
        let transport = transport.unwrap();
        assert_eq!(transport.get_config().connection_timeout, Duration::from_secs(10));
        assert!(!transport.get_config().auto_retry);
        assert_eq!(transport.get_supported_protocols().len(), 2);
    }

    #[tokio::test]
    async fn test_connection_stats() {
        let transport = KizunaTransport::new().await.unwrap();
        let stats = transport.get_connection_stats().await;
        
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.active_peers, 0);
        assert_eq!(stats.average_connection_quality, 0.0);
    }

    #[tokio::test]
    async fn test_peer_address_creation() {
        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );
        
        assert_eq!(peer_addr.peer_id, "test-peer");
        assert_eq!(peer_addr.addresses.len(), 1);
        assert!(peer_addr.supports_protocol("tcp"));
    }

    // Mock callback for testing
    struct TestCallback {
        events: Arc<RwLock<Vec<ConnectionEvent>>>,
    }

    impl TestCallback {
        fn new() -> Self {
            Self {
                events: Arc::new(RwLock::new(Vec::new())),
            }
        }

        async fn get_events(&self) -> Vec<ConnectionEvent> {
            let events = self.events.read().await;
            events.clone()
        }
    }

    #[async_trait]
    impl ConnectionCallback for TestCallback {
        async fn on_connection_event(&self, event: ConnectionEvent) {
            let mut events = self.events.write().await;
            events.push(event);
        }

        async fn on_connection_quality_change(&self, _peer_id: PeerId, _quality: ConnectionQuality) {
            // Test implementation
        }

        async fn on_error(&self, _error: TransportError, _context: String) {
            // Test implementation
        }
    }

    #[tokio::test]
    async fn test_callback_registration() {
        let transport = KizunaTransport::new().await.unwrap();
        let callback = Arc::new(TestCallback::new());
        
        transport.register_callback(callback.clone()).await;
        
        // Verify callback was registered (we can't easily test the actual callback without a real connection)
        let callbacks = transport.callbacks.read().await;
        assert_eq!(callbacks.len(), 1);
    }
}