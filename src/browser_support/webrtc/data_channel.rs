//! WebRTC Data Channel Management
//! 
//! Handles creation and management of WebRTC data channels for different services.

use crate::browser_support::{BrowserResult, BrowserSupportError};
use crate::browser_support::types::*;
use webrtc::data_channel::{RTCDataChannel, data_channel_state::RTCDataChannelState};
use webrtc::peer_connection::RTCPeerConnection;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use std::collections::HashMap;
use uuid::Uuid;

/// Message handler trait for different channel types
#[async_trait::async_trait]
pub trait ChannelMessageHandler: Send + Sync {
    async fn handle_message(&self, channel_type: ChannelType, data: Vec<u8>) -> BrowserResult<()>;
    async fn handle_text(&self, channel_type: ChannelType, text: String) -> BrowserResult<()>;
}

/// Data channel manager for WebRTC connections
pub struct DataChannelManager {
    connection_id: Uuid,
    channels: Arc<Mutex<HashMap<ChannelType, Arc<RTCDataChannel>>>>,
    message_handlers: Arc<Mutex<HashMap<ChannelType, Arc<dyn ChannelMessageHandler>>>>,
    event_sender: Option<mpsc::UnboundedSender<DataChannelEvent>>,
}

/// Data channel events
#[derive(Debug, Clone)]
pub enum DataChannelEvent {
    ChannelOpened { connection_id: Uuid, channel_type: ChannelType },
    ChannelClosed { connection_id: Uuid, channel_type: ChannelType },
    MessageReceived { connection_id: Uuid, channel_type: ChannelType, data: Vec<u8> },
    TextReceived { connection_id: Uuid, channel_type: ChannelType, text: String },
    ChannelError { connection_id: Uuid, channel_type: ChannelType, error: String },
}

impl DataChannelManager {
    /// Create a new data channel manager
    pub fn new(connection_id: Uuid) -> Self {
        Self {
            connection_id,
            channels: Arc::new(Mutex::new(HashMap::new())),
            message_handlers: Arc::new(Mutex::new(HashMap::new())),
            event_sender: None,
        }
    }
    
    /// Initialize the data channel manager with event handling
    pub async fn initialize(&mut self) -> BrowserResult<mpsc::UnboundedReceiver<DataChannelEvent>> {
        let (sender, receiver) = mpsc::unbounded_channel();
        self.event_sender = Some(sender);
        Ok(receiver)
    }
    
    /// Register a message handler for a specific channel type
    pub async fn register_handler(&self, channel_type: ChannelType, handler: Arc<dyn ChannelMessageHandler>) {
        let mut handlers = self.message_handlers.lock().await;
        handlers.insert(channel_type, handler);
    }
    
    /// Create a data channel for a specific service
    pub async fn create_data_channel(
        &self,
        peer_connection: &Arc<RTCPeerConnection>,
        channel_type: ChannelType,
    ) -> BrowserResult<Arc<RTCDataChannel>> {
        let label = self.get_channel_label(&channel_type);
        
        // Create data channel configuration
        let config = webrtc::data_channel::data_channel_init::RTCDataChannelInit {
            ordered: Some(true),
            max_retransmits: Some(3),
            ..Default::default()
        };
        
        // Create the data channel
        let data_channel = peer_connection
            .create_data_channel(&label, Some(config))
            .await
            .map_err(|e| BrowserSupportError::WebRTCError {
                reason: format!("Failed to create data channel '{}': {}", label, e),
            })?;
        
        // Set up data channel event handlers
        self.setup_data_channel_handlers(&data_channel, &channel_type).await?;
        
        // Store the data channel
        let mut channels = self.channels.lock().await;
        channels.insert(channel_type, data_channel.clone());
        
        Ok(data_channel)
    }
    
    /// Set up event handlers for a data channel
    async fn setup_data_channel_handlers(
        &self,
        data_channel: &Arc<RTCDataChannel>,
        channel_type: &ChannelType,
    ) -> BrowserResult<()> {
        let channel_type_clone = channel_type.clone();
        
        // Set up open handler
        data_channel.on_open(Box::new(move || {
            let channel_type = channel_type_clone.clone();
            println!("Data channel opened: {:?}", channel_type);
            Box::pin(async move {
                // TODO: Notify that the channel is ready
            })
        }));
        
        let channel_type_clone = channel_type.clone();
        
        // Set up message handler
        data_channel.on_message(Box::new(move |msg| {
            let channel_type = channel_type_clone.clone();
            println!("Data channel message on {:?}: {} bytes", channel_type, msg.data.len());
            
            Box::pin(async move {
                // TODO: Route message to appropriate handler based on channel type
                match channel_type {
                    ChannelType::FileTransfer => {
                        // Handle file transfer messages
                    },
                    ChannelType::Clipboard => {
                        // Handle clipboard sync messages
                    },
                    ChannelType::Command => {
                        // Handle command execution messages
                    },
                    ChannelType::Video => {
                        // Handle video streaming messages
                    },
                    ChannelType::Control => {
                        // Handle control messages
                    },
                }
            })
        }));
        
        let channel_type_clone = channel_type.clone();
        
        // Set up close handler
        data_channel.on_close(Box::new(move || {
            let channel_type = channel_type_clone.clone();
            println!("Data channel closed: {:?}", channel_type);
            Box::pin(async move {
                // TODO: Clean up channel resources
            })
        }));
        
        let channel_type_clone = channel_type.clone();
        
        // Set up error handler
        data_channel.on_error(Box::new(move |err| {
            let channel_type = channel_type_clone.clone();
            println!("Data channel error on {:?}: {}", channel_type, err);
            Box::pin(async move {
                // TODO: Handle channel errors
            })
        }));
        
        Ok(())
    }
    
    /// Send data through a specific channel
    pub async fn send_data(&self, channel_type: &ChannelType, data: &[u8]) -> BrowserResult<()> {
        let channels = self.channels.lock().await;
        
        if let Some(channel) = channels.get(channel_type) {
            // In webrtc v0.11, use Bytes::from for binary data
            use bytes::Bytes;
            channel.send(&Bytes::from(data.to_vec()))
                .await
                .map_err(|e| BrowserSupportError::WebRTCError {
                    reason: format!("Failed to send data on {:?} channel: {}", channel_type, e),
                })?;
        } else {
            return Err(BrowserSupportError::WebRTCError {
                reason: format!("Data channel {:?} not found", channel_type),
            });
        }
        
        Ok(())
    }
    
    /// Send text through a specific channel
    pub async fn send_text(&self, channel_type: &ChannelType, text: &str) -> BrowserResult<()> {
        let channels = self.channels.lock().await;
        
        if let Some(channel) = channels.get(channel_type) {
            // In webrtc v0.11, convert text to Bytes
            use bytes::Bytes;
            channel.send(&Bytes::from(text.as_bytes().to_vec()))
                .await
                .map_err(|e| BrowserSupportError::WebRTCError {
                    reason: format!("Failed to send text on {:?} channel: {}", channel_type, e),
                })?;
        } else {
            return Err(BrowserSupportError::WebRTCError {
                reason: format!("Data channel {:?} not found", channel_type),
            });
        }
        
        Ok(())
    }
    
    /// Get the ready state of a data channel
    pub async fn get_channel_state(&self, channel_type: &ChannelType) -> Option<DataChannelState> {
        let channels = self.channels.lock().await;
        
        if let Some(channel) = channels.get(channel_type) {
            match channel.ready_state() {
                RTCDataChannelState::Connecting => Some(DataChannelState::Connecting),
                RTCDataChannelState::Open => Some(DataChannelState::Open),
                RTCDataChannelState::Closing => Some(DataChannelState::Closing),
                RTCDataChannelState::Closed => Some(DataChannelState::Closed),
                _ => None,
            }
        } else {
            None
        }
    }
    
    /// Close a specific data channel
    pub async fn close_channel(&self, channel_type: &ChannelType) -> BrowserResult<()> {
        let mut channels = self.channels.lock().await;
        
        if let Some(channel) = channels.remove(channel_type) {
            channel.close().await
                .map_err(|e| BrowserSupportError::WebRTCError {
                    reason: format!("Failed to close {:?} channel: {}", channel_type, e),
                })?;
        }
        
        Ok(())
    }
    
    /// Close all data channels
    pub async fn close_all_channels(&self) -> BrowserResult<()> {
        let mut channels = self.channels.lock().await;
        
        for (channel_type, channel) in channels.drain() {
            if let Err(e) = channel.close().await {
                println!("Error closing {:?} channel: {}", channel_type, e);
            }
        }
        
        Ok(())
    }
    
    /// Get the label for a channel type
    fn get_channel_label(&self, channel_type: &ChannelType) -> String {
        match channel_type {
            ChannelType::FileTransfer => "kizuna-file-transfer".to_string(),
            ChannelType::Clipboard => "kizuna-clipboard".to_string(),
            ChannelType::Command => "kizuna-command".to_string(),
            ChannelType::Video => "kizuna-video".to_string(),
            ChannelType::Control => "kizuna-control".to_string(),
        }
    }
    
    /// Get statistics for all channels
    pub async fn get_channel_stats(&self) -> HashMap<ChannelType, DataChannelInfo> {
        let channels = self.channels.lock().await;
        let mut stats = HashMap::new();
        
        for (channel_type, channel) in channels.iter() {
            let ready_state = match channel.ready_state() {
                RTCDataChannelState::Connecting => DataChannelState::Connecting,
                RTCDataChannelState::Open => DataChannelState::Open,
                RTCDataChannelState::Closing => DataChannelState::Closing,
                RTCDataChannelState::Closed => DataChannelState::Closed,
                _ => DataChannelState::Closed,
            };
            
            let info = DataChannelInfo {
                channel_type: channel_type.clone(),
                ready_state,
                bytes_sent: 0, // TODO: Get actual stats from WebRTC
                bytes_received: 0, // TODO: Get actual stats from WebRTC
            };
            
            stats.insert(channel_type.clone(), info);
        }
        
        stats
    }
}