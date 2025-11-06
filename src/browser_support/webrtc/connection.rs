//! WebRTC Connection Establishment
//! 
//! Handles the creation and management of WebRTC peer connections with browser clients.

use crate::browser_support::{BrowserResult, BrowserSupportError};
use crate::browser_support::types::*;
use super::ConnectionStats;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::ice_transport::ice_candidate::RTCIceCandidate;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;

/// Connection establisher for WebRTC peer connections
pub struct ConnectionEstablisher {
    webrtc_api: Option<webrtc::api::API>,
    active_connections: Arc<Mutex<HashMap<Uuid, Arc<RTCPeerConnection>>>>,
    ice_candidate_sender: Option<mpsc::UnboundedSender<(Uuid, RTCIceCandidate)>>,
}

impl ConnectionEstablisher {
    /// Create a new connection establisher
    pub fn new() -> Self {
        Self {
            webrtc_api: None,
            active_connections: Arc::new(Mutex::new(HashMap::new())),
            ice_candidate_sender: None,
        }
    }
    
    /// Initialize the connection establisher
    pub async fn initialize(&mut self) -> BrowserResult<()> {
        // Create WebRTC API instance
        let api = APIBuilder::new().build();
        self.webrtc_api = Some(api);
        
        // Set up ICE candidate channel
        let (sender, mut receiver) = mpsc::unbounded_channel();
        self.ice_candidate_sender = Some(sender);
        
        // Start ICE candidate processing task
        tokio::spawn(async move {
            while let Some((connection_id, candidate)) = receiver.recv().await {
                println!("Processing ICE candidate for connection {}: {:?}", connection_id, candidate);
                // TODO: Send candidate to browser via signaling server
            }
        });
        
        Ok(())
    }
    
    /// Create a WebRTC peer connection
    pub async fn create_peer_connection(&self, signaling_info: &SignalingInfo) -> BrowserResult<(WebRTCConnection, Arc<RTCPeerConnection>)> {
        let api = self.webrtc_api.as_ref()
            .ok_or_else(|| BrowserSupportError::ConfigurationError {
                parameter: "webrtc_api".to_string(),
                issue: "Not initialized".to_string(),
            })?;
        
        // Convert ICE servers with default STUN servers if none provided
        let mut ice_servers = signaling_info.ice_servers
            .iter()
            .map(|server| RTCIceServer {
                urls: server.urls.clone(),
                username: server.username.clone().unwrap_or_default(),
                credential: server.credential.clone().unwrap_or_default(),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        
        // Add default STUN servers if none provided
        if ice_servers.is_empty() {
            ice_servers.push(RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                ..Default::default()
            });
            ice_servers.push(RTCIceServer {
                urls: vec!["stun:stun1.l.google.com:19302".to_string()],
                ..Default::default()
            });
        }
        
        // Create RTCConfiguration
        let config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };
        
        // Create peer connection
        let peer_connection = Arc::new(api.new_peer_connection(config).await
            .map_err(|e| BrowserSupportError::WebRTCError {
                reason: format!("Failed to create peer connection: {}", e),
            })?);
        
        let connection_id = Uuid::new_v4();
        
        // Set up connection event handlers
        self.setup_connection_handlers(&peer_connection, connection_id).await?;
        
        // Store the connection
        {
            let mut connections = self.active_connections.lock().await;
            connections.insert(connection_id, peer_connection.clone());
        }
        
        // Create WebRTC connection wrapper
        let connection = WebRTCConnection {
            connection_id,
            peer_id: "browser_client".to_string(), // Will be set properly later
            data_channels: HashMap::new(),
            connection_state: ConnectionState::New,
            ice_connection_state: IceConnectionState::New,
        };
        
        Ok((connection, peer_connection))
    }
    
    /// Setup connection event handlers
    async fn setup_connection_handlers(&self, peer_connection: &Arc<RTCPeerConnection>, connection_id: Uuid) -> BrowserResult<()> {
        let connection_id_clone = connection_id;
        
        // Set up connection state change handler
        peer_connection.on_connection_state_change(Box::new(move |state| {
            let conn_id = connection_id_clone;
            println!("WebRTC connection {} state changed: {:?}", conn_id, state);
            Box::pin(async move {
                // TODO: Update connection state in storage
            })
        }));
        
        let connection_id_clone = connection_id;
        
        // Set up ICE connection state change handler
        peer_connection.on_ice_connection_state_change(Box::new(move |state| {
            let conn_id = connection_id_clone;
            println!("ICE connection {} state changed: {:?}", conn_id, state);
            Box::pin(async move {
                // TODO: Update ICE connection state in storage
            })
        }));
        
        let ice_sender = self.ice_candidate_sender.clone();
        let connection_id_clone = connection_id;
        
        // Set up ICE candidate handler
        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let sender = ice_sender.clone();
            let conn_id = connection_id_clone;
            
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    println!("New ICE candidate for connection {}: {:?}", conn_id, candidate);
                    if let Some(sender) = sender {
                        let _ = sender.send((conn_id, candidate));
                    }
                }
            })
        }));
        
        let connection_id_clone = connection_id;
        
        // Set up data channel handler
        peer_connection.on_data_channel(Box::new(move |data_channel| {
            let conn_id = connection_id_clone;
            let label = data_channel.label().to_string();
            
            println!("New data channel for connection {}: {}", conn_id, label);
            
            // Set up data channel handlers
            let label_clone = label.clone();
            data_channel.on_open(Box::new(move || {
                let label = label_clone.clone();
                println!("Data channel '{}' opened for connection {}", label, conn_id);
                Box::pin(async move {
                    // TODO: Update data channel state
                })
            }));
            
            let label_clone = label.clone();
            data_channel.on_message(Box::new(move |msg| {
                let label = label_clone.clone();
                println!("Data channel '{}' message for connection {}: {} bytes", 
                        label, conn_id, msg.data.len());
                Box::pin(async move {
                    // TODO: Route message to appropriate handler
                })
            }));
            
            data_channel.on_close(Box::new(move || {
                println!("Data channel '{}' closed for connection {}", label, conn_id);
                Box::pin(async move {
                    // TODO: Clean up data channel state
                })
            }));
            
            Box::pin(async {})
        }));
        
        Ok(())
    }
    
    /// Handle WebRTC offer from browser
    pub async fn handle_offer(&self, connection_id: Uuid, offer: RTCSessionDescription) -> BrowserResult<RTCSessionDescription> {
        let connections = self.active_connections.lock().await;
        let peer_connection = connections.get(&connection_id)
            .ok_or_else(|| BrowserSupportError::WebRTCError {
                reason: format!("Connection {} not found", connection_id),
            })?;
        
        // Set remote description (offer)
        peer_connection.set_remote_description(offer).await
            .map_err(|e| BrowserSupportError::WebRTCError {
                reason: format!("Failed to set remote description: {}", e),
            })?;
        
        // Create answer
        let answer = peer_connection.create_answer(None).await
            .map_err(|e| BrowserSupportError::WebRTCError {
                reason: format!("Failed to create answer: {}", e),
            })?;
        
        // Set local description (answer)
        peer_connection.set_local_description(answer.clone()).await
            .map_err(|e| BrowserSupportError::WebRTCError {
                reason: format!("Failed to set local description: {}", e),
            })?;
        
        println!("Created answer for connection {}", connection_id);
        Ok(answer)
    }
    
    /// Handle WebRTC answer from browser
    pub async fn handle_answer(&self, connection_id: Uuid, answer: RTCSessionDescription) -> BrowserResult<()> {
        let connections = self.active_connections.lock().await;
        let peer_connection = connections.get(&connection_id)
            .ok_or_else(|| BrowserSupportError::WebRTCError {
                reason: format!("Connection {} not found", connection_id),
            })?;
        
        // Set remote description (answer)
        peer_connection.set_remote_description(answer).await
            .map_err(|e| BrowserSupportError::WebRTCError {
                reason: format!("Failed to set remote description: {}", e),
            })?;
        
        println!("Set answer for connection {}", connection_id);
        Ok(())
    }
    
    /// Handle ICE candidate from browser
    pub async fn handle_ice_candidate(&self, connection_id: Uuid, candidate: RTCIceCandidate) -> BrowserResult<()> {
        let connections = self.active_connections.lock().await;
        let peer_connection = connections.get(&connection_id)
            .ok_or_else(|| BrowserSupportError::WebRTCError {
                reason: format!("Connection {} not found", connection_id),
            })?;
        
        // Add ICE candidate
        peer_connection.add_ice_candidate(candidate).await
            .map_err(|e| BrowserSupportError::WebRTCError {
                reason: format!("Failed to add ICE candidate: {}", e),
            })?;
        
        println!("Added ICE candidate for connection {}", connection_id);
        Ok(())
    }
    
    /// Create an offer for the browser
    pub async fn create_offer(&self, connection_id: Uuid) -> BrowserResult<RTCSessionDescription> {
        let connections = self.active_connections.lock().await;
        let peer_connection = connections.get(&connection_id)
            .ok_or_else(|| BrowserSupportError::WebRTCError {
                reason: format!("Connection {} not found", connection_id),
            })?;
        
        // Create offer
        let offer = peer_connection.create_offer(None).await
            .map_err(|e| BrowserSupportError::WebRTCError {
                reason: format!("Failed to create offer: {}", e),
            })?;
        
        // Set local description (offer)
        peer_connection.set_local_description(offer.clone()).await
            .map_err(|e| BrowserSupportError::WebRTCError {
                reason: format!("Failed to set local description: {}", e),
            })?;
        
        println!("Created offer for connection {}", connection_id);
        Ok(offer)
    }
    
    /// Get connection statistics
    pub async fn get_connection_stats(&self, connection_id: Uuid) -> BrowserResult<ConnectionStats> {
        let connections = self.active_connections.lock().await;
        let peer_connection = connections.get(&connection_id)
            .ok_or_else(|| BrowserSupportError::WebRTCError {
                reason: format!("Connection {} not found", connection_id),
            })?;
        
        // Get connection stats
        let stats = peer_connection.get_stats().await;
        
        // Convert to our stats format
        let connection_stats = ConnectionStats {
            bytes_sent: 0, // TODO: Extract from WebRTC stats
            bytes_received: 0, // TODO: Extract from WebRTC stats
            packets_sent: 0, // TODO: Extract from WebRTC stats
            packets_received: 0, // TODO: Extract from WebRTC stats
            round_trip_time: None, // TODO: Extract from WebRTC stats
            jitter: None, // TODO: Extract from WebRTC stats
            packet_loss_rate: None, // TODO: Extract from WebRTC stats
        };
        
        Ok(connection_stats)
    }
    
    /// Close a specific connection
    pub async fn close_connection(&self, connection_id: Uuid) -> BrowserResult<()> {
        let mut connections = self.active_connections.lock().await;
        if let Some(peer_connection) = connections.remove(&connection_id) {
            peer_connection.close().await
                .map_err(|e| BrowserSupportError::WebRTCError {
                    reason: format!("Failed to close connection: {}", e),
                })?;
            println!("Closed connection {}", connection_id);
        }
        Ok(())
    }
    
    /// Shutdown the connection establisher
    pub async fn shutdown(&mut self) -> BrowserResult<()> {
        // Close all active connections
        let mut connections = self.active_connections.lock().await;
        for (connection_id, peer_connection) in connections.drain() {
            if let Err(e) = peer_connection.close().await {
                println!("Error closing connection {}: {}", connection_id, e);
            }
        }
        
        self.webrtc_api = None;
        self.ice_candidate_sender = None;
        Ok(())
    }
}