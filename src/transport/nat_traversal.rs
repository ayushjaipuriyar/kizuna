use std::collections::HashMap;
use std::net::{SocketAddr, IpAddr, Ipv4Addr, UdpSocket};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket as TokioUdpSocket;
use tokio::sync::RwLock;
use tokio::time::{timeout, sleep};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use rand::Rng;

use crate::transport::{TransportError, PeerId, PeerAddress};

/// NAT traversal coordinator for establishing direct peer connections
#[derive(Debug)]
pub struct NatTraversal {
    /// STUN servers for NAT type detection and external address discovery
    stun_servers: Vec<SocketAddr>,
    /// Local network interface candidates
    local_candidates: Arc<RwLock<Vec<SocketAddr>>>,
    /// Discovered external addresses
    external_addresses: Arc<RwLock<Vec<SocketAddr>>>,
    /// Active hole punching sessions
    active_sessions: Arc<RwLock<HashMap<String, HolePunchSession>>>,
    /// NAT type cache
    nat_type_cache: Arc<RwLock<Option<(NatType, SystemTime)>>>,
    /// Configuration parameters
    config: NatTraversalConfig,
}

/// Configuration for NAT traversal behavior
#[derive(Debug, Clone)]
pub struct NatTraversalConfig {
    /// Timeout for STUN requests
    pub stun_timeout: Duration,
    /// Number of retry attempts for hole punching
    pub hole_punch_retries: u32,
    /// Interval between hole punch attempts
    pub hole_punch_interval: Duration,
    /// Maximum time to wait for hole punching to succeed
    pub hole_punch_timeout: Duration,
    /// Cache duration for NAT type detection
    pub nat_type_cache_duration: Duration,
    /// Port range for hole punching attempts
    pub port_range: (u16, u16),
}

impl Default for NatTraversalConfig {
    fn default() -> Self {
        Self {
            stun_timeout: Duration::from_secs(5),
            hole_punch_retries: 10,
            hole_punch_interval: Duration::from_millis(200),
            hole_punch_timeout: Duration::from_secs(30),
            nat_type_cache_duration: Duration::from_secs(300), // 5 minutes
            port_range: (49152, 65535), // Dynamic/private port range
        }
    }
}

/// Types of NAT configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NatType {
    /// No NAT - direct internet connection
    Open,
    /// Full cone NAT - allows any external host to send packets
    FullCone,
    /// Restricted cone NAT - only allows packets from previously contacted hosts
    RestrictedCone,
    /// Port-restricted cone NAT - only allows packets from specific host:port combinations
    PortRestrictedCone,
    /// Symmetric NAT - uses different mapping for each destination
    Symmetric,
    /// Unknown or detection failed
    Unknown,
}

impl NatType {
    /// Check if this NAT type supports hole punching
    pub fn supports_hole_punching(&self) -> bool {
        match self {
            NatType::Open => true,
            NatType::FullCone => true,
            NatType::RestrictedCone => true,
            NatType::PortRestrictedCone => true,
            NatType::Symmetric => false, // Difficult but not impossible
            NatType::Unknown => false,
        }
    }

    /// Get the difficulty level for hole punching
    pub fn hole_punch_difficulty(&self) -> HolePunchDifficulty {
        match self {
            NatType::Open => HolePunchDifficulty::None,
            NatType::FullCone => HolePunchDifficulty::Easy,
            NatType::RestrictedCone => HolePunchDifficulty::Medium,
            NatType::PortRestrictedCone => HolePunchDifficulty::Hard,
            NatType::Symmetric => HolePunchDifficulty::VeryHard,
            NatType::Unknown => HolePunchDifficulty::Unknown,
        }
    }
}

/// Difficulty levels for hole punching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HolePunchDifficulty {
    None,
    Easy,
    Medium,
    Hard,
    VeryHard,
    Unknown,
}

/// Active hole punching session state
#[derive(Debug, Clone)]
struct HolePunchSession {
    session_id: String,
    peer_id: PeerId,
    local_addr: SocketAddr,
    remote_addr: SocketAddr,
    started_at: SystemTime,
    attempts: u32,
    status: HolePunchStatus,
}

/// Status of a hole punching session
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HolePunchStatus {
    Initiating,
    Coordinating,
    Punching,
    Success,
    Failed,
    Timeout,
}

/// Hole punching coordination message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolePunchMessage {
    pub session_id: String,
    pub message_type: HolePunchMessageType,
    pub sender_id: PeerId,
    pub timestamp: u64,
    pub payload: HolePunchPayload,
}

/// Types of hole punching messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum HolePunchMessageType {
    InitiateRequest,
    InitiateResponse,
    CoordinationSync,
    PunchAttempt,
    PunchSuccess,
    PunchFailure,
}

/// Payload data for hole punching messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolePunchPayload {
    pub local_addresses: Vec<SocketAddr>,
    pub external_addresses: Vec<SocketAddr>,
    pub nat_type: Option<NatType>,
    pub sync_timestamp: Option<u64>,
    pub sequence_number: Option<u32>,
}

impl NatTraversal {
    /// Create a new NAT traversal coordinator
    pub fn new(stun_servers: Vec<SocketAddr>) -> Self {
        Self {
            stun_servers,
            local_candidates: Arc::new(RwLock::new(Vec::new())),
            external_addresses: Arc::new(RwLock::new(Vec::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            nat_type_cache: Arc::new(RwLock::new(None)),
            config: NatTraversalConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(stun_servers: Vec<SocketAddr>, config: NatTraversalConfig) -> Self {
        Self {
            stun_servers,
            local_candidates: Arc::new(RwLock::new(Vec::new())),
            external_addresses: Arc::new(RwLock::new(Vec::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            nat_type_cache: Arc::new(RwLock::new(None)),
            config,
        }
    }

    /// Discover local network interface addresses
    pub async fn discover_local_candidates(&self) -> Result<Vec<SocketAddr>, TransportError> {
        let mut candidates = Vec::new();
        
        // Get local network interfaces
        match local_ip_address::list_afinet_netifas() {
            Ok(interfaces) => {
                for (_, ip) in interfaces {
                    if !ip.is_loopback() && !ip.is_multicast() {
                        // Try multiple ports in the configured range
                        let mut rng = rand::thread_rng();
                        for _ in 0..5 {
                            let port = rng.gen_range(self.config.port_range.0..=self.config.port_range.1);
                            candidates.push(SocketAddr::new(ip, port));
                        }
                    }
                }
            }
            Err(e) => {
                return Err(TransportError::NatTraversalFailed {
                    method: format!("Local interface discovery failed: {}", e),
                });
            }
        }

        // Update cached candidates
        {
            let mut local_candidates = self.local_candidates.write().await;
            *local_candidates = candidates.clone();
        }

        Ok(candidates)
    }

    /// Detect NAT type using STUN protocol
    pub async fn discover_nat_type(&self) -> Result<NatType, TransportError> {
        // Check cache first
        {
            let cache = self.nat_type_cache.read().await;
            if let Some((nat_type, cached_at)) = &*cache {
                if cached_at.elapsed().unwrap_or(Duration::MAX) < self.config.nat_type_cache_duration {
                    return Ok(*nat_type);
                }
            }
        }

        let nat_type = self.perform_nat_detection().await?;

        // Update cache
        {
            let mut cache = self.nat_type_cache.write().await;
            *cache = Some((nat_type, SystemTime::now()));
        }

        Ok(nat_type)
    }

    /// Perform actual NAT type detection using STUN
    async fn perform_nat_detection(&self) -> Result<NatType, TransportError> {
        if self.stun_servers.is_empty() {
            return Ok(NatType::Unknown);
        }

        // Create a UDP socket for STUN requests
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to bind UDP socket for STUN: {}", e),
        })?;

        let local_addr = socket.local_addr().map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to get local address: {}", e),
        })?;

        // Test 1: Basic STUN request to get external address
        let external_addr1 = self.stun_request(&socket, &self.stun_servers[0]).await?;
        
        // Test 2: STUN request to different server
        let external_addr2 = if self.stun_servers.len() > 1 {
            self.stun_request(&socket, &self.stun_servers[1]).await?
        } else {
            external_addr1
        };

        // Analyze results to determine NAT type
        if local_addr.ip() == external_addr1.ip() {
            // No NAT detected
            Ok(NatType::Open)
        } else if external_addr1 == external_addr2 {
            // Same external address from different servers
            // Need more tests to distinguish between cone types
            self.determine_cone_type(&socket, external_addr1).await
        } else {
            // Different external addresses = Symmetric NAT
            Ok(NatType::Symmetric)
        }
    }

    /// Determine the specific type of cone NAT
    async fn determine_cone_type(&self, _socket: &UdpSocket, _external_addr: SocketAddr) -> Result<NatType, TransportError> {
        // For now, assume restricted cone NAT as it's most common
        // A full implementation would require more sophisticated STUN testing
        Ok(NatType::RestrictedCone)
    }

    /// Perform a STUN request to discover external address
    async fn stun_request(&self, socket: &UdpSocket, stun_server: &SocketAddr) -> Result<SocketAddr, TransportError> {
        // Simplified STUN implementation - in a real scenario you'd use a proper STUN client
        // For now, we'll simulate the external address discovery
        
        // Create a simple STUN binding request packet
        let mut request = vec![0u8; 20]; // Minimal STUN header
        request[0] = 0x00; // Message Type: Binding Request (0x0001)
        request[1] = 0x01;
        request[2] = 0x00; // Message Length: 0
        request[3] = 0x00;
        // Transaction ID (12 bytes) - using zeros for simplicity
        
        // Send STUN request
        socket.send_to(&request, stun_server).map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to send STUN request: {}", e),
        })?;

        // Receive response with timeout
        let mut buf = [0u8; 1024];
        socket.set_read_timeout(Some(self.config.stun_timeout)).map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to set socket timeout: {}", e),
        })?;

        let (len, _) = socket.recv_from(&mut buf).map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to receive STUN response: {}", e),
        })?;

        if len < 20 {
            return Err(TransportError::NatTraversalFailed {
                method: "Invalid STUN response length".to_string(),
            });
        }

        // For now, return a simulated external address
        // In a real implementation, you would parse the XOR-MAPPED-ADDRESS attribute
        let local_addr = socket.local_addr().map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to get local address: {}", e),
        })?;
        
        // Simulate external address by using a different IP but same port
        let external_ip = match stun_server.ip() {
            IpAddr::V4(_) => IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)), // TEST-NET-3
            IpAddr::V6(_) => IpAddr::V6("2001:db8::1".parse().unwrap()), // Documentation prefix
        };
        
        Ok(SocketAddr::new(external_ip, local_addr.port()))
    }

    /// Discover external addresses using STUN
    pub async fn discover_external_addresses(&self) -> Result<Vec<SocketAddr>, TransportError> {
        let mut external_addrs = Vec::new();

        for stun_server in &self.stun_servers {
            match self.discover_external_address_via_stun(stun_server).await {
                Ok(addr) => {
                    if !external_addrs.contains(&addr) {
                        external_addrs.push(addr);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to discover external address via {}: {}", stun_server, e);
                }
            }
        }

        // Update cached external addresses
        {
            let mut external_addresses = self.external_addresses.write().await;
            *external_addresses = external_addrs.clone();
        }

        Ok(external_addrs)
    }

    /// Discover external address via a specific STUN server
    async fn discover_external_address_via_stun(&self, stun_server: &SocketAddr) -> Result<SocketAddr, TransportError> {
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to bind UDP socket: {}", e),
        })?;

        self.stun_request(&socket, stun_server).await
    }

    /// Get cached local candidates
    pub async fn get_local_candidates(&self) -> Vec<SocketAddr> {
        self.local_candidates.read().await.clone()
    }

    /// Get cached external addresses
    pub async fn get_external_addresses(&self) -> Vec<SocketAddr> {
        self.external_addresses.read().await.clone()
    }

    /// Get cached NAT type
    pub async fn get_cached_nat_type(&self) -> Option<NatType> {
        let cache = self.nat_type_cache.read().await;
        cache.as_ref().map(|(nat_type, _)| *nat_type)
    }

    /// Initiate hole punching with a peer
    pub async fn initiate_hole_punch(&self, peer_info: &PeerAddress) -> Result<String, TransportError> {
        let session_id = Uuid::new_v4().to_string();
        
        // Discover our local and external addresses if not cached
        let local_candidates = if self.get_local_candidates().await.is_empty() {
            self.discover_local_candidates().await?
        } else {
            self.get_local_candidates().await
        };

        let _external_addresses = if self.get_external_addresses().await.is_empty() {
            self.discover_external_addresses().await?
        } else {
            self.get_external_addresses().await
        };

        // Detect NAT type
        let _nat_type = self.discover_nat_type().await?;

        // Create hole punch session
        let session = HolePunchSession {
            session_id: session_id.clone(),
            peer_id: peer_info.peer_id.clone(),
            local_addr: local_candidates.first().copied().unwrap_or_else(|| {
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)
            }),
            remote_addr: peer_info.addresses.first().copied().unwrap_or_else(|| {
                SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 0)
            }),
            started_at: SystemTime::now(),
            attempts: 0,
            status: HolePunchStatus::Initiating,
        };

        // Store session
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session_id.clone(), session);
        }

        Ok(session_id)
    }

    /// Create a hole punch initiation message
    pub async fn create_initiate_message(&self, session_id: &str, peer_id: &PeerId) -> Result<HolePunchMessage, TransportError> {
        let local_candidates = self.get_local_candidates().await;
        let external_addresses = self.get_external_addresses().await;
        let nat_type = self.get_cached_nat_type().await;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Ok(HolePunchMessage {
            session_id: session_id.to_string(),
            message_type: HolePunchMessageType::InitiateRequest,
            sender_id: peer_id.clone(),
            timestamp,
            payload: HolePunchPayload {
                local_addresses: local_candidates,
                external_addresses,
                nat_type,
                sync_timestamp: None,
                sequence_number: None,
            },
        })
    }

    /// Handle incoming hole punch message
    pub async fn handle_hole_punch_message(&self, message: HolePunchMessage) -> Result<Option<HolePunchMessage>, TransportError> {
        match message.message_type {
            HolePunchMessageType::InitiateRequest => {
                self.handle_initiate_request(message).await
            }
            HolePunchMessageType::InitiateResponse => {
                self.handle_initiate_response(message).await
            }
            HolePunchMessageType::CoordinationSync => {
                self.handle_coordination_sync(message).await
            }
            HolePunchMessageType::PunchAttempt => {
                self.handle_punch_attempt(message).await
            }
            HolePunchMessageType::PunchSuccess => {
                self.handle_punch_success(message).await
            }
            HolePunchMessageType::PunchFailure => {
                self.handle_punch_failure(message).await
            }
        }
    }

    /// Handle hole punch initiation request
    async fn handle_initiate_request(&self, message: HolePunchMessage) -> Result<Option<HolePunchMessage>, TransportError> {
        // Create response with our own address information
        let local_candidates = self.get_local_candidates().await;
        let external_addresses = self.get_external_addresses().await;
        let nat_type = self.get_cached_nat_type().await;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let response = HolePunchMessage {
            session_id: message.session_id.clone(),
            message_type: HolePunchMessageType::InitiateResponse,
            sender_id: "local-peer".to_string(), // This should come from peer configuration
            timestamp,
            payload: HolePunchPayload {
                local_addresses: local_candidates,
                external_addresses,
                nat_type,
                sync_timestamp: Some(timestamp + 1), // Sync time for coordination
                sequence_number: Some(0),
            },
        };

        Ok(Some(response))
    }

    /// Handle hole punch initiation response
    async fn handle_initiate_response(&self, message: HolePunchMessage) -> Result<Option<HolePunchMessage>, TransportError> {
        // Update session with peer information and start coordination
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&message.session_id) {
                session.status = HolePunchStatus::Coordinating;
            }
        }

        // Create coordination sync message
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let sync_message = HolePunchMessage {
            session_id: message.session_id.clone(),
            message_type: HolePunchMessageType::CoordinationSync,
            sender_id: "local-peer".to_string(),
            timestamp,
            payload: HolePunchPayload {
                local_addresses: vec![],
                external_addresses: vec![],
                nat_type: None,
                sync_timestamp: Some(timestamp + 2), // Synchronized punch time
                sequence_number: Some(1),
            },
        };

        Ok(Some(sync_message))
    }

    /// Handle coordination sync message
    async fn handle_coordination_sync(&self, message: HolePunchMessage) -> Result<Option<HolePunchMessage>, TransportError> {
        if let Some(sync_time) = message.payload.sync_timestamp {
            // Schedule hole punching at synchronized time
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if sync_time > current_time {
                let delay = Duration::from_secs(sync_time - current_time);
                tokio::spawn(async move {
                    sleep(delay).await;
                    // Perform actual hole punching here
                });
            }
        }

        Ok(None)
    }

    /// Handle punch attempt message
    async fn handle_punch_attempt(&self, _message: HolePunchMessage) -> Result<Option<HolePunchMessage>, TransportError> {
        // This would be called when receiving actual punch packets
        Ok(None)
    }

    /// Handle punch success message
    async fn handle_punch_success(&self, message: HolePunchMessage) -> Result<Option<HolePunchMessage>, TransportError> {
        // Update session status to success
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&message.session_id) {
                session.status = HolePunchStatus::Success;
            }
        }

        Ok(None)
    }

    /// Handle punch failure message
    async fn handle_punch_failure(&self, message: HolePunchMessage) -> Result<Option<HolePunchMessage>, TransportError> {
        // Update session status to failed
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&message.session_id) {
                session.status = HolePunchStatus::Failed;
            }
        }

        Ok(None)
    }

    /// Get session status
    pub async fn get_session_status(&self, session_id: &str) -> Option<HolePunchStatus> {
        let sessions = self.active_sessions.read().await;
        sessions.get(session_id).map(|s| s.status.clone())
    }

    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) {
        let mut sessions = self.active_sessions.write().await;
        let now = SystemTime::now();
        
        sessions.retain(|_, session| {
            now.duration_since(session.started_at).unwrap_or_default() < self.config.hole_punch_timeout
        });
    }

    /// Perform UDP hole punching to establish direct connection
    pub async fn perform_hole_punch(&self, peer_addr: &SocketAddr) -> Result<SocketAddr, TransportError> {
        let nat_type = self.discover_nat_type().await?;
        
        if !nat_type.supports_hole_punching() {
            return Err(TransportError::NatTraversalFailed {
                method: format!("NAT type {:?} does not support hole punching", nat_type),
            });
        }

        match nat_type {
            NatType::Open => self.direct_connect(peer_addr).await,
            NatType::FullCone | NatType::RestrictedCone => self.cone_nat_hole_punch(peer_addr).await,
            NatType::PortRestrictedCone => self.port_restricted_hole_punch(peer_addr).await,
            NatType::Symmetric => self.symmetric_nat_hole_punch(peer_addr).await,
            _ => Err(TransportError::NatTraversalFailed {
                method: "Unsupported NAT type for hole punching".to_string(),
            }),
        }
    }

    /// Direct connection for open NAT
    async fn direct_connect(&self, peer_addr: &SocketAddr) -> Result<SocketAddr, TransportError> {
        // For open NAT, we can connect directly
        let socket = TokioUdpSocket::bind("0.0.0.0:0").await.map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to bind socket for direct connection: {}", e),
        })?;

        // Test connectivity
        let test_message = b"HOLE_PUNCH_TEST";
        socket.send_to(test_message, peer_addr).await.map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to send test message: {}", e),
        })?;

        Ok(socket.local_addr().map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to get local address: {}", e),
        })?)
    }

    /// Hole punching for cone NAT types
    async fn cone_nat_hole_punch(&self, peer_addr: &SocketAddr) -> Result<SocketAddr, TransportError> {
        let socket = TokioUdpSocket::bind("0.0.0.0:0").await.map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to bind socket for cone NAT hole punch: {}", e),
        })?;

        let local_addr = socket.local_addr().map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to get local address: {}", e),
        })?;

        // Perform simultaneous hole punching
        let punch_result = self.simultaneous_hole_punch(&socket, peer_addr).await?;

        Ok(punch_result.unwrap_or(local_addr))
    }

    /// Hole punching for port-restricted cone NAT
    async fn port_restricted_hole_punch(&self, peer_addr: &SocketAddr) -> Result<SocketAddr, TransportError> {
        // Port-restricted NAT requires more precise timing
        let socket = TokioUdpSocket::bind("0.0.0.0:0").await.map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to bind socket for port-restricted hole punch: {}", e),
        })?;

        let local_addr = socket.local_addr().map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to get local address: {}", e),
        })?;

        // Use multiple ports and precise timing
        let punch_result = self.multi_port_hole_punch(&socket, peer_addr).await?;

        Ok(punch_result.unwrap_or(local_addr))
    }

    /// Hole punching for symmetric NAT (more complex)
    async fn symmetric_nat_hole_punch(&self, peer_addr: &SocketAddr) -> Result<SocketAddr, TransportError> {
        // Symmetric NAT is the most challenging - requires port prediction
        let socket = TokioUdpSocket::bind("0.0.0.0:0").await.map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to bind socket for symmetric NAT hole punch: {}", e),
        })?;

        let local_addr = socket.local_addr().map_err(|e| TransportError::NatTraversalFailed {
            method: format!("Failed to get local address: {}", e),
        })?;

        // Attempt port prediction and rapid fire hole punching
        let punch_result = self.port_prediction_hole_punch(&socket, peer_addr).await?;

        Ok(punch_result.unwrap_or(local_addr))
    }

    /// Perform simultaneous hole punching with retry logic
    async fn simultaneous_hole_punch(&self, socket: &TokioUdpSocket, peer_addr: &SocketAddr) -> Result<Option<SocketAddr>, TransportError> {
        let punch_message = b"HOLE_PUNCH";
        let mut attempts = 0;

        while attempts < self.config.hole_punch_retries {
            // Send punch packet
            if let Err(e) = socket.send_to(punch_message, peer_addr).await {
                eprintln!("Hole punch attempt {} failed: {}", attempts + 1, e);
            }

            // Try to receive response with timeout
            let mut buf = [0u8; 1024];
            match timeout(self.config.hole_punch_interval, socket.recv_from(&mut buf)).await {
                Ok(Ok((len, addr))) => {
                    if len >= punch_message.len() && &buf[..punch_message.len()] == punch_message {
                        // Successful hole punch
                        return Ok(Some(addr));
                    }
                }
                Ok(Err(e)) => {
                    eprintln!("Receive error during hole punch: {}", e);
                }
                Err(_) => {
                    // Timeout - continue to next attempt
                }
            }

            attempts += 1;
            
            if attempts < self.config.hole_punch_retries {
                sleep(self.config.hole_punch_interval).await;
            }
        }

        Err(TransportError::NatTraversalFailed {
            method: format!("Hole punching failed after {} attempts", attempts),
        })
    }

    /// Multi-port hole punching for port-restricted NAT
    async fn multi_port_hole_punch(&self, socket: &TokioUdpSocket, peer_addr: &SocketAddr) -> Result<Option<SocketAddr>, TransportError> {
        let punch_message = b"HOLE_PUNCH_MULTI";
        let mut attempts = 0;

        // Try multiple ports around the target port
        let base_port = peer_addr.port();
        let port_range = 10; // Try ports ±10 from the base port

        while attempts < self.config.hole_punch_retries {
            for port_offset in 0..port_range {
                // Try both positive and negative offsets
                for &sign in &[1i32, -1i32] {
                    let new_port = base_port as i32 + (sign * port_offset as i32);
                    if new_port > 0 && new_port <= 65535 {
                        let mut target_addr = *peer_addr;
                        target_addr.set_port(new_port as u16);

                        // Send punch packet to this port
                        if let Err(_e) = socket.send_to(punch_message, &target_addr).await {
                            continue; // Try next port
                        }

                        // Quick check for response
                        let mut buf = [0u8; 1024];
                        if let Ok(Ok((len, addr))) = timeout(Duration::from_millis(50), socket.recv_from(&mut buf)).await {
                            if len >= punch_message.len() && &buf[..punch_message.len()] == punch_message {
                                return Ok(Some(addr));
                            }
                        }
                    }
                }
            }

            attempts += 1;
            sleep(self.config.hole_punch_interval).await;
        }

        Err(TransportError::NatTraversalFailed {
            method: "Multi-port hole punching failed".to_string(),
        })
    }

    /// Port prediction hole punching for symmetric NAT
    async fn port_prediction_hole_punch(&self, socket: &TokioUdpSocket, peer_addr: &SocketAddr) -> Result<Option<SocketAddr>, TransportError> {
        let punch_message = b"HOLE_PUNCH_PREDICT";
        
        // For symmetric NAT, we need to predict the port mapping
        // This is a simplified implementation - real-world scenarios are more complex
        let base_port = peer_addr.port();
        let mut rng = rand::thread_rng();

        for attempt in 0..self.config.hole_punch_retries {
            // Rapid-fire approach: send to multiple predicted ports quickly
            for _i in 0..20 {
                let predicted_port = base_port.wrapping_add(rng.gen_range(0..1000));
                let mut target_addr = *peer_addr;
                target_addr.set_port(predicted_port);

                if let Err(_) = socket.send_to(punch_message, &target_addr).await {
                    continue;
                }

                // Very short timeout for each attempt
                let mut buf = [0u8; 1024];
                if let Ok(Ok((len, addr))) = timeout(Duration::from_millis(10), socket.recv_from(&mut buf)).await {
                    if len >= punch_message.len() && &buf[..punch_message.len()] == punch_message {
                        return Ok(Some(addr));
                    }
                }
            }

            if attempt < self.config.hole_punch_retries - 1 {
                sleep(Duration::from_millis(100)).await;
            }
        }

        Err(TransportError::NatTraversalFailed {
            method: "Port prediction hole punching failed".to_string(),
        })
    }

    /// Coordinate hole punching with timing synchronization
    pub async fn coordinate_traversal(&self, peer_info: &PeerAddress) -> Result<SocketAddr, TransportError> {
        // Start hole punching session
        let session_id = self.initiate_hole_punch(peer_info).await?;

        // Create coordination message
        let _init_message = self.create_initiate_message(&session_id, &peer_info.peer_id).await?;

        // In a real implementation, this message would be sent through a signaling channel
        // For now, we'll simulate the coordination and proceed directly to hole punching

        // Wait a bit for coordination (simulated)
        sleep(Duration::from_millis(500)).await;

        // Attempt hole punching to all peer addresses
        let mut last_error = TransportError::NatTraversalFailed {
            method: "No addresses to try".to_string(),
        };

        for addr in &peer_info.addresses {
            match self.perform_hole_punch(addr).await {
                Ok(result_addr) => {
                    // Mark session as successful
                    {
                        let mut sessions = self.active_sessions.write().await;
                        if let Some(session) = sessions.get_mut(&session_id) {
                            session.status = HolePunchStatus::Success;
                        }
                    }
                    return Ok(result_addr);
                }
                Err(e) => {
                    last_error = e;
                    continue;
                }
            }
        }

        // Mark session as failed
        {
            let mut sessions = self.active_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.status = HolePunchStatus::Failed;
            }
        }

        Err(last_error)
    }

    /// Handle hole punching failure and determine fallback strategy
    pub async fn handle_hole_punch_failure(&self, _peer_info: &PeerAddress, error: &TransportError) -> Result<FallbackStrategy, TransportError> {
        let nat_type = self.get_cached_nat_type().await.unwrap_or(NatType::Unknown);
        
        match nat_type {
            NatType::Symmetric | NatType::Unknown => {
                // For symmetric NAT or unknown, recommend relay
                Ok(FallbackStrategy::UseRelay)
            }
            _ => {
                // For other NAT types, we might retry with different parameters
                if error.is_recoverable() {
                    Ok(FallbackStrategy::RetryWithDifferentPorts)
                } else {
                    Ok(FallbackStrategy::UseRelay)
                }
            }
        }
    }
}

/// Fallback strategies when hole punching fails
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FallbackStrategy {
    /// Retry hole punching with different port ranges
    RetryWithDifferentPorts,
    /// Use relay server for connection
    UseRelay,
    /// Try alternative transport protocol
    UseAlternativeTransport,
    /// Give up - connection not possible
    GiveUp,
}

// Add local_ip_address as a simple implementation since it's not in dependencies
mod local_ip_address {
    use std::net::IpAddr;
    use std::io;

    pub fn list_afinet_netifas() -> io::Result<Vec<(String, IpAddr)>> {
        // Simple implementation - in a real scenario you'd use a proper network interface library
        // For now, just return localhost and a common private IP
        Ok(vec![
            ("lo".to_string(), "127.0.0.1".parse().unwrap()),
            ("eth0".to_string(), "192.168.1.100".parse().unwrap()),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, IpAddr};

    #[test]
    fn test_nat_type_hole_punch_support() {
        assert!(NatType::Open.supports_hole_punching());
        assert!(NatType::FullCone.supports_hole_punching());
        assert!(NatType::RestrictedCone.supports_hole_punching());
        assert!(NatType::PortRestrictedCone.supports_hole_punching());
        assert!(!NatType::Symmetric.supports_hole_punching());
        assert!(!NatType::Unknown.supports_hole_punching());
    }

    #[test]
    fn test_nat_type_difficulty() {
        assert_eq!(NatType::Open.hole_punch_difficulty(), HolePunchDifficulty::None);
        assert_eq!(NatType::FullCone.hole_punch_difficulty(), HolePunchDifficulty::Easy);
        assert_eq!(NatType::RestrictedCone.hole_punch_difficulty(), HolePunchDifficulty::Medium);
        assert_eq!(NatType::PortRestrictedCone.hole_punch_difficulty(), HolePunchDifficulty::Hard);
        assert_eq!(NatType::Symmetric.hole_punch_difficulty(), HolePunchDifficulty::VeryHard);
        assert_eq!(NatType::Unknown.hole_punch_difficulty(), HolePunchDifficulty::Unknown);
    }

    #[tokio::test]
    async fn test_nat_traversal_creation() {
        let stun_servers = vec![
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(74, 125, 250, 129)), 19302), // Google STUN
        ];
        
        let nat_traversal = NatTraversal::new(stun_servers.clone());
        assert_eq!(nat_traversal.stun_servers, stun_servers);
        
        let candidates = nat_traversal.get_local_candidates().await;
        assert!(candidates.is_empty()); // Should be empty initially
        
        let external_addrs = nat_traversal.get_external_addresses().await;
        assert!(external_addrs.is_empty()); // Should be empty initially
    }

    #[tokio::test]
    async fn test_discover_local_candidates() {
        let nat_traversal = NatTraversal::new(vec![]);
        
        let candidates = nat_traversal.discover_local_candidates().await.unwrap();
        assert!(!candidates.is_empty());
        
        // Check that candidates are cached
        let cached_candidates = nat_traversal.get_local_candidates().await;
        assert_eq!(candidates, cached_candidates);
    }

    #[test]
    fn test_hole_punch_message_serialization() {
        let message = HolePunchMessage {
            session_id: "test-session".to_string(),
            message_type: HolePunchMessageType::InitiateRequest,
            sender_id: "peer-123".to_string(),
            timestamp: 1234567890,
            payload: HolePunchPayload {
                local_addresses: vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080)],
                external_addresses: vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)), 8080)],
                nat_type: Some(NatType::RestrictedCone),
                sync_timestamp: Some(1234567890),
                sequence_number: Some(1),
            },
        };

        let json = serde_json::to_string(&message).expect("Failed to serialize");
        let deserialized: HolePunchMessage = serde_json::from_str(&json).expect("Failed to deserialize");
        
        assert_eq!(message.session_id, deserialized.session_id);
        assert_eq!(message.sender_id, deserialized.sender_id);
        assert_eq!(message.timestamp, deserialized.timestamp);
    }

    #[test]
    fn test_nat_traversal_config_defaults() {
        let config = NatTraversalConfig::default();
        
        assert_eq!(config.stun_timeout, Duration::from_secs(5));
        assert_eq!(config.hole_punch_retries, 10);
        assert_eq!(config.hole_punch_interval, Duration::from_millis(200));
        assert_eq!(config.hole_punch_timeout, Duration::from_secs(30));
        assert_eq!(config.nat_type_cache_duration, Duration::from_secs(300));
        assert_eq!(config.port_range, (49152, 65535));
    }

    #[tokio::test]
    async fn test_hole_punch_session_creation() {
        let nat_traversal = NatTraversal::new(vec![]);
        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080)],
            vec!["udp".to_string()],
            crate::transport::TransportCapabilities::default(),
        );

        let session_id = nat_traversal.initiate_hole_punch(&peer_addr).await.unwrap();
        assert!(!session_id.is_empty());

        let status = nat_traversal.get_session_status(&session_id).await;
        assert_eq!(status, Some(HolePunchStatus::Initiating));
    }

    #[tokio::test]
    async fn test_hole_punch_message_handling() {
        let nat_traversal = NatTraversal::new(vec![]);
        
        let init_message = HolePunchMessage {
            session_id: "test-session".to_string(),
            message_type: HolePunchMessageType::InitiateRequest,
            sender_id: "peer-123".to_string(),
            timestamp: 1234567890,
            payload: HolePunchPayload {
                local_addresses: vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080)],
                external_addresses: vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)), 8080)],
                nat_type: Some(NatType::RestrictedCone),
                sync_timestamp: None,
                sequence_number: None,
            },
        };

        let response = nat_traversal.handle_hole_punch_message(init_message).await.unwrap();
        assert!(response.is_some());
        
        if let Some(resp) = response {
            assert_eq!(resp.message_type, HolePunchMessageType::InitiateResponse);
            assert_eq!(resp.session_id, "test-session");
        }
    }

    #[tokio::test]
    async fn test_session_cleanup() {
        let mut config = NatTraversalConfig::default();
        config.hole_punch_timeout = Duration::from_millis(100); // Very short timeout for testing
        
        let nat_traversal = NatTraversal::with_config(vec![], config);
        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080)],
            vec!["udp".to_string()],
            crate::transport::TransportCapabilities::default(),
        );

        let session_id = nat_traversal.initiate_hole_punch(&peer_addr).await.unwrap();
        
        // Wait for session to expire
        sleep(Duration::from_millis(150)).await;
        
        // Clean up expired sessions
        nat_traversal.cleanup_expired_sessions().await;
        
        // Session should be gone
        let status = nat_traversal.get_session_status(&session_id).await;
        assert_eq!(status, None);
    }

    #[test]
    fn test_fallback_strategy() {
        assert_eq!(FallbackStrategy::UseRelay, FallbackStrategy::UseRelay);
        assert_ne!(FallbackStrategy::UseRelay, FallbackStrategy::RetryWithDifferentPorts);
    }
}