//! WebSocket Fallback Manager
//! 
//! Provides WebSocket-based communication for browsers that don't support WebRTC
//! or when WebRTC connections fail.

use crate::browser_support::{BrowserResult, BrowserSupportError, BrowserConnectionInfo, BrowserSession, BrowserMessage};
use crate::browser_support::types::*;
use crate::browser_support::webrtc::ConnectionStats;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use uuid::Uuid;
use async_trait::async_trait;

/// WebSocket fallback manager for browsers without WebRTC support
pub struct WebSocketFallbackManager {
    active_connections: Arc<RwLock<HashMap<Uuid, WebSocketSession>>>,
    message_handlers: HashMap<Uuid, mpsc::UnboundedSender<BrowserMessage>>,
}

/// WebSocket session information
#[derive(Debug, Clone)]
pub struct WebSocketSession {
    pub session_id: Uuid,
    pub browser_info: BrowserInfo,
    pub websocket_connection: WebSocketConnection,
    pub permissions: BrowserPermissions,
    pub created_at: std::time::SystemTime,
    pub last_activity: std::time::SystemTime,
    pub message_sender: mpsc::UnboundedSender<Message>,
}

impl WebSocketFallbackManager {
    /// Create a new WebSocket fallback manager
    pub fn new() -> Self {
        Self {
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            message_handlers: HashMap::new(),
        }
    }
    
    /// Initialize the WebSocket fallback manager
    pub async fn initialize(&mut self) -> BrowserResult<()> {
        // Initialize any required resources
        Ok(())
    }
    
    /// Establish a WebSocket connection with a browser client
    pub async fn establish_connection(&mut self, connection_info: BrowserConnectionInfo) -> BrowserResult<BrowserSession> {
        let session_id = Uuid::new_v4();
        let connection_id = Uuid::new_v4();
        
        // Create message channel for this session
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        
        // Create WebSocket connection info
        let websocket_connection = WebSocketConnection {
            connection_id,
            peer_id: connection_info.peer_id.clone(),
            connection_state: ConnectionState::New,
            bytes_sent: 0,
            bytes_received: 0,
            created_at: std::time::SystemTime::now(),
        };
        
        // Create WebSocket session
        let websocket_session = WebSocketSession {
            session_id,
            browser_info: connection_info.browser_info.clone(),
            websocket_connection: websocket_connection.clone(),
            permissions: BrowserPermissions::default(),
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
            message_sender: message_tx.clone(),
        };
        
        // Store the session
        {
            let mut connections = self.active_connections.write().await;
            connections.insert(session_id, websocket_session.clone());
        }
        
        self.message_handlers.insert(session_id, message_tx);
        
        // Create browser session compatible with WebRTC interface
        let browser_session = BrowserSession {
            session_id,
            browser_info: connection_info.browser_info,
            webrtc_connection: WebRTCConnection {
                connection_id,
                peer_id: connection_info.peer_id,
                data_channels: HashMap::new(), // WebSocket doesn't use data channels
                connection_state: ConnectionState::New,
                ice_connection_state: IceConnectionState::New,
            },
            permissions: BrowserPermissions::default(),
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
        };
        
        Ok(browser_session)
    }
    
    /// Handle incoming WebSocket connection
    pub async fn handle_websocket_connection<S>(&self, session_id: Uuid, ws_stream: WebSocketStream<S>) -> BrowserResult<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let connections = self.active_connections.clone();
        
        // Update connection state
        {
            let mut connections_guard = connections.write().await;
            if let Some(session) = connections_guard.get_mut(&session_id) {
                session.websocket_connection.connection_state = ConnectionState::Connected;
                session.last_activity = std::time::SystemTime::now();
            }
        }
        
        // Split WebSocket stream
        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        
        // Get message receiver for this session
        let message_rx = {
            let connections_guard = connections.read().await;
            if let Some(session) = connections_guard.get(&session_id) {
                let (tx, rx) = mpsc::unbounded_channel();
                // Store the sender for outgoing messages
                drop(connections_guard);
                let mut connections_guard = connections.write().await;
                if let Some(session) = connections_guard.get_mut(&session_id) {
                    session.message_sender = tx;
                }
                rx
            } else {
                return Err(BrowserSupportError::SessionError {
                    session_id: session_id.to_string(),
                    error: "Session not found".to_string(),
                });
            }
        };
        
        // Spawn task to handle outgoing messages
        let connections_clone = connections.clone();
        let session_id_clone = session_id;
        tokio::spawn(async move {
            let mut message_rx = message_rx;
            while let Some(message) = message_rx.recv().await {
                if let Err(e) = ws_sender.send(message).await {
                    eprintln!("Failed to send WebSocket message: {}", e);
                    break;
                }
                
                // Update bytes sent
                {
                    let mut connections_guard = connections_clone.write().await;
                    if let Some(session) = connections_guard.get_mut(&session_id_clone) {
                        session.websocket_connection.bytes_sent += 1; // Approximate
                        session.last_activity = std::time::SystemTime::now();
                    }
                }
            }
        });
        
        // Handle incoming messages
        while let Some(message) = ws_receiver.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    // Parse and handle browser message
                    if let Ok(browser_message) = serde_json::from_str::<BrowserMessage>(&text) {
                        self.handle_browser_message(session_id, browser_message).await?;
                    }
                    
                    // Update bytes received
                    {
                        let mut connections_guard = connections.write().await;
                        if let Some(session) = connections_guard.get_mut(&session_id) {
                            session.websocket_connection.bytes_received += text.len() as u64;
                            session.last_activity = std::time::SystemTime::now();
                        }
                    }
                }
                Ok(Message::Binary(data)) => {
                    // Handle binary data (file transfers, etc.)
                    self.handle_binary_data(session_id, data).await?;
                    
                    // Update bytes received
                    {
                        let mut connections_guard = connections.write().await;
                        if let Some(session) = connections_guard.get_mut(&session_id) {
                            session.websocket_connection.bytes_received += data.len() as u64;
                            session.last_activity = std::time::SystemTime::now();
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    // Handle connection close
                    self.handle_connection_close(session_id).await?;
                    break;
                }
                Ok(Message::Ping(data)) => {
                    // Respond to ping with pong
                    let connections_guard = connections.read().await;
                    if let Some(session) = connections_guard.get(&session_id) {
                        let _ = session.message_sender.send(Message::Pong(data));
                    }
                }
                Ok(Message::Pong(_)) => {
                    // Handle pong response
                    continue;
                }
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
            }
        }
        
        // Clean up connection
        self.handle_connection_close(session_id).await?;
        Ok(())
    }
    
    /// Handle browser message received via WebSocket
    async fn handle_browser_message(&self, session_id: Uuid, message: BrowserMessage) -> BrowserResult<()> {
        match message.message_type {
            BrowserMessageType::FileTransferRequest => {
                // Handle file transfer request
                self.handle_file_transfer_request(session_id, message).await
            }
            BrowserMessageType::ClipboardSync => {
                // Handle clipboard synchronization
                self.handle_clipboard_sync(session_id, message).await
            }
            BrowserMessageType::CommandExecution => {
                // Handle command execution request
                self.handle_command_execution(session_id, message).await
            }
            BrowserMessageType::PeerDiscovery => {
                // Handle peer discovery request
                self.handle_peer_discovery(session_id, message).await
            }
            BrowserMessageType::WebSocketHandshake => {
                // Handle WebSocket handshake
                self.handle_websocket_handshake(session_id, message).await
            }
            BrowserMessageType::ProtocolNegotiation => {
                // Handle protocol negotiation
                self.handle_protocol_negotiation(session_id, message).await
            }
            _ => {
                // Handle other message types
                Ok(())
            }
        }
    }
    
    /// Handle binary data received via WebSocket
    async fn handle_binary_data(&self, session_id: Uuid, data: Vec<u8>) -> BrowserResult<()> {
        // Handle binary data (typically file chunks)
        // This would integrate with the file transfer system
        println!("Received {} bytes of binary data for session {}", data.len(), session_id);
        Ok(())
    }
    
    /// Handle WebSocket connection close
    async fn handle_connection_close(&self, session_id: Uuid) -> BrowserResult<()> {
        let mut connections = self.active_connections.write().await;
        if let Some(mut session) = connections.remove(&session_id) {
            session.websocket_connection.connection_state = ConnectionState::Closed;
        }
        Ok(())
    }
    
    /// Handle file transfer request
    async fn handle_file_transfer_request(&self, _session_id: Uuid, _message: BrowserMessage) -> BrowserResult<()> {
        // TODO: Integrate with file transfer system
        Ok(())
    }
    
    /// Handle clipboard synchronization
    async fn handle_clipboard_sync(&self, _session_id: Uuid, _message: BrowserMessage) -> BrowserResult<()> {
        // TODO: Integrate with clipboard system
        Ok(())
    }
    
    /// Handle command execution
    async fn handle_command_execution(&self, _session_id: Uuid, _message: BrowserMessage) -> BrowserResult<()> {
        // TODO: Integrate with command execution system
        Ok(())
    }
    
    /// Handle peer discovery
    async fn handle_peer_discovery(&self, session_id: Uuid, _message: BrowserMessage) -> BrowserResult<()> {
        // Send peer information back to browser
        let response = BrowserMessage {
            message_id: Uuid::new_v4(),
            message_type: BrowserMessageType::PeerDiscovery,
            payload: serde_json::json!({
                "peers": [],
                "protocol": "websocket"
            }),
            timestamp: std::time::SystemTime::now(),
            session_id,
        };
        
        self.send_message(session_id, response).await
    }
    
    /// Handle WebSocket handshake
    async fn handle_websocket_handshake(&self, session_id: Uuid, _message: BrowserMessage) -> BrowserResult<()> {
        // Send handshake response
        let response = BrowserMessage {
            message_id: Uuid::new_v4(),
            message_type: BrowserMessageType::WebSocketHandshake,
            payload: serde_json::json!({
                "status": "connected",
                "protocol": "websocket",
                "capabilities": {
                    "file_transfer": true,
                    "clipboard_sync": true,
                    "command_execution": true,
                    "video_streaming": false // WebSocket doesn't support video streaming
                }
            }),
            timestamp: std::time::SystemTime::now(),
            session_id,
        };
        
        self.send_message(session_id, response).await
    }
    
    /// Handle protocol negotiation
    async fn handle_protocol_negotiation(&self, session_id: Uuid, message: BrowserMessage) -> BrowserResult<()> {
        // Extract requested capabilities from message
        let requested_capabilities = message.payload.get("capabilities")
            .and_then(|c| serde_json::from_value::<ProtocolCapabilities>(c.clone()).ok())
            .unwrap_or_else(|| ProtocolCapabilities {
                supports_webrtc: false,
                supports_websocket: true,
                supports_file_transfer: true,
                supports_clipboard: true,
                supports_video_streaming: false,
                supports_command_execution: true,
            });
        
        // Send negotiation response
        let response = BrowserMessage {
            message_id: Uuid::new_v4(),
            message_type: BrowserMessageType::ProtocolNegotiation,
            payload: serde_json::json!({
                "protocol": "websocket",
                "capabilities": requested_capabilities
            }),
            timestamp: std::time::SystemTime::now(),
            session_id,
        };
        
        self.send_message(session_id, response).await
    }
    
    /// Send message to browser via WebSocket
    pub async fn send_message(&self, session_id: Uuid, message: BrowserMessage) -> BrowserResult<()> {
        let connections = self.active_connections.read().await;
        if let Some(session) = connections.get(&session_id) {
            let message_json = serde_json::to_string(&message)
                .map_err(|e| BrowserSupportError::APIError {
                    endpoint: "websocket".to_string(),
                    error: format!("Failed to serialize message: {}", e),
                })?;
            
            session.message_sender.send(Message::Text(message_json))
                .map_err(|e| BrowserSupportError::NetworkError {
                    details: format!("Failed to send WebSocket message: {}", e),
                })?;
            
            Ok(())
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    /// Receive message from browser via WebSocket
    pub async fn receive_message(&self, _session_id: Uuid) -> BrowserResult<Option<BrowserMessage>> {
        // WebSocket messages are handled asynchronously in handle_websocket_connection
        // This method is for compatibility with the unified interface
        Ok(None)
    }
    
    /// Check if WebSocket connection is active
    pub async fn is_connected(&self, session_id: Uuid) -> BrowserResult<bool> {
        let connections = self.active_connections.read().await;
        if let Some(session) = connections.get(&session_id) {
            Ok(matches!(session.websocket_connection.connection_state, ConnectionState::Connected))
        } else {
            Ok(false)
        }
    }
    
    /// Close WebSocket connection
    pub async fn close_connection(&self, session_id: Uuid) -> BrowserResult<()> {
        let connections = self.active_connections.read().await;
        if let Some(session) = connections.get(&session_id) {
            // Send close message
            let _ = session.message_sender.send(Message::Close(None));
        }
        drop(connections);
        
        // Remove from active connections
        let mut connections = self.active_connections.write().await;
        connections.remove(&session_id);
        Ok(())
    }
    
    /// Get connection statistics for WebSocket
    pub async fn get_connection_stats(&self, session_id: Uuid) -> BrowserResult<ConnectionStats> {
        let connections = self.active_connections.read().await;
        if let Some(session) = connections.get(&session_id) {
            Ok(ConnectionStats {
                bytes_sent: session.websocket_connection.bytes_sent,
                bytes_received: session.websocket_connection.bytes_received,
                packets_sent: 0, // WebSocket doesn't track packets
                packets_received: 0,
                round_trip_time: None, // Not available for WebSocket
                jitter: None,
                packet_loss_rate: None,
            })
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    /// Shutdown the WebSocket fallback manager
    pub async fn shutdown(&mut self) -> BrowserResult<()> {
        // Close all active connections
        let mut connections = self.active_connections.write().await;
        for (session_id, session) in connections.drain() {
            let _ = session.message_sender.send(Message::Close(None));
        }
        
        // Clear message handlers
        self.message_handlers.clear();
        Ok(())
    }
}