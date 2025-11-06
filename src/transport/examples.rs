/*!
# Kizuna Transport System Examples

This module provides comprehensive examples of how to use the Kizuna Transport System
for peer-to-peer communication with automatic discovery integration.

## Basic Usage

### Simple Transport Setup

```rust
use kizuna::transport::{KizunaTransport, KizunaTransportConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create transport with default configuration
    let transport = KizunaTransport::new().await?;
    
    // Start listening for incoming connections
    let bind_addr = "127.0.0.1:8080".parse()?;
    transport.start_listening(bind_addr).await?;
    
    println!("Transport system started on {}", bind_addr);
    Ok(())
}
```

### Custom Configuration

```rust
use kizuna::transport::{KizunaTransportBuilder, NatTraversalConfig, RelayConfig};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let transport = KizunaTransportBuilder::new()
        .connection_timeout(Duration::from_secs(15))
        .auto_retry(true)
        .max_retry_attempts(5)
        .enable_protocols(vec!["tcp".to_string(), "quic".to_string()])
        .nat_traversal_config(NatTraversalConfig {
            stun_servers: vec!["stun:stun.l.google.com:19302".to_string()],
            enable_hole_punching: true,
            hole_punch_timeout: Duration::from_secs(10),
        })
        .build()
        .await?;
    
    let bind_addr = "0.0.0.0:8080".parse()?;
    transport.start_listening(bind_addr).await?;
    
    println!("Custom transport system started");
    Ok(())
}
```

## Connection Management

### Manual Connection

```rust
use kizuna::transport::{KizunaTransport, PeerAddress, TransportCapabilities};
use std::net::SocketAddr;

async fn connect_to_peer() -> Result<(), Box<dyn std::error::Error>> {
    let transport = KizunaTransport::new().await?;
    
    // Create peer address
    let peer_address = PeerAddress::new(
        "peer-123".to_string(),
        vec!["192.168.1.100:8080".parse::<SocketAddr>()?],
        vec!["tcp".to_string(), "quic".to_string()],
        TransportCapabilities::quic(),
    );
    
    // Connect to peer
    let connection = transport.connect_to_peer(&peer_address).await?;
    
    // Send data
    let message = b"Hello, peer!";
    connection.write(message).await?;
    connection.flush().await?;
    
    // Read response
    let mut buffer = [0u8; 1024];
    let bytes_read = connection.read(&mut buffer).await?;
    println!("Received: {}", String::from_utf8_lossy(&buffer[..bytes_read]));
    
    Ok(())
}
```

### Protocol-Specific Connection

```rust
use kizuna::transport::{KizunaTransport, PeerAddress, TransportCapabilities};

async fn connect_with_quic() -> Result<(), Box<dyn std::error::Error>> {
    let transport = KizunaTransport::new().await?;
    
    let peer_address = PeerAddress::new(
        "quic-peer".to_string(),
        vec!["192.168.1.100:8080".parse()?],
        vec!["quic".to_string()],
        TransportCapabilities::quic(),
    );
    
    // Force QUIC protocol
    let connection = transport.connect_with_protocol(&peer_address, "quic").await?;
    
    println!("Connected via QUIC protocol");
    Ok(())
}
```

## Event Handling

### Connection Callbacks

```rust
use kizuna::transport::{
    KizunaTransport, ConnectionCallback, ConnectionEvent, ConnectionQuality, 
    TransportError, PeerId
};
use async_trait::async_trait;
use std::sync::Arc;

struct MyConnectionCallback;

#[async_trait]
impl ConnectionCallback for MyConnectionCallback {
    async fn on_connection_event(&self, event: ConnectionEvent) {
        match event {
            ConnectionEvent::Connected { peer_id, protocol, connection_info } => {
                println!("✅ Connected to {} via {} ({})", 
                    peer_id, protocol, connection_info.remote_addr);
            }
            ConnectionEvent::ConnectionFailed { peer_id, protocol, error, attempt } => {
                println!("❌ Connection to {} failed via {} (attempt {}): {}", 
                    peer_id, protocol, attempt, error);
            }
            ConnectionEvent::Disconnected { peer_id, reason } => {
                println!("🔌 Disconnected from {}: {}", peer_id, reason);
            }
            ConnectionEvent::DataReceived { peer_id, bytes } => {
                println!("📥 Received {} bytes from {}", bytes, peer_id);
            }
            ConnectionEvent::DataSent { peer_id, bytes } => {
                println!("📤 Sent {} bytes to {}", bytes, peer_id);
            }
            _ => {}
        }
    }
    
    async fn on_connection_quality_change(&self, peer_id: PeerId, quality: ConnectionQuality) {
        println!("📊 Connection quality for {}: {:?} (RTT: {:?})", 
            peer_id, quality.quality_class, quality.rtt_ms);
    }
    
    async fn on_error(&self, error: TransportError, context: String) {
        eprintln!("🚨 Transport error in {}: {}", context, error);
    }
}

async fn setup_with_callbacks() -> Result<(), Box<dyn std::error::Error>> {
    let transport = KizunaTransport::new().await?;
    
    // Register callback
    let callback = Arc::new(MyConnectionCallback);
    transport.register_callback(callback).await;
    
    // Start listening
    transport.start_listening("127.0.0.1:8080".parse()?).await?;
    
    Ok(())
}
```

## Discovery Integration

### Automatic Peer Discovery and Connection

```rust
use kizuna::transport::{
    TransportDiscoveryBridge, KizunaTransportConfig, TransportDiscoveryConfig,
    TransportDiscoveryCallback, TransportDiscoveryEvent, ServiceRecord, PeerId
};
use async_trait::async_trait;
use std::sync::Arc;

struct MyDiscoveryCallback;

#[async_trait]
impl TransportDiscoveryCallback for MyDiscoveryCallback {
    async fn on_transport_discovery_event(&self, event: TransportDiscoveryEvent) {
        match event {
            TransportDiscoveryEvent::PeerDiscovered { peer_id, service_record, discovery_method } => {
                println!("🔍 Discovered peer {} via {} ({})", 
                    peer_id, discovery_method, service_record.device_name);
            }
            TransportDiscoveryEvent::AutoConnectSucceeded { peer_id, protocol, connection_info } => {
                println!("🤝 Auto-connected to {} via {} ({})", 
                    peer_id, protocol, connection_info);
            }
            TransportDiscoveryEvent::AutoConnectFailed { peer_id, protocol, error, will_retry } => {
                println!("💥 Auto-connect to {} failed via {}: {} (retry: {})", 
                    peer_id, protocol, error, will_retry);
            }
            TransportDiscoveryEvent::PeerLost { peer_id, reason } => {
                println!("👻 Lost peer {}: {}", peer_id, reason);
            }
            _ => {}
        }
    }
    
    async fn should_auto_connect(&self, peer_id: &PeerId, service_record: &ServiceRecord) -> bool {
        // Only auto-connect to peers with "kizuna" in their name
        service_record.device_name.to_lowercase().contains("kizuna")
    }
    
    async fn select_protocol(&self, _peer_id: &PeerId, available_protocols: &[String]) -> Option<String> {
        // Prefer QUIC if available
        if available_protocols.contains(&"quic".to_string()) {
            Some("quic".to_string())
        } else {
            None // Use default negotiation
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure transport
    let transport_config = KizunaTransportConfig::default();
    
    // Configure discovery integration
    let integration_config = TransportDiscoveryConfig {
        auto_connect: true,
        max_auto_connections: 2,
        advertised_protocols: vec!["tcp".to_string(), "quic".to_string()],
        ..Default::default()
    };
    
    // Create integrated bridge
    let bridge = TransportDiscoveryBridge::new(transport_config, integration_config).await?;
    
    // Register callback
    let callback = Arc::new(MyDiscoveryCallback);
    bridge.register_callback(callback).await;
    
    // Start the integrated system
    let bind_addr = "0.0.0.0:8080".parse()?;
    bridge.start(bind_addr).await?;
    
    // Announce our presence
    bridge.announce_presence("My Kizuna Node".to_string(), 8080).await?;
    
    println!("🚀 Integrated transport-discovery system started!");
    
    // Keep running
    tokio::signal::ctrl_c().await?;
    
    // Graceful shutdown
    bridge.stop().await?;
    println!("👋 System stopped gracefully");
    
    Ok(())
}
```

### Manual Discovery Integration

```rust
use kizuna::transport::{TransportDiscoveryBridge, KizunaTransportConfig, TransportDiscoveryConfig};

async fn manual_discovery_example() -> Result<(), Box<dyn std::error::Error>> {
    let transport_config = KizunaTransportConfig::default();
    let integration_config = TransportDiscoveryConfig {
        auto_connect: false, // Disable auto-connect
        ..Default::default()
    };
    
    let bridge = TransportDiscoveryBridge::new(transport_config, integration_config).await?;
    
    // Start without auto-connect
    bridge.start("127.0.0.1:8080".parse()?).await?;
    
    // Wait a bit for discovery
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    
    // Get discovered peers
    let peers = bridge.get_discovered_peers().await;
    println!("Discovered {} peers", peers.len());
    
    // Manually connect to first peer
    if let Some(peer) = peers.first() {
        match bridge.connect_to_peer(&peer.peer_id).await {
            Ok(connection) => {
                println!("✅ Manually connected to {}", peer.peer_id);
                
                // Use the connection
                let message = b"Hello from manual connection!";
                connection.write(message).await?;
            }
            Err(e) => {
                println!("❌ Failed to connect to {}: {}", peer.peer_id, e);
            }
        }
    }
    
    Ok(())
}
```

## Advanced Usage

### Connection Pool Management

```rust
use kizuna::transport::KizunaTransport;

async fn connection_pool_example() -> Result<(), Box<dyn std::error::Error>> {
    let transport = KizunaTransport::new().await?;
    transport.start_listening("127.0.0.1:8080".parse()?).await?;
    
    // Get connection statistics
    let stats = transport.get_connection_stats().await;
    println!("Active connections: {}", stats.total_connections);
    println!("Connected peers: {}", stats.active_peers);
    println!("Average quality: {:.2}", stats.average_connection_quality);
    
    // List connections by protocol
    for (protocol, count) in stats.connections_by_protocol {
        println!("  {}: {} connections", protocol, count);
    }
    
    // Get all active peers
    let active_peers = transport.get_active_peers().await;
    for peer_id in active_peers {
        let connections = transport.get_connections(&peer_id).await;
        println!("Peer {}: {} connections", peer_id, connections.len());
        
        for connection in connections {
            let info = connection.info().await;
            let quality = connection.quality().await;
            println!("  - {} ({}): {:?}", 
                info.protocol, info.remote_addr, quality.quality_class);
        }
    }
    
    Ok(())
}
```

### Health Monitoring

```rust
use kizuna::transport::KizunaTransport;

async fn health_monitoring_example() -> Result<(), Box<dyn std::error::Error>> {
    let transport = KizunaTransport::new().await?;
    transport.start_listening("127.0.0.1:8080".parse()?).await?;
    
    // Monitor system health
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        
        loop {
            interval.tick().await;
            
            let health_report = transport.get_health_report().await;
            println!("🏥 System Health: {:?}", health_report.overall_health);
            
            let system_state = transport.get_system_state().await;
            println!("📊 System State: degraded={}, uptime={:?}", 
                system_state.degraded_mode, system_state.uptime);
            
            // Check if system needs attention
            if matches!(health_report.overall_health, 
                       kizuna::transport::HealthStatus::Degraded | 
                       kizuna::transport::HealthStatus::Unhealthy) {
                println!("⚠️  System health degraded, consider investigation");
            }
        }
    });
    
    Ok(())
}
```

### Error Handling and Retry Logic

```rust
use kizuna::transport::{KizunaTransport, PeerAddress, TransportCapabilities, TransportError};
use std::time::Duration;

async fn robust_connection_example() -> Result<(), Box<dyn std::error::Error>> {
    let transport = KizunaTransport::new().await?;
    
    let peer_address = PeerAddress::new(
        "target-peer".to_string(),
        vec!["192.168.1.100:8080".parse()?],
        vec!["tcp".to_string(), "quic".to_string()],
        TransportCapabilities::default(),
    );
    
    // Retry connection with exponential backoff
    let mut retry_delay = Duration::from_millis(100);
    let max_retries = 5;
    
    for attempt in 1..=max_retries {
        match transport.connect_to_peer(&peer_address).await {
            Ok(connection) => {
                println!("✅ Connected on attempt {}", attempt);
                
                // Test connection with ping
                match test_connection(&connection).await {
                    Ok(_) => {
                        println!("🏓 Connection test passed");
                        return Ok(());
                    }
                    Err(e) => {
                        println!("❌ Connection test failed: {}", e);
                        let _ = connection.close().await;
                    }
                }
            }
            Err(TransportError::ConnectionTimeout { .. }) => {
                println!("⏰ Connection timeout on attempt {}", attempt);
            }
            Err(TransportError::ConnectionFailed { reason }) => {
                println!("💥 Connection failed on attempt {}: {}", attempt, reason);
            }
            Err(e) => {
                println!("🚨 Unexpected error on attempt {}: {}", attempt, e);
                break; // Don't retry on unexpected errors
            }
        }
        
        if attempt < max_retries {
            println!("⏳ Retrying in {:?}...", retry_delay);
            tokio::time::sleep(retry_delay).await;
            retry_delay = std::cmp::min(retry_delay * 2, Duration::from_secs(30));
        }
    }
    
    Err("Failed to establish connection after all retries".into())
}

async fn test_connection(connection: &kizuna::transport::ConnectionHandle) -> Result<(), Box<dyn std::error::Error>> {
    // Send ping
    let ping = b"PING";
    connection.write(ping).await?;
    connection.flush().await?;
    
    // Wait for pong
    let mut buffer = [0u8; 4];
    let bytes_read = connection.read(&mut buffer).await?;
    
    if bytes_read == 4 && &buffer == b"PONG" {
        Ok(())
    } else {
        Err("Invalid ping response".into())
    }
}
```

## Performance Optimization

### Bandwidth Management

```rust
use kizuna::transport::{KizunaTransportBuilder, KizunaTransportConfig};
use std::time::Duration;

async fn bandwidth_optimized_setup() -> Result<(), Box<dyn std::error::Error>> {
    let transport = KizunaTransportBuilder::new()
        .connection_timeout(Duration::from_secs(10))
        .keep_alive_interval(Duration::from_secs(30))
        .performance_monitoring(true)
        .enable_protocols(vec![
            "quic".to_string(),    // Prefer QUIC for efficiency
            "tcp".to_string(),     // Fallback to TCP
        ])
        .build()
        .await?;
    
    transport.start_listening("0.0.0.0:8080".parse()?).await?;
    
    // Monitor bandwidth usage
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            
            let stats = transport.get_connection_stats().await;
            println!("📈 Bandwidth stats: {} connections, quality: {:.2}", 
                stats.total_connections, stats.average_connection_quality);
            
            // Implement custom bandwidth throttling if needed
            if stats.average_connection_quality < 0.5 {
                println!("⚠️  Poor connection quality detected, consider reducing load");
            }
        }
    });
    
    Ok(())
}
```

## Testing and Development

### Mock Transport for Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kizuna::transport::{KizunaTransport, TransportDiscoveryBridge};
    
    #[tokio::test]
    async fn test_transport_setup() {
        let transport = KizunaTransport::new().await.unwrap();
        
        // Test configuration
        let config = transport.get_config();
        assert!(config.auto_retry);
        assert_eq!(config.max_retry_attempts, 3);
        
        // Test supported protocols
        assert!(transport.supports_protocol("tcp"));
        assert!(transport.supports_protocol("quic"));
    }
    
    #[tokio::test]
    async fn test_discovery_integration() {
        let transport_config = KizunaTransportConfig::default();
        let integration_config = TransportDiscoveryConfig::default();
        
        let bridge = TransportDiscoveryBridge::new(transport_config, integration_config)
            .await.unwrap();
        
        let stats = bridge.get_integration_stats().await;
        assert_eq!(stats.total_discovered_peers, 0);
        assert!(stats.auto_connect_enabled);
    }
}
```

## Configuration Reference

### Transport Configuration Options

```rust
use kizuna::transport::{KizunaTransportConfig, NatTraversalConfig, RelayConfig};
use std::time::Duration;

fn create_production_config() -> KizunaTransportConfig {
    KizunaTransportConfig {
        max_connections_per_peer: 3,
        connection_timeout: Duration::from_secs(20),
        keep_alive_interval: Duration::from_secs(45),
        auto_retry: true,
        max_retry_attempts: 5,
        enable_connection_pooling: true,
        enable_performance_monitoring: true,
        enable_detailed_logging: false, // Disable in production for performance
        enabled_protocols: vec![
            "quic".to_string(),
            "webrtc".to_string(),
            "tcp".to_string(),
            "websocket".to_string(),
        ],
        nat_traversal_config: Some(NatTraversalConfig {
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
                "stun:stun.cloudflare.com:3478".to_string(),
            ],
            enable_hole_punching: true,
            hole_punch_timeout: Duration::from_secs(15),
        }),
        relay_config: Some(RelayConfig {
            relay_servers: vec![
                "wss://relay1.example.com/kizuna".to_string(),
                "wss://relay2.example.com/kizuna".to_string(),
            ],
            enable_auto_fallback: true,
            relay_timeout: Duration::from_secs(10),
        }),
    }
}
```

### Discovery Integration Configuration

```rust
use kizuna::transport::{TransportDiscoveryConfig, TransportCapabilities};
use std::time::Duration;

fn create_discovery_config() -> TransportDiscoveryConfig {
    TransportDiscoveryConfig {
        auto_connect: true,
        max_auto_connections: 2,
        auto_connect_timeout: Duration::from_secs(25),
        advertised_protocols: vec![
            "quic".to_string(),
            "webrtc".to_string(),
        ],
        advertised_capabilities: TransportCapabilities {
            reliable: true,
            ordered: true,
            multiplexed: true,
            resumable: true,
            nat_traversal: true,
            max_message_size: Some(1024 * 1024), // 1MB
        },
        enable_capability_exchange: true,
        retry_failed_connections: true,
        max_retry_attempts: 3,
        retry_delay: Duration::from_secs(2),
    }
}
```
*/

// This module is for documentation and examples only
// The actual implementation is in the other modules