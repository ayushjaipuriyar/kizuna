//! WebRTC Connection Management
//! 
//! This module handles WebRTC peer connections, signaling, and data channel management
//! for browser clients connecting to Kizuna peers.

pub mod connection;
pub mod signaling;
pub mod data_channel;

use crate::browser_support::{BrowserResult, BrowserSupportError, BrowserConnectionInfo, BrowserSession, WebRTCConnection};
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
    pub async fn establish_connection(&mut self, connection_info: BrowserConnectionInfo) -> BrowserResult<BrowserSession> {
        // Create WebRTC peer connection
        let (webrtc_connection, _peer_connection) = self.connection_establisher
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
        
        // Store the session
        self.active_connections.insert(session_id, session.clone());
        
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
    
    /// Handle WebRTC offer from browser
    pub async fn handle_offer(&self, session_id: Uuid, offer_sdp: String) -> BrowserResult<String> {
        if let Some(session) = self.active_connections.get(&session_id) {
            // Parse SDP offer
            let offer = webrtc::peer_connection::sdp::session_description::RTCSessionDescription {
                sdp_type: webrtc::peer_connection::sdp::sdp_type::RTCSdpType::Offer,
                sdp: offer_sdp,
            };
            
            // Handle offer and get answer
            let answer = self.connection_establisher
                .handle_offer(session.webrtc_connection.connection_id, offer)
                .await?;
            
            Ok(answer.sdp)
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    /// Handle WebRTC answer from browser
    pub async fn handle_answer(&self, session_id: Uuid, answer_sdp: String) -> BrowserResult<()> {
        if let Some(session) = self.active_connections.get(&session_id) {
            // Parse SDP answer
            let answer = webrtc::peer_connection::sdp::session_description::RTCSessionDescription {
                sdp_type: webrtc::peer_connection::sdp::sdp_type::RTCSdpType::Answer,
                sdp: answer_sdp,
            };
            
            // Handle answer
            self.connection_establisher
                .handle_answer(session.webrtc_connection.connection_id, answer)
                .await
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    /// Handle ICE candidate from browser
    pub async fn handle_ice_candidate(&self, session_id: Uuid, candidate: String, sdp_mid: Option<String>, sdp_mline_index: Option<u16>) -> BrowserResult<()> {
        if let Some(session) = self.active_connections.get(&session_id) {
            // Create ICE candidate
            let ice_candidate = webrtc::ice_transport::ice_candidate::RTCIceCandidate::new(
                candidate,
                sdp_mid.unwrap_or_default(),
                sdp_mline_index,
            );
            
            // Handle ICE candidate
            self.connection_establisher
                .handle_ice_candidate(session.webrtc_connection.connection_id, ice_candidate)
                .await
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    /// Create an offer for the browser
    pub async fn create_offer(&self, session_id: Uuid) -> BrowserResult<String> {
        if let Some(session) = self.active_connections.get(&session_id) {
            let offer = self.connection_establisher
                .create_offer(session.webrtc_connection.connection_id)
                .await?;
            
            Ok(offer.sdp)
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    /// Get connection statistics
    pub async fn get_connection_stats(&self, session_id: Uuid) -> BrowserResult<ConnectionStats> {
        if let Some(session) = self.active_connections.get(&session_id) {
            self.connection_establisher
                .get_connection_stats(session.webrtc_connection.connection_id)
                .await
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    /// Close a browser connection
    pub async fn close_connection(&mut self, session_id: Uuid) -> BrowserResult<()> {
        if let Some(session) = self.active_connections.remove(&session_id) {
            // Clean up WebRTC connection
            self.connection_establisher
                .close_connection(session.webrtc_connection.connection_id)
                .await?;
        }
        Ok(())
    }
    
    /// Send message via WebRTC data channel
    pub async fn send_message(&self, session_id: Uuid, message: crate::browser_support::BrowserMessage) -> BrowserResult<()> {
        if let Some(_session) = self.active_connections.get(&session_id) {
            // TODO: Implement message sending via WebRTC data channel
            // This would route the message to the appropriate data channel based on message type
            Ok(())
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    /// Receive message from WebRTC data channel
    pub async fn receive_message(&self, session_id: Uuid) -> BrowserResult<Option<crate::browser_support::BrowserMessage>> {
        if let Some(_session) = self.active_connections.get(&session_id) {
            // TODO: Implement message receiving from WebRTC data channel
            // This would poll the appropriate data channel for incoming messages
            Ok(None)
        } else {
            Err(BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Session not found".to_string(),
            })
        }
    }
    
    /// Check if WebRTC connection is active
    pub async fn is_connected(&self, session_id: Uuid) -> BrowserResult<bool> {
        if let Some(session) = self.active_connections.get(&session_id) {
            Ok(matches!(session.webrtc_connection.connection_state, ConnectionState::Connected))
        } else {
            Ok(false)
        }
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