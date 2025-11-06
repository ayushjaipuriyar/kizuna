use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{RwLock, Mutex, mpsc};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{interval, timeout};
use tokio_tungstenite::{accept_async, WebSocketStream};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::transport::{
    PeerId, TransportError
};

/// Configuration for relay node functionality
#[derive(Debug, Clone)]
pub struct RelayConfig {
    /// Maximum number of concurrent relay connections
    pub max_connections: usize,
    /// Maximum bandwidth per connection in bytes per second
    pub max_bandwidth_per_connection: u64,
    /// Total bandwidth limit for the relay node in bytes per second
    pub total_bandwidth_limit: u64,
    /// Connection timeout for relay establishment
    pub connection_timeout: Duration,
    /// Idle timeout before closing inactive relay connections
    pub idle_timeout: Duration,
    /// Authentication token for relay access (optional)
    pub auth_token: Option<String>,
    /// Whether to require authentication for relay access
    pub require_auth: bool,
    /// Health check interval for monitoring relay performance
    pub health_check_interval: Duration,
    /// Maximum message size for relay forwarding
    pub max_message_size: usize,
    /// Buffer size for message queuing
    pub message_buffer_size: usize,
    /// Rate limiting: maximum messages per second per connection
    pub max_messages_per_second: u32,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            max_connections: 100,
            max_bandwidth_per_connection: 1024 * 1024, // 1MB/s per connection
            total_bandwidth_limit: 100 * 1024 * 1024,  // 100MB/s total
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300), // 5 minutes
            auth_token: None,
            require_auth: false,
            health_check_interval: Duration::from_secs(60),
            max_message_size: 16 * 1024 * 1024, // 16MB
            message_buffer_size: 1024,
            max_messages_per_second: 100,
        }
    }
}

/// Information about a relay node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayNodeInfo {
    /// Unique identifier for the relay node
    pub node_id: String,
    /// Network address of the relay node
    pub address: SocketAddr,
    /// Public key for authentication (optional)
    pub public_key: Option<Vec<u8>>,
    /// Bandwidth capacity in bytes per second
    pub bandwidth_capacity: u64,
    /// Current connection count
    pub connection_count: usize,
    /// Average latency to this relay node
    pub latency: Option<Duration>,
    /// Reliability score (0-100)
    pub reliability_score: u8,
    /// Last seen timestamp
    pub last_seen: SystemTime,
    /// Supported features
    pub features: Vec<String>,
}

impl RelayNodeInfo {
    pub fn new(node_id: String, address: SocketAddr, bandwidth_capacity: u64) -> Self {
        Self {
            node_id,
            address,
            public_key: None,
            bandwidth_capacity,
            connection_count: 0,
            latency: None,
            reliability_score: 100,
            last_seen: SystemTime::now(),
            features: vec!["websocket".to_string(), "tcp".to_string()],
        }
    }

    /// Calculate selection score for relay node selection
    pub fn selection_score(&self) -> f64 {
        let mut score = self.reliability_score as f64;
        
        // Penalize high connection count (load balancing)
        let load_factor = self.connection_count as f64 / 100.0;
        score -= load_factor * 20.0;
        
        // Reward low latency
        if let Some(latency) = self.latency {
            let latency_ms = latency.as_millis() as f64;
            if latency_ms < 50.0 {
                score += 10.0;
            } else if latency_ms > 200.0 {
                score -= 15.0;
            }
        }
        
        // Reward high bandwidth capacity
        let bandwidth_gb = self.bandwidth_capacity as f64 / (1024.0 * 1024.0 * 1024.0);
        score += bandwidth_gb * 5.0;
        
        score.max(0.0).min(100.0)
    }

    /// Update reliability score based on connection success/failure
    pub fn update_reliability(&mut self, success: bool) {
        if success {
            self.reliability_score = (self.reliability_score + 1).min(100);
        } else {
            self.reliability_score = self.reliability_score.saturating_sub(5);
        }
        self.last_seen = SystemTime::now();
    }
}

/// Statistics for relay node performance monitoring
#[derive(Debug, Clone)]
pub struct RelayStats {
    /// Total number of relay connections handled
    pub total_connections: u64,
    /// Currently active relay connections
    pub active_connections: usize,
    /// Total bytes relayed
    pub bytes_relayed: u64,
    /// Average connection duration
    pub average_connection_duration: Duration,
    /// Connection success rate (0-100)
    pub success_rate: f64,
    /// Current bandwidth usage in bytes per second
    pub current_bandwidth_usage: u64,
    /// Peak bandwidth usage
    pub peak_bandwidth_usage: u64,
    /// Number of authentication failures
    pub auth_failures: u64,
    /// Number of rate limit violations
    pub rate_limit_violations: u64,
}

impl Default for RelayStats {
    fn default() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            bytes_relayed: 0,
            average_connection_duration: Duration::ZERO,
            success_rate: 100.0,
            current_bandwidth_usage: 0,
            peak_bandwidth_usage: 0,
            auth_failures: 0,
            rate_limit_violations: 0,
        }
    }
}

/// Bandwidth limiter for controlling relay traffic
#[derive(Debug)]
pub struct BandwidthLimiter {
    /// Maximum bytes per second
    max_bps: u64,
    /// Token bucket for rate limiting
    tokens: Arc<Mutex<f64>>,
    /// Last refill time
    last_refill: Arc<Mutex<Instant>>,
}

impl BandwidthLimiter {
    pub fn new(max_bps: u64) -> Self {
        Self {
            max_bps,
            tokens: Arc::new(Mutex::new(max_bps as f64)),
            last_refill: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Check if we can send the specified number of bytes
    pub async fn can_send(&self, bytes: usize) -> bool {
        let mut tokens = self.tokens.lock().await;
        let mut last_refill = self.last_refill.lock().await;
        
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill).as_secs_f64();
        
        // Refill tokens based on elapsed time
        *tokens = (*tokens + elapsed * self.max_bps as f64).min(self.max_bps as f64);
        *last_refill = now;
        
        if *tokens >= bytes as f64 {
            *tokens -= bytes as f64;
            true
        } else {
            false
        }
    }

    /// Wait until we can send the specified number of bytes
    pub async fn wait_for_capacity(&self, bytes: usize) -> Result<(), TransportError> {
        let start = Instant::now();
        let timeout_duration = Duration::from_secs(30);
        
        while start.elapsed() < timeout_duration {
            if self.can_send(bytes).await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        Err(TransportError::ConnectionTimeout {
            timeout: timeout_duration,
        })
    }
}

/// Active relay session between two peers
#[derive(Debug)]
pub struct RelaySession {
    /// Unique session identifier
    pub session_id: String,
    /// Source peer ID
    pub source_peer_id: PeerId,
    /// Target peer ID
    pub target_peer_id: PeerId,
    /// Session creation time
    pub created_at: Instant,
    /// Last activity time
    pub last_activity: Instant,
    /// Bytes transferred in this session
    pub bytes_transferred: AtomicU64,
    /// Message count
    pub message_count: AtomicU64,
    /// Bandwidth limiter for this session
    pub bandwidth_limiter: BandwidthLimiter,
    /// Rate limiter for message frequency
    pub rate_limiter: Arc<Mutex<RateLimiter>>,
    /// Connection health status
    pub health_status: Arc<RwLock<ConnectionHealth>>,
}

impl RelaySession {
    pub fn new(
        source_peer_id: PeerId,
        target_peer_id: PeerId,
        max_bandwidth: u64,
        max_messages_per_second: u32,
    ) -> Self {
        Self {
            session_id: Uuid::new_v4().to_string(),
            source_peer_id,
            target_peer_id,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            bytes_transferred: AtomicU64::new(0),
            message_count: AtomicU64::new(0),
            bandwidth_limiter: BandwidthLimiter::new(max_bandwidth),
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(max_messages_per_second))),
            health_status: Arc::new(RwLock::new(ConnectionHealth::new())),
        }
    }

    /// Update activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Check if session is idle
    pub fn is_idle(&self, idle_timeout: Duration) -> bool {
        self.last_activity.elapsed() > idle_timeout
    }

    /// Add bytes to transfer count
    pub fn add_bytes_transferred(&self, bytes: u64) {
        self.bytes_transferred.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increment message count
    pub fn increment_message_count(&self) {
        self.message_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get session duration
    pub fn duration(&self) -> Duration {
        self.created_at.elapsed()
    }
}

impl Clone for RelaySession {
    fn clone(&self) -> Self {
        Self {
            session_id: self.session_id.clone(),
            source_peer_id: self.source_peer_id.clone(),
            target_peer_id: self.target_peer_id.clone(),
            created_at: self.created_at,
            last_activity: self.last_activity,
            bytes_transferred: AtomicU64::new(self.bytes_transferred.load(Ordering::Relaxed)),
            message_count: AtomicU64::new(self.message_count.load(Ordering::Relaxed)),
            bandwidth_limiter: BandwidthLimiter::new(self.bandwidth_limiter.max_bps),
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(100))), // Default rate
            health_status: Arc::new(RwLock::new(ConnectionHealth::new())),
        }
    }
}

/// Rate limiter for controlling message frequency
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum messages per second
    max_messages_per_second: u32,
    /// Message timestamps for rate limiting
    message_timestamps: VecDeque<Instant>,
}

impl RateLimiter {
    pub fn new(max_messages_per_second: u32) -> Self {
        Self {
            max_messages_per_second,
            message_timestamps: VecDeque::new(),
        }
    }

    /// Check if a message can be sent based on rate limits
    pub fn can_send_message(&mut self) -> bool {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(1);

        // Remove old timestamps outside the window
        while let Some(&front_time) = self.message_timestamps.front() {
            if front_time < window_start {
                self.message_timestamps.pop_front();
            } else {
                break;
            }
        }

        // Check if we're under the rate limit
        if self.message_timestamps.len() < self.max_messages_per_second as usize {
            self.message_timestamps.push_back(now);
            true
        } else {
            false
        }
    }

    /// Get current message rate (messages per second)
    pub fn current_rate(&self) -> f64 {
        let now = Instant::now();
        let window_start = now - Duration::from_secs(1);
        
        let recent_messages = self.message_timestamps.iter()
            .filter(|&&timestamp| timestamp >= window_start)
            .count();
        
        recent_messages as f64
    }
}

/// Connection health status for monitoring
#[derive(Debug, Clone)]
pub struct ConnectionHealth {
    /// Whether the connection is currently healthy
    pub is_healthy: bool,
    /// Last health check timestamp
    pub last_check: Instant,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    /// Average response time
    pub average_response_time: Duration,
    /// Error rate (0.0 to 1.0)
    pub error_rate: f64,
    /// Last error message
    pub last_error: Option<String>,
}

impl ConnectionHealth {
    pub fn new() -> Self {
        Self {
            is_healthy: true,
            last_check: Instant::now(),
            consecutive_failures: 0,
            average_response_time: Duration::from_millis(50),
            error_rate: 0.0,
            last_error: None,
        }
    }

    /// Update health status based on operation result
    pub fn update_health(&mut self, success: bool, response_time: Option<Duration>, error: Option<String>) {
        self.last_check = Instant::now();

        if success {
            self.consecutive_failures = 0;
            if let Some(rt) = response_time {
                // Update average response time with exponential moving average
                self.average_response_time = Duration::from_nanos(
                    ((self.average_response_time.as_nanos() as f64 * 0.9) + (rt.as_nanos() as f64 * 0.1)) as u64
                );
            }
        } else {
            self.consecutive_failures += 1;
            self.last_error = error;
        }

        // Update error rate (exponential moving average)
        let error_value = if success { 0.0 } else { 1.0 };
        self.error_rate = self.error_rate * 0.9 + error_value * 0.1;

        // Determine overall health
        self.is_healthy = self.consecutive_failures < 3 && self.error_rate < 0.5;
    }

    /// Check if connection needs attention
    pub fn needs_attention(&self) -> bool {
        !self.is_healthy || 
        self.consecutive_failures > 1 || 
        self.error_rate > 0.3 ||
        self.average_response_time > Duration::from_millis(500)
    }
}

/// Traffic isolation and security context for relay connections
#[derive(Debug, Clone)]
pub struct TrafficIsolation {
    /// Unique isolation context ID
    pub context_id: String,
    /// Allowed source peer IDs
    pub allowed_sources: Vec<PeerId>,
    /// Allowed target peer IDs
    pub allowed_targets: Vec<PeerId>,
    /// Maximum data rate for this context
    pub max_data_rate: u64,
    /// Encryption key for hop-by-hop encryption (optional)
    pub encryption_key: Option<Vec<u8>>,
    /// Whether to log traffic for this context
    pub enable_logging: bool,
}

impl TrafficIsolation {
    pub fn new(context_id: String) -> Self {
        Self {
            context_id,
            allowed_sources: Vec::new(),
            allowed_targets: Vec::new(),
            max_data_rate: u64::MAX,
            encryption_key: None,
            enable_logging: false,
        }
    }

    /// Check if a connection is allowed in this isolation context
    pub fn is_connection_allowed(&self, source: &PeerId, target: &PeerId) -> bool {
        (self.allowed_sources.is_empty() || self.allowed_sources.contains(source)) &&
        (self.allowed_targets.is_empty() || self.allowed_targets.contains(target))
    }

    /// Add allowed source peer
    pub fn add_allowed_source(&mut self, peer_id: PeerId) {
        if !self.allowed_sources.contains(&peer_id) {
            self.allowed_sources.push(peer_id);
        }
    }

    /// Add allowed target peer
    pub fn add_allowed_target(&mut self, peer_id: PeerId) {
        if !self.allowed_targets.contains(&peer_id) {
            self.allowed_targets.push(peer_id);
        }
    }
}

/// Enhanced relay session with traffic isolation and monitoring
#[derive(Debug)]
pub struct EnhancedRelaySession {
    /// Base relay session
    pub base_session: RelaySession,
    /// Traffic isolation context
    pub isolation_context: TrafficIsolation,
    /// Connection endpoints for forwarding
    pub source_endpoint: Option<mpsc::Sender<Vec<u8>>>,
    pub target_endpoint: Option<mpsc::Sender<Vec<u8>>>,
    /// Performance metrics
    pub performance_metrics: Arc<RwLock<SessionMetrics>>,
    /// Failover status
    pub failover_status: Arc<RwLock<FailoverStatus>>,
}

/// Performance metrics for relay sessions
#[derive(Debug, Clone)]
pub struct SessionMetrics {
    /// Total messages forwarded
    pub messages_forwarded: u64,
    /// Total bytes forwarded
    pub bytes_forwarded: u64,
    /// Average message size
    pub average_message_size: f64,
    /// Peak bandwidth usage
    pub peak_bandwidth: u64,
    /// Current bandwidth usage
    pub current_bandwidth: u64,
    /// Forwarding latency statistics
    pub forwarding_latency: Duration,
    /// Error count
    pub error_count: u64,
    /// Last update timestamp
    pub last_updated: Instant,
}

impl Default for SessionMetrics {
    fn default() -> Self {
        Self {
            messages_forwarded: 0,
            bytes_forwarded: 0,
            average_message_size: 0.0,
            peak_bandwidth: 0,
            current_bandwidth: 0,
            forwarding_latency: Duration::ZERO,
            error_count: 0,
            last_updated: Instant::now(),
        }
    }
}

/// Failover status and configuration
#[derive(Debug, Clone)]
pub struct FailoverStatus {
    /// Whether failover is enabled
    pub enabled: bool,
    /// Primary relay node ID
    pub primary_relay: Option<String>,
    /// Backup relay nodes
    pub backup_relays: Vec<String>,
    /// Current active relay
    pub active_relay: Option<String>,
    /// Failover trigger threshold (error rate)
    pub failover_threshold: f64,
    /// Last failover timestamp
    pub last_failover: Option<Instant>,
    /// Failover count
    pub failover_count: u32,
}

impl Default for FailoverStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            primary_relay: None,
            backup_relays: Vec::new(),
            active_relay: None,
            failover_threshold: 0.5,
            last_failover: None,
            failover_count: 0,
        }
    }
}

/// Main relay manager for handling relay node functionality
#[derive(Debug)]
pub struct RelayManager {
    /// Configuration for relay operations
    config: RelayConfig,
    /// Known relay nodes
    relay_nodes: Arc<RwLock<HashMap<String, RelayNodeInfo>>>,
    /// Active relay sessions
    active_sessions: Arc<RwLock<HashMap<String, Arc<RelaySession>>>>,
    /// Global bandwidth limiter
    global_bandwidth_limiter: BandwidthLimiter,
    /// Relay statistics
    stats: Arc<RwLock<RelayStats>>,
    /// Connection counter
    connection_counter: AtomicUsize,
}

impl RelayManager {
    /// Create a new relay manager
    pub fn new() -> Self {
        let config = RelayConfig::default();
        Self::with_config(config)
    }

    /// Create a new relay manager with custom configuration
    pub fn with_config(config: RelayConfig) -> Self {
        Self {
            global_bandwidth_limiter: BandwidthLimiter::new(config.total_bandwidth_limit),
            config,
            relay_nodes: Arc::new(RwLock::new(HashMap::new())),
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RelayStats::default())),
            connection_counter: AtomicUsize::new(0),
        }
    }

    /// Register a relay node
    pub async fn register_relay_node(&self, node_info: RelayNodeInfo) -> Result<(), TransportError> {
        let mut nodes = self.relay_nodes.write().await;
        nodes.insert(node_info.node_id.clone(), node_info);
        Ok(())
    }

    /// Remove a relay node
    pub async fn remove_relay_node(&self, node_id: &str) -> Result<(), TransportError> {
        let mut nodes = self.relay_nodes.write().await;
        nodes.remove(node_id);
        Ok(())
    }

    /// Find the best relay node for a connection
    pub async fn find_best_relay(&self, _target_peer: &PeerId) -> Option<RelayNodeInfo> {
        let nodes = self.relay_nodes.read().await;
        
        if nodes.is_empty() {
            return None;
        }

        // Find the relay node with the highest selection score
        nodes.values()
            .max_by(|a, b| a.selection_score().partial_cmp(&b.selection_score()).unwrap_or(std::cmp::Ordering::Equal))
            .cloned()
    }

    /// Get all available relay nodes sorted by selection score
    pub async fn get_available_relays(&self) -> Vec<RelayNodeInfo> {
        let nodes = self.relay_nodes.read().await;
        let mut relay_list: Vec<RelayNodeInfo> = nodes.values().cloned().collect();
        
        // Sort by selection score (highest first)
        relay_list.sort_by(|a, b| {
            b.selection_score().partial_cmp(&a.selection_score()).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        relay_list
    }

    /// Create a new relay session
    pub async fn create_relay_session(
        &self,
        source_peer_id: PeerId,
        target_peer_id: PeerId,
    ) -> Result<Arc<RelaySession>, TransportError> {
        // Check connection limits
        let current_connections = self.connection_counter.load(Ordering::Relaxed);
        if current_connections >= self.config.max_connections {
            return Err(TransportError::ResourceLimitExceeded {
                resource: format!("Maximum relay connections ({}) exceeded", self.config.max_connections),
            });
        }

        let session = Arc::new(RelaySession::new(
            source_peer_id,
            target_peer_id,
            self.config.max_bandwidth_per_connection,
            self.config.max_messages_per_second,
        ));

        // Add to active sessions
        {
            let mut sessions = self.active_sessions.write().await;
            sessions.insert(session.session_id.clone(), session.clone());
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_connections += 1;
            stats.active_connections = self.connection_counter.fetch_add(1, Ordering::Relaxed) + 1;
        }

        Ok(session)
    }

    /// Create an enhanced relay session with traffic isolation
    pub async fn create_enhanced_relay_session(
        &self,
        source_peer_id: PeerId,
        target_peer_id: PeerId,
        isolation_context: TrafficIsolation,
    ) -> Result<Arc<EnhancedRelaySession>, TransportError> {
        // Check if connection is allowed in isolation context
        if !isolation_context.is_connection_allowed(&source_peer_id, &target_peer_id) {
            return Err(TransportError::ConnectionFailed {
                reason: "Connection not allowed in traffic isolation context".to_string(),
            });
        }

        let base_session = self.create_relay_session(source_peer_id, target_peer_id).await?;
        
        let (source_tx, _source_rx) = mpsc::channel(self.config.message_buffer_size);
        let (target_tx, _target_rx) = mpsc::channel(self.config.message_buffer_size);

        let enhanced_session = Arc::new(EnhancedRelaySession {
            base_session: (*base_session).clone(),
            isolation_context,
            source_endpoint: Some(source_tx),
            target_endpoint: Some(target_tx),
            performance_metrics: Arc::new(RwLock::new(SessionMetrics::default())),
            failover_status: Arc::new(RwLock::new(FailoverStatus::default())),
        });

        Ok(enhanced_session)
    }

    /// Remove a relay session
    pub async fn remove_relay_session(&self, session_id: &str) -> Result<(), TransportError> {
        let session = {
            let mut sessions = self.active_sessions.write().await;
            sessions.remove(session_id)
        };

        if let Some(session) = session {
            // Update statistics
            let mut stats = self.stats.write().await;
            stats.active_connections = self.connection_counter.fetch_sub(1, Ordering::Relaxed).saturating_sub(1);
            stats.bytes_relayed += session.bytes_transferred.load(Ordering::Relaxed);
            
            // Update average connection duration
            let duration = session.duration();
            let total_duration = stats.average_connection_duration.as_secs_f64() * (stats.total_connections - 1) as f64;
            stats.average_connection_duration = Duration::from_secs_f64(
                (total_duration + duration.as_secs_f64()) / stats.total_connections as f64
            );
        }

        Ok(())
    }

    /// Get relay session by ID
    pub async fn get_relay_session(&self, session_id: &str) -> Option<Arc<RelaySession>> {
        let sessions = self.active_sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Forward data between relay connections
    pub async fn forward_data(
        &self,
        session_id: &str,
        data: &[u8],
    ) -> Result<(), TransportError> {
        let session = self.get_relay_session(session_id).await
            .ok_or_else(|| TransportError::ConnectionNotFound)?;

        // Check message size limit
        if data.len() > self.config.max_message_size {
            return Err(TransportError::ResourceLimitExceeded {
                resource: format!("Message size ({}) exceeds limit ({})", data.len(), self.config.max_message_size),
            });
        }

        // Check rate limits
        {
            let mut rate_limiter = session.rate_limiter.lock().await;
            if !rate_limiter.can_send_message() {
                let mut stats = self.stats.write().await;
                stats.rate_limit_violations += 1;
                return Err(TransportError::ResourceLimitExceeded {
                    resource: "Message rate limit exceeded".to_string(),
                });
            }
        }

        // Check bandwidth limits
        if !self.global_bandwidth_limiter.can_send(data.len()).await {
            return Err(TransportError::ResourceLimitExceeded {
                resource: "Global bandwidth limit exceeded".to_string(),
            });
        }

        if !session.bandwidth_limiter.can_send(data.len()).await {
            return Err(TransportError::ResourceLimitExceeded {
                resource: "Session bandwidth limit exceeded".to_string(),
            });
        }

        // Update session statistics
        session.add_bytes_transferred(data.len() as u64);
        session.increment_message_count();

        // Update health status
        {
            let mut health = session.health_status.write().await;
            health.update_health(true, Some(Duration::from_millis(10)), None);
        }

        // In a real implementation, this would forward the data to the target peer
        // For now, we'll just validate the operation
        Ok(())
    }

    /// Forward data with enhanced security and isolation
    pub async fn forward_data_secure(
        &self,
        session: &Arc<EnhancedRelaySession>,
        data: &[u8],
        source_peer: &PeerId,
        target_peer: &PeerId,
    ) -> Result<(), TransportError> {
        let start_time = Instant::now();

        // Verify traffic isolation
        if !session.isolation_context.is_connection_allowed(source_peer, target_peer) {
            return Err(TransportError::ConnectionFailed {
                reason: "Connection not allowed by traffic isolation policy".to_string(),
            });
        }

        // Check data rate limits for this isolation context
        if data.len() as u64 > session.isolation_context.max_data_rate {
            return Err(TransportError::ResourceLimitExceeded {
                resource: "Data rate exceeds isolation context limit".to_string(),
            });
        }

        // Perform hop-by-hop encryption if configured
        let processed_data = if let Some(_encryption_key) = &session.isolation_context.encryption_key {
            // In a real implementation, this would encrypt the data
            // For now, we'll just pass through the data
            data.to_vec()
        } else {
            data.to_vec()
        };

        // Forward through the appropriate endpoint
        if let Some(endpoint) = &session.target_endpoint {
            if let Err(_) = endpoint.try_send(processed_data) {
                // Update error metrics
                let mut metrics = session.performance_metrics.write().await;
                metrics.error_count += 1;
                
                return Err(TransportError::ResourceLimitExceeded {
                    resource: "Target endpoint buffer full".to_string(),
                });
            }
        }

        // Update performance metrics
        {
            let mut metrics = session.performance_metrics.write().await;
            metrics.messages_forwarded += 1;
            metrics.bytes_forwarded += data.len() as u64;
            
            // Update average message size
            metrics.average_message_size = 
                (metrics.average_message_size * (metrics.messages_forwarded - 1) as f64 + data.len() as f64) 
                / metrics.messages_forwarded as f64;
            
            // Update forwarding latency
            let latency = start_time.elapsed();
            metrics.forwarding_latency = Duration::from_nanos(
                ((metrics.forwarding_latency.as_nanos() as f64 * 0.9) + (latency.as_nanos() as f64 * 0.1)) as u64
            );
            
            metrics.last_updated = Instant::now();
        }

        // Log traffic if enabled
        if session.isolation_context.enable_logging {
            println!("Relay traffic: {} -> {}, {} bytes", source_peer, target_peer, data.len());
        }

        Ok(())
    }

    /// Monitor and enforce resource limits across all sessions
    pub async fn enforce_resource_limits(&self) -> Result<(), TransportError> {
        let sessions = {
            let sessions_guard = self.active_sessions.read().await;
            sessions_guard.values().cloned().collect::<Vec<_>>()
        };

        let mut total_bandwidth_usage = 0u64;
        let mut sessions_to_throttle = Vec::new();

        // Calculate current resource usage
        for session in &sessions {
            let bytes_per_second = session.bytes_transferred.load(Ordering::Relaxed) / 
                session.created_at.elapsed().as_secs().max(1);
            total_bandwidth_usage += bytes_per_second;

            // Check if session needs throttling
            if bytes_per_second > self.config.max_bandwidth_per_connection {
                sessions_to_throttle.push(session.session_id.clone());
            }
        }

        // Update global bandwidth usage statistics
        {
            let mut stats = self.stats.write().await;
            stats.current_bandwidth_usage = total_bandwidth_usage;
            stats.peak_bandwidth_usage = stats.peak_bandwidth_usage.max(total_bandwidth_usage);
        }

        // Throttle sessions that exceed limits
        for session_id in sessions_to_throttle {
            if let Some(_session) = self.get_relay_session(&session_id).await {
                // In a real implementation, this would apply throttling
                println!("Throttling session {} due to bandwidth limit", session_id);
            }
        }

        Ok(())
    }

    /// Perform automatic failover for unhealthy connections
    pub async fn perform_automatic_failover(&self) -> Result<(), TransportError> {
        let sessions = {
            let sessions_guard = self.active_sessions.read().await;
            sessions_guard.values().cloned().collect::<Vec<_>>()
        };

        for session in sessions {
            let health = session.health_status.read().await;
            if health.needs_attention() {
                println!("Session {} needs attention: health={}, errors={}, latency={:?}", 
                    session.session_id, health.is_healthy, health.consecutive_failures, health.average_response_time);
                
                // In a real implementation, this would trigger failover to backup relays
                // For now, we'll just log the issue
            }
        }

        Ok(())
    }

    /// Authenticate a relay connection request
    pub async fn authenticate_connection(&self, auth_token: Option<&str>) -> Result<bool, TransportError> {
        if !self.config.require_auth {
            return Ok(true);
        }

        match (&self.config.auth_token, auth_token) {
            (Some(expected), Some(provided)) => Ok(expected == provided),
            (None, _) => Ok(true),
            (Some(_), None) => {
                let mut stats = self.stats.write().await;
                stats.auth_failures += 1;
                Ok(false)
            }
        }
    }

    /// Clean up idle relay sessions
    pub async fn cleanup_idle_sessions(&self) -> Result<(), TransportError> {
        let mut sessions_to_remove = Vec::new();
        
        {
            let sessions = self.active_sessions.read().await;
            for (session_id, session) in sessions.iter() {
                if session.is_idle(self.config.idle_timeout) {
                    sessions_to_remove.push(session_id.clone());
                }
            }
        }

        for session_id in sessions_to_remove {
            self.remove_relay_session(&session_id).await?;
        }

        Ok(())
    }

    /// Get current relay statistics
    pub async fn get_stats(&self) -> RelayStats {
        let stats = self.stats.read().await;
        let mut current_stats = stats.clone();
        
        // Update current active connections
        current_stats.active_connections = self.connection_counter.load(Ordering::Relaxed);
        
        current_stats
    }

    /// Update relay node statistics
    pub async fn update_node_stats(&self, node_id: &str, success: bool, latency: Option<Duration>) {
        let mut nodes = self.relay_nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.update_reliability(success);
            if let Some(lat) = latency {
                node.latency = Some(lat);
            }
        }
    }

    /// Start relay service on the specified address
    pub async fn start_relay_service(&self, bind_addr: SocketAddr) -> Result<(), TransportError> {
        let listener = TcpListener::bind(bind_addr).await
            .map_err(|e| TransportError::Io(e))?;

        println!("Relay service started on {}", bind_addr);

        // Start cleanup task
        let manager_clone = Arc::new(self.clone());
        let cleanup_manager = manager_clone.clone();
        tokio::spawn(async move {
            let mut interval = interval(cleanup_manager.config.health_check_interval);
            loop {
                interval.tick().await;
                if let Err(e) = cleanup_manager.cleanup_idle_sessions().await {
                    eprintln!("Error during relay cleanup: {}", e);
                }
            }
        });

        // Accept incoming connections
        loop {
            match listener.accept().await {
                Ok((stream, addr)) => {
                    let manager = manager_clone.clone();
                    tokio::spawn(async move {
                        if let Err(e) = manager.handle_relay_connection(stream, addr).await {
                            eprintln!("Error handling relay connection from {}: {}", addr, e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error accepting relay connection: {}", e);
                }
            }
        }
    }

    /// Handle an incoming relay connection
    async fn handle_relay_connection(&self, stream: TcpStream, addr: SocketAddr) -> Result<(), TransportError> {
        println!("New relay connection from {}", addr);

        // Upgrade to WebSocket
        let ws_stream = timeout(self.config.connection_timeout, accept_async(stream))
            .await
            .map_err(|_| TransportError::ConnectionTimeout {
                timeout: self.config.connection_timeout,
            })?
            .map_err(|e| TransportError::WebSocket(e.to_string()))?;

        // Handle WebSocket relay protocol
        self.handle_websocket_relay(ws_stream, addr).await
    }

    /// Handle WebSocket relay protocol
    async fn handle_websocket_relay(
        &self,
        mut ws_stream: WebSocketStream<TcpStream>,
        _addr: SocketAddr,
    ) -> Result<(), TransportError> {
        use tokio_tungstenite::tungstenite::Message;

        // Wait for initial relay request
        match timeout(Duration::from_secs(10), ws_stream.next()).await {
            Ok(Some(Ok(Message::Text(_request_text)))) => {
                // Parse relay request (this would use the RelayMessage from websocket.rs)
                // For now, we'll create a simple session
                let session = self.create_relay_session(
                    "source_peer".to_string(),
                    "target_peer".to_string(),
                ).await?;

                // Send success response
                let response = format!(r#"{{"type":"ConnectResponse","success":true,"session_id":"{}"}}"#, session.session_id);
                ws_stream.send(Message::Text(response)).await
                    .map_err(|e| TransportError::WebSocket(e.to_string()))?;

                // Handle relay data forwarding
                while let Some(message) = ws_stream.next().await {
                    match message {
                        Ok(Message::Binary(data)) => {
                            if let Err(e) = self.forward_data(&session.session_id, &data).await {
                                eprintln!("Error forwarding relay data: {}", e);
                                break;
                            }
                        }
                        Ok(Message::Close(_)) => {
                            break;
                        }
                        Err(e) => {
                            eprintln!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {
                            // Ignore other message types
                        }
                    }
                }

                // Clean up session
                self.remove_relay_session(&session.session_id).await?;
            }
            _ => {
                return Err(TransportError::NegotiationTimeout);
            }
        }

        Ok(())
    }

    /// Start comprehensive health monitoring for relay nodes and sessions
    pub fn start_health_monitoring(manager: Arc<RelayManager>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(manager.config.health_check_interval);
            loop {
                interval.tick().await;
                
                // Perform health checks on all relay nodes
                let nodes = {
                    let nodes_guard = manager.relay_nodes.read().await;
                    nodes_guard.values().cloned().collect::<Vec<_>>()
                };

                for node in nodes {
                    // In a real implementation, this would ping the relay node
                    // and update its health status
                    let success = true; // Placeholder
                    manager.update_node_stats(&node.node_id, success, None).await;
                }

                // Enforce resource limits
                if let Err(e) = manager.enforce_resource_limits().await {
                    eprintln!("Error enforcing resource limits: {}", e);
                }

                // Perform automatic failover checks
                if let Err(e) = manager.perform_automatic_failover().await {
                    eprintln!("Error during automatic failover: {}", e);
                }

                // Clean up idle sessions
                if let Err(e) = manager.cleanup_idle_sessions().await {
                    eprintln!("Error during session cleanup: {}", e);
                }
            }
        })
    }

    /// Get detailed performance metrics for all active sessions
    pub async fn get_session_metrics(&self) -> HashMap<String, SessionMetrics> {
        let mut metrics_map = HashMap::new();
        
        let sessions = self.active_sessions.read().await;
        for (session_id, session) in sessions.iter() {
            // Create basic metrics from session data
            let bytes_transferred = session.bytes_transferred.load(Ordering::Relaxed);
            let message_count = session.message_count.load(Ordering::Relaxed);
            let duration = session.duration().as_secs().max(1);
            
            let metrics = SessionMetrics {
                messages_forwarded: message_count,
                bytes_forwarded: bytes_transferred,
                average_message_size: if message_count > 0 { 
                    bytes_transferred as f64 / message_count as f64 
                } else { 
                    0.0 
                },
                peak_bandwidth: bytes_transferred / duration,
                current_bandwidth: bytes_transferred / duration,
                forwarding_latency: Duration::from_millis(10), // Placeholder
                error_count: 0, // Would be tracked in real implementation
                last_updated: Instant::now(),
            };
            
            metrics_map.insert(session_id.clone(), metrics);
        }
        
        metrics_map
    }

    /// Generate comprehensive relay health report
    pub async fn generate_health_report(&self) -> RelayHealthReport {
        let stats = self.get_stats().await;
        let session_metrics = self.get_session_metrics().await;
        let relay_nodes = self.get_available_relays().await;
        
        let total_sessions = session_metrics.len();
        let healthy_sessions = session_metrics.values()
            .filter(|m| m.error_count == 0)
            .count();
        
        let average_latency = if !session_metrics.is_empty() {
            let total_latency: Duration = session_metrics.values()
                .map(|m| m.forwarding_latency)
                .sum();
            total_latency / session_metrics.len() as u32
        } else {
            Duration::ZERO
        };
        
        RelayHealthReport {
            timestamp: SystemTime::now(),
            total_relay_nodes: relay_nodes.len(),
            healthy_relay_nodes: relay_nodes.iter().filter(|n| n.reliability_score > 80).count(),
            total_sessions,
            healthy_sessions,
            total_bandwidth_usage: stats.current_bandwidth_usage,
            average_session_latency: average_latency,
            error_rate: if total_sessions > 0 { 
                (total_sessions - healthy_sessions) as f64 / total_sessions as f64 
            } else { 
                0.0 
            },
            resource_utilization: ResourceUtilization {
                connection_usage: stats.active_connections as f64 / self.config.max_connections as f64,
                bandwidth_usage: stats.current_bandwidth_usage as f64 / self.config.total_bandwidth_limit as f64,
                memory_usage: 0.0, // Would be calculated in real implementation
            },
        }
    }
}

impl Clone for RelayManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            relay_nodes: self.relay_nodes.clone(),
            active_sessions: self.active_sessions.clone(),
            global_bandwidth_limiter: BandwidthLimiter::new(self.config.total_bandwidth_limit),
            stats: self.stats.clone(),
            connection_counter: AtomicUsize::new(self.connection_counter.load(Ordering::Relaxed)),
        }
    }
}

impl Default for RelayManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive health report for relay system
#[derive(Debug, Clone)]
pub struct RelayHealthReport {
    /// Report generation timestamp
    pub timestamp: SystemTime,
    /// Total number of relay nodes
    pub total_relay_nodes: usize,
    /// Number of healthy relay nodes
    pub healthy_relay_nodes: usize,
    /// Total active sessions
    pub total_sessions: usize,
    /// Number of healthy sessions
    pub healthy_sessions: usize,
    /// Current total bandwidth usage
    pub total_bandwidth_usage: u64,
    /// Average session latency
    pub average_session_latency: Duration,
    /// Overall error rate
    pub error_rate: f64,
    /// Resource utilization metrics
    pub resource_utilization: ResourceUtilization,
}

/// Resource utilization metrics
#[derive(Debug, Clone)]
pub struct ResourceUtilization {
    /// Connection pool utilization (0.0 to 1.0)
    pub connection_usage: f64,
    /// Bandwidth utilization (0.0 to 1.0)
    pub bandwidth_usage: f64,
    /// Memory utilization (0.0 to 1.0)
    pub memory_usage: f64,
}

impl RelayHealthReport {
    /// Check if the relay system is healthy overall
    pub fn is_healthy(&self) -> bool {
        self.error_rate < 0.1 && 
        self.resource_utilization.connection_usage < 0.9 &&
        self.resource_utilization.bandwidth_usage < 0.8 &&
        self.healthy_relay_nodes > 0
    }

    /// Get health score (0-100)
    pub fn health_score(&self) -> u8 {
        let mut score = 100.0;
        
        // Penalize high error rate
        score -= self.error_rate * 50.0;
        
        // Penalize high resource utilization
        score -= self.resource_utilization.connection_usage * 20.0;
        score -= self.resource_utilization.bandwidth_usage * 20.0;
        
        // Penalize if no healthy relay nodes
        if self.healthy_relay_nodes == 0 {
            score -= 30.0;
        }
        
        // Penalize high latency
        if self.average_session_latency > Duration::from_millis(200) {
            score -= 10.0;
        }
        
        score.max(0.0).min(100.0) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_relay_config_default() {
        let config = RelayConfig::default();
        assert_eq!(config.max_connections, 100);
        assert_eq!(config.max_bandwidth_per_connection, 1024 * 1024);
        assert!(!config.require_auth);
    }

    #[test]
    fn test_relay_node_info_creation() {
        let node = RelayNodeInfo::new(
            "test-node".to_string(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            1024 * 1024 * 10, // 10MB/s
        );

        assert_eq!(node.node_id, "test-node");
        assert_eq!(node.reliability_score, 100);
        assert!(node.selection_score() > 0.0);
    }

    #[test]
    fn test_relay_node_selection_score() {
        let mut node = RelayNodeInfo::new(
            "test-node".to_string(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            1024 * 1024 * 10,
        );

        let initial_score = node.selection_score();
        
        // Update reliability negatively
        node.update_reliability(false);
        assert!(node.selection_score() < initial_score);
        
        // Update reliability positively
        node.update_reliability(true);
        assert!(node.selection_score() > node.selection_score());
    }

    #[tokio::test]
    async fn test_relay_manager_creation() {
        let manager = RelayManager::new();
        let stats = manager.get_stats().await;
        
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_connections, 0);
    }

    #[tokio::test]
    async fn test_relay_node_registration() {
        let manager = RelayManager::new();
        let node = RelayNodeInfo::new(
            "test-node".to_string(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            1024 * 1024 * 10,
        );

        let result = manager.register_relay_node(node.clone()).await;
        assert!(result.is_ok());

        let relays = manager.get_available_relays().await;
        assert_eq!(relays.len(), 1);
        assert_eq!(relays[0].node_id, "test-node");
    }

    #[tokio::test]
    async fn test_relay_session_creation() {
        let manager = RelayManager::new();
        
        let session = manager.create_relay_session(
            "source-peer".to_string(),
            "target-peer".to_string(),
        ).await;

        assert!(session.is_ok());
        let session = session.unwrap();
        assert_eq!(session.source_peer_id, "source-peer");
        assert_eq!(session.target_peer_id, "target-peer");

        let stats = manager.get_stats().await;
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.total_connections, 1);
    }

    #[tokio::test]
    async fn test_enhanced_relay_session_creation() {
        let manager = RelayManager::new();
        
        let mut isolation_context = TrafficIsolation::new("test-context".to_string());
        isolation_context.add_allowed_source("source-peer".to_string());
        isolation_context.add_allowed_target("target-peer".to_string());
        
        let session = manager.create_enhanced_relay_session(
            "source-peer".to_string(),
            "target-peer".to_string(),
            isolation_context,
        ).await;

        assert!(session.is_ok());
        let session = session.unwrap();
        assert_eq!(session.base_session.source_peer_id, "source-peer");
        assert_eq!(session.base_session.target_peer_id, "target-peer");
        assert_eq!(session.isolation_context.context_id, "test-context");
    }

    #[tokio::test]
    async fn test_relay_session_cleanup() {
        let manager = RelayManager::new();
        
        let session = manager.create_relay_session(
            "source-peer".to_string(),
            "target-peer".to_string(),
        ).await.unwrap();

        let session_id = session.session_id.clone();
        
        let result = manager.remove_relay_session(&session_id).await;
        assert!(result.is_ok());

        let stats = manager.get_stats().await;
        assert_eq!(stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_bandwidth_limiter() {
        let limiter = BandwidthLimiter::new(1000); // 1000 bytes per second
        
        // Should allow small transfers
        assert!(limiter.can_send(100).await);
        
        // Should not allow transfers exceeding limit
        assert!(!limiter.can_send(2000).await);
    }

    #[tokio::test]
    async fn test_relay_authentication() {
        let mut config = RelayConfig::default();
        config.require_auth = true;
        config.auth_token = Some("secret-token".to_string());
        
        let manager = RelayManager::with_config(config);
        
        // Valid token should succeed
        let result = manager.authenticate_connection(Some("secret-token")).await;
        assert!(result.is_ok() && result.unwrap());
        
        // Invalid token should fail
        let result = manager.authenticate_connection(Some("wrong-token")).await;
        assert!(result.is_ok() && !result.unwrap());
        
        // Missing token should fail when required
        let result = manager.authenticate_connection(None).await;
        assert!(result.is_ok() && !result.unwrap());
    }

    #[tokio::test]
    async fn test_connection_limits() {
        let mut config = RelayConfig::default();
        config.max_connections = 2;
        
        let manager = RelayManager::with_config(config);
        
        // First two connections should succeed
        let session1 = manager.create_relay_session("peer1".to_string(), "peer2".to_string()).await;
        assert!(session1.is_ok());
        
        let session2 = manager.create_relay_session("peer3".to_string(), "peer4".to_string()).await;
        assert!(session2.is_ok());
        
        // Third connection should fail due to limit
        let session3 = manager.create_relay_session("peer5".to_string(), "peer6".to_string()).await;
        assert!(session3.is_err());
    }

    #[tokio::test]
    async fn test_data_forwarding() {
        let manager = RelayManager::new();
        
        let session = manager.create_relay_session(
            "source-peer".to_string(),
            "target-peer".to_string(),
        ).await.unwrap();

        let data = b"test message";
        let result = manager.forward_data(&session.session_id, data).await;
        assert!(result.is_ok());

        // Check that bytes were counted
        assert_eq!(session.bytes_transferred.load(Ordering::Relaxed), data.len() as u64);
        assert_eq!(session.message_count.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn test_relay_session_idle_detection() {
        let session = RelaySession::new(
            "source".to_string(),
            "target".to_string(),
            1024 * 1024,
            100,
        );

        // Should not be idle immediately
        assert!(!session.is_idle(Duration::from_secs(300)));
        
        // Would be idle if enough time passed (we can't easily test this without waiting)
        // In a real test, we might use a mock time source
    }

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(5); // 5 messages per second
        
        // Should allow first few messages
        assert!(limiter.can_send_message());
        assert!(limiter.can_send_message());
        assert!(limiter.can_send_message());
        assert!(limiter.can_send_message());
        assert!(limiter.can_send_message());
        
        // Should reject the 6th message
        assert!(!limiter.can_send_message());
        
        // Check current rate
        assert!(limiter.current_rate() >= 5.0);
    }

    #[test]
    fn test_connection_health() {
        let mut health = ConnectionHealth::new();
        
        // Initially healthy
        assert!(health.is_healthy);
        assert!(!health.needs_attention());
        
        // Update with failure
        health.update_health(false, None, Some("Test error".to_string()));
        assert_eq!(health.consecutive_failures, 1);
        assert!(health.error_rate > 0.0);
        
        // Multiple failures should make it unhealthy
        health.update_health(false, None, None);
        health.update_health(false, None, None);
        assert!(!health.is_healthy);
        assert!(health.needs_attention());
        
        // Recovery
        health.update_health(true, Some(Duration::from_millis(50)), None);
        assert_eq!(health.consecutive_failures, 0);
    }

    #[test]
    fn test_traffic_isolation() {
        let mut isolation = TrafficIsolation::new("test-context".to_string());
        
        // Initially allows all connections
        assert!(isolation.is_connection_allowed(&"peer1".to_string(), &"peer2".to_string()));
        
        // Add restrictions
        isolation.add_allowed_source("peer1".to_string());
        isolation.add_allowed_target("peer2".to_string());
        
        // Should still allow the specific connection
        assert!(isolation.is_connection_allowed(&"peer1".to_string(), &"peer2".to_string()));
        
        // Should not allow other connections
        assert!(!isolation.is_connection_allowed(&"peer3".to_string(), &"peer2".to_string()));
        assert!(!isolation.is_connection_allowed(&"peer1".to_string(), &"peer3".to_string()));
    }

    #[tokio::test]
    async fn test_resource_limit_enforcement() {
        let manager = RelayManager::new();
        
        // Create a session
        let session = manager.create_relay_session(
            "source-peer".to_string(),
            "target-peer".to_string(),
        ).await.unwrap();

        // Simulate high bandwidth usage
        session.add_bytes_transferred(1024 * 1024 * 100); // 100MB
        
        // Enforce limits (should not error in this test setup)
        let result = manager.enforce_resource_limits().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_report_generation() {
        let manager = RelayManager::new();
        
        // Add a relay node
        let node = RelayNodeInfo::new(
            "test-node".to_string(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            1024 * 1024 * 10,
        );
        manager.register_relay_node(node).await.unwrap();
        
        // Create a session
        let _session = manager.create_relay_session(
            "source-peer".to_string(),
            "target-peer".to_string(),
        ).await.unwrap();
        
        // Generate health report
        let report = manager.generate_health_report().await;
        
        assert_eq!(report.total_relay_nodes, 1);
        assert_eq!(report.total_sessions, 1);
        assert!(report.is_healthy());
        assert!(report.health_score() > 80);
    }
}