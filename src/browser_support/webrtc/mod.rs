//! WebRTC Connection Management
//! 
//! This module handles WebRTC peer connections, signaling, and data channel management
//! for browser clients connecting to Kizuna peers.

pub mod connection;
pub mod signaling;
pub mod data_channel;

use crate::browser_support::{BrowserResult, BrowserConnectionInfo, BrowserSession, WebRTCConnection};
use crate::browser_support::types::*;
use std::collections::HashMap;
use uuid::Uuid;

/// WebRTC manager for handling browser connections
pub struct WebRTCManager {
    active_connections: HashMap<Uuid, BrowserSession>,
    signaling_coordinator: signaling::SignalingCoordinator,
    connection_establisher: connection::ConnectionEstablisher,
}

impl WebRTCManager {
    /// Create a new WebRTC manager
    pub fn new() -> Self {
        Self {
            active_connections: HashMap::new(),
            signaling_coordinator: signaling::SignalingCoordinator::new(),
            connection_establisher: connection::ConnectionEstablisher::new(),
        }
    }
    
    /// Initialize the WebRTC manager
    pub async fn initialize(&mut self) -> BrowserResult<()> {
        self.signaling_coordinator.initialize().await?;
        self.connection_establisher.initialize().await?;
        Ok(())
    }
    
    /// Establish a WebRTC connection with a browser client
    pub async fn establish_connection(&self, connection_info: BrowserConnectionInfo) -> BrowserResult<BrowserSession> {
        // Create WebRTC peer connection
        let webrtc_connection = self.connection_establisher
            .create_peer_connection(&connection_info.signaling_info)
            .await?;
        
        // Create browser session
        let session_id = Uuid::new_v4();
        let session = BrowserSession {
            session_id,
            browser_info: connection_info.browser_info,
            webrtc_connection,
            permissions: BrowserPermissions::default(),
            created_at: std::time::SystemTime::now(),
            last_activity: std::time::SystemTime::now(),
        };
        
        Ok(session)
    }
    
    /// Create a data channel for a specific service
    pub async fn create_data_channel(
        &self, 
        session_id: Uuid, 
        channel_type: ChannelType
    ) -> BrowserResult<()> {
        // Implementation will be added in data_channel module
        Ok(())
    }
    
    /// Handle signaling message from browser
    pub async fn handle_signaling_message(&self, message: SignalingMessage) -> BrowserResult<()> {
        self.signaling_coordinator.handle_message(message).await
    }
    
    /// Get connection statistics
    pub async fn get_connection_stats(&self, session_id: Uuid) -> BrowserResult<ConnectionStats> {
        // Implementation will be added
        todo!("Implement connection statistics")
    }
    
    /// Close a browser connection
    pub async fn close_connection(&mut self, session_id: Uuid) -> BrowserResult<()> {
        if let Some(session) = self.active_connections.remove(&session_id) {
            // Clean up WebRTC connection
            // Implementation will be added
        }
        Ok(())
    }
    
    /// Shutdown the WebRTC manager
    pub async fn shutdown(&mut self) -> BrowserResult<()> {
        // Close all active connections
        for (session_id, _) in self.active_connections.drain() {
            // Clean up connections
        }
        
        self.connection_establisher.shutdown().await?;
        self.signaling_coordinator.shutdown().await?;
        Ok(())
    }
}

/// Signaling message for WebRTC negotiation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SignalingMessage {
    pub message_type: SignalingMessageType,
    pub session_id: Uuid,
    pub payload: serde_json::Value,
}

/// Signaling message types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SignalingMessageType {
    Offer,
    Answer,
    IceCandidate,
    Close,
}

/// Connection statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectionStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub round_trip_time: Option<f64>,
    pub jitter: Option<f64>,
    pub packet_loss_rate: Option<f64>,
}