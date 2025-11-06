# Transport/Networking Layer Design Document

## Overview

The Transport/Networking layer implements a multi-protocol peer-to-peer communication system for Kizuna, providing reliable and efficient data transfer across various network topologies. The system uses a unified trait-based architecture supporting TCP, QUIC, WebRTC DataChannels, and WebSocket protocols with intelligent connection negotiation, NAT traversal, and optional relay support.

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                  Connection Manager                          │
├─────────────────────────────────────────────────────────────┤
│  Protocol Negotiation │  Connection Pool │  NAT Traversal   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Transport Trait                           │
├─────────────────────────────────────────────────────────────┤
│  async fn connect() -> Result<Connection>                   │
│  async fn listen() -> Result<Listener>                      │
│  fn protocol_name() -> &'static str                         │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│TCP Transport│    │QUIC Transport│   │WebRTC       │
│             │    │             │    │Transport    │
└─────────────┘    └─────────────┘    └─────────────┘
        ▼                     ▼
┌─────────────┐    ┌─────────────┐
│WebSocket    │    │Relay Node   │
│Transport    │    │Manager      │
└─────────────┘    └─────────────┘
```

### Module Structure

```
src/transport/
├── mod.rs              # Public API and Transport trait
├── manager.rs          # Connection Manager and protocol negotiation
├── connection.rs       # Connection abstraction and lifecycle
├── nat_traversal.rs    # NAT traversal and hole punching
├── relay.rs            # Relay node functionality
├── protocols/
│   ├── mod.rs          # Protocol implementations
│   ├── tcp.rs          # TCP transport implementation
│   ├── quic.rs         # QUIC transport implementation
│   ├── webrtc.rs       # WebRTC DataChannel transport
│   └── websocket.rs    # WebSocket transport implementation
├── routing/
│   ├── mod.rs          # Multi-hop routing (experimental)
│   ├── mesh.rs         # Mesh routing implementation
│   └── table.rs        # Routing table management
└── error.rs            # Transport-specific error types
```

## Components and Interfaces

### Transport Trait

```rust
#[async_trait]
pub trait Transport: Send + Sync {
    type Connection: Connection;
    type Listener: Listener<Connection = Self::Connection>;
    
    /// Connect to a remote peer
    async fn connect(&self, addr: &PeerAddress) -> Result<Self::Connection, TransportError>;
    
    /// Start listening for incoming connections
    async fn listen(&self, bind_addr: &SocketAddr) -> Result<Self::Listener, TransportError>;
    
    /// Get the protocol name for identification
    fn protocol_name(&self) -> &'static str;
    
    /// Check if this transport is available on the current platform
    fn is_available(&self) -> bool;
    
    /// Get the priority of this transport (higher = preferred)
    fn priority(&self) -> u8;
    
    /// Get transport capabilities and features
    fn capabilities(&self) -> TransportCapabilities;
}
```

### Connection Interface

```rust
#[async_trait]
pub trait Connection: Send + Sync {
    /// Read data from the connection
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;
    
    /// Write data to the connection
    async fn write(&mut self, buf: &[u8]) -> Result<usize, TransportError>;
    
    /// Flush any buffered data
    async fn flush(&mut self) -> Result<(), TransportError>;
    
    /// Close the connection gracefully
    async fn close(&mut self) -> Result<(), TransportError>;
    
    /// Get connection metadata and statistics
    fn info(&self) -> ConnectionInfo;
    
    /// Check if connection is still active
    fn is_connected(&self) -> bool;
}
```

### Connection Manager

```rust
pub struct ConnectionManager {
    transports: Vec<Box<dyn Transport>>,
    active_connections: Arc<RwLock<HashMap<PeerId, Vec<Box<dyn Connection>>>>>,
    connection_pool: ConnectionPool,
    nat_traversal: NatTraversal,
    relay_manager: Option<RelayManager>,
}

impl ConnectionManager {
    pub fn new() -> Self;
    pub fn add_transport(&mut self, transport: Box<dyn Transport>);
    pub async fn connect_to_peer(&self, peer: &PeerInfo) -> Result<Box<dyn Connection>>;
    pub async fn start_listening(&self) -> Result<()>;
    pub fn get_connections(&self, peer_id: &PeerId) -> Vec<&dyn Connection>;
    pub async fn negotiate_protocol(&self, peer: &PeerInfo) -> Result<&dyn Transport>;
}
```

## Data Models

### PeerAddress and Connection Info

```rust
#[derive(Debug, Clone)]
pub struct PeerAddress {
    pub peer_id: PeerId,
    pub addresses: Vec<SocketAddr>,
    pub transport_hints: Vec<String>,
    pub capabilities: TransportCapabilities,
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub peer_id: PeerId,
    pub local_addr: SocketAddr,
    pub remote_addr: SocketAddr,
    pub protocol: String,
    pub established_at: SystemTime,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub rtt: Option<Duration>,
    pub bandwidth: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct TransportCapabilities {
    pub reliable: bool,
    pub ordered: bool,
    pub multiplexed: bool,
    pub resumable: bool,
    pub nat_traversal: bool,
    pub max_message_size: Option<usize>,
}
```

### Protocol Negotiation

```rust
#[derive(Debug, Clone)]
pub struct ProtocolNegotiation {
    pub offered_protocols: Vec<String>,
    pub selected_protocol: Option<String>,
    pub fallback_protocols: Vec<String>,
    pub negotiation_timeout: Duration,
}

impl ProtocolNegotiation {
    pub fn new(local_capabilities: &[String]) -> Self;
    pub fn add_peer_capabilities(&mut self, peer_capabilities: &[String]);
    pub fn select_best_protocol(&self) -> Option<String>;
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("Connection failed: {reason}")]
    ConnectionFailed { reason: String },
    
    #[error("Protocol not supported: {protocol}")]
    UnsupportedProtocol { protocol: String },
    
    #[error("NAT traversal failed: {method}")]
    NatTraversalFailed { method: String },
    
    #[error("Relay connection failed: {relay_addr}")]
    RelayFailed { relay_addr: SocketAddr },
    
    #[error("Protocol negotiation timeout")]
    NegotiationTimeout,
    
    #[error("Connection timeout after {timeout:?}")]
    ConnectionTimeout { timeout: Duration },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("QUIC error: {0}")]
    Quic(String),
    
    #[error("WebRTC error: {0}")]
    WebRTC(String),
}
```

### Error Recovery Strategy

1. **Connection Failures**: Retry with different transport protocols
2. **NAT Traversal Failures**: Fall back to relay connections
3. **Protocol Negotiation Failures**: Use lowest common denominator protocol
4. **Timeout Errors**: Reduce timeout and retry with faster protocols
5. **Relay Failures**: Try alternative relay nodes or direct connection

## Implementation Details

### TCP Transport (tcp.rs)

```rust
pub struct TcpTransport {
    config: TcpConfig,
}

impl Transport for TcpTransport {
    type Connection = TcpConnection;
    type Listener = TcpListener;
    
    async fn connect(&self, addr: &PeerAddress) -> Result<Self::Connection, TransportError> {
        // Implement TCP connection with timeout and keep-alive
        // Handle connection pooling and reuse
        // Add connection health monitoring
    }
    
    async fn listen(&self, bind_addr: &SocketAddr) -> Result<Self::Listener, TransportError> {
        // Bind TCP listener with SO_REUSEADDR
        // Configure accept backlog and connection limits
    }
}
```

### QUIC Transport (quic.rs)

```rust
pub struct QuicTransport {
    endpoint: quinn::Endpoint,
    config: QuicConfig,
}

impl Transport for QuicTransport {
    type Connection = QuicConnection;
    type Listener = QuicListener;
    
    async fn connect(&self, addr: &PeerAddress) -> Result<Self::Connection, TransportError> {
        // Implement QUIC connection with 0-RTT resumption
        // Handle connection migration and multiplexing
        // Configure congestion control algorithms
    }
}
```

### WebRTC Transport (webrtc.rs)

```rust
pub struct WebRtcTransport {
    peer_connection_factory: Arc<PeerConnectionFactory>,
    ice_servers: Vec<IceServer>,
}

impl Transport for WebRtcTransport {
    type Connection = WebRtcConnection;
    type Listener = WebRtcListener;
    
    async fn connect(&self, addr: &PeerAddress) -> Result<Self::Connection, TransportError> {
        // Implement WebRTC DataChannel establishment
        // Handle ICE candidate exchange through signaling
        // Configure STUN/TURN servers for NAT traversal
    }
}
```

### WebSocket Transport (websocket.rs)

```rust
pub struct WebSocketTransport {
    relay_servers: Vec<Url>,
    config: WebSocketConfig,
}

impl Transport for WebSocketTransport {
    type Connection = WebSocketConnection;
    type Listener = WebSocketListener;
    
    async fn connect(&self, addr: &PeerAddress) -> Result<Self::Connection, TransportError> {
        // Connect through relay server with WebSocket subprotocol
        // Handle relay authentication and peer routing
        // Implement connection upgrade attempts
    }
}
```

## NAT Traversal Implementation

### UDP Hole Punching

```rust
pub struct NatTraversal {
    stun_servers: Vec<SocketAddr>,
    local_candidates: Vec<SocketAddr>,
}

impl NatTraversal {
    pub async fn discover_nat_type(&self) -> Result<NatType, TransportError>;
    pub async fn perform_hole_punch(&self, peer_addr: &SocketAddr) -> Result<SocketAddr, TransportError>;
    pub async fn coordinate_traversal(&self, peer_info: &PeerInfo) -> Result<Connection, TransportError>;
}

#[derive(Debug, Clone)]
pub enum NatType {
    Open,
    FullCone,
    RestrictedCone,
    PortRestrictedCone,
    Symmetric,
}
```

## Relay Node Implementation

### Relay Manager

```rust
pub struct RelayManager {
    relay_nodes: Vec<RelayNode>,
    bandwidth_limit: Option<u64>,
    connection_limit: usize,
}

impl RelayManager {
    pub async fn start_relay_service(&self, bind_addr: SocketAddr) -> Result<(), TransportError>;
    pub async fn forward_connection(&self, from: Connection, to: Connection) -> Result<(), TransportError>;
    pub fn register_relay_node(&mut self, node: RelayNode);
    pub async fn find_best_relay(&self, target_peer: &PeerId) -> Option<&RelayNode>;
}

#[derive(Debug, Clone)]
pub struct RelayNode {
    pub address: SocketAddr,
    pub public_key: PublicKey,
    pub bandwidth_capacity: u64,
    pub connection_count: usize,
    pub latency: Duration,
}
```

## Multi-Hop Routing (Experimental)

### Mesh Routing

```rust
pub struct MeshRouter {
    routing_table: RoutingTable,
    trusted_peers: HashSet<PeerId>,
    max_hops: u8,
}

impl MeshRouter {
    pub async fn route_to_peer(&self, target: &PeerId, data: &[u8]) -> Result<(), TransportError>;
    pub fn update_routing_table(&mut self, peer_id: PeerId, route: Route);
    pub fn find_route(&self, target: &PeerId) -> Option<Route>;
}

#[derive(Debug, Clone)]
pub struct Route {
    pub hops: Vec<PeerId>,
    pub cost: u32,
    pub last_updated: SystemTime,
}
```

## Performance Considerations

### Connection Management

- **Connection Pooling**: Reuse existing connections for multiple transfers
- **Protocol Selection**: Choose optimal protocol based on network conditions and requirements
- **Bandwidth Management**: Implement fair queuing and throttling across connections
- **Resource Limits**: Enforce maximum connection counts and memory usage

### Protocol-Specific Optimizations

- **TCP**: Enable TCP_NODELAY for low latency, use connection keep-alive
- **QUIC**: Configure appropriate congestion control, enable 0-RTT resumption
- **WebRTC**: Optimize DataChannel configuration for different use cases
- **WebSocket**: Implement efficient message framing and compression

### Monitoring and Metrics

```rust
#[derive(Debug, Clone)]
pub struct ConnectionMetrics {
    pub connections_established: u64,
    pub connections_failed: u64,
    pub bytes_transferred: u64,
    pub average_latency: Duration,
    pub protocol_usage: HashMap<String, u64>,
}
```

## Security Considerations

### Connection Security

- All connections MUST use TLS or equivalent encryption
- Implement proper certificate validation for QUIC and WebSocket
- Use secure random number generation for connection IDs
- Validate peer identity during connection establishment

### Relay Security

- Authenticate relay nodes using cryptographic signatures
- Implement rate limiting to prevent relay abuse
- Isolate relay traffic to prevent data leakage
- Monitor relay usage for suspicious patterns

### Multi-Hop Security

- Implement hop-by-hop encryption for multi-hop routes
- Validate each hop in the routing path
- Prevent routing loops and resource exhaustion attacks
- Limit routing to explicitly trusted peers only

## Testing Strategy

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tcp_connection() {
        // Test TCP connection establishment and data transfer
    }
    
    #[tokio::test]
    async fn test_quic_resumption() {
        // Test QUIC 0-RTT connection resumption
    }
    
    #[tokio::test]
    async fn test_protocol_negotiation() {
        // Test automatic protocol selection
    }
    
    #[tokio::test]
    async fn test_nat_traversal() {
        // Test UDP hole punching
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_cross_protocol_communication() {
    // Test communication between different transport protocols
}

#[tokio::test]
async fn test_relay_fallback() {
    // Test fallback to relay when direct connection fails
}

#[tokio::test]
async fn test_connection_migration() {
    // Test QUIC connection migration during network changes
}
```

### Platform-Specific Tests

- **Linux**: Test all protocols, focus on performance optimization
- **macOS**: Test network framework integration, handle permission requirements
- **Windows**: Test WinSock compatibility, handle Windows firewall
- **Mobile**: Focus on battery optimization and network efficiency
- **Browser**: Test WebRTC DataChannel compatibility across browsers