// Transport Layer Integration Module
//
// Integrates file transfer system with transport layer for connection management,
// protocol-specific optimizations, and connection pooling

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::*,
    ChunkStream,
};
use crate::transport::{
    Connection, PeerAddress, PeerId as TransportPeerId,
    TransportCapabilities as TransportCaps,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Transport integration for file transfers
pub struct FileTransferTransport {
    /// Connection pool for reusing connections across transfers
    connection_pool: Arc<RwLock<HashMap<PeerId, Arc<RwLock<Box<dyn Connection>>>>>>,
    /// Protocol-specific configurations
    protocol_configs: Arc<RwLock<HashMap<TransportProtocol, ProtocolConfig>>>,
}

/// Protocol-specific configuration and optimizations
#[derive(Debug, Clone)]
pub struct ProtocolConfig {
    /// Buffer size for chunk transmission
    pub buffer_size: usize,
    /// Maximum concurrent chunks in flight
    pub max_concurrent_chunks: usize,
    /// Enable Nagle's algorithm (TCP only)
    pub enable_nagle: bool,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Keep-alive interval in seconds
    pub keep_alive_interval: Option<u64>,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            buffer_size: 64 * 1024, // 64KB
            max_concurrent_chunks: 4,
            enable_nagle: false,
            connection_timeout: 30,
            keep_alive_interval: Some(60),
        }
    }
}

impl ProtocolConfig {
    /// Get optimized configuration for QUIC protocol
    pub fn quic() -> Self {
        Self {
            buffer_size: 128 * 1024, // 128KB for QUIC
            max_concurrent_chunks: 8, // QUIC supports more parallelism
            enable_nagle: false,
            connection_timeout: 30,
            keep_alive_interval: Some(30),
        }
    }

    /// Get optimized configuration for TCP protocol
    pub fn tcp() -> Self {
        Self {
            buffer_size: 64 * 1024, // 64KB for TCP
            max_concurrent_chunks: 4,
            enable_nagle: false, // Disable for lower latency
            connection_timeout: 30,
            keep_alive_interval: Some(60),
        }
    }

    /// Get optimized configuration for WebRTC protocol
    pub fn webrtc() -> Self {
        Self {
            buffer_size: 16 * 1024, // 16KB for WebRTC (DataChannel limit)
            max_concurrent_chunks: 4,
            enable_nagle: false,
            connection_timeout: 45, // Longer timeout for NAT traversal
            keep_alive_interval: Some(30),
        }
    }
}

impl FileTransferTransport {
    /// Create a new transport integration
    pub fn new() -> Self {
        let mut protocol_configs = HashMap::new();
        protocol_configs.insert(TransportProtocol::Quic, ProtocolConfig::quic());
        protocol_configs.insert(TransportProtocol::Tcp, ProtocolConfig::tcp());
        protocol_configs.insert(TransportProtocol::WebRtc, ProtocolConfig::webrtc());

        Self {
            connection_pool: Arc::new(RwLock::new(HashMap::new())),
            protocol_configs: Arc::new(RwLock::new(protocol_configs)),
        }
    }

    /// Add a connection to the pool
    pub async fn add_connection(&self, peer_id: PeerId, connection: Box<dyn Connection>) {
        let mut pool = self.connection_pool.write().await;
        pool.insert(peer_id, Arc::new(RwLock::new(connection)));
    }

    /// Get a connection from the pool
    pub async fn get_connection(&self, peer_id: &PeerId) -> Result<Arc<RwLock<Box<dyn Connection>>>> {
        let pool = self.connection_pool.read().await;
        
        if let Some(conn) = pool.get(peer_id) {
            // Verify connection is still active
            let conn_guard = conn.read().await;
            if conn_guard.is_connected() {
                drop(conn_guard);
                return Ok(Arc::clone(conn));
            }
        }

        Err(FileTransferError::NetworkError {
            reason: format!("No active connection to peer {}", peer_id),
        })
    }

    /// Release a connection back to the pool
    pub async fn release_connection(&self, peer_id: &PeerId) {
        // Connection remains in pool for reuse
        // Actual cleanup happens in cleanup_idle_connections
    }

    /// Remove a connection from the pool
    pub async fn remove_connection(&self, peer_id: &PeerId) {
        let mut pool = self.connection_pool.write().await;
        pool.remove(peer_id);
    }

    /// Cleanup idle connections from the pool
    pub async fn cleanup_idle_connections(&self) -> usize {
        let mut pool = self.connection_pool.write().await;
        let initial_count = pool.len();

        // Collect disconnected peer IDs
        let mut disconnected = Vec::new();
        for (peer_id, conn) in pool.iter() {
            let conn_guard = conn.read().await;
            if !conn_guard.is_connected() {
                disconnected.push(peer_id.clone());
            }
        }

        // Remove disconnected connections
        for peer_id in &disconnected {
            pool.remove(peer_id);
        }

        disconnected.len()
    }

    /// Get protocol configuration
    pub async fn get_protocol_config(&self, protocol: TransportProtocol) -> ProtocolConfig {
        let configs = self.protocol_configs.read().await;
        configs.get(&protocol).cloned().unwrap_or_default()
    }

    /// Set protocol configuration
    pub async fn set_protocol_config(&self, protocol: TransportProtocol, config: ProtocolConfig) {
        let mut configs = self.protocol_configs.write().await;
        configs.insert(protocol, config);
    }

    /// Get transport capabilities for a peer
    pub async fn get_peer_capabilities(&self, peer_id: &PeerId) -> Result<TransportCapabilities> {
        // Query transport layer for peer capabilities
        // For now, return default capabilities
        // TODO: Implement actual capability query from transport layer
        Ok(TransportCapabilities::default())
    }

    /// Create a chunk stream wrapper for a connection
    pub async fn create_chunk_stream(
        &self,
        peer_id: &PeerId,
        protocol: TransportProtocol,
    ) -> Result<Box<dyn ChunkStream>> {
        let connection = self.get_connection(peer_id).await?;
        let config = self.get_protocol_config(protocol).await;
        
        Ok(Box::new(TransportChunkStream::new(Arc::clone(&connection), config)))
    }

    /// Get connection pool statistics
    pub async fn get_pool_stats(&self) -> ConnectionPoolStats {
        let pool = self.connection_pool.read().await;
        let total_connections = pool.len();
        
        let mut active_connections = 0;
        for conn in pool.values() {
            let conn_guard = conn.read().await;
            if conn_guard.is_connected() {
                active_connections += 1;
            }
        }

        ConnectionPoolStats {
            total_connections,
            active_connections,
            idle_connections: total_connections - active_connections,
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct ConnectionPoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub idle_connections: usize,
}

/// Chunk stream implementation using transport layer connection
struct TransportChunkStream {
    connection: Arc<RwLock<Box<dyn Connection>>>,
    config: ProtocolConfig,
    buffer: Vec<u8>,
}

impl TransportChunkStream {
    fn new(connection: Arc<RwLock<Box<dyn Connection>>>, config: ProtocolConfig) -> Self {
        Self {
            connection,
            buffer: vec![0u8; config.buffer_size],
            config,
        }
    }
}

#[async_trait]
impl ChunkStream for TransportChunkStream {
    async fn send(&mut self, data: &[u8]) -> Result<()> {
        let mut conn = self.connection.write().await;
        conn.write(data)
            .await
            .map_err(|e| FileTransferError::NetworkError {
                reason: format!("Failed to send data: {}", e),
            })?;
        Ok(())
    }

    async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let mut conn = self.connection.write().await;
        conn.read(buffer)
            .await
            .map_err(|e| FileTransferError::NetworkError {
                reason: format!("Failed to receive data: {}", e),
            })
    }

    async fn flush(&mut self) -> Result<()> {
        let mut conn = self.connection.write().await;
        conn.flush()
            .await
            .map_err(|e| FileTransferError::NetworkError {
                reason: format!("Failed to flush: {}", e),
            })
    }
}

/// Transport protocol mapper
pub struct ProtocolMapper;

impl ProtocolMapper {
    /// Map file transfer transport protocol to transport layer protocol hint
    pub fn to_transport_hint(protocol: TransportProtocol) -> String {
        match protocol {
            TransportProtocol::Quic => "quic".to_string(),
            TransportProtocol::Tcp => "tcp".to_string(),
            TransportProtocol::WebRtc => "webrtc".to_string(),
        }
    }

    /// Map transport layer capabilities to file transfer capabilities
    pub fn from_transport_capabilities(caps: &TransportCaps) -> TransportCapabilities {
        TransportCapabilities {
            supports_quic: caps.multiplexed && caps.resumable,
            supports_tcp: caps.reliable && caps.ordered,
            supports_webrtc: caps.nat_traversal,
            max_parallel_streams: if caps.multiplexed { 8 } else { 1 },
            max_bandwidth: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{ConnectionInfo, TransportError};
    use std::net::SocketAddr;

    // Mock connection for testing
    struct MockConnection {
        connected: bool,
    }

    #[async_trait]
    impl Connection for MockConnection {
        async fn read(&mut self, buf: &mut [u8]) -> std::result::Result<usize, TransportError> {
            Ok(buf.len())
        }

        async fn write(&mut self, _buf: &[u8]) -> std::result::Result<usize, TransportError> {
            Ok(_buf.len())
        }

        async fn flush(&mut self) -> std::result::Result<(), TransportError> {
            Ok(())
        }

        async fn close(&mut self) -> std::result::Result<(), TransportError> {
            Ok(())
        }

        fn info(&self) -> ConnectionInfo {
            ConnectionInfo::new(
                "test-peer".to_string(),
                "127.0.0.1:9090".parse().unwrap(),
                "127.0.0.1:8080".parse().unwrap(),
                "tcp".to_string(),
            )
        }

        fn is_connected(&self) -> bool {
            self.connected
        }
    }

    #[tokio::test]
    async fn test_protocol_config_defaults() {
        let config = ProtocolConfig::default();
        assert_eq!(config.buffer_size, 64 * 1024);
        assert_eq!(config.max_concurrent_chunks, 4);
        assert!(!config.enable_nagle);
    }

    #[tokio::test]
    async fn test_protocol_config_quic() {
        let config = ProtocolConfig::quic();
        assert_eq!(config.buffer_size, 128 * 1024);
        assert_eq!(config.max_concurrent_chunks, 8);
    }

    #[tokio::test]
    async fn test_protocol_config_tcp() {
        let config = ProtocolConfig::tcp();
        assert_eq!(config.buffer_size, 64 * 1024);
        assert_eq!(config.max_concurrent_chunks, 4);
    }

    #[tokio::test]
    async fn test_protocol_config_webrtc() {
        let config = ProtocolConfig::webrtc();
        assert_eq!(config.buffer_size, 16 * 1024);
        assert_eq!(config.connection_timeout, 45);
    }

    #[tokio::test]
    async fn test_file_transfer_transport_creation() {
        let transport = FileTransferTransport::new();
        
        let stats = transport.get_pool_stats().await;
        assert_eq!(stats.total_connections, 0);
    }

    #[tokio::test]
    async fn test_add_and_get_connection() {
        let transport = FileTransferTransport::new();
        
        let peer_id = "test-peer".to_string();
        let conn = Box::new(MockConnection { connected: true });
        
        transport.add_connection(peer_id.clone(), conn).await;
        
        let retrieved = transport.get_connection(&peer_id).await.unwrap();
        let conn_guard = retrieved.read().await;
        assert!(conn_guard.is_connected());
    }

    #[tokio::test]
    async fn test_connection_pooling() {
        let transport = FileTransferTransport::new();
        
        let peer_id = "test-peer".to_string();
        let conn = Box::new(MockConnection { connected: true });
        
        transport.add_connection(peer_id.clone(), conn).await;
        
        let stats1 = transport.get_pool_stats().await;
        assert_eq!(stats1.total_connections, 1);
        
        // Second request should reuse connection
        let _conn2 = transport.get_connection(&peer_id).await.unwrap();
        let stats2 = transport.get_pool_stats().await;
        assert_eq!(stats2.total_connections, 1);
    }

    #[tokio::test]
    async fn test_remove_connection() {
        let transport = FileTransferTransport::new();
        
        let peer_id = "test-peer".to_string();
        let conn = Box::new(MockConnection { connected: true });
        
        transport.add_connection(peer_id.clone(), conn).await;
        let stats1 = transport.get_pool_stats().await;
        assert_eq!(stats1.total_connections, 1);
        
        // Remove connection
        transport.remove_connection(&peer_id).await;
        let stats2 = transport.get_pool_stats().await;
        assert_eq!(stats2.total_connections, 0);
    }

    #[tokio::test]
    async fn test_get_protocol_config() {
        let transport = FileTransferTransport::new();
        
        let quic_config = transport.get_protocol_config(TransportProtocol::Quic).await;
        assert_eq!(quic_config.buffer_size, 128 * 1024);
        
        let tcp_config = transport.get_protocol_config(TransportProtocol::Tcp).await;
        assert_eq!(tcp_config.buffer_size, 64 * 1024);
    }

    #[tokio::test]
    async fn test_set_protocol_config() {
        let transport = FileTransferTransport::new();
        
        let custom_config = ProtocolConfig {
            buffer_size: 256 * 1024,
            max_concurrent_chunks: 16,
            enable_nagle: true,
            connection_timeout: 60,
            keep_alive_interval: Some(120),
        };
        
        transport.set_protocol_config(TransportProtocol::Quic, custom_config.clone()).await;
        
        let retrieved_config = transport.get_protocol_config(TransportProtocol::Quic).await;
        assert_eq!(retrieved_config.buffer_size, 256 * 1024);
        assert_eq!(retrieved_config.max_concurrent_chunks, 16);
    }

    #[tokio::test]
    async fn test_protocol_mapper_to_transport_hint() {
        assert_eq!(ProtocolMapper::to_transport_hint(TransportProtocol::Quic), "quic");
        assert_eq!(ProtocolMapper::to_transport_hint(TransportProtocol::Tcp), "tcp");
        assert_eq!(ProtocolMapper::to_transport_hint(TransportProtocol::WebRtc), "webrtc");
    }

    #[tokio::test]
    async fn test_protocol_mapper_from_transport_capabilities() {
        let transport_caps = TransportCaps {
            reliable: true,
            ordered: true,
            multiplexed: true,
            resumable: true,
            nat_traversal: false,
            max_message_size: None,
        };
        
        let ft_caps = ProtocolMapper::from_transport_capabilities(&transport_caps);
        assert!(ft_caps.supports_quic);
        assert!(ft_caps.supports_tcp);
        assert!(!ft_caps.supports_webrtc);
    }

    #[tokio::test]
    async fn test_chunk_stream_send_receive() {
        let connection = Arc::new(RwLock::new(Box::new(MockConnection { connected: true }) as Box<dyn Connection>));
        let config = ProtocolConfig::default();
        let mut stream = TransportChunkStream::new(connection, config);
        
        // Test send
        let data = vec![1, 2, 3, 4, 5];
        let result = stream.send(&data).await;
        assert!(result.is_ok());
        
        // Test receive
        let mut buffer = vec![0u8; 10];
        let result = stream.receive(&mut buffer).await;
        assert!(result.is_ok());
    }
}
