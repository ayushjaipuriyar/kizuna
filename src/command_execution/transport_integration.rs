// Transport Integration for Command Execution
//
// This module provides integration between the command execution system and the transport layer,
// enabling reliable command request/response communication with automatic reconnection support.

use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc};
use serde::{Deserialize, Serialize};

use crate::command_execution::{
    CommandRequest, CommandResult, ScriptRequest, ScriptResult, Notification,
    NotificationResult, SystemInfo, SystemInfoQuery, PeerId,
};
use crate::command_execution::error::{CommandError, CommandResult as CmdResult};
use crate::command_execution::security_integration::{
    CommandSecurityIntegration, EncryptedCommandMessage, CommandMessage,
};
use crate::transport::{KizunaTransport, ConnectionHandle, PeerAddress, TransportCapabilities};

/// Transport integration for command execution
pub struct CommandTransportIntegration {
    transport: Arc<KizunaTransport>,
    security: Arc<CommandSecurityIntegration>,
    active_connections: Arc<RwLock<HashMap<PeerId, ConnectionHandle>>>,
    response_channels: Arc<RwLock<HashMap<uuid::Uuid, mpsc::UnboundedSender<CommandMessage>>>>,
}

impl CommandTransportIntegration {
    /// Create a new transport integration
    pub fn new(
        transport: Arc<KizunaTransport>,
        security: Arc<CommandSecurityIntegration>,
    ) -> Self {
        Self {
            transport,
            security,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            response_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get or establish a connection to a peer
    async fn get_or_connect(&self, peer_address: &PeerAddress) -> CmdResult<ConnectionHandle> {
        // Check if we already have an active connection
        {
            let connections = self.active_connections.read().await;
            if let Some(handle) = connections.get(&peer_address.peer_id) {
                if handle.is_connected().await {
                    return Ok(handle.clone());
                }
            }
        }

        // Establish new connection
        let handle = self.transport.connect_to_peer(peer_address)
            .await
            .map_err(|e| CommandError::TransportError(format!("Connection failed: {}", e)))?;

        // Store connection
        {
            let mut connections = self.active_connections.write().await;
            connections.insert(peer_address.peer_id.clone(), handle.clone());
        }

        Ok(handle)
    }

    /// Send an encrypted message over the transport
    async fn send_encrypted_message(
        &self,
        message: CommandMessage,
        peer_id: &PeerId,
        peer_address: &PeerAddress,
    ) -> CmdResult<()> {
        // Encrypt the message
        let encrypted = self.security.encrypt_message(message, peer_id).await?;

        // Serialize encrypted message
        let data = serde_json::to_vec(&encrypted)
            .map_err(|e| CommandError::SerializationError(e.to_string()))?;

        // Get connection
        let handle = self.get_or_connect(peer_address).await?;

        // Send data
        handle.write(&data)
            .await
            .map_err(|e| CommandError::TransportError(format!("Send failed: {}", e)))?;

        handle.flush()
            .await
            .map_err(|e| CommandError::TransportError(format!("Flush failed: {}", e)))?;

        Ok(())
    }

    /// Receive and decrypt a message from the transport
    async fn receive_encrypted_message(
        &self,
        handle: &ConnectionHandle,
    ) -> CmdResult<CommandMessage> {
        // Read data from connection
        let mut buffer = vec![0u8; 65536]; // 64KB buffer
        let bytes_read = handle.read(&mut buffer)
            .await
            .map_err(|e| CommandError::TransportError(format!("Receive failed: {}", e)))?;

        if bytes_read == 0 {
            return Err(CommandError::TransportError("Connection closed".to_string()));
        }

        // Deserialize encrypted message
        let encrypted: EncryptedCommandMessage = serde_json::from_slice(&buffer[..bytes_read])
            .map_err(|e| CommandError::SerializationError(e.to_string()))?;

        // Decrypt message
        let message = self.security.decrypt_message(encrypted).await?;

        Ok(message)
    }

    /// Send a command request and wait for result
    pub async fn send_command_request(
        &self,
        request: CommandRequest,
        peer_address: &PeerAddress,
    ) -> CmdResult<CommandResult> {
        let request_id = request.request_id;
        let peer_id = &peer_address.peer_id;

        // Create response channel
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut channels = self.response_channels.write().await;
            channels.insert(request_id, tx);
        }

        // Send request
        let message = CommandMessage::CommandRequest(request);
        self.send_encrypted_message(message, peer_id, peer_address).await?;

        // Wait for response with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(300), // 5 minute timeout
            rx.recv()
        )
        .await
        .map_err(|_| CommandError::Timeout(std::time::Duration::from_secs(300)))?
        .ok_or_else(|| CommandError::TransportError("Response channel closed".to_string()))?;

        // Clean up response channel
        {
            let mut channels = self.response_channels.write().await;
            channels.remove(&request_id);
        }

        // Extract command result
        match result {
            CommandMessage::CommandResult(cmd_result) => Ok(cmd_result),
            _ => Err(CommandError::TransportError("Unexpected response type".to_string())),
        }
    }

    /// Send a script request and wait for result
    pub async fn send_script_request(
        &self,
        request: ScriptRequest,
        peer_address: &PeerAddress,
    ) -> CmdResult<ScriptResult> {
        let request_id = request.request_id;
        let peer_id = &peer_address.peer_id;

        // Create response channel
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut channels = self.response_channels.write().await;
            channels.insert(request_id, tx);
        }

        // Send request
        let message = CommandMessage::ScriptRequest(request);
        self.send_encrypted_message(message, peer_id, peer_address).await?;

        // Wait for response with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(600), // 10 minute timeout for scripts
            rx.recv()
        )
        .await
        .map_err(|_| CommandError::Timeout(std::time::Duration::from_secs(600)))?
        .ok_or_else(|| CommandError::TransportError("Response channel closed".to_string()))?;

        // Clean up response channel
        {
            let mut channels = self.response_channels.write().await;
            channels.remove(&request_id);
        }

        // Extract script result
        match result {
            CommandMessage::ScriptResult(script_result) => Ok(script_result),
            _ => Err(CommandError::TransportError("Unexpected response type".to_string())),
        }
    }

    /// Send a system info query and wait for response
    pub async fn send_system_info_query(
        &self,
        query: SystemInfoQuery,
        peer_address: &PeerAddress,
    ) -> CmdResult<SystemInfo> {
        let query_id = query.query_id;
        let peer_id = &peer_address.peer_id;

        // Create response channel
        let (tx, mut rx) = mpsc::unbounded_channel();
        {
            let mut channels = self.response_channels.write().await;
            channels.insert(query_id, tx);
        }

        // Send query
        let message = CommandMessage::SystemInfoQuery(query);
        self.send_encrypted_message(message, peer_id, peer_address).await?;

        // Wait for response with timeout
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(30), // 30 second timeout
            rx.recv()
        )
        .await
        .map_err(|_| CommandError::Timeout(std::time::Duration::from_secs(30)))?
        .ok_or_else(|| CommandError::TransportError("Response channel closed".to_string()))?;

        // Clean up response channel
        {
            let mut channels = self.response_channels.write().await;
            channels.remove(&query_id);
        }

        // Extract system info
        match result {
            CommandMessage::SystemInfoResponse(info) => Ok(info),
            _ => Err(CommandError::TransportError("Unexpected response type".to_string())),
        }
    }

    /// Send a notification (fire and forget)
    pub async fn send_notification(
        &self,
        notification: Notification,
        peer_address: &PeerAddress,
    ) -> CmdResult<()> {
        let peer_id = &peer_address.peer_id;
        let message = CommandMessage::NotificationRequest(notification);
        self.send_encrypted_message(message, peer_id, peer_address).await
    }

    /// Handle incoming message (to be called by message receiver loop)
    pub async fn handle_incoming_message(&self, message: CommandMessage) -> CmdResult<()> {
        // Route message to appropriate response channel
        let message_id = match &message {
            CommandMessage::CommandResult(result) => Some(result.request_id),
            CommandMessage::ScriptResult(result) => Some(result.request_id),
            CommandMessage::SystemInfoResponse(_) => None, // Need to extract query_id differently
            CommandMessage::NotificationResult(result) => Some(result.notification_id),
            _ => None,
        };

        if let Some(id) = message_id {
            let channels = self.response_channels.read().await;
            if let Some(tx) = channels.get(&id) {
                let _ = tx.send(message);
            }
        }

        Ok(())
    }

    /// Close connection to a peer
    pub async fn disconnect_peer(&self, peer_id: &PeerId) -> CmdResult<()> {
        let mut connections = self.active_connections.write().await;
        if let Some(handle) = connections.remove(peer_id) {
            handle.close()
                .await
                .map_err(|e| CommandError::TransportError(format!("Disconnect failed: {}", e)))?;
        }
        Ok(())
    }

    /// Get all active peer connections
    pub async fn get_active_peers(&self) -> Vec<PeerId> {
        let connections = self.active_connections.read().await;
        connections.keys().cloned().collect()
    }

    /// Check if connected to a peer
    pub async fn is_connected(&self, peer_id: &PeerId) -> bool {
        let connections = self.active_connections.read().await;
        if let Some(handle) = connections.get(peer_id) {
            handle.is_connected().await
        } else {
            false
        }
    }
}

/// Configuration for command execution API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecutionConfig {
    /// Command execution timeout
    pub command_timeout: std::time::Duration,
    /// Script execution timeout
    pub script_timeout: std::time::Duration,
    /// System info query timeout
    pub query_timeout: std::time::Duration,
    /// Enable automatic reconnection
    pub auto_reconnect: bool,
    /// Maximum reconnection attempts
    pub max_reconnect_attempts: u32,
}

impl Default for CommandExecutionConfig {
    fn default() -> Self {
        Self {
            command_timeout: std::time::Duration::from_secs(300),
            script_timeout: std::time::Duration::from_secs(600),
            query_timeout: std::time::Duration::from_secs(30),
            auto_reconnect: true,
            max_reconnect_attempts: 3,
        }
    }
}

/// High-level command execution API
pub struct CommandExecutionApi {
    transport_integration: Arc<CommandTransportIntegration>,
    config: CommandExecutionConfig,
}

impl CommandExecutionApi {
    /// Create a new command execution API
    pub fn new(
        transport_integration: Arc<CommandTransportIntegration>,
        config: CommandExecutionConfig,
    ) -> Self {
        Self {
            transport_integration,
            config,
        }
    }

    /// Execute a command on a remote peer
    pub async fn execute_command(
        &self,
        request: CommandRequest,
        peer_address: &PeerAddress,
    ) -> CmdResult<CommandResult> {
        self.transport_integration.send_command_request(request, peer_address).await
    }

    /// Execute a script on a remote peer
    pub async fn execute_script(
        &self,
        request: ScriptRequest,
        peer_address: &PeerAddress,
    ) -> CmdResult<ScriptResult> {
        self.transport_integration.send_script_request(request, peer_address).await
    }

    /// Query system information from a remote peer
    pub async fn query_system_info(
        &self,
        query: SystemInfoQuery,
        peer_address: &PeerAddress,
    ) -> CmdResult<SystemInfo> {
        self.transport_integration.send_system_info_query(query, peer_address).await
    }

    /// Send a notification to a remote peer
    pub async fn send_notification(
        &self,
        notification: Notification,
        peer_address: &PeerAddress,
    ) -> CmdResult<()> {
        self.transport_integration.send_notification(notification, peer_address).await
    }

    /// Disconnect from a peer
    pub async fn disconnect(&self, peer_id: &PeerId) -> CmdResult<()> {
        self.transport_integration.disconnect_peer(peer_id).await
    }

    /// Get list of connected peers
    pub async fn get_connected_peers(&self) -> Vec<PeerId> {
        self.transport_integration.get_active_peers().await
    }

    /// Check if connected to a peer
    pub async fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.transport_integration.is_connected(peer_id).await
    }

    /// Get configuration
    pub fn config(&self) -> &CommandExecutionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::SecuritySystem;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[tokio::test]
    async fn test_transport_integration_creation() {
        // Create security system
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let security_integration = Arc::new(CommandSecurityIntegration::new(security_system));

        // Create transport
        let transport = Arc::new(KizunaTransport::new().await.unwrap());

        // Create transport integration
        let integration = CommandTransportIntegration::new(transport, security_integration);

        // Verify no active connections initially
        let peers = integration.get_active_peers().await;
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_command_execution_api_creation() {
        // Create security system
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let security_integration = Arc::new(CommandSecurityIntegration::new(security_system));

        // Create transport
        let transport = Arc::new(KizunaTransport::new().await.unwrap());

        // Create transport integration
        let transport_integration = Arc::new(
            CommandTransportIntegration::new(transport, security_integration)
        );

        // Create API
        let api = CommandExecutionApi::new(
            transport_integration,
            CommandExecutionConfig::default(),
        );

        // Verify configuration
        assert_eq!(api.config().command_timeout, std::time::Duration::from_secs(300));
        assert_eq!(api.config().script_timeout, std::time::Duration::from_secs(600));
        assert!(api.config().auto_reconnect);
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
    }
}
