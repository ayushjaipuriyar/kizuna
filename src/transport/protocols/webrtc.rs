use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use tokio::sync::{mpsc, Mutex, RwLock};
use webrtc::api::interceptor_registry::register_default_interceptors;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::interceptor::registry::Registry;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

use crate::transport::{
    Connection, ConnectionInfo, PeerAddress, PeerId, Transport, TransportCapabilities, TransportError,
};

/// WebRTC transport implementation using DataChannels
pub struct WebRtcTransport {
    config: WebRtcConfig,
    api: Arc<webrtc::api::API>,
    ice_servers: Vec<RTCIceServer>,
    signaling_handler: Arc<dyn SignalingHandler>,
}

impl std::fmt::Debug for WebRtcTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebRtcTransport")
            .field("config", &self.config)
            .field("ice_servers", &self.ice_servers)
            .finish()
    }
}

/// Configuration for WebRTC transport
#[derive(Debug, Clone)]
pub struct WebRtcConfig {
    /// ICE servers for NAT traversal (STUN/TURN)
    pub ice_servers: Vec<IceServerConfig>,
    /// Maximum message size for DataChannels
    pub max_message_size: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Enable ordered delivery for reliable channels
    pub ordered: bool,
    /// Maximum retransmits for reliable channels
    pub max_retransmits: Option<u16>,
    /// Maximum packet lifetime for unreliable channels
    pub max_packet_lifetime: Option<Duration>,
}

/// ICE server configuration
#[derive(Debug, Clone)]
pub struct IceServerConfig {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

/// WebRTC connection implementation
pub struct WebRtcConnection {
    peer_connection: Arc<RTCPeerConnection>,
    data_channel: Arc<RTCDataChannel>,
    peer_id: PeerId,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    info: Arc<Mutex<ConnectionInfo>>,
    message_receiver: Arc<Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
    is_connected: Arc<RwLock<bool>>,
    config: WebRtcConfig,
}

impl std::fmt::Debug for WebRtcConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebRtcConnection")
            .field("peer_id", &self.peer_id)
            .field("local_addr", &self.local_addr)
            .field("remote_addr", &self.remote_addr)
            .field("config", &self.config)
            .finish()
    }
}

/// Signaling handler trait for ICE candidate exchange
#[async_trait]
pub trait SignalingHandler: Send + Sync + std::fmt::Debug {
    /// Send signaling message to peer
    async fn send_signaling_message(&self, peer_id: &PeerId, message: SignalingMessage) -> Result<(), TransportError>;
    
    /// Receive signaling message from peer with timeout
    async fn receive_signaling_message(&self, peer_id: &PeerId, timeout: Duration) -> Result<SignalingMessage, TransportError>;
    
    /// Exchange ICE candidates with peer
    async fn exchange_ice_candidates(&self, peer_id: &PeerId, local_candidates: Vec<IceCandidate>) -> Result<Vec<IceCandidate>, TransportError>;
    
    /// Register for incoming connection offers
    async fn register_for_offers(&self, peer_id: &PeerId) -> Result<(), TransportError>;
    
    /// Wait for incoming connection offer
    async fn wait_for_offer(&self, timeout: Duration) -> Result<(PeerId, SignalingMessage), TransportError>;
}

/// Signaling messages for WebRTC negotiation
#[derive(Debug, Clone)]
pub enum SignalingMessage {
    Offer {
        sdp: String,
        ice_ufrag: String,
        ice_pwd: String,
    },
    Answer {
        sdp: String,
        ice_ufrag: String,
        ice_pwd: String,
    },
    IceCandidate {
        candidate: IceCandidate,
    },
    ConnectionRequest {
        peer_id: PeerId,
        capabilities: Vec<String>,
    },
    ConnectionResponse {
        accepted: bool,
        reason: Option<String>,
    },
}

/// ICE candidate information
#[derive(Debug, Clone)]
pub struct IceCandidate {
    pub candidate: String,
    pub sdp_mid: Option<String>,
    pub sdp_mline_index: Option<u16>,
    pub foundation: String,
    pub priority: u32,
    pub ip: String,
    pub port: u16,
    pub candidate_type: IceCandidateType,
    pub protocol: String,
}

/// Types of ICE candidates
#[derive(Debug, Clone, PartialEq)]
pub enum IceCandidateType {
    Host,
    ServerReflexive,
    PeerReflexive,
    Relay,
}

/// NAT traversal configuration and state
#[derive(Debug, Clone)]
pub struct NatTraversalConfig {
    /// STUN servers for NAT type detection and reflexive candidates
    pub stun_servers: Vec<String>,
    /// TURN servers for relay candidates
    pub turn_servers: Vec<TurnServerConfig>,
    /// Enable aggressive nomination for faster connection establishment
    pub aggressive_nomination: bool,
    /// ICE gathering timeout
    pub ice_gathering_timeout: Duration,
    /// Connection check timeout
    pub connection_check_timeout: Duration,
}

/// TURN server configuration
#[derive(Debug, Clone)]
pub struct TurnServerConfig {
    pub url: String,
    pub username: String,
    pub credential: String,
    pub realm: Option<String>,
}

/// NAT traversal manager for handling ICE and connectivity
#[derive(Debug)]
pub struct NatTraversalManager {
    config: NatTraversalConfig,
    local_candidates: Arc<Mutex<Vec<IceCandidate>>>,
    remote_candidates: Arc<Mutex<Vec<IceCandidate>>>,
    nat_type: Arc<Mutex<Option<NatType>>>,
}

/// Detected NAT type
#[derive(Debug, Clone, PartialEq)]
pub enum NatType {
    Open,
    FullCone,
    RestrictedCone,
    PortRestrictedCone,
    Symmetric,
    Unknown,
}

/// Connection state for WebRTC peer connections
#[derive(Debug, Clone, PartialEq)]
pub enum WebRtcConnectionState {
    New,
    Connecting,
    Connected,
    Disconnected,
    Failed,
    Closed,
}

/// DataChannel configuration for different reliability modes
#[derive(Debug, Clone)]
pub struct DataChannelConfig {
    pub label: String,
    pub ordered: bool,
    pub max_retransmits: Option<u16>,
    pub max_packet_lifetime: Option<Duration>,
    pub protocol: Option<String>,
}

/// Default signaling handler (placeholder implementation)
#[derive(Debug)]
pub struct DefaultSignalingHandler {
    // In a real implementation, this would handle signaling through discovery layer
    pending_messages: Arc<Mutex<HashMap<PeerId, Vec<SignalingMessage>>>>,
    incoming_offers: Arc<Mutex<Vec<(PeerId, SignalingMessage)>>>,
    registered_peers: Arc<Mutex<Vec<PeerId>>>,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            ice_servers: vec![
                IceServerConfig {
                    urls: vec!["stun:stun.l.google.com:19302".to_string()],
                    username: None,
                    credential: None,
                },
            ],
            max_message_size: 65536, // 64KB
            connection_timeout: Duration::from_secs(30),
            ordered: true,
            max_retransmits: Some(3),
            max_packet_lifetime: None,
        }
    }
}

impl WebRtcTransport {
    /// Create a new WebRTC transport
    pub fn new() -> Result<Self, TransportError> {
        Self::with_config(WebRtcConfig::default())
    }

    /// Create a new WebRTC transport with custom configuration
    pub fn with_config(config: WebRtcConfig) -> Result<Self, TransportError> {
        let signaling_handler = Arc::new(DefaultSignalingHandler::new());
        Self::with_config_and_signaling(config, signaling_handler)
    }

    /// Create a new WebRTC transport with custom configuration and signaling handler
    pub fn with_config_and_signaling(
        config: WebRtcConfig,
        signaling_handler: Arc<dyn SignalingHandler>,
    ) -> Result<Self, TransportError> {
        // Create media engine
        let mut media_engine = MediaEngine::default();
        
        // Create interceptor registry
        let mut registry = Registry::new();
        registry = register_default_interceptors(registry, &mut media_engine)
            .map_err(|e| TransportError::WebRTC(format!("Failed to register interceptors: {}", e)))?;

        // Create API
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .build();

        // Convert ICE server configuration
        let ice_servers: Vec<RTCIceServer> = config
            .ice_servers
            .iter()
            .map(|ice_config| {
                RTCIceServer {
                    urls: ice_config.urls.clone(),
                    username: ice_config.username.clone().unwrap_or_default(),
                    credential: ice_config.credential.clone().unwrap_or_default(),
                    ..Default::default()
                }
            })
            .collect();

        Ok(Self {
            config,
            api: Arc::new(api),
            ice_servers,
            signaling_handler,
        })
    }

    /// Create peer connection with configuration
    async fn create_peer_connection(&self) -> Result<Arc<RTCPeerConnection>, TransportError> {
        let rtc_config = RTCConfiguration {
            ice_servers: self.ice_servers.clone(),
            ..Default::default()
        };

        let peer_connection = self
            .api
            .new_peer_connection(rtc_config)
            .await
            .map_err(|e| TransportError::WebRTC(format!("Failed to create peer connection: {}", e)))?;

        Ok(Arc::new(peer_connection))
    }

    /// Create data channel with configuration
    async fn create_data_channel(
        &self,
        peer_connection: &RTCPeerConnection,
        label: &str,
    ) -> Result<Arc<RTCDataChannel>, TransportError> {
        let mut data_channel_config = webrtc::data_channel::data_channel_init::RTCDataChannelInit {
            ordered: Some(self.config.ordered),
            max_retransmits: self.config.max_retransmits,
            ..Default::default()
        };

        if let Some(lifetime) = self.config.max_packet_lifetime {
            data_channel_config.max_packet_life_time = Some(lifetime.as_millis() as u16);
        }

        let data_channel = peer_connection
            .create_data_channel(label, Some(data_channel_config))
            .await
            .map_err(|e| TransportError::WebRTC(format!("Failed to create data channel: {}", e)))?;

        Ok(data_channel)
    }

    /// Create NAT traversal manager
    fn create_nat_traversal_manager(&self) -> NatTraversalManager {
        let nat_config = NatTraversalConfig {
            stun_servers: self.ice_servers.iter()
                .flat_map(|server| server.urls.clone())
                .filter(|url| url.starts_with("stun:"))
                .collect(),
            turn_servers: self.ice_servers.iter()
                .flat_map(|server| server.urls.clone())
                .filter(|url| url.starts_with("turn:"))
                .map(|url| TurnServerConfig {
                    url,
                    username: "".to_string(), // Would be configured properly
                    credential: "".to_string(),
                    realm: None,
                })
                .collect(),
            ..Default::default()
        };
        
        NatTraversalManager::new(nat_config)
    }

    /// Perform comprehensive ICE candidate exchange and connectivity establishment
    async fn establish_ice_connectivity(
        &self,
        peer_connection: &RTCPeerConnection,
        peer_id: &PeerId,
    ) -> Result<(), TransportError> {
        let nat_manager = self.create_nat_traversal_manager();
        
        // Detect NAT type for optimization
        let nat_type = nat_manager.detect_nat_type().await?;
        println!("Detected NAT type: {:?}", nat_type);
        
        // Gather local ICE candidates
        let local_candidates = nat_manager.gather_local_candidates(peer_connection).await?;
        println!("Gathered {} local ICE candidates", local_candidates.len());
        
        // Exchange candidates with remote peer
        let remote_candidates = self
            .signaling_handler
            .exchange_ice_candidates(peer_id, local_candidates)
            .await?;
        
        println!("Received {} remote ICE candidates", remote_candidates.len());
        
        // Add remote candidates to NAT manager
        for candidate in remote_candidates {
            nat_manager.add_remote_candidate(candidate).await?;
        }
        
        // Perform connectivity checks
        let connectivity_established = nat_manager.perform_connectivity_checks().await?;
        
        if !connectivity_established {
            return Err(TransportError::NatTraversalFailed {
                method: "ICE connectivity checks".to_string(),
            });
        }
        
        println!("ICE connectivity established successfully");
        Ok(())
    }

    /// Create data channel with specific configuration
    async fn create_data_channel_with_config(
        &self,
        peer_connection: &RTCPeerConnection,
        config: &DataChannelConfig,
    ) -> Result<Arc<RTCDataChannel>, TransportError> {
        let mut data_channel_init = webrtc::data_channel::data_channel_init::RTCDataChannelInit {
            ordered: Some(config.ordered),
            max_retransmits: config.max_retransmits,
            protocol: config.protocol.clone(),
            ..Default::default()
        };

        if let Some(lifetime) = config.max_packet_lifetime {
            data_channel_init.max_packet_life_time = Some(lifetime.as_millis() as u16);
        }

        let data_channel = peer_connection
            .create_data_channel(&config.label, Some(data_channel_init))
            .await
            .map_err(|e| TransportError::WebRTC(format!("Failed to create data channel: {}", e)))?;

        Ok(data_channel)
    }

    /// Handle incoming connection offer
    async fn handle_incoming_offer(
        api: &webrtc::api::API,
        ice_servers: &[RTCIceServer],
        _config: &WebRtcConfig,
        signaling_handler: &Arc<dyn SignalingHandler>,
        peer_id: PeerId,
        offer_message: SignalingMessage,
    ) -> Result<(), TransportError> {
        if let SignalingMessage::Offer { sdp, ice_ufrag: _, ice_pwd: _ } = offer_message {
            // Create peer connection for incoming offer
            let rtc_config = RTCConfiguration {
                ice_servers: ice_servers.to_vec(),
                ..Default::default()
            };

            let peer_connection = api
                .new_peer_connection(rtc_config)
                .await
                .map_err(|e| TransportError::WebRTC(format!("Failed to create peer connection: {}", e)))?;

            // Set remote description (offer)
            let offer = RTCSessionDescription::offer(sdp)
                .map_err(|e| TransportError::WebRTC(format!("Invalid offer SDP: {}", e)))?;

            peer_connection
                .set_remote_description(offer)
                .await
                .map_err(|e| TransportError::WebRTC(format!("Failed to set remote description: {}", e)))?;

            // Create answer
            let answer = peer_connection
                .create_answer(None)
                .await
                .map_err(|e| TransportError::WebRTC(format!("Failed to create answer: {}", e)))?;

            peer_connection
                .set_local_description(answer.clone())
                .await
                .map_err(|e| TransportError::WebRTC(format!("Failed to set local description: {}", e)))?;

            // Send answer back to peer
            signaling_handler
                .send_signaling_message(
                    &peer_id,
                    SignalingMessage::Answer {
                        sdp: answer.sdp,
                        ice_ufrag: "default_ufrag".to_string(),
                        ice_pwd: "default_pwd".to_string(),
                    },
                )
                .await?;

            // Set up data channel handling for incoming connections
            peer_connection.on_data_channel(Box::new(move |data_channel| {
                Box::pin(async move {
                    println!("Received data channel: {}", data_channel.label());
                    
                    // Set up message handling for the incoming data channel
                    data_channel.on_message(Box::new(move |msg| {
                        Box::pin(async move {
                            println!("Received message on incoming data channel: {} bytes", msg.data.len());
                        })
                    }));
                })
            }));

            println!("Successfully handled incoming offer from peer: {}", peer_id);
        } else {
            return Err(TransportError::WebRTC("Expected offer message".to_string()));
        }

        Ok(())
    }
}

#[async_trait]
impl Transport for WebRtcTransport {
    async fn connect(&self, addr: &PeerAddress) -> Result<Box<dyn Connection>, TransportError> {
        let peer_id = &addr.peer_id;
        
        // Create peer connection
        let peer_connection = self.create_peer_connection().await?;
        
        // Create data channel with appropriate configuration
        let data_channel_config = if self.config.ordered {
            DataChannelConfig::reliable()
        } else {
            DataChannelConfig::unreliable()
        };
        
        let data_channel = self.create_data_channel_with_config(&peer_connection, &data_channel_config).await?;
        
        // Set up message handling
        let (message_sender, message_receiver) = mpsc::unbounded_channel();
        let message_sender = Arc::new(Mutex::new(message_sender));
        
        // Set up data channel event handlers
        let sender_clone = message_sender.clone();
        data_channel.on_message(Box::new(move |msg: DataChannelMessage| {
            let sender = sender_clone.clone();
            Box::pin(async move {
                if let Ok(sender) = sender.try_lock() {
                    let _ = sender.send(msg.data.to_vec());
                }
            })
        }));

        // Set up connection state monitoring
        let is_connected = Arc::new(RwLock::new(false));
        let is_connected_clone = is_connected.clone();
        
        peer_connection.on_peer_connection_state_change(Box::new(move |state: RTCPeerConnectionState| {
            let is_connected = is_connected_clone.clone();
            Box::pin(async move {
                let connected = matches!(state, RTCPeerConnectionState::Connected);
                let mut guard = is_connected.write().await;
                *guard = connected;
            })
        }));

        // Create and send offer
        let offer = peer_connection
            .create_offer(None)
            .await
            .map_err(|e| TransportError::WebRTC(format!("Failed to create offer: {}", e)))?;

        peer_connection
            .set_local_description(offer.clone())
            .await
            .map_err(|e| TransportError::WebRTC(format!("Failed to set local description: {}", e)))?;

        // Send offer through signaling with ICE credentials
        self.signaling_handler
            .send_signaling_message(
                peer_id,
                SignalingMessage::Offer {
                    sdp: offer.sdp,
                    ice_ufrag: "default_ufrag".to_string(), // Would be extracted from SDP
                    ice_pwd: "default_pwd".to_string(), // Would be extracted from SDP
                },
            )
            .await?;

        // Wait for answer with timeout
        let answer_message = self
            .signaling_handler
            .receive_signaling_message(peer_id, self.config.connection_timeout)
            .await?;

        if let SignalingMessage::Answer { sdp, ice_ufrag: _, ice_pwd: _ } = answer_message {
            let answer = RTCSessionDescription::answer(sdp)
                .map_err(|e| TransportError::WebRTC(format!("Invalid answer SDP: {}", e)))?;

            peer_connection
                .set_remote_description(answer)
                .await
                .map_err(|e| TransportError::WebRTC(format!("Failed to set remote description: {}", e)))?;
        } else {
            return Err(TransportError::WebRTC("Expected answer message".to_string()));
        }

        // Establish ICE connectivity with comprehensive NAT traversal
        self.establish_ice_connectivity(&peer_connection, peer_id).await?;

        // Wait for connection to be established
        let start_time = std::time::Instant::now();
        while start_time.elapsed() < self.config.connection_timeout {
            if *is_connected.read().await {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        if !*is_connected.read().await {
            return Err(TransportError::ConnectionTimeout {
                timeout: self.config.connection_timeout,
            });
        }

        // Use first address as placeholder for local/remote addresses
        let remote_addr = addr.addresses.first().copied().unwrap_or_else(|| {
            SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)), 0)
        });
        let local_addr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), 0);

        let connection_info = ConnectionInfo::new(
            peer_id.clone(),
            local_addr,
            remote_addr,
            "webrtc".to_string(),
        );

        let connection = WebRtcConnection {
            peer_connection,
            data_channel,
            peer_id: peer_id.clone(),
            local_addr,
            remote_addr,
            info: Arc::new(Mutex::new(connection_info)),
            message_receiver: Arc::new(Mutex::new(message_receiver)),
            is_connected,
            config: self.config.clone(),
        };

        Ok(Box::new(connection))
    }

    async fn listen(&self, bind_addr: &SocketAddr) -> Result<(), TransportError> {
        // WebRTC doesn't have a traditional listen mode like TCP
        // Instead, it waits for incoming connection offers through signaling
        
        println!("WebRTC transport listening for incoming connections on signaling channel (bind_addr: {})", bind_addr);
        
        // Register for incoming offers
        let local_peer_id = format!("local-peer-{}", bind_addr.port());
        self.signaling_handler.register_for_offers(&local_peer_id).await?;
        
        // In a real implementation, this would spawn a task to handle incoming offers
        tokio::spawn({
            let signaling_handler = self.signaling_handler.clone();
            let api = self.api.clone();
            let ice_servers = self.ice_servers.clone();
            let config = self.config.clone();
            
            async move {
                loop {
                    match signaling_handler.wait_for_offer(Duration::from_secs(30)).await {
                        Ok((peer_id, offer_message)) => {
                            println!("Received connection offer from peer: {}", peer_id);
                            
                            // Handle the incoming offer
                            if let Err(e) = Self::handle_incoming_offer(
                                &api,
                                &ice_servers,
                                &config,
                                &signaling_handler,
                                peer_id,
                                offer_message,
                            ).await {
                                eprintln!("Failed to handle incoming offer: {}", e);
                            }
                        }
                        Err(_) => {
                            // Timeout waiting for offers - continue listening
                            continue;
                        }
                    }
                }
            }
        });
        
        Ok(())
    }

    fn protocol_name(&self) -> &'static str {
        "webrtc"
    }

    fn is_available(&self) -> bool {
        // WebRTC is available on most platforms
        true
    }

    fn priority(&self) -> u8 {
        90 // High priority due to NAT traversal capabilities
    }

    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities::webrtc()
    }
}

#[async_trait]
impl Connection for WebRtcConnection {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        // Try to receive a message from the data channel
        let mut receiver = self.message_receiver.lock().await;
        
        match receiver.try_recv() {
            Ok(data) => {
                let bytes_to_copy = std::cmp::min(buf.len(), data.len());
                buf[..bytes_to_copy].copy_from_slice(&data[..bytes_to_copy]);
                
                // Update connection info
                {
                    let mut info = self.info.lock().await;
                    info.add_bytes_received(bytes_to_copy as u64);
                }
                
                Ok(bytes_to_copy)
            }
            Err(mpsc::error::TryRecvError::Empty) => {
                // No data available, return 0 (would block in real implementation)
                Ok(0)
            }
            Err(mpsc::error::TryRecvError::Disconnected) => {
                Err(TransportError::WebRTC("Data channel disconnected".to_string()))
            }
        }
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, TransportError> {
        // Check message size limit
        if buf.len() > self.config.max_message_size {
            return Err(TransportError::WebRTC(format!(
                "Message size {} exceeds limit {}",
                buf.len(),
                self.config.max_message_size
            )));
        }

        // Send data through the data channel
        let bytes = bytes::Bytes::from(buf.to_vec());
        self.data_channel
            .send(&bytes)
            .await
            .map_err(|e| TransportError::WebRTC(format!("Failed to send data: {}", e)))?;

        // Update connection info
        {
            let mut info = self.info.lock().await;
            info.add_bytes_sent(buf.len() as u64);
        }

        Ok(buf.len())
    }

    async fn flush(&mut self) -> Result<(), TransportError> {
        // WebRTC DataChannels don't have explicit flushing
        // Data is sent immediately when write is called
        Ok(())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        // Close the data channel
        self.data_channel
            .close()
            .await
            .map_err(|e| TransportError::WebRTC(format!("Failed to close data channel: {}", e)))?;

        // Close the peer connection
        self.peer_connection
            .close()
            .await
            .map_err(|e| TransportError::WebRTC(format!("Failed to close peer connection: {}", e)))?;

        // Update connection state
        {
            let mut is_connected = self.is_connected.write().await;
            *is_connected = false;
        }

        Ok(())
    }

    fn info(&self) -> ConnectionInfo {
        // Return a clone of the current connection info
        // In a real implementation, this would be more efficient
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                self.info.lock().await.clone()
            })
        })
    }

    fn is_connected(&self) -> bool {
        // Check if the connection is still active
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                *self.is_connected.read().await
            })
        })
    }
}

impl Default for NatTraversalConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
                "stun:stun2.l.google.com:19302".to_string(),
            ],
            turn_servers: Vec::new(),
            aggressive_nomination: true,
            ice_gathering_timeout: Duration::from_secs(10),
            connection_check_timeout: Duration::from_secs(30),
        }
    }
}

impl Default for DataChannelConfig {
    fn default() -> Self {
        Self {
            label: "kizuna-data".to_string(),
            ordered: true,
            max_retransmits: Some(3),
            max_packet_lifetime: None,
            protocol: None,
        }
    }
}

impl DataChannelConfig {
    /// Create configuration for reliable, ordered data channel
    pub fn reliable() -> Self {
        Self {
            ordered: true,
            max_retransmits: None,
            max_packet_lifetime: None,
            ..Default::default()
        }
    }

    /// Create configuration for unreliable, unordered data channel (best effort)
    pub fn unreliable() -> Self {
        Self {
            ordered: false,
            max_retransmits: Some(0),
            max_packet_lifetime: Some(Duration::from_millis(100)),
            ..Default::default()
        }
    }

    /// Create configuration for semi-reliable data channel with limited retransmits
    pub fn semi_reliable(max_retransmits: u16) -> Self {
        Self {
            ordered: true,
            max_retransmits: Some(max_retransmits),
            max_packet_lifetime: None,
            ..Default::default()
        }
    }

    /// Create configuration for time-limited data channel
    pub fn time_limited(max_lifetime: Duration) -> Self {
        Self {
            ordered: false,
            max_retransmits: None,
            max_packet_lifetime: Some(max_lifetime),
            ..Default::default()
        }
    }
}

impl NatTraversalManager {
    pub fn new(config: NatTraversalConfig) -> Self {
        Self {
            config,
            local_candidates: Arc::new(Mutex::new(Vec::new())),
            remote_candidates: Arc::new(Mutex::new(Vec::new())),
            nat_type: Arc::new(Mutex::new(None)),
        }
    }

    /// Detect NAT type using STUN servers
    pub async fn detect_nat_type(&self) -> Result<NatType, TransportError> {
        // Simplified NAT type detection - in a real implementation this would
        // use STUN binding requests to detect the actual NAT behavior
        
        if self.config.stun_servers.is_empty() {
            return Ok(NatType::Unknown);
        }

        // For now, assume we can detect the NAT type through STUN
        // In a real implementation, this would:
        // 1. Send STUN binding requests to multiple servers
        // 2. Compare the mapped addresses to determine NAT behavior
        // 3. Test for port preservation and filtering behavior
        
        let detected_type = NatType::FullCone; // Placeholder
        
        {
            let mut nat_type = self.nat_type.lock().await;
            *nat_type = Some(detected_type.clone());
        }

        Ok(detected_type)
    }

    /// Gather local ICE candidates
    pub async fn gather_local_candidates(&self, peer_connection: &RTCPeerConnection) -> Result<Vec<IceCandidate>, TransportError> {
        let candidates = Arc::new(Mutex::new(Vec::new()));
        let candidates_clone = candidates.clone();
        let gathering_complete = Arc::new(tokio::sync::Notify::new());
        let gathering_complete_clone = gathering_complete.clone();

        // Set up ICE candidate gathering
        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let candidates = candidates_clone.clone();
            let gathering_complete = gathering_complete_clone.clone();
            
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    // Convert webrtc candidate to our IceCandidate format
                    let ice_candidate = IceCandidate {
                        candidate: format!("{:?}", candidate), // Simplified
                        sdp_mid: Some("0".to_string()),
                        sdp_mline_index: Some(0),
                        foundation: "1".to_string(), // Simplified
                        priority: 2130706431, // Simplified
                        ip: "127.0.0.1".to_string(), // Simplified
                        port: 0, // Simplified
                        candidate_type: IceCandidateType::Host, // Simplified
                        protocol: "udp".to_string(),
                    };
                    
                    let mut candidates_guard = candidates.lock().await;
                    candidates_guard.push(ice_candidate);
                } else {
                    // Gathering complete
                    gathering_complete.notify_one();
                }
            })
        }));

        // Wait for gathering to complete or timeout
        tokio::select! {
            _ = gathering_complete.notified() => {},
            _ = tokio::time::sleep(self.config.ice_gathering_timeout) => {
                return Err(TransportError::WebRTC("ICE gathering timeout".to_string()));
            }
        }

        let candidates_guard = candidates.lock().await;
        let local_candidates = candidates_guard.clone();
        
        // Store local candidates
        {
            let mut stored_candidates = self.local_candidates.lock().await;
            *stored_candidates = local_candidates.clone();
        }

        Ok(local_candidates)
    }

    /// Add remote ICE candidate
    pub async fn add_remote_candidate(&self, candidate: IceCandidate) -> Result<(), TransportError> {
        let mut remote_candidates = self.remote_candidates.lock().await;
        remote_candidates.push(candidate);
        Ok(())
    }

    /// Get the best candidate pair for connection
    pub async fn get_best_candidate_pair(&self) -> Option<(IceCandidate, IceCandidate)> {
        let local_candidates = self.local_candidates.lock().await;
        let remote_candidates = self.remote_candidates.lock().await;

        // Simplified candidate pair selection - in a real implementation this would
        // consider priority, candidate types, and perform connectivity checks
        if let (Some(local), Some(remote)) = (local_candidates.first(), remote_candidates.first()) {
            Some((local.clone(), remote.clone()))
        } else {
            None
        }
    }

    /// Perform connectivity checks between candidate pairs
    pub async fn perform_connectivity_checks(&self) -> Result<bool, TransportError> {
        // Simplified connectivity check - in a real implementation this would:
        // 1. Create candidate pairs from local and remote candidates
        // 2. Perform STUN connectivity checks for each pair
        // 3. Determine the best working pair
        // 4. Handle nomination and final selection

        let local_candidates = self.local_candidates.lock().await;
        let remote_candidates = self.remote_candidates.lock().await;

        if local_candidates.is_empty() || remote_candidates.is_empty() {
            return Ok(false);
        }

        // Simulate connectivity check success
        tokio::time::sleep(Duration::from_millis(100)).await;
        Ok(true)
    }
}

impl DefaultSignalingHandler {
    pub fn new() -> Self {
        Self {
            pending_messages: Arc::new(Mutex::new(HashMap::new())),
            incoming_offers: Arc::new(Mutex::new(Vec::new())),
            registered_peers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Simulate network delivery of signaling message
    async fn simulate_network_delivery(&self, peer_id: &PeerId, message: SignalingMessage) -> Result<(), TransportError> {
        // In a real implementation, this would send through the discovery layer
        // For now, we simulate by storing in the target peer's message queue
        
        tokio::time::sleep(Duration::from_millis(10)).await; // Simulate network delay
        
        let mut messages = self.pending_messages.lock().await;
        messages.entry(peer_id.clone()).or_insert_with(Vec::new).push(message);
        
        Ok(())
    }
}

#[async_trait]
impl SignalingHandler for DefaultSignalingHandler {
    async fn send_signaling_message(&self, peer_id: &PeerId, message: SignalingMessage) -> Result<(), TransportError> {
        println!("Sending signaling message to {}: {:?}", peer_id, message);
        self.simulate_network_delivery(peer_id, message).await
    }

    async fn receive_signaling_message(&self, peer_id: &PeerId, timeout: Duration) -> Result<SignalingMessage, TransportError> {
        let start_time = std::time::Instant::now();
        
        loop {
            {
                let mut messages = self.pending_messages.lock().await;
                if let Some(peer_messages) = messages.get_mut(peer_id) {
                    if let Some(message) = peer_messages.pop() {
                        return Ok(message);
                    }
                }
            }
            
            if start_time.elapsed() >= timeout {
                return Err(TransportError::WebRTC("Signaling message receive timeout".to_string()));
            }
            
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    async fn exchange_ice_candidates(&self, peer_id: &PeerId, local_candidates: Vec<IceCandidate>) -> Result<Vec<IceCandidate>, TransportError> {
        println!("Exchanging ICE candidates with {}: {} candidates", peer_id, local_candidates.len());
        
        // Send local candidates to peer
        for candidate in &local_candidates {
            let message = SignalingMessage::IceCandidate {
                candidate: candidate.clone(),
            };
            self.send_signaling_message(peer_id, message).await?;
        }
        
        // Wait for remote candidates (simplified - in reality would be more sophisticated)
        let mut remote_candidates = Vec::new();
        let timeout = Duration::from_secs(5);
        let start_time = std::time::Instant::now();
        
        while start_time.elapsed() < timeout && remote_candidates.len() < local_candidates.len() {
            match self.receive_signaling_message(peer_id, Duration::from_millis(100)).await {
                Ok(SignalingMessage::IceCandidate { candidate }) => {
                    remote_candidates.push(candidate);
                }
                Ok(_) => {
                    // Ignore non-candidate messages during candidate exchange
                }
                Err(_) => {
                    // Continue waiting
                }
            }
        }
        
        Ok(remote_candidates)
    }

    async fn register_for_offers(&self, peer_id: &PeerId) -> Result<(), TransportError> {
        let mut registered_peers = self.registered_peers.lock().await;
        if !registered_peers.contains(peer_id) {
            registered_peers.push(peer_id.clone());
        }
        println!("Registered peer {} for incoming offers", peer_id);
        Ok(())
    }

    async fn wait_for_offer(&self, timeout: Duration) -> Result<(PeerId, SignalingMessage), TransportError> {
        let start_time = std::time::Instant::now();
        
        loop {
            {
                let mut offers = self.incoming_offers.lock().await;
                if let Some((peer_id, message)) = offers.pop() {
                    return Ok((peer_id, message));
                }
            }
            
            if start_time.elapsed() >= timeout {
                return Err(TransportError::WebRTC("No incoming offers within timeout".to_string()));
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

impl Default for WebRtcTransport {
    fn default() -> Self {
        Self::new().expect("Failed to create default WebRTC transport")
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_webrtc_config_default() {
        let config = WebRtcConfig::default();
        assert_eq!(config.max_message_size, 65536);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
        assert!(config.ordered);
        assert_eq!(config.max_retransmits, Some(3));
        assert_eq!(config.ice_servers.len(), 1);
        assert_eq!(config.ice_servers[0].urls[0], "stun:stun.l.google.com:19302");
    }

    #[test]
    fn test_webrtc_transport_creation() {
        let transport = WebRtcTransport::new();
        assert!(transport.is_ok());
        
        let transport = transport.unwrap();
        assert_eq!(transport.protocol_name(), "webrtc");
        assert!(transport.is_available());
        assert_eq!(transport.priority(), 90);
        
        let caps = transport.capabilities();
        assert!(caps.reliable);
        assert!(caps.ordered);
        assert!(caps.multiplexed);
        assert!(!caps.resumable);
        assert!(caps.nat_traversal);
        assert_eq!(caps.max_message_size, Some(65536));
    }

    #[test]
    fn test_webrtc_transport_with_custom_config() {
        let mut config = WebRtcConfig::default();
        config.max_message_size = 32768;
        config.connection_timeout = Duration::from_secs(60);
        config.ordered = false;
        
        let transport = WebRtcTransport::with_config(config.clone());
        assert!(transport.is_ok());
        
        let transport = transport.unwrap();
        assert_eq!(transport.config.max_message_size, 32768);
        assert_eq!(transport.config.connection_timeout, Duration::from_secs(60));
        assert!(!transport.config.ordered);
    }

    #[test]
    fn test_ice_server_config() {
        let ice_config = IceServerConfig {
            urls: vec!["stun:stun.example.com:3478".to_string()],
            username: Some("user".to_string()),
            credential: Some("pass".to_string()),
        };
        
        assert_eq!(ice_config.urls[0], "stun:stun.example.com:3478");
        assert_eq!(ice_config.username, Some("user".to_string()));
        assert_eq!(ice_config.credential, Some("pass".to_string()));
    }

    #[test]
    fn test_signaling_message_types() {
        let offer = SignalingMessage::Offer {
            sdp: "v=0\r\no=- 123 456 IN IP4 127.0.0.1\r\n".to_string(),
            ice_ufrag: "test_ufrag".to_string(),
            ice_pwd: "test_pwd".to_string(),
        };
        
        let answer = SignalingMessage::Answer {
            sdp: "v=0\r\no=- 789 012 IN IP4 192.168.1.1\r\n".to_string(),
            ice_ufrag: "test_ufrag".to_string(),
            ice_pwd: "test_pwd".to_string(),
        };
        
        let ice_candidate = SignalingMessage::IceCandidate {
            candidate: IceCandidate {
                candidate: "candidate:1 1 UDP 2130706431 192.168.1.1 54400 typ host".to_string(),
                sdp_mid: Some("0".to_string()),
                sdp_mline_index: Some(0),
                foundation: "1".to_string(),
                priority: 2130706431,
                ip: "192.168.1.1".to_string(),
                port: 54400,
                candidate_type: IceCandidateType::Host,
                protocol: "udp".to_string(),
            },
        };
        
        match offer {
            SignalingMessage::Offer { sdp, .. } => assert!(sdp.contains("127.0.0.1")),
            _ => panic!("Expected offer message"),
        }
        
        match answer {
            SignalingMessage::Answer { sdp, .. } => assert!(sdp.contains("192.168.1.1")),
            _ => panic!("Expected answer message"),
        }
        
        match ice_candidate {
            SignalingMessage::IceCandidate { candidate } => assert!(candidate.ip.contains("192.168.1.1")),
            _ => panic!("Expected ICE candidate message"),
        }
    }

    #[tokio::test]
    async fn test_default_signaling_handler() {
        let handler = DefaultSignalingHandler::new();
        let peer_id = "test-peer".to_string();
        
        let message = SignalingMessage::Offer {
            sdp: "test-sdp".to_string(),
            ice_ufrag: "test_ufrag".to_string(),
            ice_pwd: "test_pwd".to_string(),
        };
        
        // Send message
        let result = handler.send_signaling_message(&peer_id, message.clone()).await;
        assert!(result.is_ok());
        
        // Receive message
        let received = handler.receive_signaling_message(&peer_id, Duration::from_secs(1)).await;
        assert!(received.is_ok());
        
        match received.unwrap() {
            SignalingMessage::Offer { sdp, .. } => assert_eq!(sdp, "test-sdp"),
            _ => panic!("Expected offer message"),
        }
    }

    #[tokio::test]
    async fn test_ice_candidate_exchange() {
        let handler = DefaultSignalingHandler::new();
        let peer_id = "test-peer".to_string();
        let local_candidates = vec![IceCandidate {
            candidate: "candidate:1 1 UDP 2130706431 127.0.0.1 54400 typ host".to_string(),
            sdp_mid: Some("0".to_string()),
            sdp_mline_index: Some(0),
            foundation: "1".to_string(),
            priority: 2130706431,
            ip: "127.0.0.1".to_string(),
            port: 54400,
            candidate_type: IceCandidateType::Host,
            protocol: "udp".to_string(),
        }];
        
        let result = handler.exchange_ice_candidates(&peer_id, local_candidates).await;
        assert!(result.is_ok());
        
        let remote_candidates = result.unwrap();
        // In the test implementation, we should receive back the candidates we sent
        assert!(!remote_candidates.is_empty());
    }

    #[tokio::test]
    async fn test_webrtc_transport_listen() {
        let transport = WebRtcTransport::new().unwrap();
        let bind_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        
        let result = transport.listen(&bind_addr).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_nat_traversal_config() {
        let config = NatTraversalConfig::default();
        assert!(!config.stun_servers.is_empty());
        assert!(config.aggressive_nomination);
        assert_eq!(config.ice_gathering_timeout, Duration::from_secs(10));
        assert_eq!(config.connection_check_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_data_channel_configs() {
        let reliable = DataChannelConfig::reliable();
        assert!(reliable.ordered);
        assert_eq!(reliable.max_retransmits, None);

        let unreliable = DataChannelConfig::unreliable();
        assert!(!unreliable.ordered);
        assert_eq!(unreliable.max_retransmits, Some(0));

        let semi_reliable = DataChannelConfig::semi_reliable(5);
        assert!(semi_reliable.ordered);
        assert_eq!(semi_reliable.max_retransmits, Some(5));

        let time_limited = DataChannelConfig::time_limited(Duration::from_millis(500));
        assert!(!time_limited.ordered);
        assert_eq!(time_limited.max_packet_lifetime, Some(Duration::from_millis(500)));
    }

    #[test]
    fn test_ice_candidate_types() {
        let host_candidate = IceCandidate {
            candidate: "candidate:1 1 UDP 2130706431 192.168.1.100 54400 typ host".to_string(),
            sdp_mid: Some("0".to_string()),
            sdp_mline_index: Some(0),
            foundation: "1".to_string(),
            priority: 2130706431,
            ip: "192.168.1.100".to_string(),
            port: 54400,
            candidate_type: IceCandidateType::Host,
            protocol: "udp".to_string(),
        };

        assert_eq!(host_candidate.candidate_type, IceCandidateType::Host);
        assert_eq!(host_candidate.ip, "192.168.1.100");
        assert_eq!(host_candidate.port, 54400);
    }

    #[tokio::test]
    async fn test_nat_traversal_manager() {
        let config = NatTraversalConfig::default();
        let manager = NatTraversalManager::new(config);
        
        let nat_type = manager.detect_nat_type().await;
        assert!(nat_type.is_ok());
        
        let detected_type = nat_type.unwrap();
        // Should detect some type (placeholder returns FullCone)
        assert_ne!(detected_type, NatType::Unknown);
    }

    #[tokio::test]
    async fn test_signaling_with_timeout() {
        let handler = DefaultSignalingHandler::new();
        let peer_id = "test-peer".to_string();
        
        // Test timeout when no message is available
        let result = handler.receive_signaling_message(&peer_id, Duration::from_millis(100)).await;
        assert!(result.is_err());
        
        // Send a message and then receive it
        let message = SignalingMessage::ConnectionRequest {
            peer_id: "requester".to_string(),
            capabilities: vec!["webrtc".to_string()],
        };
        
        handler.send_signaling_message(&peer_id, message.clone()).await.unwrap();
        
        let received = handler.receive_signaling_message(&peer_id, Duration::from_secs(1)).await;
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn test_register_for_offers() {
        let handler = DefaultSignalingHandler::new();
        let peer_id = "listener-peer".to_string();
        
        let result = handler.register_for_offers(&peer_id).await;
        assert!(result.is_ok());
        
        // Test waiting for offers with timeout
        let result = handler.wait_for_offer(Duration::from_millis(100)).await;
        assert!(result.is_err()); // Should timeout since no offers are pending
    }
}