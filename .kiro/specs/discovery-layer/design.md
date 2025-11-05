# Discovery Layer Design Document

## Overview

The Discovery Layer implements a multi-strategy peer discovery system for Kizuna, providing reliable peer detection across various network topologies and device capabilities. The system uses a unified trait-based architecture that supports mDNS, UDP broadcast, TCP handshake, Bluetooth LE, and libp2p hybrid discovery methods with intelligent auto-selection.

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Discovery Manager                         │
├─────────────────────────────────────────────────────────────┤
│  Auto-Selection Strategy │  Discovery Coordinator           │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                   Discovery Trait                          │
├─────────────────────────────────────────────────────────────┤
│  async fn discover() -> Result<Vec<ServiceRecord>>         │
│  async fn announce() -> Result<()>                         │
│  fn strategy_name() -> &'static str                        │
└─────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        ▼                     ▼                     ▼
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│mDNS Strategy│    │UDP Strategy │    │TCP Strategy │
└─────────────┘    └─────────────┘    └─────────────┘
        ▼                     ▼                     ▼
┌─────────────┐    ┌─────────────┐
│Bluetooth LE │    │libp2p Hybrid│
│Strategy     │    │Strategy     │
└─────────────┘    └─────────────┘
```

### Module Structure

```
src/discovery/
├── mod.rs              # Public API and Discovery trait
├── manager.rs          # Discovery Manager and auto-selection
├── service_record.rs   # ServiceRecord data structure
├── strategies/
│   ├── mod.rs          # Strategy trait implementations
│   ├── mdns.rs         # mDNS discovery implementation
│   ├── udp.rs          # UDP broadcast discovery
│   ├── tcp.rs          # TCP handshake beacon
│   ├── bluetooth.rs    # Bluetooth LE discovery
│   └── libp2p.rs       # libp2p hybrid discovery
└── error.rs            # Discovery-specific error types
```

## Components and Interfaces

### Discovery Trait

```rust
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
}
```

### ServiceRecord Data Structure

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceRecord {
    pub peer_id: String,
    pub name: String,
    pub addresses: Vec<SocketAddr>,
    pub port: u16,
    pub discovery_method: String,
    pub capabilities: HashMap<String, String>,
    pub last_seen: SystemTime,
}

impl ServiceRecord {
    pub fn new(peer_id: String, name: String, port: u16) -> Self;
    pub fn add_address(&mut self, addr: SocketAddr);
    pub fn add_capability(&mut self, key: String, value: String);
    pub fn is_expired(&self, timeout: Duration) -> bool;
}
```

### Discovery Manager

```rust
pub struct DiscoveryManager {
    strategies: Vec<Box<dyn Discovery>>,
    auto_select: bool,
    active_strategy: Option<String>,
    discovered_peers: Arc<RwLock<HashMap<String, ServiceRecord>>>,
}

impl DiscoveryManager {
    pub fn new() -> Self;
    pub fn add_strategy(&mut self, strategy: Box<dyn Discovery>);
    pub async fn discover_peers(&self, timeout: Duration) -> Result<Vec<ServiceRecord>>;
    pub async fn announce_presence(&self) -> Result<()>;
    pub fn set_auto_select(&mut self, enabled: bool);
    pub fn get_discovered_peers(&self) -> Vec<ServiceRecord>;
}
```

## Data Models

### mDNS Strategy Data Flow

```
Announce: _kizuna._tcp.local
├── TXT Records:
│   ├── peer_id=<unique_id>
│   ├── name=<device_name>
│   ├── version=<kizuna_version>
│   └── capabilities=<feature_flags>
└── SRV Record: port, target

Discovery: Browse _kizuna._tcp.local
├── Parse TXT records → ServiceRecord
├── Resolve A/AAAA records → IP addresses
└── Return discovered peers
```

### UDP Broadcast Protocol

```
Discovery Message:
DISCOVER_KIZUNA|<peer_id>|<name>|<port>|<capabilities>

Response Message:
KIZUNA_PEER|<peer_id>|<name>|<port>|<addresses>|<capabilities>
```

### TCP Handshake Protocol

```
1. TCP Connect to target:port
2. Send: KIZUNA_HELLO|<version>|<peer_id>
3. Receive: KIZUNA_PEER|<peer_id>|<name>|<capabilities>
4. Close connection
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),
    
    #[error("Strategy not available: {strategy}")]
    StrategyUnavailable { strategy: String },
    
    #[error("Discovery timeout after {timeout:?}")]
    Timeout { timeout: Duration },
    
    #[error("Invalid service record: {reason}")]
    InvalidServiceRecord { reason: String },
    
    #[error("Bluetooth error: {0}")]
    Bluetooth(String),
    
    #[error("libp2p error: {0}")]
    Libp2p(String),
}
```

### Error Recovery Strategy

1. **Network Failures**: Retry with exponential backoff
2. **Strategy Unavailable**: Fall back to next available strategy
3. **Timeout**: Reduce timeout and retry with different strategy
4. **Invalid Records**: Log and skip, continue with other peers
5. **Platform Errors**: Disable strategy and use alternatives

## Testing Strategy

### Unit Tests

```rust
// Test each strategy implementation
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mdns_discovery() {
        // Test mDNS announcement and discovery
    }
    
    #[tokio::test]
    async fn test_udp_broadcast() {
        // Test UDP broadcast protocol
    }
    
    #[tokio::test]
    async fn test_tcp_handshake() {
        // Test TCP probe and handshake
    }
    
    #[tokio::test]
    async fn test_auto_selection() {
        // Test strategy selection logic
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_cross_strategy_discovery() {
    // Test peer announced via mDNS discovered via UDP
}

#[tokio::test]
async fn test_fallback_behavior() {
    // Test fallback when primary strategy fails
}

#[tokio::test]
async fn test_concurrent_strategies() {
    // Test multiple strategies running simultaneously
}
```

### Platform-Specific Tests

- **Linux**: All strategies enabled, test privilege requirements
- **macOS**: mDNS via Bonjour, Bluetooth LE permissions
- **Windows**: mDNS via Bonjour service, Windows firewall
- **Android/iOS**: Bluetooth LE focus, network permissions

## Implementation Details

### mDNS Strategy (mdns.rs)

- Use `mdns` crate for cross-platform mDNS support
- Service name: `_kizuna._tcp.local`
- TXT record format: `peer_id=<id>,name=<name>,version=<ver>`
- Handle IPv4/IPv6 dual-stack scenarios
- Implement proper cleanup on shutdown

### UDP Broadcast Strategy (udp.rs)

- Broadcast on port 41337 (configurable)
- Rate limiting: max 1 broadcast per 5 seconds
- Listen on all interfaces for responses
- Handle subnet broadcast addresses correctly
- Implement message parsing with error recovery

### TCP Handshake Strategy (tcp.rs)

- Scan common ports: 41337, 8080, 3000-3010
- Connection timeout: 2 seconds per probe
- Parallel scanning with connection limits
- Proper handshake protocol implementation
- Handle connection refused gracefully

### Bluetooth LE Strategy (bluetooth.rs)

- Service UUID: `6ba7b810-9dad-11d1-80b4-00c04fd430c8`
- Advertisement data includes peer_id and name
- Scan duration: 10 seconds default
- Handle platform permission requirements
- Graceful degradation when Bluetooth unavailable

### libp2p Hybrid Strategy (libp2p.rs)

- Combine local mDNS with Kademlia DHT
- Generate persistent peer IDs
- Implement NAT traversal with relay support
- Handle bootstrap node configuration
- Manage connection lifecycle properly

### Auto-Selection Algorithm

```rust
impl DiscoveryManager {
    async fn select_best_strategy(&self) -> Option<&dyn Discovery> {
        let mut available: Vec<_> = self.strategies
            .iter()
            .filter(|s| s.is_available())
            .collect();
        
        // Sort by priority (higher first)
        available.sort_by_key(|s| std::cmp::Reverse(s.priority()));
        
        // Test connectivity and latency
        for strategy in available {
            if let Ok(_) = self.test_strategy_connectivity(strategy).await {
                return Some(strategy.as_ref());
            }
        }
        
        None
    }
}
```

## Performance Considerations

### Discovery Timing

- **mDNS**: 1-3 seconds typical response time
- **UDP Broadcast**: Sub-second response time
- **TCP Handshake**: 2-5 seconds depending on scan range
- **Bluetooth LE**: 5-10 seconds scan duration
- **libp2p**: Variable, 1-30 seconds depending on DHT

### Resource Usage

- Memory: ~1MB per active strategy
- Network: Minimal bandwidth usage (<1KB/s per strategy)
- CPU: Low impact, mostly I/O bound operations
- Battery: Bluetooth LE scanning has highest impact

### Scalability

- Support up to 100 concurrent peers per strategy
- Implement peer cache with TTL to reduce redundant discoveries
- Use connection pooling for TCP handshake strategy
- Rate limit announcements to prevent network flooding

## Security Considerations

### Peer Verification

- Validate peer_id format and uniqueness
- Implement basic sanity checks on service records
- Rate limit incoming discovery requests
- Validate network addresses and ports

### Privacy Protection

- Optional anonymous mode (random peer_id)
- Local-only visibility option
- No sensitive information in discovery payloads
- Proper cleanup of announced services on shutdown

### Attack Mitigation

- Prevent discovery amplification attacks
- Validate message formats strictly
- Implement connection limits per IP
- Log suspicious discovery patterns