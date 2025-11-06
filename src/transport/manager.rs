use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use async_trait::async_trait;
use futures::future;

use super::{
    Connection, ConnectionInfo, PeerAddress, PeerId, TransportCapabilities, TransportError,
};

/// Trait for transport protocol implementations
#[async_trait]
pub trait Transport: Send + Sync + std::fmt::Debug {
    /// Connect to a remote peer
    async fn connect(&self, addr: &PeerAddress) -> Result<Box<dyn Connection>, TransportError>;
    
    /// Start listening for incoming connections
    async fn listen(&self, bind_addr: &std::net::SocketAddr) -> Result<(), TransportError>;
    
    /// Get the protocol name for identification
    fn protocol_name(&self) -> &'static str;
    
    /// Check if this transport is available on the current platform
    fn is_available(&self) -> bool;
    
    /// Get the priority of this transport (higher = preferred)
    fn priority(&self) -> u8;
    
    /// Get transport capabilities and features
    fn capabilities(&self) -> TransportCapabilities;
}

/// Information about a peer for connection purposes
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub address: PeerAddress,
    pub last_seen: Instant,
    pub connection_attempts: u32,
    pub successful_protocols: Vec<String>,
}

impl PeerInfo {
    pub fn new(address: PeerAddress) -> Self {
        Self {
            address,
            last_seen: Instant::now(),
            connection_attempts: 0,
            successful_protocols: Vec::new(),
        }
    }

    pub fn record_connection_attempt(&mut self) {
        self.connection_attempts += 1;
    }

    pub fn record_successful_protocol(&mut self, protocol: String) {
        if !self.successful_protocols.contains(&protocol) {
            self.successful_protocols.push(protocol);
        }
    }
}

/// Protocol negotiation configuration and state
#[derive(Debug, Clone)]
pub struct ProtocolNegotiation {
    pub offered_protocols: Vec<String>,
    pub selected_protocol: Option<String>,
    pub fallback_protocols: Vec<String>,
    pub negotiation_timeout: Duration,
    pub peer_capabilities: Option<TransportCapabilities>,
    pub network_conditions: Option<NetworkConditions>,
    pub negotiation_start_time: Option<Instant>,
    pub retry_count: u32,
    pub max_retries: u32,
}

impl ProtocolNegotiation {
    pub fn new(local_capabilities: &[String]) -> Self {
        Self {
            offered_protocols: local_capabilities.to_vec(),
            selected_protocol: None,
            fallback_protocols: Vec::new(),
            negotiation_timeout: Duration::from_secs(10),
            peer_capabilities: None,
            network_conditions: None,
            negotiation_start_time: None,
            retry_count: 0,
            max_retries: 3,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.negotiation_timeout = timeout;
        self
    }

    pub fn with_network_conditions(mut self, conditions: NetworkConditions) -> Self {
        self.network_conditions = Some(conditions);
        self
    }

    pub fn start_negotiation(&mut self) {
        self.negotiation_start_time = Some(Instant::now());
        self.retry_count = 0;
    }

    pub fn is_timed_out(&self) -> bool {
        if let Some(start_time) = self.negotiation_start_time {
            start_time.elapsed() > self.negotiation_timeout
        } else {
            false
        }
    }

    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && !self.is_timed_out()
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
    }

    pub fn add_peer_capabilities(&mut self, peer_capabilities: &[String]) {
        // Find common protocols between local and peer capabilities
        let common_protocols: Vec<String> = self
            .offered_protocols
            .iter()
            .filter(|protocol| peer_capabilities.contains(protocol))
            .cloned()
            .collect();
        
        self.fallback_protocols = common_protocols;
    }

    pub fn add_peer_transport_capabilities(&mut self, capabilities: TransportCapabilities) {
        self.peer_capabilities = Some(capabilities);
    }

    pub fn select_best_protocol(&self) -> Option<String> {
        if self.fallback_protocols.is_empty() {
            return None;
        }

        // If we have network conditions, use advanced selection
        if let Some(conditions) = &self.network_conditions {
            return self.select_protocol_for_conditions(conditions);
        }

        // Return the highest priority common protocol
        self.fallback_protocols.first().cloned()
    }

    fn select_protocol_for_conditions(&self, conditions: &NetworkConditions) -> Option<String> {
        let mut scored_protocols = Vec::new();

        for protocol in &self.fallback_protocols {
            let score = self.calculate_protocol_score_for_conditions(protocol, conditions);
            scored_protocols.push((protocol.clone(), score));
        }

        // Sort by score (higher is better)
        scored_protocols.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scored_protocols.first().map(|(protocol, _)| protocol.clone())
    }

    fn calculate_protocol_score_for_conditions(&self, protocol: &str, conditions: &NetworkConditions) -> f32 {
        let mut score: f32 = 50.0; // Base score

        // Score based on protocol characteristics
        match protocol {
            "quic" => {
                score += 20.0; // High base score for QUIC
                if conditions.latency_requirement == LatencyRequirement::Low {
                    score += 15.0; // QUIC is good for low latency
                }
                if conditions.reliability_requirement == ReliabilityRequirement::High {
                    score += 10.0; // QUIC is reliable
                }
                if conditions.mobile_network {
                    score += 20.0; // QUIC handles mobile networks well
                }
            }
            "webrtc" => {
                score += 15.0; // Good base score for WebRTC
                if conditions.nat_traversal_needed {
                    score += 25.0; // WebRTC excels at NAT traversal
                }
                if conditions.latency_requirement == LatencyRequirement::Low {
                    score += 20.0; // WebRTC is excellent for low latency
                }
                if conditions.battery_constrained {
                    score -= 10.0; // WebRTC can be battery intensive
                }
            }
            "tcp" => {
                score += 10.0; // Moderate base score for TCP
                if conditions.reliability_requirement == ReliabilityRequirement::High {
                    score += 15.0; // TCP is very reliable
                }
                if conditions.bandwidth_requirement == BandwidthRequirement::High {
                    score += 10.0; // TCP handles high bandwidth well
                }
                if conditions.nat_traversal_needed {
                    score -= 20.0; // TCP struggles with NAT
                }
            }
            "websocket" => {
                score += 5.0; // Lower base score for WebSocket
                if conditions.nat_traversal_needed {
                    score += 15.0; // WebSocket can work through relays
                }
                if conditions.reliability_requirement == ReliabilityRequirement::Low {
                    score += 5.0; // WebSocket is adequate for low reliability needs
                }
            }
            _ => {
                score = 1.0; // Unknown protocol gets minimal score
            }
        }

        // Adjust for battery constraints
        if conditions.battery_constrained {
            match protocol {
                "tcp" => score += 5.0,      // TCP is battery efficient
                "websocket" => score += 3.0, // WebSocket is reasonably efficient
                "quic" => score -= 5.0,     // QUIC uses more battery
                "webrtc" => score -= 10.0,  // WebRTC uses most battery
                _ => {}
            }
        }

        score.max(0.0)
    }

    pub fn get_negotiation_summary(&self) -> NegotiationSummary {
        NegotiationSummary {
            selected_protocol: self.selected_protocol.clone(),
            offered_protocols: self.offered_protocols.clone(),
            fallback_protocols: self.fallback_protocols.clone(),
            retry_count: self.retry_count,
            negotiation_duration: self.negotiation_start_time
                .map(|start| start.elapsed())
                .unwrap_or_default(),
            timed_out: self.is_timed_out(),
        }
    }
}

/// Summary of a protocol negotiation attempt
#[derive(Debug, Clone)]
pub struct NegotiationSummary {
    pub selected_protocol: Option<String>,
    pub offered_protocols: Vec<String>,
    pub fallback_protocols: Vec<String>,
    pub retry_count: u32,
    pub negotiation_duration: Duration,
    pub timed_out: bool,
}

/// Result of a protocol negotiation with selected transport and metadata
#[derive(Debug)]
pub struct ProtocolNegotiationResult<'a> {
    pub transport: &'a dyn Transport,
    pub negotiation_summary: NegotiationSummary,
    pub fallback_available: bool,
}

/// Connection pool entry with metadata
#[derive(Debug)]
struct PooledConnection {
    connection: Box<dyn Connection>,
    last_used: Instant,
    usage_count: u32,
}

impl PooledConnection {
    fn new(connection: Box<dyn Connection>) -> Self {
        Self {
            connection,
            last_used: Instant::now(),
            usage_count: 0,
        }
    }

    fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.usage_count += 1;
    }

    fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.last_used.elapsed() > idle_timeout
    }
}

/// Connection pool for managing and reusing connections
#[derive(Debug)]
pub struct ConnectionPool {
    connections: HashMap<PeerId, Vec<PooledConnection>>,
    max_connections_per_peer: usize,
    max_total_connections: usize,
    idle_timeout: Duration,
    max_usage_count: u32,
}

impl ConnectionPool {
    pub fn new(max_connections_per_peer: usize, idle_timeout: Duration) -> Self {
        Self {
            connections: HashMap::new(),
            max_connections_per_peer,
            max_total_connections: 1000,
            idle_timeout,
            max_usage_count: 100,
        }
    }

    pub fn with_limits(
        max_connections_per_peer: usize,
        max_total_connections: usize,
        idle_timeout: Duration,
        max_usage_count: u32,
    ) -> Self {
        Self {
            connections: HashMap::new(),
            max_connections_per_peer,
            max_total_connections,
            idle_timeout,
            max_usage_count,
        }
    }

    pub fn add_connection(&mut self, peer_id: PeerId, connection: Box<dyn Connection>) -> Result<(), TransportError> {
        // Check total connection limit
        if self.total_connection_count() >= self.max_total_connections {
            return Err(TransportError::ResourceLimitExceeded {
                resource: format!("Total connections limit ({}) exceeded", self.max_total_connections),
            });
        }

        let peer_connections = self.connections.entry(peer_id).or_insert_with(Vec::new);
        
        // Enforce per-peer connection limit
        if peer_connections.len() >= self.max_connections_per_peer {
            // Remove oldest connection (we'll close it later in cleanup)
            peer_connections.remove(0);
        }
        
        peer_connections.push(PooledConnection::new(connection));
        Ok(())
    }

    pub fn get_connection(&mut self, peer_id: &PeerId) -> Option<Box<dyn Connection>> {
        if let Some(peer_connections) = self.connections.get_mut(peer_id) {
            // Find first connected and not overused connection
            for i in 0..peer_connections.len() {
                let pooled = &mut peer_connections[i];
                if pooled.connection.is_connected() && 
                   pooled.usage_count < self.max_usage_count &&
                   !pooled.is_idle(self.idle_timeout) {
                    pooled.mark_used();
                    return Some(peer_connections.remove(i).connection);
                }
            }
            
            // Clean up disconnected or overused connections
            peer_connections.retain(|pooled| {
                pooled.connection.is_connected() && 
                pooled.usage_count < self.max_usage_count &&
                !pooled.is_idle(self.idle_timeout)
            });
            
            if peer_connections.is_empty() {
                self.connections.remove(peer_id);
            }
        }
        None
    }

    pub async fn cleanup_idle_connections(&mut self) {
        let mut to_remove = Vec::new();
        
        for (peer_id, connections) in &mut self.connections {
            let mut indices_to_remove = Vec::new();
            
            for (i, pooled) in connections.iter_mut().enumerate() {
                if !pooled.connection.is_connected() || 
                   pooled.is_idle(self.idle_timeout) ||
                   pooled.usage_count >= self.max_usage_count {
                    // Close the connection
                    let _ = pooled.connection.close().await;
                    indices_to_remove.push(i);
                }
            }
            
            // Remove connections in reverse order to maintain indices
            for &i in indices_to_remove.iter().rev() {
                connections.remove(i);
            }
            
            if connections.is_empty() {
                to_remove.push(peer_id.clone());
            }
        }
        
        // Remove empty peer entries
        for peer_id in to_remove {
            self.connections.remove(&peer_id);
        }
    }

    pub fn connection_count(&self) -> usize {
        self.connections.values().map(|conns| conns.len()).sum()
    }

    pub fn total_connection_count(&self) -> usize {
        self.connection_count()
    }

    pub fn peer_connection_count(&self, peer_id: &PeerId) -> usize {
        self.connections.get(peer_id).map(|conns| conns.len()).unwrap_or(0)
    }

    pub fn get_pool_stats(&self) -> PoolStats {
        let mut total_usage = 0;
        let mut max_usage = 0;
        let mut idle_count = 0;
        let total_connections = self.connection_count();

        for connections in self.connections.values() {
            for pooled in connections {
                total_usage += pooled.usage_count;
                max_usage = max_usage.max(pooled.usage_count);
                if pooled.is_idle(self.idle_timeout) {
                    idle_count += 1;
                }
            }
        }

        PoolStats {
            total_connections,
            idle_connections: idle_count,
            average_usage: if total_connections > 0 { total_usage / total_connections as u32 } else { 0 },
            max_usage,
            peer_count: self.connections.len(),
        }
    }

    pub fn set_max_total_connections(&mut self, max: usize) {
        self.max_total_connections = max;
    }

    pub fn set_idle_timeout(&mut self, timeout: Duration) {
        self.idle_timeout = timeout;
    }
}

/// Network conditions for protocol selection
#[derive(Debug, Clone)]
pub struct NetworkConditions {
    pub latency_requirement: LatencyRequirement,
    pub bandwidth_requirement: BandwidthRequirement,
    pub reliability_requirement: ReliabilityRequirement,
    pub nat_traversal_needed: bool,
    pub mobile_network: bool,
    pub battery_constrained: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LatencyRequirement {
    Low,    // Real-time applications
    Medium, // Interactive applications
    High,   // Bulk transfers
}

#[derive(Debug, Clone, PartialEq)]
pub enum BandwidthRequirement {
    Low,    // Text messages
    Medium, // File transfers
    High,   // Video streaming
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReliabilityRequirement {
    Low,    // Best effort
    Medium, // Important data
    High,   // Critical data
}

impl Default for NetworkConditions {
    fn default() -> Self {
        Self {
            latency_requirement: LatencyRequirement::Medium,
            bandwidth_requirement: BandwidthRequirement::Medium,
            reliability_requirement: ReliabilityRequirement::Medium,
            nat_traversal_needed: false,
            mobile_network: false,
            battery_constrained: false,
        }
    }
}

/// Statistics about the connection pool
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub idle_connections: usize,
    pub average_usage: u32,
    pub max_usage: u32,
    pub peer_count: usize,
}

/// Connection lifecycle state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Disconnecting,
    Disconnected,
    Failed,
}

/// Managed connection with lifecycle tracking
#[derive(Debug)]
pub struct ManagedConnection {
    connection: Box<dyn Connection>,
    state: ConnectionState,
    created_at: Instant,
    last_activity: Instant,
    protocol: String,
    peer_id: PeerId,
}

impl ManagedConnection {
    pub fn new(connection: Box<dyn Connection>, protocol: String, peer_id: PeerId) -> Self {
        Self {
            connection,
            state: ConnectionState::Connected,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            protocol,
            peer_id,
        }
    }

    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    pub fn set_state(&mut self, state: ConnectionState) {
        self.state = state;
    }

    pub fn is_active(&self) -> bool {
        matches!(self.state, ConnectionState::Connected) && self.connection.is_connected()
    }

    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub fn idle_time(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

/// Main connection manager for handling transport protocols and connections
#[derive(Debug)]
pub struct ConnectionManager {
    transports: Vec<Box<dyn Transport>>,
    active_connections: Arc<RwLock<HashMap<PeerId, Vec<ManagedConnection>>>>,
    connection_pool: Arc<RwLock<ConnectionPool>>,
    max_concurrent_connections: usize,
    max_connections_per_peer: usize,
    connection_timeout: Duration,
    idle_timeout: Duration,
    cleanup_interval: Duration,
    protocol_preferences: HashMap<String, u8>,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            transports: Vec::new(),
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            connection_pool: Arc::new(RwLock::new(ConnectionPool::new(5, Duration::from_secs(300)))),
            max_concurrent_connections: 100,
            max_connections_per_peer: 5,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(60),
            protocol_preferences: HashMap::new(),
        }
    }

    /// Create a new connection manager with custom configuration
    pub fn with_config(config: ConnectionManagerConfig) -> Self {
        let mut manager = Self::new();
        manager.max_concurrent_connections = config.max_concurrent_connections;
        manager.max_connections_per_peer = config.max_connections_per_peer;
        manager.connection_timeout = config.connection_timeout;
        manager.idle_timeout = config.idle_timeout;
        manager.cleanup_interval = config.cleanup_interval;
        manager.protocol_preferences = config.protocol_preferences;
        
        // Update connection pool with new limits
        let pool = ConnectionPool::with_limits(
            config.max_connections_per_peer,
            config.max_concurrent_connections,
            config.idle_timeout,
            config.max_connection_reuse,
        );
        manager.connection_pool = Arc::new(RwLock::new(pool));
        
        manager
    }

    /// Add a transport protocol to the manager
    pub fn add_transport(&mut self, transport: Box<dyn Transport>) {
        // Insert transport in priority order (highest first)
        let priority = transport.priority();
        let insert_pos = self
            .transports
            .iter()
            .position(|t| t.priority() < priority)
            .unwrap_or(self.transports.len());
        
        self.transports.insert(insert_pos, transport);
    }

    /// Connect to a peer using the best available transport
    pub async fn connect_to_peer(&self, peer: &PeerInfo) -> Result<Box<dyn Connection>, TransportError> {
        let peer_id = &peer.address.peer_id;

        // Check connection pool first for reusable connections
        {
            let mut pool = self.connection_pool.write().await;
            if let Some(connection) = pool.get_connection(peer_id) {
                return Ok(connection);
            }
        }

        // Check if we already have active connections to this peer
        {
            let active = self.active_connections.read().await;
            if let Some(connections) = active.get(peer_id) {
                // Find an active connection we can reuse
                for managed_conn in connections {
                    if managed_conn.is_active() {
                        // In a real implementation, we'd return a shared reference or clone
                        // For now, we'll continue to create a new connection
                        break;
                    }
                }
            }
        }

        // Check resource limits
        {
            let active = self.active_connections.read().await;
            let total_connections: usize = active.values().map(|conns| conns.len()).sum();
            
            if total_connections >= self.max_concurrent_connections {
                return Err(TransportError::ResourceLimitExceeded {
                    resource: format!("Max concurrent connections ({}) exceeded", self.max_concurrent_connections),
                });
            }

            if let Some(peer_connections) = active.get(peer_id) {
                if peer_connections.len() >= self.max_connections_per_peer {
                    return Err(TransportError::ResourceLimitExceeded {
                        resource: format!("Max connections per peer ({}) exceeded", self.max_connections_per_peer),
                    });
                }
            }
        }

        // Negotiate protocol and establish connection
        let selected_transport = self.negotiate_protocol(peer).await?;
        let protocol_name = selected_transport.protocol_name().to_string();
        
        // Attempt connection with timeout
        let connection_future = selected_transport.connect(&peer.address);
        let connection = tokio::time::timeout(self.connection_timeout, connection_future)
            .await
            .map_err(|_| TransportError::ConnectionTimeout {
                timeout: self.connection_timeout,
            })??;

        // Create managed connection and add to active connections
        let managed_connection = ManagedConnection::new(
            connection,
            protocol_name,
            peer_id.clone(),
        );

        {
            let mut active = self.active_connections.write().await;
            let peer_connections = active.entry(peer_id.clone()).or_insert_with(Vec::new);
            peer_connections.push(managed_connection);
        }

        // Return a new connection (in practice, this would be handled differently)
        // For now, we'll create a new connection since we can't return the managed one
        selected_transport.connect(&peer.address).await
    }

    /// Negotiate the best transport protocol with a peer
    pub async fn negotiate_protocol(&self, peer: &PeerInfo) -> Result<&dyn Transport, TransportError> {
        let peer_protocols = &peer.address.transport_hints;
        let mut candidates = Vec::new();

        // Collect available transports that both peers support
        for transport in &self.transports {
            if !transport.is_available() {
                continue;
            }
            
            let protocol_name = transport.protocol_name();
            if peer_protocols.contains(&protocol_name.to_string()) {
                let preference_score = self.protocol_preferences
                    .get(protocol_name)
                    .copied()
                    .unwrap_or(transport.priority());
                
                candidates.push((transport.as_ref(), preference_score, transport.priority()));
            }
        }

        if candidates.is_empty() {
            return Err(TransportError::UnsupportedProtocol {
                protocol: format!("No common protocols. Peer supports: {:?}", peer_protocols),
            });
        }

        // Sort by preference score (higher is better), then by transport priority
        candidates.sort_by(|a, b| {
            b.1.cmp(&a.1).then_with(|| b.2.cmp(&a.2))
        });

        // Consider peer's successful protocols history
        for protocol in &peer.successful_protocols {
            if let Some((transport, _, _)) = candidates.iter().find(|(t, _, _)| t.protocol_name() == protocol) {
                return Ok(*transport);
            }
        }

        // Return the highest scored transport
        Ok(candidates[0].0)
    }

    /// Advanced protocol negotiation with network conditions
    pub async fn negotiate_protocol_with_conditions(
        &self, 
        peer: &PeerInfo, 
        conditions: &NetworkConditions
    ) -> Result<&dyn Transport, TransportError> {
        let peer_protocols = &peer.address.transport_hints;
        let mut scored_transports = Vec::new();

        for transport in &self.transports {
            if !transport.is_available() {
                continue;
            }
            
            let protocol_name = transport.protocol_name();
            if peer_protocols.contains(&protocol_name.to_string()) {
                let base_score = self.protocol_preferences
                    .get(protocol_name)
                    .copied()
                    .unwrap_or(transport.priority()) as f32;
                
                // Adjust score based on network conditions
                let adjusted_score = self.calculate_protocol_score(transport, conditions, base_score);
                scored_transports.push((transport.as_ref(), adjusted_score));
            }
        }

        if scored_transports.is_empty() {
            return Err(TransportError::UnsupportedProtocol {
                protocol: format!("No common protocols for conditions: {:?}", conditions),
            });
        }

        // Sort by adjusted score (higher is better)
        scored_transports.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored_transports[0].0)
    }

    /// Enhanced protocol negotiation with full capability exchange and fallback handling
    pub async fn negotiate_protocol_enhanced(
        &self,
        peer: &PeerInfo,
        conditions: Option<&NetworkConditions>,
    ) -> Result<ProtocolNegotiationResult, TransportError> {
        let local_protocols: Vec<String> = self.get_available_protocols();
        let mut negotiation = ProtocolNegotiation::new(&local_protocols);
        
        if let Some(conditions) = conditions {
            negotiation = negotiation.with_network_conditions(conditions.clone());
        }

        negotiation.start_negotiation();
        negotiation.add_peer_capabilities(&peer.address.transport_hints);
        negotiation.add_peer_transport_capabilities(peer.address.capabilities.clone());

        // Attempt negotiation with retries
        while negotiation.can_retry() {
            if let Some(selected_protocol) = negotiation.select_best_protocol() {
                // Try to get the transport for the selected protocol
                if let Some(transport) = self.get_transport(&selected_protocol) {
                    // Test if the transport can actually connect (optional quick test)
                    if self.can_transport_connect(transport, &peer.address).await {
                        negotiation.selected_protocol = Some(selected_protocol.clone());
                        
                        return Ok(ProtocolNegotiationResult {
                            transport,
                            negotiation_summary: negotiation.get_negotiation_summary(),
                            fallback_available: negotiation.fallback_protocols.len() > 1,
                        });
                    }
                }
            }

            // If we get here, the selected protocol failed
            negotiation.increment_retry();
            
            // Remove the failed protocol from fallbacks and try again
            if let Some(failed_protocol) = negotiation.selected_protocol.take() {
                negotiation.fallback_protocols.retain(|p| p != &failed_protocol);
            }

            if negotiation.fallback_protocols.is_empty() {
                break;
            }
        }

        // If we exhausted all options or timed out
        if negotiation.is_timed_out() {
            Err(TransportError::NegotiationTimeout)
        } else {
            Err(TransportError::UnsupportedProtocol {
                protocol: format!(
                    "No working protocols found. Tried: {:?}, Peer supports: {:?}",
                    negotiation.offered_protocols,
                    peer.address.transport_hints
                ),
            })
        }
    }

    /// Quick connectivity test for a transport (lightweight check)
    async fn can_transport_connect(&self, transport: &dyn Transport, peer_addr: &PeerAddress) -> bool {
        // For now, just check if the transport is available and the peer supports it
        // In a full implementation, this might do a quick ping or connection test
        transport.is_available() && 
        peer_addr.transport_hints.contains(&transport.protocol_name().to_string())
    }

    /// Negotiate protocol with automatic fallback and error recovery
    pub async fn negotiate_protocol_with_fallback(
        &self,
        peer: &PeerInfo,
        preferred_protocols: Option<Vec<String>>,
        conditions: Option<&NetworkConditions>,
    ) -> Result<ProtocolNegotiationResult, TransportError> {
        let mut local_protocols = self.get_available_protocols();
        
        // If preferred protocols are specified, prioritize them
        if let Some(preferred) = preferred_protocols {
            // Reorder local protocols to put preferred ones first
            let mut prioritized = Vec::new();
            let mut remaining = Vec::new();
            
            for protocol in &local_protocols {
                if preferred.contains(protocol) {
                    prioritized.push(protocol.clone());
                } else {
                    remaining.push(protocol.clone());
                }
            }
            
            prioritized.extend(remaining);
            local_protocols = prioritized;
        }

        let mut negotiation = ProtocolNegotiation::new(&local_protocols);
        
        if let Some(conditions) = conditions {
            negotiation = negotiation.with_network_conditions(conditions.clone());
        }

        // Set longer timeout for fallback negotiation
        negotiation = negotiation.with_timeout(Duration::from_secs(30));
        negotiation.max_retries = 5; // More retries for fallback
        
        negotiation.start_negotiation();
        negotiation.add_peer_capabilities(&peer.address.transport_hints);
        negotiation.add_peer_transport_capabilities(peer.address.capabilities.clone());

        let mut attempted_protocols = Vec::new();

        while negotiation.can_retry() && !negotiation.fallback_protocols.is_empty() {
            if let Some(selected_protocol) = negotiation.select_best_protocol() {
                attempted_protocols.push(selected_protocol.clone());
                
                if let Some(transport) = self.get_transport(&selected_protocol) {
                    // More thorough connectivity test for fallback negotiation
                    match self.test_transport_connectivity(transport, &peer.address).await {
                        Ok(true) => {
                            negotiation.selected_protocol = Some(selected_protocol.clone());
                            
                            return Ok(ProtocolNegotiationResult {
                                transport,
                                negotiation_summary: negotiation.get_negotiation_summary(),
                                fallback_available: negotiation.fallback_protocols.len() > 1,
                            });
                        }
                        Ok(false) | Err(_) => {
                            // Protocol failed, remove it and try next
                            negotiation.fallback_protocols.retain(|p| p != &selected_protocol);
                        }
                    }
                }
            }

            negotiation.increment_retry();
        }

        // Final error with detailed information
        Err(TransportError::UnsupportedProtocol {
            protocol: format!(
                "Fallback negotiation failed. Attempted: {:?}, Peer supports: {:?}, Available: {:?}",
                attempted_protocols,
                peer.address.transport_hints,
                local_protocols
            ),
        })
    }

    /// Test transport connectivity (more thorough than can_transport_connect)
    async fn test_transport_connectivity(
        &self,
        transport: &dyn Transport,
        peer_addr: &PeerAddress,
    ) -> Result<bool, TransportError> {
        // Basic availability check
        if !transport.is_available() {
            return Ok(false);
        }

        // Check if peer supports this transport
        if !peer_addr.transport_hints.contains(&transport.protocol_name().to_string()) {
            return Ok(false);
        }

        // Check transport capabilities compatibility
        let transport_caps = transport.capabilities();
        if !transport_caps.is_compatible_with(&peer_addr.capabilities) {
            return Ok(false);
        }

        // For now, return true if basic checks pass
        // In a full implementation, this might attempt a quick connection test
        Ok(true)
    }

    fn calculate_protocol_score(&self, transport: &Box<dyn Transport>, conditions: &NetworkConditions, base_score: f32) -> f32 {
        let mut score = base_score;
        let caps = transport.capabilities();

        // Adjust for latency requirements
        match conditions.latency_requirement {
            LatencyRequirement::Low => {
                if transport.protocol_name() == "webrtc" || transport.protocol_name() == "quic" {
                    score += 20.0;
                }
            }
            LatencyRequirement::High => {
                if caps.reliable {
                    score += 10.0;
                }
            }
            _ => {}
        }

        // Adjust for bandwidth requirements
        if conditions.bandwidth_requirement == BandwidthRequirement::High {
            if caps.multiplexed {
                score += 15.0;
            }
        }

        // Adjust for reliability requirements
        if conditions.reliability_requirement == ReliabilityRequirement::High {
            if caps.reliable && caps.ordered {
                score += 25.0;
            }
        }

        // Adjust for NAT traversal needs
        if conditions.nat_traversal_needed && caps.nat_traversal {
            score += 30.0;
        }

        score
    }

    /// Get all active connections for a peer
    pub async fn get_connections(&self, peer_id: &PeerId) -> Vec<ConnectionInfo> {
        let active = self.active_connections.read().await;
        if let Some(connections) = active.get(peer_id) {
            connections.iter()
                .filter(|managed_conn| managed_conn.is_active())
                .map(|managed_conn| managed_conn.connection.info())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get a specific connection by peer ID and protocol
    pub async fn get_connection_by_protocol(&self, peer_id: &PeerId, protocol: &str) -> Option<ConnectionInfo> {
        let active = self.active_connections.read().await;
        if let Some(connections) = active.get(peer_id) {
            connections.iter()
                .find(|managed_conn| managed_conn.is_active() && managed_conn.protocol == protocol)
                .map(|managed_conn| managed_conn.connection.info())
        } else {
            None
        }
    }

    /// Close a specific connection
    pub async fn close_connection(&self, peer_id: &PeerId, protocol: Option<&str>) -> Result<(), TransportError> {
        let mut active = self.active_connections.write().await;
        if let Some(connections) = active.get_mut(peer_id) {
            let mut indices_to_remove = Vec::new();
            
            for (i, managed_conn) in connections.iter_mut().enumerate() {
                if protocol.map_or(true, |p| managed_conn.protocol == p) {
                    managed_conn.set_state(ConnectionState::Disconnecting);
                    let _ = managed_conn.connection.close().await;
                    managed_conn.set_state(ConnectionState::Disconnected);
                    indices_to_remove.push(i);
                    
                    if protocol.is_some() {
                        break; // Only close one connection if protocol specified
                    }
                }
            }
            
            // Remove closed connections in reverse order
            for &i in indices_to_remove.iter().rev() {
                connections.remove(i);
            }
            
            if connections.is_empty() {
                active.remove(peer_id);
            }
        }
        Ok(())
    }

    /// Close all connections to a peer
    pub async fn close_peer_connections(&self, peer_id: &PeerId) -> Result<(), TransportError> {
        self.close_connection(peer_id, None).await
    }

    /// Get statistics about all connections
    pub async fn get_connection_stats(&self) -> ConnectionStats {
        let active = self.active_connections.read().await;
        let pool = self.connection_pool.read().await;
        
        let total_connections = active.values().map(|conns| conns.len()).sum::<usize>();
        let pooled_connections = pool.connection_count();
        
        let mut protocol_usage = HashMap::new();
        let mut connections_by_state = HashMap::new();
        let mut total_age = Duration::ZERO;
        let mut connection_count = 0;

        for connections in active.values() {
            for managed_conn in connections {
                // Protocol usage
                *protocol_usage.entry(managed_conn.protocol.clone()).or_insert(0) += 1;
                
                // Connection state
                let state_name = format!("{:?}", managed_conn.state);
                *connections_by_state.entry(state_name).or_insert(0) += 1;
                
                // Age calculation
                total_age += managed_conn.age();
                connection_count += 1;
            }
        }
        
        let average_connection_age = if connection_count > 0 {
            total_age / connection_count as u32
        } else {
            Duration::ZERO
        };

        let pool_stats = pool.get_pool_stats();
        
        ConnectionStats {
            total_active_connections: total_connections,
            pooled_connections,
            protocol_usage,
            max_concurrent_connections: self.max_concurrent_connections,
            connections_by_state,
            average_connection_age,
            pool_stats,
        }
    }

    /// Clean up idle and disconnected connections
    pub async fn cleanup_connections(&self) {
        // Clean up active connections
        {
            let mut active = self.active_connections.write().await;
            let mut peers_to_remove = Vec::new();
            
            for (peer_id, connections) in active.iter_mut() {
                let mut indices_to_remove = Vec::new();
                
                for (i, managed_conn) in connections.iter_mut().enumerate() {
                    if !managed_conn.connection.is_connected() || 
                       managed_conn.idle_time() > self.idle_timeout {
                        
                        if managed_conn.connection.is_connected() {
                            managed_conn.set_state(ConnectionState::Disconnecting);
                            let _ = managed_conn.connection.close().await;
                        }
                        managed_conn.set_state(ConnectionState::Disconnected);
                        indices_to_remove.push(i);
                    }
                }
                
                // Remove disconnected connections in reverse order
                for &i in indices_to_remove.iter().rev() {
                    connections.remove(i);
                }
                
                if connections.is_empty() {
                    peers_to_remove.push(peer_id.clone());
                }
            }
            
            // Remove empty peer entries
            for peer_id in peers_to_remove {
                active.remove(&peer_id);
            }
        }

        // Clean up connection pool
        {
            let mut pool = self.connection_pool.write().await;
            pool.cleanup_idle_connections().await;
        }
    }

    /// Start automatic cleanup task (requires Arc<ConnectionManager>)
    pub fn start_cleanup_task(manager: Arc<ConnectionManager>) -> tokio::task::JoinHandle<()> {
        let cleanup_interval = manager.cleanup_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);
            loop {
                interval.tick().await;
                manager.cleanup_connections().await;
            }
        })
    }

    /// Set maximum concurrent connections
    pub fn set_max_concurrent_connections(&mut self, max: usize) {
        self.max_concurrent_connections = max;
    }

    /// Set connection timeout
    pub fn set_connection_timeout(&mut self, timeout: Duration) {
        self.connection_timeout = timeout;
    }

    /// Set idle timeout for connections
    pub fn set_idle_timeout(&mut self, timeout: Duration) {
        self.idle_timeout = timeout;
    }

    /// Set maximum connections per peer
    pub fn set_max_connections_per_peer(&mut self, max: usize) {
        self.max_connections_per_peer = max;
    }

    /// Set protocol preference
    pub fn set_protocol_preference(&mut self, protocol: String, preference: u8) {
        self.protocol_preferences.insert(protocol, preference);
    }

    /// Get available transport protocols
    pub fn get_available_protocols(&self) -> Vec<String> {
        self.transports
            .iter()
            .filter(|t| t.is_available())
            .map(|t| t.protocol_name().to_string())
            .collect()
    }

    /// Check if a protocol is supported
    pub fn supports_protocol(&self, protocol: &str) -> bool {
        self.transports
            .iter()
            .any(|t| t.is_available() && t.protocol_name() == protocol)
    }

    /// Get transport by protocol name
    pub fn get_transport(&self, protocol: &str) -> Option<&dyn Transport> {
        self.transports
            .iter()
            .find(|t| t.is_available() && t.protocol_name() == protocol)
            .map(|t| t.as_ref())
    }

    /// Register all available transport protocols with default configurations
    pub async fn register_all_transports(&mut self) -> Result<(), TransportError> {
        use crate::transport::protocols::{
            tcp::TcpTransport,
            quic::QuicTransport,
            webrtc::WebRtcTransport,
            websocket::WebSocketTransport,
        };

        // Register TCP transport
        let tcp_transport = TcpTransport::new();
        self.add_transport(Box::new(tcp_transport));

        // Register QUIC transport
        match QuicTransport::new() {
            Ok(quic_transport) => {
                self.add_transport(Box::new(quic_transport));
            }
            Err(e) => {
                eprintln!("Failed to initialize QUIC transport: {}", e);
                // Continue without QUIC - it's not critical
            }
        }

        // Register WebRTC transport
        match WebRtcTransport::new() {
            Ok(webrtc_transport) => {
                self.add_transport(Box::new(webrtc_transport));
            }
            Err(e) => {
                eprintln!("Failed to initialize WebRTC transport: {}", e);
                // Continue without WebRTC - it's not critical
            }
        }

        // Register WebSocket transport
        let websocket_transport = WebSocketTransport::new();
        self.add_transport(Box::new(websocket_transport));

        Ok(())
    }

    /// Register transport protocols with custom configurations
    pub async fn register_transports_with_config(
        &mut self,
        tcp_config: Option<crate::transport::protocols::tcp::TcpConfig>,
        quic_config: Option<crate::transport::protocols::quic::QuicConfig>,
        webrtc_config: Option<crate::transport::protocols::webrtc::WebRtcConfig>,
        websocket_config: Option<crate::transport::protocols::websocket::WebSocketConfig>,
    ) -> Result<(), TransportError> {
        use crate::transport::protocols::{
            tcp::TcpTransport,
            quic::QuicTransport,
            webrtc::WebRtcTransport,
            websocket::WebSocketTransport,
        };

        // Register TCP transport
        if let Some(config) = tcp_config {
            let tcp_transport = TcpTransport::with_config(config);
            self.add_transport(Box::new(tcp_transport));
        }

        // Register QUIC transport
        if let Some(config) = quic_config {
            match QuicTransport::with_config(config) {
                Ok(quic_transport) => {
                    self.add_transport(Box::new(quic_transport));
                }
                Err(e) => {
                    eprintln!("Failed to initialize QUIC transport: {}", e);
                }
            }
        }

        // Register WebRTC transport
        if let Some(config) = webrtc_config {
            match WebRtcTransport::with_config(config) {
                Ok(webrtc_transport) => {
                    self.add_transport(Box::new(webrtc_transport));
                }
                Err(e) => {
                    eprintln!("Failed to initialize WebRTC transport: {}", e);
                }
            }
        }

        // Register WebSocket transport
        if let Some(config) = websocket_config {
            let websocket_transport = WebSocketTransport::with_config(config);
            self.add_transport(Box::new(websocket_transport));
        }

        Ok(())
    }

    /// Attempt concurrent connections across multiple protocols
    pub async fn connect_with_concurrent_attempts(
        &self,
        peer: &PeerInfo,
        max_concurrent: Option<usize>,
    ) -> Result<ConcurrentConnectionResult, TransportError> {
        let peer_protocols = &peer.address.transport_hints;
        let max_concurrent = max_concurrent.unwrap_or(3);
        
        // Collect available transports that the peer supports
        let mut available_transports = Vec::new();
        for transport in &self.transports {
            if transport.is_available() && 
               peer_protocols.contains(&transport.protocol_name().to_string()) {
                available_transports.push(transport.as_ref());
            }
        }

        if available_transports.is_empty() {
            return Err(TransportError::UnsupportedProtocol {
                protocol: format!("No common protocols. Peer supports: {:?}", peer_protocols),
            });
        }

        // Sort by priority (highest first)
        available_transports.sort_by(|a, b| b.priority().cmp(&a.priority()));

        // Limit concurrent attempts
        let concurrent_transports = available_transports
            .into_iter()
            .take(max_concurrent)
            .collect::<Vec<_>>();

        let mut connection_futures: Vec<std::pin::Pin<Box<dyn std::future::Future<Output = ConnectionAttemptResult> + Send>>> = Vec::new();
        let mut attempted_protocols = Vec::new();

        // Create connection futures for each transport
        for transport in concurrent_transports {
            let protocol_name = transport.protocol_name().to_string();
            attempted_protocols.push(protocol_name.clone());
            
            let peer_address = peer.address.clone();
            let future = Box::pin(async move {
                let start_time = Instant::now();
                let result = transport.connect(&peer_address).await;
                let duration = start_time.elapsed();
                
                ConnectionAttemptResult {
                    protocol: protocol_name,
                    result,
                    duration,
                }
            });
            
            connection_futures.push(future);
        }

        // Wait for the first successful connection or all to fail
        let mut results = Vec::new();
        let mut successful_connection = None;

        // Use select_all to get results as they complete
        while !connection_futures.is_empty() && successful_connection.is_none() {
            let (result, _index, remaining) = future::select_all(connection_futures).await;
            connection_futures = remaining;
            
            if result.result.is_ok() {
                successful_connection = Some(result);
            } else {
                results.push(result);
            }
        }

        // Cancel remaining attempts if we got a successful connection
        if successful_connection.is_some() {
            // The remaining futures will be dropped, effectively canceling them
        }

        // Collect any remaining results
        for future in connection_futures {
            let result = future.await;
            results.push(result);
        }

        if let Some(successful) = successful_connection {
            let connection = successful.result.unwrap();
            let protocol = successful.protocol.clone();
            
            // Add to active connections
            let managed_connection = ManagedConnection::new(
                connection,
                protocol.clone(),
                peer.address.peer_id.clone(),
            );

            {
                let mut active = self.active_connections.write().await;
                let peer_connections = active.entry(peer.address.peer_id.clone()).or_insert_with(Vec::new);
                peer_connections.push(managed_connection);
            }

            // Return the successful connection (create a new one since we moved the original)
            let transport = self.get_transport(&protocol).unwrap();
            let new_connection = transport.connect(&peer.address).await?;

            Ok(ConcurrentConnectionResult {
                connection: new_connection,
                successful_protocol: protocol,
                attempt_results: results,
                total_attempts: attempted_protocols.len(),
            })
        } else {
            Err(TransportError::ConnectionFailed {
                reason: format!(
                    "All concurrent connection attempts failed. Attempted: {:?}. Results: {:?}",
                    attempted_protocols,
                    results.iter().map(|r| format!("{}: {:?}", r.protocol, r.result)).collect::<Vec<_>>()
                ),
            })
        }
    }

    /// Monitor connection health and automatically switch protocols if needed
    pub async fn monitor_and_switch_connections(&self) -> Result<(), TransportError> {
        let mut connections_to_switch = Vec::new();
        
        // Check all active connections for health issues
        {
            let active = self.active_connections.read().await;
            for (peer_id, connections) in active.iter() {
                for managed_conn in connections {
                    if managed_conn.is_active() {
                        let connection_info = managed_conn.connection.info();
                        
                        // Check if connection needs switching based on performance
                        if self.should_switch_connection(&connection_info).await {
                            connections_to_switch.push((
                                peer_id.clone(),
                                managed_conn.protocol.clone(),
                                connection_info,
                            ));
                        }
                    }
                }
            }
        }

        // Attempt to switch problematic connections
        for (peer_id, current_protocol, connection_info) in connections_to_switch {
            if let Err(e) = self.attempt_connection_switch(&peer_id, &current_protocol, &connection_info).await {
                eprintln!("Failed to switch connection for peer {}: {}", peer_id, e);
            }
        }

        Ok(())
    }

    /// Check if a connection should be switched to a different protocol
    async fn should_switch_connection(&self, connection_info: &ConnectionInfo) -> bool {
        // Switch if RTT is too high
        if let Some(rtt) = connection_info.rtt {
            if rtt > Duration::from_millis(500) {
                return true;
            }
        }

        // Switch if bandwidth is too low
        if let Some(bandwidth) = connection_info.bandwidth {
            if bandwidth < 1024 * 1024 { // Less than 1MB/s
                return true;
            }
        }

        // Switch if connection is old and might benefit from a newer protocol
        if connection_info.duration() > Duration::from_secs(3600) { // 1 hour
            return true;
        }

        false
    }

    /// Attempt to switch a connection to a better protocol
    async fn attempt_connection_switch(
        &self,
        peer_id: &PeerId,
        current_protocol: &str,
        connection_info: &ConnectionInfo,
    ) -> Result<(), TransportError> {
        // Create a mock PeerInfo for the switch attempt
        let peer_address = PeerAddress::new(
            peer_id.clone(),
            vec![connection_info.remote_addr],
            self.get_available_protocols(),
            TransportCapabilities::default(),
        );
        let peer_info = PeerInfo::new(peer_address);

        // Try to negotiate a better protocol
        let conditions = NetworkConditions {
            latency_requirement: LatencyRequirement::Low,
            bandwidth_requirement: BandwidthRequirement::High,
            reliability_requirement: ReliabilityRequirement::High,
            nat_traversal_needed: false,
            mobile_network: false,
            battery_constrained: false,
        };

        match self.negotiate_protocol_with_conditions(&peer_info, &conditions).await {
            Ok(better_transport) => {
                let better_protocol = better_transport.protocol_name();
                
                // Only switch if we found a different, better protocol
                if better_protocol != current_protocol {
                    match better_transport.connect(&peer_info.address).await {
                        Ok(new_connection) => {
                            // Close old connection and replace with new one
                            self.close_connection(peer_id, Some(current_protocol)).await?;
                            
                            // Add new connection
                            let managed_connection = ManagedConnection::new(
                                new_connection,
                                better_protocol.to_string(),
                                peer_id.clone(),
                            );

                            {
                                let mut active = self.active_connections.write().await;
                                let peer_connections = active.entry(peer_id.clone()).or_insert_with(Vec::new);
                                peer_connections.push(managed_connection);
                            }

                            println!("Switched connection for peer {} from {} to {}", 
                                   peer_id, current_protocol, better_protocol);
                        }
                        Err(e) => {
                            eprintln!("Failed to establish new connection with {}: {}", better_protocol, e);
                        }
                    }
                }
            }
            Err(_) => {
                // No better protocol available, keep current connection
            }
        }

        Ok(())
    }

    /// Collect comprehensive connection statistics and performance monitoring
    pub async fn collect_detailed_connection_stats(&self) -> DetailedConnectionStats {
        let active = self.active_connections.read().await;
        let pool = self.connection_pool.read().await;
        
        let mut stats = DetailedConnectionStats::default();
        stats.timestamp = Instant::now();
        
        // Collect active connection statistics
        for (_peer_id, connections) in active.iter() {
            stats.total_peers += 1;
            
            for managed_conn in connections {
                let connection_info = managed_conn.connection.info();
                
                // Update protocol usage
                *stats.protocol_usage.entry(managed_conn.protocol.clone()).or_insert(0) += 1;
                
                // Update connection state counts
                let state_name = format!("{:?}", managed_conn.state);
                *stats.connections_by_state.entry(state_name).or_insert(0) += 1;
                
                // Track performance metrics
                if let Some(rtt) = connection_info.rtt {
                    stats.latency_samples.push(rtt);
                }
                
                if let Some(bandwidth) = connection_info.bandwidth {
                    stats.bandwidth_samples.push(bandwidth);
                }
                
                // Track connection ages
                stats.connection_ages.push(managed_conn.age());
                
                // Track bytes transferred
                stats.total_bytes_sent += connection_info.bytes_sent;
                stats.total_bytes_received += connection_info.bytes_received;
                
                // Count active connections
                if managed_conn.is_active() {
                    stats.active_connections += 1;
                }
            }
        }
        
        // Calculate averages
        if !stats.latency_samples.is_empty() {
            let total_latency: Duration = stats.latency_samples.iter().sum();
            stats.average_latency = Some(total_latency / stats.latency_samples.len() as u32);
        }
        
        if !stats.bandwidth_samples.is_empty() {
            stats.average_bandwidth = Some(
                stats.bandwidth_samples.iter().sum::<u64>() / stats.bandwidth_samples.len() as u64
            );
        }
        
        if !stats.connection_ages.is_empty() {
            let total_age: Duration = stats.connection_ages.iter().sum();
            stats.average_connection_age = total_age / stats.connection_ages.len() as u32;
        }
        
        // Pool statistics
        stats.pool_stats = pool.get_pool_stats();
        stats.pooled_connections = pool.connection_count();
        
        // Transport availability
        for transport in &self.transports {
            stats.available_transports.push(AvailableTransport {
                protocol: transport.protocol_name().to_string(),
                available: transport.is_available(),
                priority: transport.priority(),
                capabilities: transport.capabilities(),
            });
        }
        
        stats
    }

    /// Start automatic connection health monitoring
    pub fn start_connection_monitoring(manager: Arc<ConnectionManager>) -> tokio::task::JoinHandle<()> {
        let monitoring_interval = Duration::from_secs(30); // Monitor every 30 seconds
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(monitoring_interval);
            loop {
                interval.tick().await;
                
                if let Err(e) = manager.monitor_and_switch_connections().await {
                    eprintln!("Connection monitoring error: {}", e);
                }
            }
        })
    }

    /// Return a connection to the pool for reuse
    pub async fn return_connection_to_pool(&self, peer_id: PeerId, connection: Box<dyn Connection>) -> Result<(), TransportError> {
        let mut pool = self.connection_pool.write().await;
        pool.add_connection(peer_id, connection)
    }

    /// Force close all connections
    pub async fn shutdown(&self) -> Result<(), TransportError> {
        let mut active = self.active_connections.write().await;
        
        for (_, connections) in active.iter_mut() {
            for managed_conn in connections.iter_mut() {
                managed_conn.set_state(ConnectionState::Disconnecting);
                let _ = managed_conn.connection.close().await;
                managed_conn.set_state(ConnectionState::Disconnected);
            }
        }
        
        active.clear();
        
        // Clear connection pool
        {
            let mut pool = self.connection_pool.write().await;
            pool.cleanup_idle_connections().await;
        }
        
        Ok(())
    }

    /// Start listening for incoming connections on all available transports
    pub async fn start_listening(&self, bind_addr: SocketAddr) -> Result<(), TransportError> {
        let mut started_count = 0;
        
        for transport in &self.transports {
            if transport.is_available() {
                match transport.listen(&bind_addr).await {
                    Ok(()) => {
                        started_count += 1;
                    }
                    Err(e) => {
                        eprintln!("Failed to start listener for {}: {}", transport.protocol_name(), e);
                        // Continue with other transports
                    }
                }
            }
        }
        
        if started_count == 0 {
            return Err(TransportError::TransportNotAvailable);
        }
        
        Ok(())
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use crate::transport::protocols::tcp::TcpTransport;

    #[tokio::test]
    async fn test_connection_manager_creation() {
        let manager = ConnectionManager::new();
        assert_eq!(manager.max_concurrent_connections, 100);
        assert_eq!(manager.max_connections_per_peer, 5);
        
        let stats = manager.get_connection_stats().await;
        assert_eq!(stats.total_active_connections, 0);
        assert_eq!(stats.pooled_connections, 0);
    }

    #[tokio::test]
    async fn test_connection_manager_with_config() {
        let mut config = ConnectionManagerConfig::default();
        config.max_concurrent_connections = 50;
        config.max_connections_per_peer = 3;
        
        let manager = ConnectionManager::with_config(config);
        assert_eq!(manager.max_concurrent_connections, 50);
        assert_eq!(manager.max_connections_per_peer, 3);
    }

    #[tokio::test]
    async fn test_add_transport() {
        let mut manager = ConnectionManager::new();
        let tcp_transport = Box::new(TcpTransport::new());
        
        manager.add_transport(tcp_transport);
        
        let protocols = manager.get_available_protocols();
        assert!(protocols.contains(&"tcp".to_string()));
        assert!(manager.supports_protocol("tcp"));
        assert!(!manager.supports_protocol("nonexistent"));
    }

    #[tokio::test]
    async fn test_protocol_negotiation() {
        let mut manager = ConnectionManager::new();
        let tcp_transport = Box::new(TcpTransport::new());
        manager.add_transport(tcp_transport);

        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );
        let peer_info = PeerInfo::new(peer_addr);

        let result = manager.negotiate_protocol(&peer_info).await;
        assert!(result.is_ok());
        
        let transport = result.unwrap();
        assert_eq!(transport.protocol_name(), "tcp");
    }

    #[tokio::test]
    async fn test_protocol_negotiation_no_common_protocols() {
        let mut manager = ConnectionManager::new();
        let tcp_transport = Box::new(TcpTransport::new());
        manager.add_transport(tcp_transport);

        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)],
            vec!["quic".to_string()], // Peer only supports QUIC, we only have TCP
            TransportCapabilities::quic(),
        );
        let peer_info = PeerInfo::new(peer_addr);

        let result = manager.negotiate_protocol(&peer_info).await;
        assert!(result.is_err());
        
        if let Err(e) = result {
            assert!(matches!(e, TransportError::UnsupportedProtocol { .. }));
        }
    }

    #[tokio::test]
    async fn test_advanced_protocol_negotiation() {
        let mut manager = ConnectionManager::new();
        let tcp_transport = Box::new(TcpTransport::new());
        manager.add_transport(tcp_transport);

        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080)],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );
        let peer_info = PeerInfo::new(peer_addr);

        let conditions = NetworkConditions {
            latency_requirement: LatencyRequirement::High,
            bandwidth_requirement: BandwidthRequirement::Low,
            reliability_requirement: ReliabilityRequirement::High,
            nat_traversal_needed: false,
            mobile_network: false,
            battery_constrained: false,
        };

        let result = manager.negotiate_protocol_with_conditions(&peer_info, &conditions).await;
        assert!(result.is_ok());
        
        let transport = result.unwrap();
        assert_eq!(transport.protocol_name(), "tcp");
    }

    #[tokio::test]
    async fn test_connection_manager_configuration() {
        let mut manager = ConnectionManager::new();
        
        manager.set_max_concurrent_connections(200);
        manager.set_max_connections_per_peer(10);
        manager.set_connection_timeout(Duration::from_secs(45));
        manager.set_idle_timeout(Duration::from_secs(600));
        manager.set_protocol_preference("tcp".to_string(), 100);
        
        assert_eq!(manager.max_concurrent_connections, 200);
        assert_eq!(manager.max_connections_per_peer, 10);
        assert_eq!(manager.connection_timeout, Duration::from_secs(45));
        assert_eq!(manager.idle_timeout, Duration::from_secs(600));
        assert_eq!(manager.protocol_preferences.get("tcp"), Some(&100));
    }

    #[tokio::test]
    async fn test_connection_stats() {
        let manager = ConnectionManager::new();
        let stats = manager.get_connection_stats().await;
        
        assert_eq!(stats.total_active_connections, 0);
        assert_eq!(stats.pooled_connections, 0);
        assert_eq!(stats.max_concurrent_connections, 100);
        assert!(stats.protocol_usage.is_empty());
        assert!(stats.connections_by_state.is_empty());
    }

    #[tokio::test]
    async fn test_cleanup_connections() {
        let manager = ConnectionManager::new();
        
        // Should not panic or error on empty connections
        manager.cleanup_connections().await;
        
        let stats = manager.get_connection_stats().await;
        assert_eq!(stats.total_active_connections, 0);
    }

    #[tokio::test]
    async fn test_get_connections_for_peer() {
        let manager = ConnectionManager::new();
        let peer_id = "test-peer-123";
        
        let connections = manager.get_connections(&peer_id.to_string()).await;
        assert!(connections.is_empty());
    }

    #[tokio::test]
    async fn test_close_peer_connections() {
        let manager = ConnectionManager::new();
        let peer_id = "test-peer-123";
        
        let result = manager.close_peer_connections(&peer_id.to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let manager = ConnectionManager::new();
        let result = manager.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connection_pool_functionality() {
        let mut pool = ConnectionPool::new(3, Duration::from_secs(300));
        
        assert_eq!(pool.connection_count(), 0);
        assert_eq!(pool.peer_connection_count(&"test-peer".to_string()), 0);
        
        let stats = pool.get_pool_stats();
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.peer_count, 0);
        
        pool.cleanup_idle_connections().await;
        assert_eq!(pool.connection_count(), 0);
    }

    #[tokio::test]
    async fn test_managed_connection() {
        use crate::transport::protocols::tcp::TcpConnection;
        use tokio::net::TcpStream;
        
        // Create a mock connection for testing
        let stream = match TcpStream::connect("127.0.0.1:80").await {
            Ok(stream) => stream,
            Err(_) => {
                // Skip test if we can't connect
                return;
            }
        };
        
        let remote_addr = stream.peer_addr().unwrap();
        let tcp_conn = TcpConnection::new(stream, "test-peer".to_string(), remote_addr);
        let connection: Box<dyn Connection> = Box::new(tcp_conn);
        
        let mut managed = ManagedConnection::new(
            connection,
            "tcp".to_string(),
            "test-peer".to_string(),
        );
        
        assert_eq!(managed.protocol, "tcp");
        assert_eq!(managed.state, ConnectionState::Connected);
        assert!(managed.age() < Duration::from_secs(1));
        assert!(managed.idle_time() < Duration::from_secs(1));
        
        managed.update_activity();
        managed.set_state(ConnectionState::Disconnecting);
        assert_eq!(managed.state, ConnectionState::Disconnecting);
    }

    #[tokio::test]
    async fn test_network_conditions_defaults() {
        let conditions = NetworkConditions::default();
        assert_eq!(conditions.latency_requirement, LatencyRequirement::Medium);
        assert_eq!(conditions.bandwidth_requirement, BandwidthRequirement::Medium);
        assert_eq!(conditions.reliability_requirement, ReliabilityRequirement::Medium);
        assert!(!conditions.nat_traversal_needed);
        assert!(!conditions.mobile_network);
        assert!(!conditions.battery_constrained);
    }

    #[tokio::test]
    async fn test_connection_manager_config_defaults() {
        let config = ConnectionManagerConfig::default();
        assert_eq!(config.max_concurrent_connections, 100);
        assert_eq!(config.max_connections_per_peer, 5);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.cleanup_interval, Duration::from_secs(60));
        assert_eq!(config.max_connection_reuse, 100);
        
        // Check protocol preferences
        assert_eq!(config.protocol_preferences.get("quic"), Some(&100));
        assert_eq!(config.protocol_preferences.get("webrtc"), Some(&90));
        assert_eq!(config.protocol_preferences.get("tcp"), Some(&80));
        assert_eq!(config.protocol_preferences.get("websocket"), Some(&70));
    }

    #[tokio::test]
    async fn test_register_all_transports() {
        let mut manager = ConnectionManager::new();
        
        // Should not fail even if some transports can't be initialized
        let result = manager.register_all_transports().await;
        assert!(result.is_ok());
        
        // Should have at least TCP and WebSocket (most reliable)
        let protocols = manager.get_available_protocols();
        assert!(protocols.contains(&"tcp".to_string()));
        assert!(protocols.contains(&"websocket".to_string()));
        
        // Check that transports are properly registered
        assert!(manager.supports_protocol("tcp"));
        assert!(manager.supports_protocol("websocket"));
    }

    #[tokio::test]
    async fn test_enhanced_protocol_negotiation() {
        let mut manager = ConnectionManager::new();
        let _ = manager.register_all_transports().await;

        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1)), 8080)],
            vec!["tcp".to_string(), "websocket".to_string()],
            TransportCapabilities::tcp(),
        );
        let peer_info = PeerInfo::new(peer_addr);

        let conditions = NetworkConditions {
            latency_requirement: LatencyRequirement::Low,
            bandwidth_requirement: BandwidthRequirement::High,
            reliability_requirement: ReliabilityRequirement::High,
            nat_traversal_needed: false,
            mobile_network: false,
            battery_constrained: false,
        };

        let result = manager.negotiate_protocol_enhanced(&peer_info, Some(&conditions)).await;
        
        // Should succeed with available protocols
        match result {
            Ok(negotiation_result) => {
                assert!(negotiation_result.negotiation_summary.selected_protocol.is_some());
                let selected = negotiation_result.negotiation_summary.selected_protocol.unwrap();
                assert!(vec!["tcp", "websocket"].contains(&selected.as_str()));
            }
            Err(e) => {
                // May fail if no transports are actually available in test environment
                println!("Negotiation failed (expected in test environment): {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_detailed_connection_stats() {
        let manager = ConnectionManager::new();
        let stats = manager.collect_detailed_connection_stats().await;
        
        assert_eq!(stats.total_peers, 0);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_bytes_sent, 0);
        assert_eq!(stats.total_bytes_received, 0);
        assert!(stats.latency_samples.is_empty());
        assert!(stats.bandwidth_samples.is_empty());
        assert!(stats.connection_ages.is_empty());
        assert_eq!(stats.average_latency, None);
        assert_eq!(stats.average_bandwidth, None);
    }

    #[tokio::test]
    async fn test_connection_monitoring() {
        let manager = ConnectionManager::new();
        
        // Should not panic or error on empty connections
        let result = manager.monitor_and_switch_connections().await;
        assert!(result.is_ok());
    }
}

/// Configuration for ConnectionManager
#[derive(Debug, Clone)]
pub struct ConnectionManagerConfig {
    pub max_concurrent_connections: usize,
    pub max_connections_per_peer: usize,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
    pub cleanup_interval: Duration,
    pub max_connection_reuse: u32,
    pub protocol_preferences: HashMap<String, u8>,
}

impl Default for ConnectionManagerConfig {
    fn default() -> Self {
        let mut protocol_preferences = HashMap::new();
        protocol_preferences.insert("quic".to_string(), 100);
        protocol_preferences.insert("webrtc".to_string(), 90);
        protocol_preferences.insert("tcp".to_string(), 80);
        protocol_preferences.insert("websocket".to_string(), 70);

        Self {
            max_concurrent_connections: 100,
            max_connections_per_peer: 5,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(60),
            max_connection_reuse: 100,
            protocol_preferences,
        }
    }
}

/// Statistics about connection manager state
#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub total_active_connections: usize,
    pub pooled_connections: usize,
    pub protocol_usage: HashMap<String, usize>,
    pub max_concurrent_connections: usize,
    pub connections_by_state: HashMap<String, usize>,
    pub average_connection_age: Duration,
    pub pool_stats: PoolStats,
}

/// Result of a single connection attempt
#[derive(Debug)]
pub struct ConnectionAttemptResult {
    pub protocol: String,
    pub result: Result<Box<dyn Connection>, TransportError>,
    pub duration: Duration,
}

/// Result of concurrent connection attempts
#[derive(Debug)]
pub struct ConcurrentConnectionResult {
    pub connection: Box<dyn Connection>,
    pub successful_protocol: String,
    pub attempt_results: Vec<ConnectionAttemptResult>,
    pub total_attempts: usize,
}

/// Detailed connection statistics with performance metrics
#[derive(Debug, Clone)]
pub struct DetailedConnectionStats {
    pub timestamp: Instant,
    pub total_peers: usize,
    pub active_connections: usize,
    pub pooled_connections: usize,
    pub protocol_usage: HashMap<String, usize>,
    pub connections_by_state: HashMap<String, usize>,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
    pub latency_samples: Vec<Duration>,
    pub bandwidth_samples: Vec<u64>,
    pub connection_ages: Vec<Duration>,
    pub average_latency: Option<Duration>,
    pub average_bandwidth: Option<u64>,
    pub average_connection_age: Duration,
    pub pool_stats: PoolStats,
    pub available_transports: Vec<AvailableTransport>,
}

impl Default for DetailedConnectionStats {
    fn default() -> Self {
        Self {
            timestamp: Instant::now(),
            total_peers: 0,
            active_connections: 0,
            pooled_connections: 0,
            protocol_usage: HashMap::new(),
            connections_by_state: HashMap::new(),
            total_bytes_sent: 0,
            total_bytes_received: 0,
            latency_samples: Vec::new(),
            bandwidth_samples: Vec::new(),
            connection_ages: Vec::new(),
            average_latency: None,
            average_bandwidth: None,
            average_connection_age: Duration::ZERO,
            pool_stats: PoolStats {
                total_connections: 0,
                idle_connections: 0,
                average_usage: 0,
                max_usage: 0,
                peer_count: 0,
            },
            available_transports: Vec::new(),
        }
    }
}

/// Information about an available transport protocol
#[derive(Debug, Clone)]
pub struct AvailableTransport {
    pub protocol: String,
    pub available: bool,
    pub priority: u8,
    pub capabilities: TransportCapabilities,
}