use std::net::SocketAddr;
use serde::{Deserialize, Serialize};

pub mod manager;
pub mod connection;
pub mod error;
pub mod error_handler;
pub mod logging;
pub mod performance;
pub mod integrated_system;
pub mod protocols;
pub mod nat_traversal;
pub mod relay;
pub mod routing;
pub mod api;
pub mod discovery_integration;
pub mod security_integration;

#[cfg(doc)]
pub mod examples;

// Re-export main types
pub use manager::{
    ConnectionManager, Transport, PeerInfo, ProtocolNegotiation, NegotiationSummary,
    ProtocolNegotiationResult, ConnectionManagerConfig, ConnectionStats, NetworkConditions,
    LatencyRequirement, BandwidthRequirement, ReliabilityRequirement, ConnectionState,
    ManagedConnection, ConnectionPool, PoolStats, ConnectionAttemptResult, 
    ConcurrentConnectionResult, DetailedConnectionStats, AvailableTransport
};
pub use connection::{Connection, ConnectionInfo};
pub use error::{TransportError, ErrorSeverity, RetryStrategy, ErrorCategory, ErrorContext, ContextualError};
pub use error_handler::{ErrorHandler, ErrorHandlerConfig, ErrorStats, CircuitBreaker, CircuitBreakerState, ErrorHandlerHealth};
pub use logging::{TransportLogger, LoggingConfig, LogLevel, LogCategory, LogEntry, ConnectionEvent as LogConnectionEvent, SecurityEvent as LogSecurityEvent};
pub use performance::{
    PerformanceMonitor, PerformanceConfig, ConnectionMetrics, GlobalPerformanceStats,
    BandwidthManager, BandwidthTracker, BandwidthAllocationStrategy, ConnectionPoolOptimizer,
    OptimizationRecommendation, PerformanceReport, HealthStatus
};
pub use integrated_system::{
    IntegratedTransportSystem, IntegratedSystemConfig, SystemState, SystemHealthReport,
    SystemRecommendation, SystemStatus
};
pub use nat_traversal::{NatTraversal, NatType, NatTraversalConfig, HolePunchMessage, HolePunchMessageType, HolePunchPayload};
pub use protocols::tcp::{TcpTransport, TcpConnection, TcpListener, TcpConfig, TcpServer, TcpServerStats};
pub use protocols::quic::{QuicTransport, QuicConnection, QuicConfig, QuicConnectionStats, CongestionControl};
pub use protocols::webrtc::{WebRtcTransport, WebRtcConnection, WebRtcConfig, IceServerConfig, SignalingHandler, SignalingMessage, DefaultSignalingHandler};
pub use protocols::websocket::{
    WebSocketTransport, WebSocketConnection, WebSocketListener, WebSocketConfig, 
    RelayMessage, ConnectionType, WebSocketStreamWrapper,
    ConnectionUpgradeManager, RelayServerHandler
};
pub use relay::{
    RelayManager as CoreRelayManager, RelayConfig, RelayNodeInfo, RelayStats as CoreRelayStats, 
    RelaySession, BandwidthLimiter as CoreBandwidthLimiter
};
pub use routing::{
    MeshRouter, MeshConfig, RouteDiscoveryMessage, RouteAdvertisement,
    RoutingTable, Route, RouteEntry, RouteMetrics,
    RoutingProtocolManager, RoutingProtocolConfig, RoutingProtocolMessage, 
    RouteTableEntry, RouteTableKey, NeighborState, RoutingProtocolStats
};
pub use api::{
    KizunaTransport, KizunaTransportConfig, KizunaTransportBuilder, ConnectionHandle,
    ConnectionCallback, ConnectionEvent, ConnectionQuality, QualityClass,
    NatTraversalConfig as ApiNatTraversalConfig, RelayConfig as ApiRelayConfig,
    ConnectionStats as ApiConnectionStats
};
pub use discovery_integration::{
    TransportDiscoveryBridge, TransportDiscoveryConfig, TransportDiscoveryEvent,
    TransportDiscoveryCallback, IntegrationStats
};
pub use security_integration::{
    TransportSecurityHooks, SecureConnection
};

/// Unique identifier for a peer in the network
pub type PeerId = String;

/// Address information for connecting to a peer
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PeerAddress {
    /// Unique identifier of the peer
    pub peer_id: PeerId,
    /// List of network addresses where the peer can be reached
    pub addresses: Vec<SocketAddr>,
    /// Hints about preferred transport protocols
    pub transport_hints: Vec<String>,
    /// Capabilities supported by this peer
    pub capabilities: TransportCapabilities,
}

impl PeerAddress {
    /// Create a new PeerAddress
    pub fn new(
        peer_id: PeerId,
        addresses: Vec<SocketAddr>,
        transport_hints: Vec<String>,
        capabilities: TransportCapabilities,
    ) -> Self {
        Self {
            peer_id,
            addresses,
            transport_hints,
            capabilities,
        }
    }

    /// Add a new address to this peer
    pub fn add_address(&mut self, addr: SocketAddr) {
        if !self.addresses.contains(&addr) {
            self.addresses.push(addr);
        }
    }

    /// Add a transport hint
    pub fn add_transport_hint(&mut self, hint: String) {
        if !self.transport_hints.contains(&hint) {
            self.transport_hints.push(hint);
        }
    }

    /// Check if peer supports a specific transport protocol
    pub fn supports_protocol(&self, protocol: &str) -> bool {
        self.transport_hints.iter().any(|hint| hint == protocol)
    }
}

/// Capabilities and features supported by a transport protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransportCapabilities {
    /// Whether the transport provides reliable delivery
    pub reliable: bool,
    /// Whether the transport maintains message ordering
    pub ordered: bool,
    /// Whether the transport supports multiple streams
    pub multiplexed: bool,
    /// Whether connections can be resumed after network changes
    pub resumable: bool,
    /// Whether the transport can traverse NAT
    pub nat_traversal: bool,
    /// Maximum message size supported (None = unlimited)
    pub max_message_size: Option<usize>,
}

impl TransportCapabilities {
    /// Create capabilities for TCP transport
    pub fn tcp() -> Self {
        Self {
            reliable: true,
            ordered: true,
            multiplexed: false,
            resumable: false,
            nat_traversal: false,
            max_message_size: None,
        }
    }

    /// Create capabilities for QUIC transport
    pub fn quic() -> Self {
        Self {
            reliable: true,
            ordered: true,
            multiplexed: true,
            resumable: true,
            nat_traversal: false,
            max_message_size: None,
        }
    }

    /// Create capabilities for WebRTC transport
    pub fn webrtc() -> Self {
        Self {
            reliable: true,
            ordered: true,
            multiplexed: true,
            resumable: false,
            nat_traversal: true,
            max_message_size: Some(65536), // 64KB typical DataChannel limit
        }
    }

    /// Create capabilities for WebSocket transport
    pub fn websocket() -> Self {
        Self {
            reliable: true,
            ordered: true,
            multiplexed: false,
            resumable: false,
            nat_traversal: true, // Through relay servers
            max_message_size: None,
        }
    }

    /// Check if this transport is compatible with another
    pub fn is_compatible_with(&self, other: &TransportCapabilities) -> bool {
        // Basic compatibility check - both must support at least reliable delivery
        self.reliable && other.reliable
    }
}

impl Default for TransportCapabilities {
    fn default() -> Self {
        Self::tcp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_peer_address_creation() {
        let peer_id = "test-peer-123".to_string();
        let addresses = vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080),
        ];
        let transport_hints = vec!["tcp".to_string(), "quic".to_string()];
        let capabilities = TransportCapabilities::tcp();

        let peer_addr = PeerAddress::new(
            peer_id.clone(),
            addresses.clone(),
            transport_hints.clone(),
            capabilities.clone(),
        );

        assert_eq!(peer_addr.peer_id, peer_id);
        assert_eq!(peer_addr.addresses, addresses);
        assert_eq!(peer_addr.transport_hints, transport_hints);
        assert_eq!(peer_addr.capabilities, capabilities);
    }

    #[test]
    fn test_peer_address_add_address() {
        let mut peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![],
            vec![],
            TransportCapabilities::default(),
        );

        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);

        peer_addr.add_address(addr1);
        assert_eq!(peer_addr.addresses.len(), 1);
        assert!(peer_addr.addresses.contains(&addr1));

        peer_addr.add_address(addr2);
        assert_eq!(peer_addr.addresses.len(), 2);
        assert!(peer_addr.addresses.contains(&addr2));

        // Adding duplicate should not increase length
        peer_addr.add_address(addr1);
        assert_eq!(peer_addr.addresses.len(), 2);
    }

    #[test]
    fn test_peer_address_transport_hints() {
        let mut peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![],
            vec![],
            TransportCapabilities::default(),
        );

        peer_addr.add_transport_hint("tcp".to_string());
        assert!(peer_addr.supports_protocol("tcp"));
        assert!(!peer_addr.supports_protocol("quic"));

        peer_addr.add_transport_hint("quic".to_string());
        assert!(peer_addr.supports_protocol("tcp"));
        assert!(peer_addr.supports_protocol("quic"));

        // Adding duplicate should not increase length
        peer_addr.add_transport_hint("tcp".to_string());
        assert_eq!(peer_addr.transport_hints.len(), 2);
    }

    #[test]
    fn test_transport_capabilities_presets() {
        let tcp_caps = TransportCapabilities::tcp();
        assert!(tcp_caps.reliable);
        assert!(tcp_caps.ordered);
        assert!(!tcp_caps.multiplexed);
        assert!(!tcp_caps.resumable);
        assert!(!tcp_caps.nat_traversal);
        assert_eq!(tcp_caps.max_message_size, None);

        let quic_caps = TransportCapabilities::quic();
        assert!(quic_caps.reliable);
        assert!(quic_caps.ordered);
        assert!(quic_caps.multiplexed);
        assert!(quic_caps.resumable);
        assert!(!quic_caps.nat_traversal);
        assert_eq!(quic_caps.max_message_size, None);

        let webrtc_caps = TransportCapabilities::webrtc();
        assert!(webrtc_caps.reliable);
        assert!(webrtc_caps.ordered);
        assert!(webrtc_caps.multiplexed);
        assert!(!webrtc_caps.resumable);
        assert!(webrtc_caps.nat_traversal);
        assert_eq!(webrtc_caps.max_message_size, Some(65536));

        let websocket_caps = TransportCapabilities::websocket();
        assert!(websocket_caps.reliable);
        assert!(websocket_caps.ordered);
        assert!(!websocket_caps.multiplexed);
        assert!(!websocket_caps.resumable);
        assert!(websocket_caps.nat_traversal);
        assert_eq!(websocket_caps.max_message_size, None);
    }

    #[test]
    fn test_transport_capabilities_compatibility() {
        let tcp_caps = TransportCapabilities::tcp();
        let quic_caps = TransportCapabilities::quic();
        let unreliable_caps = TransportCapabilities {
            reliable: false,
            ordered: false,
            multiplexed: false,
            resumable: false,
            nat_traversal: false,
            max_message_size: None,
        };

        assert!(tcp_caps.is_compatible_with(&quic_caps));
        assert!(quic_caps.is_compatible_with(&tcp_caps));
        assert!(!tcp_caps.is_compatible_with(&unreliable_caps));
        assert!(!unreliable_caps.is_compatible_with(&tcp_caps));
    }

    #[test]
    fn test_peer_address_serialization() {
        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );

        // Test JSON serialization
        let json = serde_json::to_string(&peer_addr).expect("Failed to serialize");
        let deserialized: PeerAddress = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(peer_addr, deserialized);
    }

    #[test]
    fn test_transport_capabilities_serialization() {
        let caps = TransportCapabilities::quic();

        // Test JSON serialization
        let json = serde_json::to_string(&caps).expect("Failed to serialize");
        let deserialized: TransportCapabilities = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(caps, deserialized);
    }
}