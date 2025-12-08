//! Unified Communication Interface
//! 
//! Provides a unified interface for both WebRTC and WebSocket communication,
//! with automatic fallback detection and protocol switching.

use crate::browser_support::{BrowserResult, BrowserSupportError, BrowserMessage, BrowserSession};
use crate::browser_support::types::*;
use crate::browser_support::webrtc::WebRTCManager;
use crate::browser_support::websocket_fallback::WebSocketFallbackManager;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;
use async_trait::async_trait;

/// Unified communication interface trait
#[async_trait]
pub trait CommunicationInterface {
    /// Send a message through the appropriate protocol
    async fn send_message(&self, session_id: Uuid, message: BrowserMessage) -> BrowserResult<()>;
    
    /// Receive a message from the connection
    async fn receive_message(&self, session_id: Uuid) -> BrowserResult<Option<BrowserMessage>>;
    
    /// Check if the connection is active
    async fn is_connected(&self, session_id: Uuid) -> BrowserResult<bool>;
    
    /// Close the connection
    async fn close_connection(&self, session_id: Uuid) -> BrowserResult<()>;
    
    /// Get connection statistics
    async fn get_connection_stats(&self, session_id: Uuid) -> BrowserResult<ConnectionStats>;
}

/// Unified communication manager that handles both WebRTC and WebSocket protocols
pub struct UnifiedCommunicationManager {
    webrtc_manager: Arc<tokio::sync::RwLock<WebRTCManager>>,
    websocket_manager: Arc<tokio::sync::RwLock<WebSocketFallbackManager>>,
    active_connections: Arc<tokio::sync::RwLock<HashMap<Uuid, UnifiedConnection>>>,
    protocol_detector: ProtocolDetector,
    fallback_enabled: bool,
}

impl UnifiedCommunicationManager {
    /// Create a new unified communication manager
    pub fn new() -> Self {
        Self {
            webrtc_manager: Arc::new(tokio::sync::RwLock::new(WebRTCManager::new())),
            websocket_manager: Arc::new(tokio::sync::RwLock::new(WebSocketFallbackManager::new())),
            active_connections: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            protocol_detector: ProtocolDetector::new(),
            fallback_enabled: true,
        }
    }
    
    /// Create a new unified communication manager with fallback configuration
    pub fn with_fallback(fallback_enabled: bool) -> Self {
        Self {
            webrtc_manager: Arc::new(tokio::sync::RwLock::new(WebRTCManager::new())),
            websocket_manager: Arc::new(tokio::sync::RwLock::new(WebSocketFallbackManager::new())),
            active_connections: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            protocol_detector: ProtocolDetector::new(),
            fallback_enabled,
        }
    }
    
    /// Initialize the communication manager
    pub async fn initialize(&mut self) -> BrowserResult<()> {
        self.webrtc_manager.write().await.initialize().await?;
        self.websocket_manager.write().await.initialize().await?;
        Ok(())
    }
    
    /// Establish connection with automatic protocol detection
    pub async fn establish_connection(&mut self, connection_info: BrowserConnectionInfo) -> BrowserResult<BrowserSession> {
        // Detect the best protocol for this browser
        let protocol = self.protocol_detector.detect_best_protocol(&connection_info.browser_info).await?;
        
        match protocol {
            CommunicationProtocol::WebRTC => {
                // Try WebRTC first, fallback to WebSocket if it fails
                match self.establish_webrtc_connection(connection_info.clone()).await {
                    Ok(session) => Ok(session),
                    Err(e) if self.fallback_enabled && self.protocol_detector.should_fallback_to_websocket(Uuid::nil(), &e).await => {
                        println!("WebRTC connection failed, falling back to WebSocket: {}", e);
                        self.establish_websocket_connection(connection_info).await
                    }
                    Err(e) => Err(e),
                }
            }
            CommunicationProtocol::WebSocket => {
                self.establish_websocket_connection(connection_info).await
            }
        }
    }
    
    /// Establish WebRTC connection
    async fn establish_webrtc_connection(&mut self, connection_info: BrowserConnectionInfo) -> BrowserResult<BrowserSession> {
        let session = self.webrtc_manager.write().await.establish_connection(connection_info.clone()).await?;
        
        // Create unified connection record
        let unified_connection = UnifiedConnection {
            connection_id: session.webrtc_connection.connection_id,
            protocol: CommunicationProtocol::WebRTC,
            session_id: session.session_id,
            capabilities: self.extract_capabilities(&connection_info.browser_info),
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
        };
        
        self.active_connections.write().await.insert(session.session_id, unified_connection);
        Ok(session)
    }
    
    /// Establish WebSocket fallback connection
    async fn establish_websocket_connection(&mut self, connection_info: BrowserConnectionInfo) -> BrowserResult<BrowserSession> {
        let session = self.websocket_manager.write().await.establish_connection(connection_info.clone()).await?;
        
        // Create unified connection record
        let unified_connection = UnifiedConnection {
            connection_id: Uuid::new_v4(), // WebSocket connections use their own ID
            protocol: CommunicationProtocol::WebSocket,
            session_id: session.session_id,
            capabilities: self.extract_capabilities(&connection_info.browser_info),
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
        };
        
        self.active_connections.write().await.insert(session.session_id, unified_connection);
        Ok(session)
    }
    
    /// Attempt fallback from WebRTC to WebSocket
    pub async fn fallback_to_websocket(&mut self, session_id: Uuid, connection_info: BrowserConnectionInfo) -> BrowserResult<()> {
        if !self.fallback_enabled {
            return Err(BrowserSupportError::ConfigurationError {
                parameter: "fallback".to_string(),
                issue: "Fallback is disabled".to_string(),
            });
        }
        
        // Close existing WebRTC connection if it exists
        {
            let connections = self.active_connections.read().await;
            if let Some(connection) = connections.get(&session_id) {
                if matches!(connection.protocol, CommunicationProtocol::WebRTC) {
                    drop(connections);
                    self.webrtc_manager.write().await.close_connection(session_id).await?;
                }
            }
        }
        
        // Establish WebSocket connection
        let session = self.websocket_manager.write().await.establish_connection(connection_info.clone()).await?;
        
        // Update unified connection record
        let unified_connection = UnifiedConnection {
            connection_id: Uuid::new_v4(),
            protocol: CommunicationProtocol::WebSocket,
            session_id,
            capabilities: self.extract_capabilities(&connection_info.browser_info),
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
        };
        
        self.active_connections.write().await.insert(session_id, unified_connection);
        
        // Notify browser about fallback activation
        let fallback_message = BrowserMessage {
            message_id: Uuid::new_v4(),
            message_type: BrowserMessageType::FallbackActivated,
            payload: serde_json::json!({
                "protocol": "websocket",
                "reason": "webrtc_unavailable"
            }),
            timestamp: std::time::SystemTime::now(),
            session_id,
        };
        
        self.send_message(session_id, fallback_message).await?;
        Ok(())
    }
    
    /// Monitor connection health and trigger fallback if needed
    pub async fn monitor_connection_health(&mut self, session_id: Uuid, connection_info: BrowserConnectionInfo) -> BrowserResult<()> {
        let should_fallback = {
            let connections = self.active_connections.read().await;
            if let Some(connection) = connections.get(&session_id) {
                // Check if using WebRTC and connection is failing
                if matches!(connection.protocol, CommunicationProtocol::WebRTC) {
                    // Check connection state
                    let is_connected = self.webrtc_manager.read().await.is_connected(session_id).await?;
                    !is_connected
                } else {
                    false
                }
            } else {
                false
            }
        };
        
        if should_fallback && self.fallback_enabled {
            println!("Connection health check failed for session {}, triggering fallback", session_id);
            self.fallback_to_websocket(session_id, connection_info).await?;
        }
        
        Ok(())
    }
    
    /// Extract protocol capabilities from browser info
    fn extract_capabilities(&self, browser_info: &BrowserInfo) -> ProtocolCapabilities {
        ProtocolCapabilities {
            supports_webrtc: browser_info.supports_webrtc,
            supports_websocket: true, // All modern browsers support WebSocket
            supports_file_transfer: true,
            supports_clipboard: browser_info.supports_clipboard_api,
            supports_video_streaming: browser_info.supports_webrtc, // Video requires WebRTC
            supports_command_execution: true,
        }
    }
    
    /// Get the protocol for a session
    pub async fn get_session_protocol(&self, session_id: Uuid) -> Option<CommunicationProtocol> {
        let connections = self.active_connections.read().await;
        connections.get(&session_id).map(|conn| conn.protocol.clone())
    }
    
    /// Check if fallback is enabled
    pub fn is_fallback_enabled(&self) -> bool {
        self.fallback_enabled
    }
    
    /// Enable or disable automatic fallback
    pub fn set_fallback_enabled(&mut self, enabled: bool) {
        self.fallback_enabled = enabled;
    }
    
    /// Shutdown the communication manager
    pub async fn shutdown(&mut self) -> BrowserResult<()> {
        // Close all active connections
        let connections: Vec<_> = {
            let mut connections = self.active_connections.write().await;
            connections.drain().collect()
        };
        
        for (session_id, connection) in connections {
            match connection.protocol {
                CommunicationProtocol::WebRTC => {
                    let _ = self.webrtc_manager.write().await.close_connection(session_id).await;
                }
                CommunicationProtocol::WebSocket => {
                    let _ = self.websocket_manager.write().await.close_connection(session_id).await;
                }
            }
        }
        
        self.webrtc_manager.write().await.shutdown().await?;
        self.websocket_manager.write().await.shutdown().await?;
        Ok(())
    }
}

#[async_trait]
impl CommunicationInterface for UnifiedCommunicationManager {
    async fn send_message(&self, session_id: Uuid, message: BrowserMessage) -> BrowserResult<()> {
        let connections = self.active_connections.read().await;
        if let Some(connection) = connections.get(&session_id) {
            let protocol = connection.protocol.clone();
            drop(connections);
            
            match protocol {
                CommunicationProtocol::WebRTC => {
                    // Route to WebRTC data channel
                    self.webrtc_manager.write().await.send_message(session_id, message).await
                }
                CommunicationProtocol::WebSocket => {
                    // Route to WebSocket connection
                    self.websocket_manager.write().await.send_message(session_id, message).await
                }
            }
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    async fn receive_message(&self, session_id: Uuid) -> BrowserResult<Option<BrowserMessage>> {
        let connections = self.active_connections.read().await;
        if let Some(connection) = connections.get(&session_id) {
            let protocol = connection.protocol.clone();
            drop(connections);
            
            match protocol {
                CommunicationProtocol::WebRTC => {
                    self.webrtc_manager.read().await.receive_message(session_id).await
                }
                CommunicationProtocol::WebSocket => {
                    self.websocket_manager.read().await.receive_message(session_id).await
                }
            }
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    async fn is_connected(&self, session_id: Uuid) -> BrowserResult<bool> {
        let connections = self.active_connections.read().await;
        if let Some(connection) = connections.get(&session_id) {
            let protocol = connection.protocol.clone();
            drop(connections);
            
            match protocol {
                CommunicationProtocol::WebRTC => {
                    self.webrtc_manager.read().await.is_connected(session_id).await
                }
                CommunicationProtocol::WebSocket => {
                    self.websocket_manager.read().await.is_connected(session_id).await
                }
            }
        } else {
            Ok(false)
        }
    }
    
    async fn close_connection(&self, session_id: Uuid) -> BrowserResult<()> {
        let connections = self.active_connections.read().await;
        if let Some(connection) = connections.get(&session_id) {
            let protocol = connection.protocol.clone();
            drop(connections);
            
            match protocol {
                CommunicationProtocol::WebRTC => {
                    self.webrtc_manager.write().await.close_connection(session_id).await
                }
                CommunicationProtocol::WebSocket => {
                    self.websocket_manager.write().await.close_connection(session_id).await
                }
            }
        } else {
            Ok(()) // Already closed
        }
    }
    
    async fn get_connection_stats(&self, session_id: Uuid) -> BrowserResult<ConnectionStats> {
        let connections = self.active_connections.read().await;
        if let Some(connection) = connections.get(&session_id) {
            let protocol = connection.protocol.clone();
            drop(connections);
            
            match protocol {
                CommunicationProtocol::WebRTC => {
                    self.webrtc_manager.read().await.get_connection_stats(session_id).await
                }
                CommunicationProtocol::WebSocket => {
                    self.websocket_manager.read().await.get_connection_stats(session_id).await
                }
            }
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
}

/// Protocol detection logic
pub struct ProtocolDetector {
    // Configuration for protocol selection
}

impl ProtocolDetector {
    pub fn new() -> Self {
        Self {}
    }
    
    /// Detect the best protocol for a browser
    pub async fn detect_best_protocol(&self, browser_info: &BrowserInfo) -> BrowserResult<CommunicationProtocol> {
        // Check WebRTC support first (preferred protocol)
        if browser_info.supports_webrtc {
            // Additional checks for WebRTC compatibility
            if self.is_webrtc_fully_supported(browser_info).await {
                return Ok(CommunicationProtocol::WebRTC);
            }
        }
        
        // Fallback to WebSocket
        Ok(CommunicationProtocol::WebSocket)
    }
    
    /// Check if WebRTC is fully supported and functional
    async fn is_webrtc_fully_supported(&self, browser_info: &BrowserInfo) -> bool {
        // Check browser-specific WebRTC limitations
        match browser_info.browser_type {
            BrowserType::Safari => {
                // Safari has some WebRTC limitations, especially on mobile
                if browser_info.platform.contains("Mobile") {
                    false // Use WebSocket fallback for mobile Safari
                } else {
                    true
                }
            }
            BrowserType::Firefox => {
                // Firefox generally has good WebRTC support
                true
            }
            BrowserType::Chrome => {
                // Chrome has the best WebRTC support
                true
            }
            BrowserType::Edge => {
                // Modern Edge (Chromium-based) has good WebRTC support
                true
            }
            BrowserType::Other(_) => {
                // For unknown browsers, be conservative and use WebSocket
                false
            }
        }
    }
    
    /// Check if fallback is needed during runtime
    pub async fn should_fallback_to_websocket(&self, session_id: Uuid, error: &BrowserSupportError) -> bool {
        match error {
            BrowserSupportError::WebRTCError { .. } => true,
            BrowserSupportError::NetworkError { details } => {
                // Check if it's a WebRTC-specific network error
                details.contains("ICE") || details.contains("DTLS") || details.contains("SCTP")
            }
            _ => false,
        }
    }
}