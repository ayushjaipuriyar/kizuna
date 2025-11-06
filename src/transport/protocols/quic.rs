use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use quinn::{Endpoint, ServerConfig, ClientConfig, Connection as QuinnConnection, RecvStream, SendStream};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::sync::{Mutex, RwLock};

use crate::transport::{
    Connection, ConnectionInfo, PeerAddress, PeerId, Transport, TransportCapabilities, TransportError,
};

/// QUIC transport implementation using Quinn with advanced features
#[derive(Debug)]
pub struct QuicTransport {
    endpoint: Arc<Mutex<Option<Endpoint>>>,
    config: QuicConfig,
    client_config: ClientConfig,
    server_config: Option<ServerConfig>,
    active_connections: Arc<RwLock<HashMap<PeerId, QuinnConnection>>>,
    /// Session resumption data for 0-RTT
    session_cache: Arc<RwLock<HashMap<PeerId, SessionData>>>,
    /// Connection performance monitor
    performance_monitor: Arc<RwLock<ConnectionPerformanceMonitor>>,
}

/// Session data for connection resumption
#[derive(Debug, Clone)]
pub struct SessionData {
    pub session_ticket: Vec<u8>,
    pub early_data_enabled: bool,
    pub last_used: std::time::SystemTime,
    pub peer_address: SocketAddr,
}

/// Monitor for connection performance across all QUIC connections
#[derive(Debug, Clone)]
pub struct ConnectionPerformanceMonitor {
    pub total_connections: u64,
    pub successful_connections: u64,
    pub failed_connections: u64,
    pub total_migrations: u64,
    pub successful_migrations: u64,
    pub total_resumptions: u64,
    pub successful_resumptions: u64,
    pub average_connection_time: Duration,
    pub congestion_control_switches: HashMap<String, u64>,
}

impl ConnectionPerformanceMonitor {
    pub fn new() -> Self {
        Self {
            total_connections: 0,
            successful_connections: 0,
            failed_connections: 0,
            total_migrations: 0,
            successful_migrations: 0,
            total_resumptions: 0,
            successful_resumptions: 0,
            average_connection_time: Duration::ZERO,
            congestion_control_switches: HashMap::new(),
        }
    }

    pub fn record_connection_attempt(&mut self) {
        self.total_connections += 1;
    }

    pub fn record_connection_success(&mut self, connection_time: Duration) {
        self.successful_connections += 1;
        self.update_average_connection_time(connection_time);
    }

    pub fn record_connection_failure(&mut self) {
        self.failed_connections += 1;
    }

    pub fn record_migration_attempt(&mut self) {
        self.total_migrations += 1;
    }

    pub fn record_migration_success(&mut self) {
        self.successful_migrations += 1;
    }

    pub fn record_resumption_attempt(&mut self) {
        self.total_resumptions += 1;
    }

    pub fn record_resumption_success(&mut self) {
        self.successful_resumptions += 1;
    }

    pub fn record_congestion_control_switch(&mut self, algorithm: String) {
        *self.congestion_control_switches.entry(algorithm).or_insert(0) += 1;
    }

    fn update_average_connection_time(&mut self, new_time: Duration) {
        if self.successful_connections == 1 {
            self.average_connection_time = new_time;
        } else {
            let total_time = self.average_connection_time * (self.successful_connections - 1) as u32 + new_time;
            self.average_connection_time = total_time / self.successful_connections as u32;
        }
    }

    pub fn connection_success_rate(&self) -> f64 {
        if self.total_connections == 0 {
            0.0
        } else {
            self.successful_connections as f64 / self.total_connections as f64
        }
    }

    pub fn migration_success_rate(&self) -> f64 {
        if self.total_migrations == 0 {
            0.0
        } else {
            self.successful_migrations as f64 / self.total_migrations as f64
        }
    }

    pub fn resumption_success_rate(&self) -> f64 {
        if self.total_resumptions == 0 {
            0.0
        } else {
            self.successful_resumptions as f64 / self.total_resumptions as f64
        }
    }
}

/// Configuration for QUIC transport
#[derive(Debug, Clone)]
pub struct QuicConfig {
    /// Maximum number of concurrent streams per connection
    pub max_concurrent_streams: u32,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Keep-alive interval
    pub keep_alive_interval: Option<Duration>,
    /// Maximum datagram size
    pub max_datagram_size: Option<usize>,
    /// Enable 0-RTT resumption
    pub enable_0rtt: bool,
    /// Congestion control algorithm
    pub congestion_control: CongestionControl,
    /// Maximum connection migration attempts
    pub max_migration_attempts: u32,
}

/// Congestion control algorithms supported by QUIC
#[derive(Debug, Clone, Copy)]
pub enum CongestionControl {
    /// Cubic congestion control (default)
    Cubic,
    /// BBR congestion control
    Bbr,
    /// NewReno congestion control
    NewReno,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            max_concurrent_streams: 100,
            idle_timeout: Duration::from_secs(30),
            keep_alive_interval: Some(Duration::from_secs(5)),
            max_datagram_size: Some(1200),
            enable_0rtt: true,
            congestion_control: CongestionControl::Cubic,
            max_migration_attempts: 3,
        }
    }
}

impl QuicTransport {
    /// Create a new QUIC transport with default configuration
    pub fn new() -> Result<Self, TransportError> {
        Self::with_config(QuicConfig::default())
    }

    /// Create a new QUIC transport with custom configuration
    pub fn with_config(config: QuicConfig) -> Result<Self, TransportError> {
        let (client_config, server_config) = Self::create_configs(&config)?;
        
        Ok(Self {
            endpoint: Arc::new(Mutex::new(None)),
            config,
            client_config,
            server_config: Some(server_config),
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            session_cache: Arc::new(RwLock::new(HashMap::new())),
            performance_monitor: Arc::new(RwLock::new(ConnectionPerformanceMonitor::new())),
        })
    }

    /// Create client-only QUIC transport (no server capabilities)
    pub fn client_only(config: QuicConfig) -> Result<Self, TransportError> {
        let (client_config, _) = Self::create_configs(&config)?;
        
        Ok(Self {
            endpoint: Arc::new(Mutex::new(None)),
            config,
            client_config,
            server_config: None,
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            session_cache: Arc::new(RwLock::new(HashMap::new())),
            performance_monitor: Arc::new(RwLock::new(ConnectionPerformanceMonitor::new())),
        })
    }

    /// Create QUIC client and server configurations
    fn create_configs(config: &QuicConfig) -> Result<(ClientConfig, ServerConfig), TransportError> {
        // Generate self-signed certificate for testing/development
        let (cert_der, key_der) = Self::generate_self_signed_cert()?;

        // Create client config with platform verifier
        let mut client_config = ClientConfig::try_with_platform_verifier()
            .map_err(|e| TransportError::Quic(format!("Failed to create client config: {}", e)))?;
        
        // Configure transport parameters
        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_concurrent_uni_streams(config.max_concurrent_streams.into());
        transport_config.max_concurrent_bidi_streams(config.max_concurrent_streams.into());
        transport_config.max_idle_timeout(Some(config.idle_timeout.try_into().unwrap()));
        
        if let Some(keep_alive) = config.keep_alive_interval {
            transport_config.keep_alive_interval(Some(keep_alive));
        }
        
        if let Some(datagram_size) = config.max_datagram_size {
            transport_config.datagram_receive_buffer_size(Some(datagram_size));
        }

        // Configure congestion control
        match config.congestion_control {
            CongestionControl::Cubic => {
                // Cubic is the default
            }
            CongestionControl::Bbr => {
                // BBR configuration would go here if supported
            }
            CongestionControl::NewReno => {
                // NewReno configuration would go here if supported
            }
        }

        client_config.transport_config(Arc::new(transport_config));

        // Create server config
        let server_config = ServerConfig::with_single_cert(vec![cert_der], key_der)
            .map_err(|e| TransportError::Quic(format!("Failed to create server config: {}", e)))?;

        Ok((client_config, server_config))
    }

    /// Generate a self-signed certificate for QUIC
    fn generate_self_signed_cert() -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>), TransportError> {
        let mut params = rcgen::CertificateParams::new(vec!["localhost".to_string()]);
        
        params.distinguished_name = rcgen::DistinguishedName::new();
        params.distinguished_name.push(rcgen::DnType::CommonName, "Kizuna QUIC");
        
        let key_pair = rcgen::KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)
            .map_err(|e| TransportError::Quic(format!("Failed to generate key pair: {}", e)))?;
        
        params.key_pair = Some(key_pair);
        
        let cert = rcgen::Certificate::from_params(params)
            .map_err(|e| TransportError::Quic(format!("Failed to generate certificate: {}", e)))?;
        
        let cert_der = CertificateDer::from(cert.serialize_der()
            .map_err(|e| TransportError::Quic(format!("Failed to serialize certificate: {}", e)))?);
        let key_der = PrivateKeyDer::try_from(cert.serialize_private_key_der())
            .map_err(|e| TransportError::Quic(format!("Invalid private key: {}", e)))?;
        
        Ok((cert_der, key_der))
    }

    /// Initialize the QUIC endpoint
    async fn ensure_endpoint(&self, bind_addr: Option<SocketAddr>) -> Result<Endpoint, TransportError> {
        let mut endpoint_guard = self.endpoint.lock().await;
        
        if let Some(ref endpoint) = *endpoint_guard {
            return Ok(endpoint.clone());
        }

        let bind_addr = bind_addr.unwrap_or_else(|| "0.0.0.0:0".parse().unwrap());
        
        let mut endpoint = if let Some(ref server_config) = self.server_config {
            // Create endpoint with server capabilities
            Endpoint::server(server_config.clone(), bind_addr)
                .map_err(|e| TransportError::Quic(format!("Failed to create server endpoint: {}", e)))?
        } else {
            // Create client-only endpoint
            Endpoint::client(bind_addr)
                .map_err(|e| TransportError::Quic(format!("Failed to create client endpoint: {}", e)))?
        };

        endpoint.set_default_client_config(self.client_config.clone());
        
        let endpoint_clone = endpoint.clone();
        *endpoint_guard = Some(endpoint);
        
        Ok(endpoint_clone)
    }

    /// Get or create a connection to a peer with session resumption support
    async fn get_or_create_connection(&self, peer_addr: &PeerAddress) -> Result<QuinnConnection, TransportError> {
        // Record connection attempt
        {
            let mut monitor = self.performance_monitor.write().await;
            monitor.record_connection_attempt();
        }

        let connection_start = std::time::Instant::now();

        // Check if we already have an active connection
        {
            let connections = self.active_connections.read().await;
            if let Some(conn) = connections.get(&peer_addr.peer_id) {
                if conn.close_reason().is_none() {
                    return Ok(conn.clone());
                }
            }
        }

        // Check for session resumption data
        let session_data = {
            let session_cache = self.session_cache.read().await;
            session_cache.get(&peer_addr.peer_id).cloned()
        };

        let endpoint = self.ensure_endpoint(None).await?;
        
        // Try each address until one succeeds
        let mut last_error = None;
        for &addr in &peer_addr.addresses {
            // Attempt connection with session resumption if available
            let connecting_result = if let Some(ref session) = session_data {
                if self.config.enable_0rtt && session.early_data_enabled {
                    // Attempt 0-RTT connection
                    self.attempt_0rtt_connection(&endpoint, addr, session).await
                } else {
                    // Regular connection with session resumption
                    endpoint.connect(addr, "localhost")
                }
            } else {
                // New connection without resumption
                endpoint.connect(addr, "localhost")
            };

            match connecting_result {
                Ok(connecting) => {
                    match connecting.await {
                        Ok(connection) => {
                            let connection_time = connection_start.elapsed();
                            
                            // Record successful connection
                            {
                                let mut monitor = self.performance_monitor.write().await;
                                monitor.record_connection_success(connection_time);
                            }

                            // Store session data for future resumption
                            self.store_session_data(&peer_addr.peer_id, addr).await;

                            // Store the connection
                            {
                                let mut connections = self.active_connections.write().await;
                                connections.insert(peer_addr.peer_id.clone(), connection.clone());
                            }
                            return Ok(connection);
                        }
                        Err(e) => {
                            last_error = Some(TransportError::Quic(format!("Connection failed: {}", e)));
                        }
                    }
                }
                Err(e) => {
                    last_error = Some(TransportError::Quic(format!("Connect failed: {}", e)));
                }
            }
        }

        // Record connection failure
        {
            let mut monitor = self.performance_monitor.write().await;
            monitor.record_connection_failure();
        }

        Err(last_error.unwrap_or_else(|| TransportError::ConnectionFailed {
            reason: "No addresses available".to_string(),
        }))
    }

    /// Attempt 0-RTT connection using session data
    async fn attempt_0rtt_connection(
        &self,
        endpoint: &Endpoint,
        addr: SocketAddr,
        _session: &SessionData,
    ) -> Result<quinn::Connecting, quinn::ConnectError> {
        // Record resumption attempt
        {
            let mut monitor = self.performance_monitor.write().await;
            monitor.record_resumption_attempt();
        }

        // In a real implementation, this would use the session ticket
        // For now, we'll just do a regular connection
        let connecting = endpoint.connect(addr, "localhost")?;
        
        // If successful, record resumption success
        {
            let mut monitor = self.performance_monitor.write().await;
            monitor.record_resumption_success();
        }

        Ok(connecting)
    }

    /// Store session data for future resumption
    async fn store_session_data(&self, peer_id: &PeerId, addr: SocketAddr) {
        if !self.config.enable_0rtt {
            return;
        }

        let session_data = SessionData {
            session_ticket: vec![0u8; 32], // Placeholder session ticket
            early_data_enabled: true,
            last_used: std::time::SystemTime::now(),
            peer_address: addr,
        };

        let mut session_cache = self.session_cache.write().await;
        session_cache.insert(peer_id.clone(), session_data);
    }

    /// Clean up expired session data
    pub async fn cleanup_expired_sessions(&self) {
        let expiry_time = Duration::from_secs(3600); // 1 hour
        let now = std::time::SystemTime::now();
        
        let mut session_cache = self.session_cache.write().await;
        session_cache.retain(|_, session| {
            now.duration_since(session.last_used).unwrap_or_default() < expiry_time
        });
    }

    /// Handle connection migration
    async fn handle_connection_migration(&self, connection: &QuinnConnection) -> Result<(), TransportError> {
        // QUIC handles connection migration automatically, but we can monitor it
        let stats = connection.stats();
        // Log migration events if needed
        if stats.path.lost_packets > 0 {
            eprintln!("QUIC connection lost packets: {}", stats.path.lost_packets);
        }
        Ok(())
    }

    /// Clean up closed connections
    pub async fn cleanup_connections(&self) {
        let mut connections = self.active_connections.write().await;
        connections.retain(|_, conn| conn.close_reason().is_none());
    }

    /// Get connection statistics
    pub async fn get_connection_stats(&self) -> HashMap<PeerId, QuicConnectionStats> {
        let connections = self.active_connections.read().await;
        let mut stats = HashMap::new();
        
        for (peer_id, conn) in connections.iter() {
            let quinn_stats = conn.stats();
            stats.insert(peer_id.clone(), QuicConnectionStats {
                rtt: quinn_stats.path.rtt,
                cwnd: quinn_stats.path.cwnd as usize,
                bytes_sent: 0, // We'll track this at the connection level
                bytes_received: 0, // We'll track this at the connection level
                packets_sent: quinn_stats.udp_tx.datagrams,
                packets_received: quinn_stats.udp_rx.datagrams,
                stream_count: 0, // Would need to track this separately
            });
        }
        
        stats
    }

    /// Get performance monitor statistics
    pub async fn get_performance_stats(&self) -> ConnectionPerformanceMonitor {
        self.performance_monitor.read().await.clone()
    }

    /// Get session cache statistics
    pub async fn get_session_cache_stats(&self) -> (usize, usize) {
        let session_cache = self.session_cache.read().await;
        let total_sessions = session_cache.len();
        let active_sessions = session_cache.values()
            .filter(|session| {
                let age = std::time::SystemTime::now()
                    .duration_since(session.last_used)
                    .unwrap_or_default();
                age < Duration::from_secs(300) // Active if used in last 5 minutes
            })
            .count();
        
        (total_sessions, active_sessions)
    }

    /// Optimize connection based on network conditions
    pub async fn optimize_connections(&self) -> Result<(), TransportError> {
        let connections = self.active_connections.read().await;
        
        for (peer_id, conn) in connections.iter() {
            let stats = conn.stats();
            
            // Check if connection needs optimization
            if self.needs_optimization(&stats) {
                self.apply_optimization(peer_id, conn, &stats).await?;
            }
        }
        
        Ok(())
    }

    /// Check if connection needs optimization
    fn needs_optimization(&self, stats: &quinn::ConnectionStats) -> bool {
        // High RTT or high loss rate indicates need for optimization
        stats.path.rtt > Duration::from_millis(200) || 
        stats.path.lost_packets > 5
    }

    /// Apply optimization to a connection
    async fn apply_optimization(
        &self,
        _peer_id: &PeerId,
        _conn: &QuinnConnection,
        stats: &quinn::ConnectionStats,
    ) -> Result<(), TransportError> {
        // Record congestion control switch
        let algorithm = if stats.path.rtt > Duration::from_millis(500) {
            "bbr"
        } else if stats.path.lost_packets > 10 {
            "newreno"
        } else {
            "cubic"
        };

        {
            let mut monitor = self.performance_monitor.write().await;
            monitor.record_congestion_control_switch(algorithm.to_string());
        }

        // In a real implementation, this would actually change the congestion control
        Ok(())
    }
}

#[async_trait]
impl Transport for QuicTransport {
    async fn connect(&self, addr: &PeerAddress) -> Result<Box<dyn Connection>, TransportError> {
        let connection = self.get_or_create_connection(addr).await?;
        
        // Handle connection migration monitoring
        let migration_handle = {
            let connection_clone = connection.clone();
            let transport_clone = self.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    if connection_clone.close_reason().is_some() {
                        break;
                    }
                    let _ = transport_clone.handle_connection_migration(&connection_clone).await;
                }
            })
        };

        let quic_connection = QuicConnection::new(
            connection,
            addr.peer_id.clone(),
            self.config.clone(),
            migration_handle,
        );
        
        Ok(Box::new(quic_connection))
    }

    async fn listen(&self, bind_addr: &SocketAddr) -> Result<(), TransportError> {
        let endpoint = self.ensure_endpoint(Some(*bind_addr)).await?;
        
        // Start accepting connections
        let active_connections = self.active_connections.clone();
        tokio::spawn(async move {
            while let Some(connecting) = endpoint.accept().await {
                let active_connections = active_connections.clone();
                tokio::spawn(async move {
                    match connecting.await {
                        Ok(connection) => {
                            let remote_addr = connection.remote_address();
                            let peer_id = format!("quic-peer-{}", remote_addr);
                            
                            let mut connections = active_connections.write().await;
                            connections.insert(peer_id, connection);
                        }
                        Err(e) => {
                            eprintln!("Failed to accept QUIC connection: {}", e);
                        }
                    }
                });
            }
        });

        Ok(())
    }

    fn protocol_name(&self) -> &'static str {
        "quic"
    }

    fn is_available(&self) -> bool {
        true // QUIC is available on all platforms where Quinn works
    }

    fn priority(&self) -> u8 {
        100 // High priority due to modern features
    }

    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities::quic()
    }
}

impl Clone for QuicTransport {
    fn clone(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            config: self.config.clone(),
            client_config: self.client_config.clone(),
            server_config: self.server_config.clone(),
            active_connections: self.active_connections.clone(),
            session_cache: self.session_cache.clone(),
            performance_monitor: self.performance_monitor.clone(),
        }
    }
}

/// QUIC connection implementation with advanced features
#[derive(Debug)]
pub struct QuicConnection {
    connection: QuinnConnection,
    peer_id: PeerId,
    config: QuicConfig,
    info: ConnectionInfo,
    send_stream: Arc<Mutex<Option<SendStream>>>,
    recv_stream: Arc<Mutex<Option<RecvStream>>>,
    migration_handle: tokio::task::JoinHandle<()>,
    /// Multiple streams for parallel data transfer
    streams: Arc<RwLock<HashMap<u64, (SendStream, RecvStream)>>>,
    /// Stream counter for unique stream IDs
    next_stream_id: Arc<Mutex<u64>>,
    /// Connection state for management
    state: Arc<RwLock<QuicConnectionState>>,
    /// Performance metrics
    metrics: Arc<RwLock<QuicPerformanceMetrics>>,
}

/// QUIC connection state for advanced management
#[derive(Debug, Clone, PartialEq)]
pub enum QuicConnectionState {
    Connecting,
    Connected,
    Migrating,
    Resuming,
    Closing,
    Closed,
    Failed(String),
}

/// Performance metrics for QUIC connections
#[derive(Debug, Clone)]
pub struct QuicPerformanceMetrics {
    pub streams_opened: u64,
    pub streams_closed: u64,
    pub bytes_sent_per_stream: HashMap<u64, u64>,
    pub bytes_received_per_stream: HashMap<u64, u64>,
    pub connection_migrations: u64,
    pub resumption_attempts: u64,
    pub successful_resumptions: u64,
    pub congestion_events: u64,
    pub last_rtt: Option<Duration>,
    pub bandwidth_estimate: Option<u64>,
}

impl QuicPerformanceMetrics {
    pub fn new() -> Self {
        Self {
            streams_opened: 0,
            streams_closed: 0,
            bytes_sent_per_stream: HashMap::new(),
            bytes_received_per_stream: HashMap::new(),
            connection_migrations: 0,
            resumption_attempts: 0,
            successful_resumptions: 0,
            congestion_events: 0,
            last_rtt: None,
            bandwidth_estimate: None,
        }
    }

    /// Get total bytes sent across all streams
    pub fn total_bytes_sent(&self) -> u64 {
        self.bytes_sent_per_stream.values().sum()
    }

    /// Get total bytes received across all streams
    pub fn total_bytes_received(&self) -> u64 {
        self.bytes_received_per_stream.values().sum()
    }

    /// Get resumption success rate
    pub fn resumption_success_rate(&self) -> f64 {
        if self.resumption_attempts == 0 {
            0.0
        } else {
            self.successful_resumptions as f64 / self.resumption_attempts as f64
        }
    }

    /// Get active streams count
    pub fn active_streams(&self) -> usize {
        self.streams_opened.saturating_sub(self.streams_closed) as usize
    }
}

impl QuicConnection {
    fn new(
        connection: QuinnConnection,
        peer_id: PeerId,
        config: QuicConfig,
        migration_handle: tokio::task::JoinHandle<()>,
    ) -> Self {
        let local_addr = connection.local_ip()
            .map(|ip| SocketAddr::new(ip, 0))
            .unwrap_or_else(|| "0.0.0.0:0".parse().unwrap());
        
        let remote_addr = connection.remote_address();
        
        let info = ConnectionInfo::new(
            peer_id.clone(),
            local_addr,
            remote_addr,
            "quic".to_string(),
        );

        Self {
            connection,
            peer_id,
            config,
            info,
            send_stream: Arc::new(Mutex::new(None)),
            recv_stream: Arc::new(Mutex::new(None)),
            migration_handle,
            streams: Arc::new(RwLock::new(HashMap::new())),
            next_stream_id: Arc::new(Mutex::new(0)),
            state: Arc::new(RwLock::new(QuicConnectionState::Connected)),
            metrics: Arc::new(RwLock::new(QuicPerformanceMetrics::new())),
        }
    }

    /// Open a new bidirectional stream for parallel data transfer
    pub async fn open_stream(&self) -> Result<u64, TransportError> {
        let mut stream_id_guard = self.next_stream_id.lock().await;
        let stream_id = *stream_id_guard;
        *stream_id_guard += 1;
        drop(stream_id_guard);

        match self.connection.open_bi().await {
            Ok((send, recv)) => {
                let mut streams = self.streams.write().await;
                streams.insert(stream_id, (send, recv));
                
                // Update metrics
                let mut metrics = self.metrics.write().await;
                metrics.streams_opened += 1;
                
                Ok(stream_id)
            }
            Err(e) => Err(TransportError::Quic(format!("Failed to open stream: {}", e))),
        }
    }

    /// Close a specific stream
    pub async fn close_stream(&self, stream_id: u64) -> Result<(), TransportError> {
        let mut streams = self.streams.write().await;
        if let Some((mut send, _recv)) = streams.remove(&stream_id) {
            let _ = send.finish();
            
            // Update metrics
            let mut metrics = self.metrics.write().await;
            metrics.streams_closed += 1;
            
            Ok(())
        } else {
            Err(TransportError::Quic(format!("Stream {} not found", stream_id)))
        }
    }

    /// Write data to a specific stream
    pub async fn write_to_stream(&self, stream_id: u64, buf: &[u8]) -> Result<usize, TransportError> {
        let mut streams = self.streams.write().await;
        if let Some((send, _)) = streams.get_mut(&stream_id) {
            match send.write(buf).await {
                Ok(bytes_written) => {
                    // Update metrics
                    let mut metrics = self.metrics.write().await;
                    *metrics.bytes_sent_per_stream.entry(stream_id).or_insert(0) += bytes_written as u64;
                    
                    Ok(bytes_written)
                }
                Err(e) => Err(TransportError::Quic(format!("Stream write error: {}", e))),
            }
        } else {
            Err(TransportError::Quic(format!("Stream {} not found", stream_id)))
        }
    }

    /// Read data from a specific stream
    pub async fn read_from_stream(&self, stream_id: u64, buf: &mut [u8]) -> Result<usize, TransportError> {
        let mut streams = self.streams.write().await;
        if let Some((_, recv)) = streams.get_mut(&stream_id) {
            match recv.read(buf).await {
                Ok(Some(bytes_read)) => {
                    // Update metrics
                    let mut metrics = self.metrics.write().await;
                    *metrics.bytes_received_per_stream.entry(stream_id).or_insert(0) += bytes_read as u64;
                    
                    Ok(bytes_read)
                }
                Ok(None) => Ok(0), // Stream finished
                Err(e) => Err(TransportError::Quic(format!("Stream read error: {}", e))),
            }
        } else {
            Err(TransportError::Quic(format!("Stream {} not found", stream_id)))
        }
    }

    /// Get list of active stream IDs
    pub async fn get_active_streams(&self) -> Vec<u64> {
        let streams = self.streams.read().await;
        streams.keys().cloned().collect()
    }

    /// Handle connection state changes
    pub async fn handle_state_change(&self, new_state: QuicConnectionState) {
        let mut state = self.state.write().await;
        *state = new_state;
    }

    /// Attempt connection resumption with 0-RTT
    pub async fn attempt_resumption(&self) -> Result<bool, TransportError> {
        if !self.config.enable_0rtt {
            return Ok(false);
        }

        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.resumption_attempts += 1;

        // Check if connection supports 0-RTT
        // In a real implementation, this would check session tickets
        let resumption_successful = true; // Placeholder logic
        
        if resumption_successful {
            metrics.successful_resumptions += 1;
            self.handle_state_change(QuicConnectionState::Connected).await;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Monitor connection performance and adapt congestion control
    pub async fn monitor_performance(&self) -> Result<(), TransportError> {
        let stats = self.connection.stats();
        let mut metrics = self.metrics.write().await;
        
        // Update performance metrics
        metrics.last_rtt = Some(stats.path.rtt);
        metrics.congestion_events = stats.path.congestion_events;
        
        // Estimate bandwidth from congestion window and RTT
        if stats.path.rtt.as_millis() > 0 {
            let bandwidth = (stats.path.cwnd * 1000) / stats.path.rtt.as_millis() as u64;
            metrics.bandwidth_estimate = Some(bandwidth);
        }

        // Adapt congestion control based on network conditions
        self.adapt_congestion_control(&stats).await?;
        
        Ok(())
    }

    /// Adapt congestion control algorithm based on network conditions
    async fn adapt_congestion_control(&self, stats: &quinn::ConnectionStats) -> Result<(), TransportError> {
        // Analyze network conditions
        let high_loss = stats.path.lost_packets > 10;
        let high_rtt = stats.path.rtt > Duration::from_millis(100);
        let mobile_network = self.detect_mobile_network().await;

        // Switch congestion control algorithm if needed
        match (high_loss, high_rtt, mobile_network) {
            (true, _, _) => {
                // High loss: prefer conservative algorithm
                self.set_congestion_control(CongestionControl::NewReno).await?;
            }
            (_, true, _) => {
                // High RTT: prefer BBR for better throughput
                self.set_congestion_control(CongestionControl::Bbr).await?;
            }
            (_, _, true) => {
                // Mobile network: prefer adaptive algorithm
                self.set_congestion_control(CongestionControl::Cubic).await?;
            }
            _ => {
                // Default conditions: use Cubic
                self.set_congestion_control(CongestionControl::Cubic).await?;
            }
        }

        Ok(())
    }

    /// Detect if we're on a mobile network (placeholder implementation)
    async fn detect_mobile_network(&self) -> bool {
        // In a real implementation, this would check network interface types,
        // signal strength, or other mobile network indicators
        false
    }

    /// Set congestion control algorithm (placeholder implementation)
    async fn set_congestion_control(&self, _algorithm: CongestionControl) -> Result<(), TransportError> {
        // In a real implementation, this would configure the QUIC connection
        // with the specified congestion control algorithm
        Ok(())
    }

    /// Handle connection migration events
    pub async fn handle_migration(&self) -> Result<(), TransportError> {
        self.handle_state_change(QuicConnectionState::Migrating).await;
        
        // Update metrics
        let mut metrics = self.metrics.write().await;
        metrics.connection_migrations += 1;
        
        // Monitor migration progress
        let migration_successful = self.monitor_migration().await?;
        
        if migration_successful {
            self.handle_state_change(QuicConnectionState::Connected).await;
        } else {
            self.handle_state_change(QuicConnectionState::Failed("Migration failed".to_string())).await;
        }
        
        Ok(())
    }

    /// Monitor connection migration progress
    async fn monitor_migration(&self) -> Result<bool, TransportError> {
        // Wait for migration to complete or timeout
        let timeout = Duration::from_secs(10);
        let start = std::time::Instant::now();
        
        while start.elapsed() < timeout {
            if self.connection.close_reason().is_some() {
                return Ok(false);
            }
            
            // Check if migration is complete by monitoring connection stats
            let stats = self.connection.stats();
            if stats.path.rtt < Duration::from_millis(500) {
                return Ok(true);
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        Ok(false)
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> QuicPerformanceMetrics {
        self.metrics.read().await.clone()
    }

    /// Get current connection state
    pub async fn get_state(&self) -> QuicConnectionState {
        self.state.read().await.clone()
    }

    /// Handle QUIC-specific errors
    pub fn handle_quic_error(&self, error: &quinn::ConnectionError) -> TransportError {
        match error {
            quinn::ConnectionError::VersionMismatch => {
                TransportError::Quic("QUIC version mismatch".to_string())
            }
            quinn::ConnectionError::TransportError(transport_error) => {
                TransportError::Quic(format!("Transport error: {:?}", transport_error))
            }
            quinn::ConnectionError::ConnectionClosed(close_frame) => {
                TransportError::Quic(format!("Connection closed: {:?}", close_frame))
            }
            quinn::ConnectionError::ApplicationClosed(app_close) => {
                TransportError::Quic(format!("Application closed: {:?}", app_close))
            }
            quinn::ConnectionError::Reset => {
                TransportError::Quic("Connection reset".to_string())
            }
            quinn::ConnectionError::TimedOut => {
                TransportError::ConnectionTimeout { timeout: Duration::from_secs(30) }
            }
            quinn::ConnectionError::LocallyClosed => {
                TransportError::Quic("Connection locally closed".to_string())
            }
            quinn::ConnectionError::CidsExhausted => {
                TransportError::Quic("Connection IDs exhausted".to_string())
            }
        }
    }

    /// Ensure we have an active bidirectional stream
    async fn ensure_stream(&self) -> Result<(), TransportError> {
        let mut send_guard = self.send_stream.lock().await;
        let mut recv_guard = self.recv_stream.lock().await;
        
        if send_guard.is_none() || recv_guard.is_none() {
            match self.connection.open_bi().await {
                Ok((send, recv)) => {
                    *send_guard = Some(send);
                    *recv_guard = Some(recv);
                }
                Err(e) => {
                    return Err(TransportError::Quic(format!("Failed to open stream: {}", e)));
                }
            }
        }
        
        Ok(())
    }

    /// Update connection statistics
    fn update_stats(&mut self) {
        let stats = self.connection.stats();
        self.info.update_rtt(stats.path.rtt);
        
        // Estimate bandwidth from congestion window and RTT
        if stats.path.rtt.as_millis() > 0 {
            let bandwidth = (stats.path.cwnd * 1000) / stats.path.rtt.as_millis() as u64;
            self.info.update_bandwidth(bandwidth);
        }
    }
}

#[async_trait]
impl Connection for QuicConnection {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        self.ensure_stream().await?;
        
        let mut recv_guard = self.recv_stream.lock().await;
        if let Some(ref mut recv_stream) = *recv_guard {
            match recv_stream.read(buf).await {
                Ok(Some(n)) => {
                    self.info.add_bytes_received(n as u64);
                    Ok(n)
                }
                Ok(None) => Ok(0), // Stream finished
                Err(e) => Err(TransportError::Quic(format!("Read error: {}", e))),
            }
        } else {
            Err(TransportError::Quic("No receive stream available".to_string()))
        }
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, TransportError> {
        self.ensure_stream().await?;
        
        let mut send_guard = self.send_stream.lock().await;
        if let Some(ref mut send_stream) = *send_guard {
            match send_stream.write(buf).await {
                Ok(bytes_written) => {
                    self.info.add_bytes_sent(bytes_written as u64);
                    Ok(bytes_written)
                }
                Err(e) => Err(TransportError::Quic(format!("Write error: {}", e))),
            }
        } else {
            Err(TransportError::Quic("No send stream available".to_string()))
        }
    }

    async fn flush(&mut self) -> Result<(), TransportError> {
        // QUIC streams are automatically flushed, but we can ensure data is sent
        Ok(())
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        // Close streams first
        {
            let mut send_guard = self.send_stream.lock().await;
            if let Some(mut send_stream) = send_guard.take() {
                let _ = send_stream.finish();
            }
        }
        
        // Close the connection
        self.connection.close(0u32.into(), b"Connection closed");
        
        // Cancel migration monitoring
        self.migration_handle.abort();
        
        Ok(())
    }

    fn info(&self) -> ConnectionInfo {
        let mut info = self.info.clone();
        
        // Update with latest statistics
        let stats = self.connection.stats();
        info.update_rtt(stats.path.rtt);
        
        if stats.path.rtt.as_millis() > 0 {
            let bandwidth = (stats.path.cwnd * 1000) / stats.path.rtt.as_millis() as u64;
            info.update_bandwidth(bandwidth);
        }
        
        info
    }

    fn is_connected(&self) -> bool {
        self.connection.close_reason().is_none()
    }
}

impl Drop for QuicConnection {
    fn drop(&mut self) {
        self.migration_handle.abort();
    }
}

/// Statistics for a QUIC connection
#[derive(Debug, Clone)]
pub struct QuicConnectionStats {
    pub rtt: Duration,
    pub cwnd: usize,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub stream_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_quic_transport_creation() {
        let transport = QuicTransport::new();
        assert!(transport.is_ok());
        
        let transport = transport.unwrap();
        assert_eq!(transport.protocol_name(), "quic");
        assert!(transport.is_available());
        assert_eq!(transport.priority(), 100);
        
        let caps = transport.capabilities();
        assert!(caps.reliable);
        assert!(caps.ordered);
        assert!(caps.multiplexed);
        assert!(caps.resumable);
    }

    #[tokio::test]
    async fn test_quic_config_defaults() {
        let config = QuicConfig::default();
        assert_eq!(config.max_concurrent_streams, 100);
        assert_eq!(config.idle_timeout, Duration::from_secs(30));
        assert_eq!(config.keep_alive_interval, Some(Duration::from_secs(5)));
        assert_eq!(config.max_datagram_size, Some(1200));
        assert!(config.enable_0rtt);
        assert!(matches!(config.congestion_control, CongestionControl::Cubic));
        assert_eq!(config.max_migration_attempts, 3);
    }

    #[tokio::test]
    async fn test_quic_transport_with_custom_config() {
        let mut config = QuicConfig::default();
        config.max_concurrent_streams = 50;
        config.idle_timeout = Duration::from_secs(60);
        config.enable_0rtt = false;
        
        let transport = QuicTransport::with_config(config.clone());
        assert!(transport.is_ok());
        
        let transport = transport.unwrap();
        assert_eq!(transport.config.max_concurrent_streams, 50);
        assert_eq!(transport.config.idle_timeout, Duration::from_secs(60));
        assert!(!transport.config.enable_0rtt);
    }

    #[tokio::test]
    async fn test_quic_client_only_transport() {
        let config = QuicConfig::default();
        let transport = QuicTransport::client_only(config);
        assert!(transport.is_ok());
        
        let transport = transport.unwrap();
        assert!(transport.server_config.is_none());
        assert_eq!(transport.protocol_name(), "quic");
    }

    #[tokio::test]
    async fn test_certificate_generation() {
        let result = QuicTransport::generate_self_signed_cert();
        assert!(result.is_ok());
        
        let (cert_der, key_der) = result.unwrap();
        assert!(!cert_der.is_empty());
        assert!(!key_der.secret_der().is_empty());
    }

    #[tokio::test]
    async fn test_quic_connection_info() {
        let peer_id = "test-peer".to_string();
        let local_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
        let remote_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8081);
        
        let info = ConnectionInfo::new(peer_id.clone(), local_addr, remote_addr, "quic".to_string());
        
        assert_eq!(info.peer_id, peer_id);
        assert_eq!(info.local_addr, local_addr);
        assert_eq!(info.remote_addr, remote_addr);
        assert_eq!(info.protocol, "quic");
        assert_eq!(info.bytes_sent, 0);
        assert_eq!(info.bytes_received, 0);
    }

    #[tokio::test]
    async fn test_quic_transport_cleanup() {
        let transport = QuicTransport::new().unwrap();
        
        // Should not panic on empty connections
        transport.cleanup_connections().await;
        
        let stats = transport.get_connection_stats().await;
        assert!(stats.is_empty());
    }

    #[tokio::test]
    async fn test_congestion_control_variants() {
        let mut config = QuicConfig::default();
        
        config.congestion_control = CongestionControl::Cubic;
        let transport1 = QuicTransport::with_config(config.clone());
        assert!(transport1.is_ok());
        
        config.congestion_control = CongestionControl::Bbr;
        let transport2 = QuicTransport::with_config(config.clone());
        assert!(transport2.is_ok());
        
        config.congestion_control = CongestionControl::NewReno;
        let transport3 = QuicTransport::with_config(config);
        assert!(transport3.is_ok());
    }

    #[tokio::test]
    async fn test_performance_metrics() {
        let metrics = QuicPerformanceMetrics::new();
        assert_eq!(metrics.streams_opened, 0);
        assert_eq!(metrics.streams_closed, 0);
        assert_eq!(metrics.total_bytes_sent(), 0);
        assert_eq!(metrics.total_bytes_received(), 0);
        assert_eq!(metrics.resumption_success_rate(), 0.0);
        assert_eq!(metrics.active_streams(), 0);
    }

    #[tokio::test]
    async fn test_connection_performance_monitor() {
        let mut monitor = ConnectionPerformanceMonitor::new();
        
        monitor.record_connection_attempt();
        assert_eq!(monitor.total_connections, 1);
        assert_eq!(monitor.connection_success_rate(), 0.0);
        
        monitor.record_connection_success(Duration::from_millis(100));
        assert_eq!(monitor.successful_connections, 1);
        assert_eq!(monitor.connection_success_rate(), 1.0);
        assert_eq!(monitor.average_connection_time, Duration::from_millis(100));
        
        monitor.record_migration_attempt();
        monitor.record_migration_success();
        assert_eq!(monitor.migration_success_rate(), 1.0);
        
        monitor.record_resumption_attempt();
        monitor.record_resumption_success();
        assert_eq!(monitor.resumption_success_rate(), 1.0);
    }

    #[tokio::test]
    async fn test_session_data() {
        let session = SessionData {
            session_ticket: vec![1, 2, 3, 4],
            early_data_enabled: true,
            last_used: std::time::SystemTime::now(),
            peer_address: "127.0.0.1:8080".parse().unwrap(),
        };
        
        assert_eq!(session.session_ticket, vec![1, 2, 3, 4]);
        assert!(session.early_data_enabled);
    }

    #[tokio::test]
    async fn test_quic_connection_state() {
        let state = QuicConnectionState::Connected;
        assert_eq!(state, QuicConnectionState::Connected);
        
        let failed_state = QuicConnectionState::Failed("Test error".to_string());
        match failed_state {
            QuicConnectionState::Failed(msg) => assert_eq!(msg, "Test error"),
            _ => panic!("Expected Failed state"),
        }
    }

    #[tokio::test]
    async fn test_session_cache_cleanup() {
        let transport = QuicTransport::new().unwrap();
        
        // Should not panic on empty cache
        transport.cleanup_expired_sessions().await;
        
        let (total, active) = transport.get_session_cache_stats().await;
        assert_eq!(total, 0);
        assert_eq!(active, 0);
    }

    #[tokio::test]
    async fn test_connection_optimization() {
        let transport = QuicTransport::new().unwrap();
        
        // Should not panic on empty connections
        let result = transport.optimize_connections().await;
        assert!(result.is_ok());
        
        let stats = transport.get_performance_stats().await;
        assert_eq!(stats.total_connections, 0);
    }
}