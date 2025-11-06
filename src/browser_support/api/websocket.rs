//! WebSocket Handler
//! 
//! WebSocket connection handling for real-time browser communication and fallback support.

use crate::browser_support::{BrowserResult, BrowserSupportError, BrowserConnectionInfo};
use crate::browser_support::websocket_fallback::WebSocketFallbackManager;
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// WebSocket connection handler with fallback support
pub struct WebSocketHandler {
    fallback_manager: Arc<RwLock<WebSocketFallbackManager>>,
}

impl WebSocketHandler {
    /// Create new WebSocket handler
    pub fn new() -> Self {
        Self {
            fallback_manager: Arc::new(RwLock::new(WebSocketFallbackManager::new())),
        }
    }
    
    /// Initialize the WebSocket handler
    pub async fn initialize(&mut self) -> BrowserResult<()> {
        let mut manager = self.fallback_manager.write().await;
        manager.initialize().await
    }
    
    /// Handle WebSocket connection with fallback support
    pub async fn handle_connection<S>(&self, ws_stream: WebSocketStream<S>, connection_info: BrowserConnectionInfo) -> BrowserResult<Uuid>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        // Establish WebSocket fallback connection
        let session = {
            let mut manager = self.fallback_manager.write().await;
            manager.establish_connection(connection_info).await?
        };
        
        let session_id = session.session_id;
        
        // Handle the WebSocket connection
        {
            let manager = self.fallback_manager.read().await;
            manager.handle_websocket_connection(session_id, ws_stream).await?;
        }
        
        Ok(session_id)
    }
    
    /// Handle WebSocket upgrade for fallback
    pub async fn handle_fallback_upgrade<S>(&self, ws_stream: WebSocketStream<S>, session_id: Uuid) -> BrowserResult<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let manager = self.fallback_manager.read().await;
        manager.handle_websocket_connection(session_id, ws_stream).await
    }
    
    /// Send message to WebSocket client
    pub async fn send_message(&self, session_id: Uuid, message: crate::browser_support::BrowserMessage) -> BrowserResult<()> {
        let manager = self.fallback_manager.read().await;
        manager.send_message(session_id, message).await
    }
    
    /// Check if WebSocket connection is active
    pub async fn is_connected(&self, session_id: Uuid) -> BrowserResult<bool> {
        let manager = self.fallback_manager.read().await;
        manager.is_connected(session_id).await
    }
    
    /// Close WebSocket connection
    pub async fn close_connection(&self, session_id: Uuid) -> BrowserResult<()> {
        let manager = self.fallback_manager.read().await;
        manager.close_connection(session_id).await
    }
    
    /// Get WebSocket connection statistics
    pub async fn get_connection_stats(&self, session_id: Uuid) -> BrowserResult<crate::browser_support::webrtc::ConnectionStats> {
        let manager = self.fallback_manager.read().await;
        manager.get_connection_stats(session_id).await
    }
    
    /// Shutdown the WebSocket handler
    pub async fn shutdown(&mut self) -> BrowserResult<()> {
        let mut manager = self.fallback_manager.write().await;
        manager.shutdown().await
    }
}