//! Transport layer integration for clipboard synchronization
//! 
//! Provides peer communication, connection management, and optimized
//! content transmission for clipboard operations.

use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::clipboard::{ClipboardContent, ClipboardResult, ClipboardError, PeerId, DeviceId};
use crate::transport::{KizunaTransport, ConnectionHandle, PeerAddress, TransportCapabilities};

/// Message types for clipboard synchronization protocol
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ClipboardMessage {
    /// Sync clipboard content to peer
    SyncContent {
        /// Encrypted clipboard content
        content: Vec<u8>,
        /// Timestamp of the content
        timestamp: u64,
        /// Sequence number for ordering
        sequence: u64,
    },
    /// Acknowledge receipt of content
    SyncAck {
        /// Sequence number being acknowledged
        sequence: u64,
        /// Success status
        success: bool,
        /// Optional error message
        error: Option<String>,
    },
    /// Request clipboard content from peer
    ContentRequest {
        /// Request ID for tracking
        request_id: String,
    },
    /// Response to content request
    ContentResponse {
        /// Request ID being responded to
        request_id: String,
        /// Encrypted clipboard content (if available)
        content: Option<Vec<u8>>,
    },
    /// Ping message for connection keep-alive
    Ping {
        /// Timestamp of ping
        timestamp: u64,
    },
    /// Pong response to ping
    Pong {
        /// Original ping timestamp
        ping_timestamp: u64,
        /// Pong timestamp
        pong_timestamp: u64,
    },
}

/// Transport integration for clipboard operations
pub struct ClipboardTransportIntegration {
    /// Transport system for peer communication
    transport: Arc<KizunaTransport>,
    /// Active connections by peer ID
    connections: Arc<RwLock<HashMap<PeerId, ConnectionHandle>>>,
    /// Pending acknowledgments by sequence number
    pending_acks: Arc<RwLock<HashMap<u64, tokio::sync::oneshot::Sender<bool>>>>,
    /// Next sequence number for outgoing messages
    next_sequence: Arc<RwLock<u64>>,
    /// Message size limit for optimization (default 64KB)
    max_message_size: usize,
}

impl ClipboardTransportIntegration {
    /// Create new transport integration with provided transport system
    pub fn new(transport: Arc<KizunaTransport>) -> Self {
        Self {
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
            pending_acks: Arc::new(RwLock::new(HashMap::new())),
            next_sequence: Arc::new(RwLock::new(0)),
            max_message_size: 65536, // 64KB default
        }
    }
    
    /// Create with custom message size limit
    pub fn with_max_message_size(transport: Arc<KizunaTransport>, max_size: usize) -> Self {
        Self {
            transport,
            connections: Arc::new(RwLock::new(HashMap::new())),
            pending_acks: Arc::new(RwLock::new(HashMap::new())),
            next_sequence: Arc::new(RwLock::new(0)),
            max_message_size: max_size,
        }
    }
    
    /// Get or establish connection to a peer
    pub async fn get_or_connect(&self, peer_id: &PeerId, peer_address: &PeerAddress) -> ClipboardResult<ConnectionHandle> {
        // Check if we already have an active connection
        {
            let connections = self.connections.read().await;
            if let Some(handle) = connections.get(peer_id) {
                if handle.is_connected().await {
                    return Ok(handle.clone());
                }
            }
        }
        
        // Establish new connection
        let handle = self.transport
            .connect_to_peer(peer_address)
            .await
            .map_err(|e| ClipboardError::sync("connect_to_peer", format!("Failed to connect: {}", e)))?;
        
        // Store connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(peer_id.clone(), handle.clone());
        }
        
        Ok(handle)
    }
    
    /// Send encrypted clipboard content to a peer
    pub async fn send_content(
        &self,
        peer_id: &PeerId,
        peer_address: &PeerAddress,
        encrypted_content: Vec<u8>,
    ) -> ClipboardResult<()> {
        // Check content size
        if encrypted_content.len() > self.max_message_size {
            return Err(ClipboardError::sync(
                "send_content",
                format!(
                    "Content size {} exceeds maximum message size {}",
                    encrypted_content.len(),
                    self.max_message_size
                ),
            ));
        }
        
        // Get connection
        let handle = self.get_or_connect(peer_id, peer_address).await?;
        
        // Get next sequence number
        let sequence = {
            let mut seq = self.next_sequence.write().await;
            let current = *seq;
            *seq += 1;
            current
        };
        
        // Create sync message
        let message = ClipboardMessage::SyncContent {
            content: encrypted_content,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            sequence,
        };
        
        // Serialize message
        let message_bytes = serde_json::to_vec(&message)
            .map_err(|e| ClipboardError::serialization("clipboard_message", e))?;
        
        // Create channel for acknowledgment
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending_acks.write().await;
            pending.insert(sequence, tx);
        }
        
        // Send message
        handle
            .write(&message_bytes)
            .await
            .map_err(|e| ClipboardError::sync("send_content", format!("Failed to send: {}", e)))?;
        
        handle
            .flush()
            .await
            .map_err(|e| ClipboardError::sync("send_content", format!("Failed to flush: {}", e)))?;
        
        // Wait for acknowledgment with timeout
        match tokio::time::timeout(std::time::Duration::from_secs(10), rx).await {
            Ok(Ok(success)) => {
                if success {
                    Ok(())
                } else {
                    Err(ClipboardError::sync("send_content", "Peer reported sync failure"))
                }
            }
            Ok(Err(_)) => Err(ClipboardError::sync("send_content", "Acknowledgment channel closed")),
            Err(_) => Err(ClipboardError::sync("send_content", "Acknowledgment timeout")),
        }
    }
    
    /// Receive and process clipboard messages from peers
    pub async fn receive_message(&self, peer_id: &PeerId) -> ClipboardResult<Option<ClipboardMessage>> {
        // Get connection
        let connections = self.connections.read().await;
        let handle = connections
            .get(peer_id)
            .ok_or_else(|| ClipboardError::sync("receive_message", format!("No connection to peer {}", peer_id)))?;
        
        // Read message
        let mut buffer = vec![0u8; self.max_message_size];
        let bytes_read = handle
            .read(&mut buffer)
            .await
            .map_err(|e| ClipboardError::sync("receive_message", format!("Failed to read: {}", e)))?;
        
        if bytes_read == 0 {
            return Ok(None);
        }
        
        // Deserialize message
        let message: ClipboardMessage = serde_json::from_slice(&buffer[..bytes_read])
            .map_err(|e| ClipboardError::serialization("clipboard_message", e))?;
        
        Ok(Some(message))
    }
    
    /// Send acknowledgment for received content
    pub async fn send_ack(
        &self,
        peer_id: &PeerId,
        sequence: u64,
        success: bool,
        error: Option<String>,
    ) -> ClipboardResult<()> {
        // Get connection
        let connections = self.connections.read().await;
        let handle = connections
            .get(peer_id)
            .ok_or_else(|| ClipboardError::sync("send_ack", format!("No connection to peer {}", peer_id)))?;
        
        // Create ack message
        let message = ClipboardMessage::SyncAck {
            sequence,
            success,
            error,
        };
        
        // Serialize and send
        let message_bytes = serde_json::to_vec(&message)
            .map_err(|e| ClipboardError::serialization("clipboard_message", e))?;
        
        handle
            .write(&message_bytes)
            .await
            .map_err(|e| ClipboardError::sync("send_ack", format!("Failed to send: {}", e)))?;
        
        handle
            .flush()
            .await
            .map_err(|e| ClipboardError::sync("send_ack", format!("Failed to flush: {}", e)))?;
        
        Ok(())
    }
    
    /// Process received acknowledgment
    pub async fn process_ack(&self, sequence: u64, success: bool) -> ClipboardResult<()> {
        let mut pending = self.pending_acks.write().await;
        if let Some(tx) = pending.remove(&sequence) {
            let _ = tx.send(success);
        }
        Ok(())
    }
    
    /// Request clipboard content from a peer
    pub async fn request_content(&self, peer_id: &PeerId) -> ClipboardResult<Option<Vec<u8>>> {
        // Get connection
        let connections = self.connections.read().await;
        let handle = connections
            .get(peer_id)
            .ok_or_else(|| ClipboardError::sync("request_content", format!("No connection to peer {}", peer_id)))?;
        
        // Generate request ID
        let request_id = uuid::Uuid::new_v4().to_string();
        
        // Create request message
        let message = ClipboardMessage::ContentRequest {
            request_id: request_id.clone(),
        };
        
        // Serialize and send
        let message_bytes = serde_json::to_vec(&message)
            .map_err(|e| ClipboardError::serialization("clipboard_message", e))?;
        
        handle
            .write(&message_bytes)
            .await
            .map_err(|e| ClipboardError::sync("request_content", format!("Failed to send: {}", e)))?;
        
        handle
            .flush()
            .await
            .map_err(|e| ClipboardError::sync("request_content", format!("Failed to flush: {}", e)))?;
        
        // Wait for response with timeout
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            self.wait_for_content_response(peer_id, &request_id),
        );
        
        match timeout.await {
            Ok(result) => result,
            Err(_) => Err(ClipboardError::sync("request_content", "Response timeout")),
        }
    }
    
    /// Wait for content response (helper method)
    async fn wait_for_content_response(
        &self,
        peer_id: &PeerId,
        request_id: &str,
    ) -> ClipboardResult<Option<Vec<u8>>> {
        // In a real implementation, this would use a proper event system
        // For now, we'll poll for messages
        loop {
            if let Some(message) = self.receive_message(peer_id).await? {
                if let ClipboardMessage::ContentResponse {
                    request_id: resp_id,
                    content,
                } = message
                {
                    if resp_id == request_id {
                        return Ok(content);
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    
    /// Send content response to a request
    pub async fn send_content_response(
        &self,
        peer_id: &PeerId,
        request_id: String,
        content: Option<Vec<u8>>,
    ) -> ClipboardResult<()> {
        // Get connection
        let connections = self.connections.read().await;
        let handle = connections
            .get(peer_id)
            .ok_or_else(|| ClipboardError::sync("send_content_response", format!("No connection to peer {}", peer_id)))?;
        
        // Create response message
        let message = ClipboardMessage::ContentResponse {
            request_id,
            content,
        };
        
        // Serialize and send
        let message_bytes = serde_json::to_vec(&message)
            .map_err(|e| ClipboardError::serialization("clipboard_message", e))?;
        
        handle
            .write(&message_bytes)
            .await
            .map_err(|e| ClipboardError::sync("send_content_response", format!("Failed to send: {}", e)))?;
        
        handle
            .flush()
            .await
            .map_err(|e| ClipboardError::sync("send_content_response", format!("Failed to flush: {}", e)))?;
        
        Ok(())
    }
    
    /// Send ping to peer for connection keep-alive
    pub async fn send_ping(&self, peer_id: &PeerId) -> ClipboardResult<u64> {
        // Get connection
        let connections = self.connections.read().await;
        let handle = connections
            .get(peer_id)
            .ok_or_else(|| ClipboardError::sync("send_ping", format!("No connection to peer {}", peer_id)))?;
        
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Create ping message
        let message = ClipboardMessage::Ping { timestamp };
        
        // Serialize and send
        let message_bytes = serde_json::to_vec(&message)
            .map_err(|e| ClipboardError::serialization("clipboard_message", e))?;
        
        handle
            .write(&message_bytes)
            .await
            .map_err(|e| ClipboardError::sync("send_ping", format!("Failed to send: {}", e)))?;
        
        handle
            .flush()
            .await
            .map_err(|e| ClipboardError::sync("send_ping", format!("Failed to flush: {}", e)))?;
        
        Ok(timestamp)
    }
    
    /// Send pong response to ping
    pub async fn send_pong(&self, peer_id: &PeerId, ping_timestamp: u64) -> ClipboardResult<()> {
        // Get connection
        let connections = self.connections.read().await;
        let handle = connections
            .get(peer_id)
            .ok_or_else(|| ClipboardError::sync("send_pong", format!("No connection to peer {}", peer_id)))?;
        
        let pong_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Create pong message
        let message = ClipboardMessage::Pong {
            ping_timestamp,
            pong_timestamp,
        };
        
        // Serialize and send
        let message_bytes = serde_json::to_vec(&message)
            .map_err(|e| ClipboardError::serialization("clipboard_message", e))?;
        
        handle
            .write(&message_bytes)
            .await
            .map_err(|e| ClipboardError::sync("send_pong", format!("Failed to send: {}", e)))?;
        
        handle
            .flush()
            .await
            .map_err(|e| ClipboardError::sync("send_pong", format!("Failed to flush: {}", e)))?;
        
        Ok(())
    }
    
    /// Disconnect from a peer
    pub async fn disconnect(&self, peer_id: &PeerId) -> ClipboardResult<()> {
        let mut connections = self.connections.write().await;
        if let Some(handle) = connections.remove(peer_id) {
            handle
                .close()
                .await
                .map_err(|e| ClipboardError::sync("disconnect", format!("Failed to close connection: {}", e)))?;
        }
        Ok(())
    }
    
    /// Disconnect from all peers
    pub async fn disconnect_all(&self) -> ClipboardResult<()> {
        let mut connections = self.connections.write().await;
        for (_, handle) in connections.drain() {
            let _ = handle.close().await;
        }
        Ok(())
    }
    
    /// Get list of connected peers
    pub async fn get_connected_peers(&self) -> Vec<PeerId> {
        let connections = self.connections.read().await;
        connections.keys().cloned().collect()
    }
    
    /// Check if connected to a peer
    pub async fn is_connected(&self, peer_id: &PeerId) -> bool {
        let connections = self.connections.read().await;
        if let Some(handle) = connections.get(peer_id) {
            handle.is_connected().await
        } else {
            false
        }
    }
    
    /// Get connection count
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }
    
    /// Get maximum message size
    pub fn max_message_size(&self) -> usize {
        self.max_message_size
    }
    
    /// Set maximum message size
    pub fn set_max_message_size(&mut self, size: usize) {
        self.max_message_size = size;
    }
}

/// Trait for transport-based clipboard operations
#[async_trait]
pub trait ClipboardTransport: Send + Sync {
    /// Send encrypted content to a peer
    async fn send_to_peer(
        &self,
        peer_id: &PeerId,
        peer_address: &PeerAddress,
        encrypted_content: Vec<u8>,
    ) -> ClipboardResult<()>;
    
    /// Receive message from a peer
    async fn receive_from_peer(&self, peer_id: &PeerId) -> ClipboardResult<Option<ClipboardMessage>>;
    
    /// Check if connected to peer
    async fn is_peer_connected(&self, peer_id: &PeerId) -> bool;
}

#[async_trait]
impl ClipboardTransport for ClipboardTransportIntegration {
    async fn send_to_peer(
        &self,
        peer_id: &PeerId,
        peer_address: &PeerAddress,
        encrypted_content: Vec<u8>,
    ) -> ClipboardResult<()> {
        self.send_content(peer_id, peer_address, encrypted_content).await
    }
    
    async fn receive_from_peer(&self, peer_id: &PeerId) -> ClipboardResult<Option<ClipboardMessage>> {
        self.receive_message(peer_id).await
    }
    
    async fn is_peer_connected(&self, peer_id: &PeerId) -> bool {
        self.is_connected(peer_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    
    #[tokio::test]
    async fn test_transport_integration_creation() {
        let transport = Arc::new(KizunaTransport::new().await.unwrap());
        let integration = ClipboardTransportIntegration::new(transport);
        
        assert_eq!(integration.connection_count().await, 0);
        assert_eq!(integration.max_message_size(), 65536);
    }
    
    #[tokio::test]
    async fn test_message_serialization() {
        let message = ClipboardMessage::SyncContent {
            content: vec![1, 2, 3, 4],
            timestamp: 12345,
            sequence: 1,
        };
        
        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: ClipboardMessage = serde_json::from_slice(&serialized).unwrap();
        
        match deserialized {
            ClipboardMessage::SyncContent { content, timestamp, sequence } => {
                assert_eq!(content, vec![1, 2, 3, 4]);
                assert_eq!(timestamp, 12345);
                assert_eq!(sequence, 1);
            }
            _ => panic!("Wrong message type"),
        }
    }
    
    #[tokio::test]
    async fn test_ack_message() {
        let message = ClipboardMessage::SyncAck {
            sequence: 42,
            success: true,
            error: None,
        };
        
        let serialized = serde_json::to_vec(&message).unwrap();
        let deserialized: ClipboardMessage = serde_json::from_slice(&serialized).unwrap();
        
        match deserialized {
            ClipboardMessage::SyncAck { sequence, success, error } => {
                assert_eq!(sequence, 42);
                assert!(success);
                assert!(error.is_none());
            }
            _ => panic!("Wrong message type"),
        }
    }
    
    #[tokio::test]
    async fn test_ping_pong_messages() {
        let ping = ClipboardMessage::Ping { timestamp: 12345 };
        let pong = ClipboardMessage::Pong {
            ping_timestamp: 12345,
            pong_timestamp: 12350,
        };
        
        let ping_serialized = serde_json::to_vec(&ping).unwrap();
        let pong_serialized = serde_json::to_vec(&pong).unwrap();
        
        let ping_deserialized: ClipboardMessage = serde_json::from_slice(&ping_serialized).unwrap();
        let pong_deserialized: ClipboardMessage = serde_json::from_slice(&pong_serialized).unwrap();
        
        match ping_deserialized {
            ClipboardMessage::Ping { timestamp } => assert_eq!(timestamp, 12345),
            _ => panic!("Wrong message type"),
        }
        
        match pong_deserialized {
            ClipboardMessage::Pong { ping_timestamp, pong_timestamp } => {
                assert_eq!(ping_timestamp, 12345);
                assert_eq!(pong_timestamp, 12350);
            }
            _ => panic!("Wrong message type"),
        }
    }
    
    #[tokio::test]
    async fn test_custom_message_size() {
        let transport = Arc::new(KizunaTransport::new().await.unwrap());
        let integration = ClipboardTransportIntegration::with_max_message_size(transport, 32768);
        
        assert_eq!(integration.max_message_size(), 32768);
    }
    
    #[tokio::test]
    async fn test_connected_peers_list() {
        let transport = Arc::new(KizunaTransport::new().await.unwrap());
        let integration = ClipboardTransportIntegration::new(transport);
        
        let peers = integration.get_connected_peers().await;
        assert_eq!(peers.len(), 0);
    }
}
