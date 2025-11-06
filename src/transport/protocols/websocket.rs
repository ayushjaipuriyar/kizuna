use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_tungstenite::{
    WebSocketStream, MaybeTlsStream, connect_async, accept_async,
    tungstenite::{Message, protocol::CloseFrame}
};
use futures_util::{SinkExt, StreamExt};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::transport::{
    Connection, ConnectionInfo, PeerAddress, PeerId, Transport, 
    TransportCapabilities, TransportError
};

/// Configuration for WebSocket transport
#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    /// Connection timeout duration
    pub connect_timeout: Duration,
    /// Relay server URLs for fallback connections
    pub relay_servers: Vec<Url>,
    /// WebSocket subprotocol for Kizuna communication
    pub subprotocol: String,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Ping interval for connection keep-alive
    pub ping_interval: Duration,
    /// Pong timeout before considering connection dead
    pub pong_timeout: Duration,
    /// Maximum number of relay connection attempts
    pub max_relay_attempts: u32,
    /// Relay authentication token
    pub relay_auth_token: Option<String>,
    /// Connection upgrade attempt interval
    pub upgrade_attempt_interval: Duration,
    /// Maximum time to spend attempting direct connection upgrade
    pub max_upgrade_time: Duration,
    /// Buffer size for message queuing
    pub message_buffer_size: usize,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(15),
            relay_servers: Vec::new(),
            subprotocol: "kizuna-p2p".to_string(),
            max_message_size: 16 * 1024 * 1024, // 16MB
            ping_interval: Duration::from_secs(30),
            pong_timeout: Duration::from_secs(10),
            max_relay_attempts: 3,
            relay_auth_token: None,
            upgrade_attempt_interval: Duration::from_secs(60),
            max_upgrade_time: Duration::from_secs(300), // 5 minutes
            message_buffer_size: 1024,
        }
    }
}

impl WebSocketConfig {
    /// Create configuration optimized for relay connections
    pub fn relay_optimized(relay_servers: Vec<Url>) -> Self {
        Self {
            connect_timeout: Duration::from_secs(20),
            relay_servers,
            subprotocol: "kizuna-p2p".to_string(),
            max_message_size: 8 * 1024 * 1024, // 8MB for relay efficiency
            ping_interval: Duration::from_secs(45), // Longer interval for relay
            pong_timeout: Duration::from_secs(15),
            max_relay_attempts: 5,
            relay_auth_token: None,
            upgrade_attempt_interval: Duration::from_secs(120),
            max_upgrade_time: Duration::from_secs(600),
            message_buffer_size: 512,
        }
    }

    /// Create configuration for direct WebSocket connections
    pub fn direct_connection() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            relay_servers: Vec::new(),
            subprotocol: "kizuna-p2p".to_string(),
            max_message_size: 32 * 1024 * 1024, // 32MB for direct connections
            ping_interval: Duration::from_secs(20),
            pong_timeout: Duration::from_secs(5),
            max_relay_attempts: 0,
            relay_auth_token: None,
            upgrade_attempt_interval: Duration::from_secs(30),
            max_upgrade_time: Duration::from_secs(120),
            message_buffer_size: 2048,
        }
    }
}

/// WebSocket transport implementation with relay support
#[derive(Debug)]
pub struct WebSocketTransport {
    config: WebSocketConfig,
    relay_manager: Arc<RelayManager>,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport with default configuration
    pub fn new() -> Self {
        Self {
            config: WebSocketConfig::default(),
            relay_manager: Arc::new(RelayManager::new()),
        }
    }

    /// Create a new WebSocket transport with custom configuration
    pub fn with_config(config: WebSocketConfig) -> Self {
        let relay_manager = Arc::new(RelayManager::new_with_servers(config.relay_servers.clone()));
        Self { 
            config,
            relay_manager,
        }
    }

    /// Attempt direct WebSocket connection to peer
    async fn connect_direct(&self, addr: &PeerAddress) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, TransportError> {
        if addr.addresses.is_empty() {
            return Err(TransportError::InvalidPeerAddress);
        }

        let mut last_error = None;

        // Try each address for direct connection
        for socket_addr in &addr.addresses {
            let ws_url = format!("ws://{}/{}", socket_addr, self.config.subprotocol);
            
            match timeout(
                self.config.connect_timeout,
                connect_async(&ws_url)
            ).await {
                Ok(Ok((ws_stream, _response))) => {
                    return Ok(ws_stream);
                }
                Ok(Err(e)) => {
                    last_error = Some(TransportError::WebSocket(format!("Direct connection failed: {}", e)));
                }
                Err(_) => {
                    last_error = Some(TransportError::ConnectionTimeout {
                        timeout: self.config.connect_timeout,
                    });
                }
            }
        }

        Err(last_error.unwrap_or_else(|| TransportError::ConnectionFailed {
            reason: "No direct WebSocket addresses available".to_string(),
        }))
    }

    /// Attempt relay connection through configured relay servers
    async fn connect_via_relay(&self, addr: &PeerAddress) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, TransportError> {
        if self.config.relay_servers.is_empty() {
            return Err(TransportError::RelayFailed {
                relay_addr: SocketAddr::from(([0, 0, 0, 0], 0)),
            });
        }

        let mut attempts = 0;
        let mut last_error = None;

        while attempts < self.config.max_relay_attempts {
            if let Some(relay_server) = self.relay_manager.select_best_relay().await {
                match self.connect_through_relay(&relay_server, addr).await {
                    Ok(ws_stream) => {
                        self.relay_manager.record_successful_connection(&relay_server).await;
                        return Ok(ws_stream);
                    }
                    Err(e) => {
                        self.relay_manager.record_failed_connection(&relay_server).await;
                        last_error = Some(e);
                        attempts += 1;
                    }
                }
            } else {
                break;
            }
        }

        Err(last_error.unwrap_or_else(|| TransportError::RelayFailed {
            relay_addr: SocketAddr::from(([0, 0, 0, 0], 0)),
        }))
    }

    /// Connect through a specific relay server
    async fn connect_through_relay(
        &self,
        relay_server: &RelayServer,
        target_peer: &PeerAddress,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, TransportError> {
        // Build relay connection URL
        let mut relay_url = relay_server.url.clone();
        relay_url.set_path(&format!("/relay/{}", target_peer.peer_id));
        
        // Add authentication if configured
        if let Some(auth_token) = &self.config.relay_auth_token {
            relay_url.query_pairs_mut()
                .append_pair("auth", auth_token);
        }

        // Connect to relay server
        let (mut ws_stream, _response) = timeout(
            self.config.connect_timeout,
            connect_async(relay_url.as_str())
        ).await
        .map_err(|_| TransportError::ConnectionTimeout {
            timeout: self.config.connect_timeout,
        })?
        .map_err(|e| TransportError::WebSocket(format!("Relay connection failed: {}", e)))?;

        // Send relay request message
        let relay_request = RelayMessage::ConnectRequest {
            target_peer_id: target_peer.peer_id.clone(),
            source_peer_id: "local".to_string(), // TODO: Get actual local peer ID
            capabilities: target_peer.capabilities.clone(),
        };

        let request_json = serde_json::to_string(&relay_request)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;

        ws_stream.send(Message::Text(request_json)).await
            .map_err(|e| TransportError::WebSocket(format!("Failed to send relay request: {}", e)))?;

        // Wait for relay response
        match timeout(Duration::from_secs(10), ws_stream.next()).await {
            Ok(Some(Ok(Message::Text(response_text)))) => {
                let response: RelayMessage = serde_json::from_str(&response_text)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?;

                match response {
                    RelayMessage::ConnectResponse { success: true, .. } => {
                        Ok(ws_stream)
                    }
                    RelayMessage::ConnectResponse { success: false, error: _, .. } => {
                        Err(TransportError::RelayFailed {
                            relay_addr: relay_server.address,
                        })
                    }
                    _ => {
                        Err(TransportError::WebSocket("Unexpected relay response".to_string()))
                    }
                }
            }
            Ok(Some(Ok(_))) => {
                Err(TransportError::WebSocket("Invalid relay response format".to_string()))
            }
            Ok(Some(Err(e))) => {
                Err(TransportError::WebSocket(format!("Relay response error: {}", e)))
            }
            Ok(None) => {
                Err(TransportError::WebSocket("Relay connection closed unexpectedly".to_string()))
            }
            Err(_) => {
                Err(TransportError::ConnectionTimeout {
                    timeout: Duration::from_secs(10),
                })
            }
        }
    }
}

impl Default for WebSocketTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn connect(&self, addr: &PeerAddress) -> Result<Box<dyn Connection>, TransportError> {
        // First, try direct connection
        match self.connect_direct(addr).await {
            Ok(ws_stream) => {
                let connection = WebSocketConnection::new(
                    ws_stream,
                    addr.peer_id.clone(),
                    addr.addresses.first().copied().unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 0))),
                    ConnectionType::Direct,
                    self.config.clone(),
                );
                return Ok(Box::new(connection));
            }
            Err(_direct_error) => {
                // If direct connection fails, try relay
                match self.connect_via_relay(addr).await {
                    Ok(ws_stream) => {
                        let connection = WebSocketConnection::new(
                            ws_stream,
                            addr.peer_id.clone(),
                            addr.addresses.first().copied().unwrap_or_else(|| SocketAddr::from(([0, 0, 0, 0], 0))),
                            ConnectionType::Relay,
                            self.config.clone(),
                        );
                        return Ok(Box::new(connection));
                    }
                    Err(relay_error) => {
                        // Return the more specific error
                        return Err(relay_error);
                    }
                }
            }
        }
    }

    async fn listen(&self, bind_addr: &SocketAddr) -> Result<(), TransportError> {
        let listener = WebSocketListener::bind(*bind_addr, self.config.clone()).await?;
        let local_addr = listener.local_addr()?;
        
        println!("WebSocket transport listening on {}", local_addr);
        
        // Spawn a task to handle incoming connections
        let listener = Arc::new(listener);
        let listener_clone = listener.clone();
        
        tokio::spawn(async move {
            let result = listener_clone.accept_loop(|_connection, remote_addr| async move {
                println!("Accepted WebSocket connection from {}", remote_addr);
                
                // Keep connection alive briefly for demonstration
                tokio::time::sleep(Duration::from_millis(100)).await;
                
                Ok(())
            }).await;
            
            if let Err(e) = result {
                eprintln!("WebSocket listener error: {}", e);
            }
        });
        
        Ok(())
    }

    fn protocol_name(&self) -> &'static str {
        "websocket"
    }

    fn is_available(&self) -> bool {
        true // WebSocket is available on all platforms
    }

    fn priority(&self) -> u8 {
        30 // Lower priority - fallback transport for restrictive networks
    }

    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities::websocket()
    }
}

/// Type of WebSocket connection
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    Direct,
    Relay,
}

/// WebSocket stream wrapper to handle different stream types
pub enum WebSocketStreamWrapper {
    MaybeTls(WebSocketStream<MaybeTlsStream<TcpStream>>),
    Plain(WebSocketStream<TcpStream>),
}

/// WebSocket connection implementation
pub struct WebSocketConnection {
    ws_stream: WebSocketStreamWrapper,
    info: ConnectionInfo,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    connected: Arc<AtomicBool>,
    connection_type: ConnectionType,
    config: WebSocketConfig,
    last_ping: Arc<std::sync::Mutex<Option<Instant>>>,
    last_pong: Arc<std::sync::Mutex<Option<Instant>>>,
    upgrade_task: Option<tokio::task::JoinHandle<()>>,
}

impl WebSocketStreamWrapper {
    async fn send(&mut self, message: Message) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        match self {
            WebSocketStreamWrapper::MaybeTls(stream) => stream.send(message).await,
            WebSocketStreamWrapper::Plain(stream) => stream.send(message).await,
        }
    }

    async fn next(&mut self) -> Option<Result<Message, tokio_tungstenite::tungstenite::Error>> {
        match self {
            WebSocketStreamWrapper::MaybeTls(stream) => stream.next().await,
            WebSocketStreamWrapper::Plain(stream) => stream.next().await,
        }
    }

    async fn close(&mut self, close_frame: Option<CloseFrame<'_>>) -> Result<(), tokio_tungstenite::tungstenite::Error> {
        match self {
            WebSocketStreamWrapper::MaybeTls(stream) => stream.close(close_frame).await,
            WebSocketStreamWrapper::Plain(stream) => stream.close(close_frame).await,
        }
    }
}

impl WebSocketConnection {
    /// Create a new WebSocket connection with MaybeTlsStream
    pub fn new(
        ws_stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        peer_id: PeerId,
        remote_addr: SocketAddr,
        connection_type: ConnectionType,
        config: WebSocketConfig,
    ) -> Self {
        Self::new_with_wrapper(
            WebSocketStreamWrapper::MaybeTls(ws_stream),
            peer_id,
            remote_addr,
            connection_type,
            config,
        )
    }

    /// Create a new WebSocket connection with plain TcpStream
    pub fn new_plain(
        ws_stream: WebSocketStream<TcpStream>,
        peer_id: PeerId,
        remote_addr: SocketAddr,
        connection_type: ConnectionType,
        config: WebSocketConfig,
    ) -> Self {
        Self::new_with_wrapper(
            WebSocketStreamWrapper::Plain(ws_stream),
            peer_id,
            remote_addr,
            connection_type,
            config,
        )
    }

    /// Create a new WebSocket connection with wrapper
    fn new_with_wrapper(
        ws_stream: WebSocketStreamWrapper,
        peer_id: PeerId,
        remote_addr: SocketAddr,
        connection_type: ConnectionType,
        config: WebSocketConfig,
    ) -> Self {
        let local_addr = SocketAddr::from(([0, 0, 0, 0], 0)); // TODO: Get actual local address

        let info = ConnectionInfo::new(
            peer_id.clone(),
            local_addr,
            remote_addr,
            "websocket".to_string(),
        );

        let mut connection = Self {
            ws_stream,
            info,
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            connected: Arc::new(AtomicBool::new(true)),
            connection_type,
            config,
            last_ping: Arc::new(std::sync::Mutex::new(None)),
            last_pong: Arc::new(std::sync::Mutex::new(Some(Instant::now()))),
            upgrade_task: None,
        };

        // Start ping/pong keep-alive task
        connection.start_keepalive_task();

        // Start connection upgrade task if this is a relay connection
        if connection.connection_type == ConnectionType::Relay {
            connection.start_upgrade_task(peer_id);
        }

        connection
    }

    /// Start ping/pong keep-alive mechanism
    fn start_keepalive_task(&mut self) {
        let connected = self.connected.clone();
        let last_ping = self.last_ping.clone();
        let last_pong = self.last_pong.clone();
        let ping_interval = self.config.ping_interval;
        let pong_timeout = self.config.pong_timeout;

        // Note: In a real implementation, we'd need to share the WebSocket stream
        // For now, we'll just track timing without actually sending pings
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(ping_interval);
            
            while connected.load(Ordering::Relaxed) {
                interval.tick().await;
                
                // Check if we've received a recent pong
                if let Ok(last_pong_time) = last_pong.lock() {
                    if let Some(pong_time) = *last_pong_time {
                        if pong_time.elapsed() > pong_timeout + ping_interval {
                            // Connection appears dead
                            connected.store(false, Ordering::Relaxed);
                            break;
                        }
                    }
                }

                // Record ping time
                if let Ok(mut last_ping_time) = last_ping.lock() {
                    *last_ping_time = Some(Instant::now());
                }
            }
        });
    }

    /// Start connection upgrade task for relay connections
    fn start_upgrade_task(&mut self, peer_id: PeerId) {
        let connected = self.connected.clone();
        let upgrade_interval = self.config.upgrade_attempt_interval;
        let max_upgrade_time = self.config.max_upgrade_time;
        let start_time = Instant::now();

        self.upgrade_task = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(upgrade_interval);
            
            while connected.load(Ordering::Relaxed) && start_time.elapsed() < max_upgrade_time {
                interval.tick().await;
                
                // TODO: Attempt to upgrade from relay to direct connection
                // This would involve:
                // 1. Attempting direct connection to peer
                // 2. If successful, migrating the connection
                // 3. Closing the relay connection
                
                // For now, we'll just log the attempt
                println!("Attempting connection upgrade for peer: {}", peer_id);
            }
        }));
    }

    /// Update connection statistics
    fn update_stats(&self) -> ConnectionInfo {
        let mut info = self.info.clone();
        info.bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        info.bytes_received = self.bytes_received.load(Ordering::Relaxed);
        info
    }

    /// Handle incoming WebSocket message
    async fn handle_message(&mut self, message: Message) -> Result<Vec<u8>, TransportError> {
        match message {
            Message::Binary(data) => {
                self.bytes_received.fetch_add(data.len() as u64, Ordering::Relaxed);
                Ok(data)
            }
            Message::Text(text) => {
                let data = text.into_bytes();
                self.bytes_received.fetch_add(data.len() as u64, Ordering::Relaxed);
                Ok(data)
            }
            Message::Ping(data) => {
                // Respond with pong
                self.ws_stream.send(Message::Pong(data)).await
                    .map_err(|e| TransportError::WebSocket(format!("Failed to send pong: {}", e)))?;
                Ok(Vec::new())
            }
            Message::Pong(_) => {
                // Update last pong time
                if let Ok(mut last_pong_time) = self.last_pong.lock() {
                    *last_pong_time = Some(Instant::now());
                }
                Ok(Vec::new())
            }
            Message::Close(_) => {
                self.connected.store(false, Ordering::Relaxed);
                Err(TransportError::ConnectionFailed {
                    reason: "Connection closed by peer".to_string(),
                })
            }
            Message::Frame(_) => {
                // Raw frames are not expected in normal operation
                Ok(Vec::new())
            }
        }
    }
}

impl std::fmt::Debug for WebSocketConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketConnection")
            .field("info", &self.info)
            .field("bytes_sent", &self.bytes_sent.load(Ordering::Relaxed))
            .field("bytes_received", &self.bytes_received.load(Ordering::Relaxed))
            .field("connected", &self.connected.load(Ordering::Relaxed))
            .field("connection_type", &self.connection_type)
            .finish()
    }
}

#[async_trait]
impl Connection for WebSocketConnection {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(TransportError::ConnectionFailed {
                reason: "Connection is closed".to_string(),
            });
        }

        // Read next WebSocket message
        match self.ws_stream.next().await {
            Some(Ok(message)) => {
                let data = self.handle_message(message).await?;
                if data.is_empty() {
                    // Control message, try reading again
                    return self.read(buf).await;
                }
                
                let bytes_to_copy = std::cmp::min(buf.len(), data.len());
                buf[..bytes_to_copy].copy_from_slice(&data[..bytes_to_copy]);
                Ok(bytes_to_copy)
            }
            Some(Err(e)) => {
                self.connected.store(false, Ordering::Relaxed);
                Err(TransportError::WebSocket(format!("Read error: {}", e)))
            }
            None => {
                self.connected.store(false, Ordering::Relaxed);
                Err(TransportError::ConnectionFailed {
                    reason: "Connection closed by peer".to_string(),
                })
            }
        }
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, TransportError> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(TransportError::ConnectionFailed {
                reason: "Connection is closed".to_string(),
            });
        }

        // Check message size limit
        if buf.len() > self.config.max_message_size {
            return Err(TransportError::WebSocket(format!(
                "Message size {} exceeds limit {}",
                buf.len(),
                self.config.max_message_size
            )));
        }

        // Send as binary message
        let message = Message::Binary(buf.to_vec());
        match self.ws_stream.send(message).await {
            Ok(()) => {
                self.bytes_sent.fetch_add(buf.len() as u64, Ordering::Relaxed);
                Ok(buf.len())
            }
            Err(e) => {
                self.connected.store(false, Ordering::Relaxed);
                Err(TransportError::WebSocket(format!("Write error: {}", e)))
            }
        }
    }

    async fn flush(&mut self) -> Result<(), TransportError> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(TransportError::ConnectionFailed {
                reason: "Connection is closed".to_string(),
            });
        }

        // WebSocket streams are automatically flushed
        // We could send a ping to ensure the connection is active
        let ping_data = b"ping".to_vec();
        self.ws_stream.send(Message::Ping(ping_data)).await
            .map_err(|e| {
                self.connected.store(false, Ordering::Relaxed);
                TransportError::WebSocket(format!("Flush error: {}", e))
            })
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        if self.connected.load(Ordering::Relaxed) {
            self.connected.store(false, Ordering::Relaxed);
            
            // Cancel upgrade task if running
            if let Some(upgrade_task) = self.upgrade_task.take() {
                upgrade_task.abort();
            }
            
            // Send close frame
            let close_frame = CloseFrame {
                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                reason: "Connection closed".into(),
            };
            
            self.ws_stream.close(Some(close_frame)).await
                .map_err(|e| TransportError::WebSocket(format!("Close error: {}", e)))?;
        }
        Ok(())
    }

    fn info(&self) -> ConnectionInfo {
        self.update_stats()
    }

    fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}

/// WebSocket listener for accepting incoming connections
pub struct WebSocketListener {
    listener: tokio::net::TcpListener,
    config: WebSocketConfig,
    active_connections: Arc<AtomicU64>,
    shutdown_signal: Arc<AtomicBool>,
}

impl WebSocketListener {
    /// Create a new WebSocket listener
    pub async fn bind(addr: SocketAddr, config: WebSocketConfig) -> Result<Self, TransportError> {
        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(TransportError::Io)?;

        Ok(Self {
            listener,
            config,
            active_connections: Arc::new(AtomicU64::new(0)),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Get the local address the listener is bound to
    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.listener.local_addr().map_err(TransportError::Io)
    }

    /// Accept incoming connections in a loop
    pub async fn accept_loop<F, Fut>(&self, mut handler: F) -> Result<(), TransportError>
    where
        F: FnMut(WebSocketConnection, SocketAddr) -> Fut,
        Fut: std::future::Future<Output = Result<(), TransportError>>,
    {
        while !self.shutdown_signal.load(Ordering::Relaxed) {
            match self.listener.accept().await {
                Ok((stream, remote_addr)) => {
                    // Perform WebSocket handshake
                    match accept_async(stream).await {
                        Ok(ws_stream) => {
                            let connection = WebSocketConnection::new_plain(
                                ws_stream,
                                format!("incoming-{}", remote_addr), // Generate peer ID
                                remote_addr,
                                ConnectionType::Direct,
                                self.config.clone(),
                            );

                            self.active_connections.fetch_add(1, Ordering::Relaxed);

                            // Handle the connection
                            if let Err(e) = handler(connection, remote_addr).await {
                                eprintln!("Connection handler error: {}", e);
                            }

                            self.active_connections.fetch_sub(1, Ordering::Relaxed);
                        }
                        Err(e) => {
                            eprintln!("WebSocket handshake failed: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Accept error: {}", e);
                    // Brief pause to avoid tight loop on persistent errors
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
        Ok(())
    }

    /// Shutdown the listener
    pub fn shutdown(&self) {
        self.shutdown_signal.store(true, Ordering::Relaxed);
    }

    /// Get the number of active connections
    pub fn active_connection_count(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }
}

/// Relay server information
#[derive(Debug, Clone)]
pub struct RelayServer {
    pub url: Url,
    pub address: SocketAddr,
    pub latency: Option<Duration>,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_used: Option<Instant>,
}

impl RelayServer {
    pub fn new(url: Url, address: SocketAddr) -> Self {
        Self {
            url,
            address,
            latency: None,
            success_count: 0,
            failure_count: 0,
            last_used: None,
        }
    }

    /// Calculate reliability score (0.0 to 1.0)
    pub fn reliability_score(&self) -> f64 {
        let total_attempts = self.success_count + self.failure_count;
        if total_attempts == 0 {
            0.5 // Neutral score for untested servers
        } else {
            self.success_count as f64 / total_attempts as f64
        }
    }

    /// Calculate overall score for server selection
    pub fn selection_score(&self) -> f64 {
        let reliability = self.reliability_score();
        
        // Factor in latency if available
        let latency_factor = if let Some(latency) = self.latency {
            // Lower latency is better, normalize to 0.0-1.0 range
            let latency_ms = latency.as_millis() as f64;
            (1000.0 - latency_ms.min(1000.0)) / 1000.0
        } else {
            0.5 // Neutral score for unknown latency
        };

        // Weighted combination of reliability and latency
        reliability * 0.7 + latency_factor * 0.3
    }
}

/// Relay manager for handling relay server selection and management
#[derive(Debug)]
pub struct RelayManager {
    servers: Arc<std::sync::Mutex<Vec<RelayServer>>>,
}

impl RelayManager {
    /// Create a new relay manager
    pub fn new() -> Self {
        Self {
            servers: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Create a new relay manager with predefined servers
    pub fn new_with_servers(server_urls: Vec<Url>) -> Self {
        let servers = server_urls.into_iter()
            .filter_map(|url| {
                // Extract address from URL
                let host = url.host_str()?;
                let port = url.port().unwrap_or(80);
                let address = format!("{}:{}", host, port).parse().ok()?;
                Some(RelayServer::new(url, address))
            })
            .collect();

        Self {
            servers: Arc::new(std::sync::Mutex::new(servers)),
        }
    }

    /// Add a relay server
    pub fn add_server(&self, url: Url) -> Result<(), TransportError> {
        let host = url.host_str().ok_or_else(|| TransportError::InvalidPeerAddress)?;
        let port = url.port().unwrap_or(80);
        let address = format!("{}:{}", host, port)
            .parse()
            .map_err(|_| TransportError::InvalidPeerAddress)?;

        let server = RelayServer::new(url, address);
        
        if let Ok(mut servers) = self.servers.lock() {
            servers.push(server);
        }
        
        Ok(())
    }

    /// Select the best available relay server
    pub async fn select_best_relay(&self) -> Option<RelayServer> {
        if let Ok(servers) = self.servers.lock() {
            if servers.is_empty() {
                return None;
            }

            // Find server with highest selection score
            servers.iter()
                .max_by(|a, b| a.selection_score().partial_cmp(&b.selection_score()).unwrap_or(std::cmp::Ordering::Equal))
                .cloned()
        } else {
            None
        }
    }

    /// Record successful connection through a relay server
    pub async fn record_successful_connection(&self, server: &RelayServer) {
        if let Ok(mut servers) = self.servers.lock() {
            if let Some(existing_server) = servers.iter_mut().find(|s| s.url == server.url) {
                existing_server.success_count += 1;
                existing_server.last_used = Some(Instant::now());
            }
        }
    }

    /// Record failed connection through a relay server
    pub async fn record_failed_connection(&self, server: &RelayServer) {
        if let Ok(mut servers) = self.servers.lock() {
            if let Some(existing_server) = servers.iter_mut().find(|s| s.url == server.url) {
                existing_server.failure_count += 1;
            }
        }
    }

    /// Update server latency measurement
    pub async fn update_server_latency(&self, server_url: &Url, latency: Duration) {
        if let Ok(mut servers) = self.servers.lock() {
            if let Some(existing_server) = servers.iter_mut().find(|s| s.url == *server_url) {
                existing_server.latency = Some(latency);
            }
        }
    }

    /// Get all relay servers
    pub fn get_servers(&self) -> Vec<RelayServer> {
        self.servers.lock().map(|servers| servers.clone()).unwrap_or_default()
    }
}

/// Messages for relay protocol communication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RelayMessage {
    ConnectRequest {
        target_peer_id: String,
        source_peer_id: String,
        capabilities: TransportCapabilities,
    },
    ConnectResponse {
        success: bool,
        error: Option<String>,
        relay_id: Option<String>,
    },
    RelayData {
        data: Vec<u8>,
    },
    Ping {
        timestamp: u64,
    },
    Pong {
        timestamp: u64,
    },
    Disconnect {
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        assert_eq!(config.subprotocol, "kizuna-p2p");
        assert_eq!(config.max_message_size, 16 * 1024 * 1024);
        assert!(config.relay_servers.is_empty());
    }

    #[test]
    fn test_websocket_config_relay_optimized() {
        let relay_urls = vec![
            "ws://relay1.example.com:8080".parse().unwrap(),
            "ws://relay2.example.com:8080".parse().unwrap(),
        ];
        let config = WebSocketConfig::relay_optimized(relay_urls.clone());
        assert_eq!(config.relay_servers, relay_urls);
        assert_eq!(config.max_message_size, 8 * 1024 * 1024);
        assert_eq!(config.max_relay_attempts, 5);
    }

    #[test]
    fn test_websocket_config_direct_connection() {
        let config = WebSocketConfig::direct_connection();
        assert!(config.relay_servers.is_empty());
        assert_eq!(config.max_message_size, 32 * 1024 * 1024);
        assert_eq!(config.max_relay_attempts, 0);
    }

    #[test]
    fn test_relay_server_creation() {
        let url: Url = "ws://relay.example.com:8080".parse().unwrap();
        let address: SocketAddr = "relay.example.com:8080".parse().unwrap();
        let server = RelayServer::new(url.clone(), address);
        
        assert_eq!(server.url, url);
        assert_eq!(server.address, address);
        assert_eq!(server.success_count, 0);
        assert_eq!(server.failure_count, 0);
    }

    #[test]
    fn test_relay_server_reliability_score() {
        let url: Url = "ws://relay.example.com:8080".parse().unwrap();
        let address: SocketAddr = "relay.example.com:8080".parse().unwrap();
        let mut server = RelayServer::new(url, address);
        
        // Initially neutral score
        assert_eq!(server.reliability_score(), 0.5);
        
        // After some successes and failures
        server.success_count = 8;
        server.failure_count = 2;
        assert_eq!(server.reliability_score(), 0.8);
    }

    #[test]
    fn test_relay_manager_creation() {
        let manager = RelayManager::new();
        assert!(manager.get_servers().is_empty());
    }

    #[test]
    fn test_relay_manager_with_servers() {
        let server_urls = vec![
            "ws://relay1.example.com:8080".parse().unwrap(),
            "ws://relay2.example.com:8080".parse().unwrap(),
        ];
        let manager = RelayManager::new_with_servers(server_urls);
        assert_eq!(manager.get_servers().len(), 2);
    }

    #[test]
    fn test_relay_manager_add_server() {
        let manager = RelayManager::new();
        let url: Url = "ws://relay.example.com:8080".parse().unwrap();
        
        assert!(manager.add_server(url).is_ok());
        assert_eq!(manager.get_servers().len(), 1);
    }

    #[tokio::test]
    async fn test_relay_manager_server_selection() {
        let server_urls = vec![
            "ws://relay1.example.com:8080".parse().unwrap(),
            "ws://relay2.example.com:8080".parse().unwrap(),
        ];
        let manager = RelayManager::new_with_servers(server_urls);
        
        let selected = manager.select_best_relay().await;
        assert!(selected.is_some());
    }

    #[test]
    fn test_relay_message_serialization() {
        let message = RelayMessage::ConnectRequest {
            target_peer_id: "target-123".to_string(),
            source_peer_id: "source-456".to_string(),
            capabilities: TransportCapabilities::websocket(),
        };
        
        let json = serde_json::to_string(&message).unwrap();
        let deserialized: RelayMessage = serde_json::from_str(&json).unwrap();
        
        match deserialized {
            RelayMessage::ConnectRequest { target_peer_id, source_peer_id, .. } => {
                assert_eq!(target_peer_id, "target-123");
                assert_eq!(source_peer_id, "source-456");
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_connection_type() {
        assert_eq!(ConnectionType::Direct, ConnectionType::Direct);
        assert_ne!(ConnectionType::Direct, ConnectionType::Relay);
    }

    #[test]
    fn test_websocket_transport_creation() {
        let transport = WebSocketTransport::new();
        assert_eq!(transport.protocol_name(), "websocket");
        assert!(transport.is_available());
        assert_eq!(transport.priority(), 30);
    }

    #[test]
    fn test_websocket_transport_capabilities() {
        let transport = WebSocketTransport::new();
        let caps = transport.capabilities();
        assert!(caps.reliable);
        assert!(caps.ordered);
        assert!(!caps.multiplexed);
        assert!(!caps.resumable);
        assert!(caps.nat_traversal);
    }
}

/// Enhanced relay manager with connection upgrade and failover capabilities
impl RelayManager {
    /// Perform health check on all relay servers
    pub async fn health_check_all_servers(&self) -> Result<(), TransportError> {
        let servers = self.get_servers();
        let mut tasks = Vec::new();

        for server in servers {
            let server_clone = server.clone();
            let servers_ref = self.servers.clone();
            
            let task = tokio::spawn(async move {
                let start_time = Instant::now();
                
                // Attempt to connect to relay server for health check
                match timeout(
                    Duration::from_secs(5),
                    connect_async(server_clone.url.as_str())
                ).await {
                    Ok(Ok((mut ws_stream, _))) => {
                        let latency = start_time.elapsed();
                        
                        // Send ping and wait for pong
                        let ping_msg = RelayMessage::Ping {
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis() as u64,
                        };
                        
                        let ping_json = serde_json::to_string(&ping_msg).unwrap_or_default();
                        
                        if ws_stream.send(Message::Text(ping_json)).await.is_ok() {
                            // Wait for pong response
                            match timeout(Duration::from_secs(2), ws_stream.next()).await {
                                Ok(Some(Ok(Message::Text(response)))) => {
                                    if let Ok(RelayMessage::Pong { .. }) = serde_json::from_str(&response) {
                                        // Update server latency
                                        if let Ok(mut servers) = servers_ref.lock() {
                                            if let Some(existing_server) = servers.iter_mut().find(|s| s.url == server_clone.url) {
                                                existing_server.latency = Some(latency);
                                                existing_server.success_count += 1;
                                            }
                                        }
                                    }
                                }
                                _ => {
                                    // Failed to get pong response
                                    if let Ok(mut servers) = servers_ref.lock() {
                                        if let Some(existing_server) = servers.iter_mut().find(|s| s.url == server_clone.url) {
                                            existing_server.failure_count += 1;
                                        }
                                    }
                                }
                            }
                        }
                        
                        let _ = ws_stream.close(None).await;
                    }
                    _ => {
                        // Connection failed
                        if let Ok(mut servers) = servers_ref.lock() {
                            if let Some(existing_server) = servers.iter_mut().find(|s| s.url == server_clone.url) {
                                existing_server.failure_count += 1;
                            }
                        }
                    }
                }
            });
            
            tasks.push(task);
        }

        // Wait for all health checks to complete
        for task in tasks {
            let _ = task.await;
        }

        Ok(())
    }

    /// Get relay servers sorted by selection score
    pub fn get_servers_by_score(&self) -> Vec<RelayServer> {
        let mut servers = self.get_servers();
        servers.sort_by(|a, b| b.selection_score().partial_cmp(&a.selection_score()).unwrap_or(std::cmp::Ordering::Equal));
        servers
    }

    /// Remove unreliable relay servers
    pub fn cleanup_unreliable_servers(&self, min_reliability: f64) {
        if let Ok(mut servers) = self.servers.lock() {
            servers.retain(|server| {
                let total_attempts = server.success_count + server.failure_count;
                if total_attempts < 5 {
                    true // Keep servers with insufficient data
                } else {
                    server.reliability_score() >= min_reliability
                }
            });
        }
    }

    /// Start periodic health check task
    pub fn start_health_check_task(manager: Arc<RelayManager>, interval: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                
                if let Err(e) = manager.health_check_all_servers().await {
                    eprintln!("Relay health check error: {}", e);
                }
                
                // Cleanup unreliable servers (less than 30% success rate)
                manager.cleanup_unreliable_servers(0.3);
            }
        })
    }
}

/// Connection upgrade manager for transitioning from relay to direct connections
#[derive(Debug)]
pub struct ConnectionUpgradeManager {
    upgrade_attempts: Arc<std::sync::Mutex<HashMap<PeerId, UpgradeAttempt>>>,
}

#[derive(Debug, Clone)]
struct UpgradeAttempt {
    peer_id: PeerId,
    peer_addresses: Vec<SocketAddr>,
    last_attempt: Instant,
    attempt_count: u32,
    max_attempts: u32,
}

impl ConnectionUpgradeManager {
    pub fn new() -> Self {
        Self {
            upgrade_attempts: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Attempt to upgrade a relay connection to direct connection
    pub async fn attempt_upgrade(
        &self,
        peer_id: PeerId,
        peer_addresses: Vec<SocketAddr>,
        config: &WebSocketConfig,
    ) -> Result<Option<WebSocketStreamWrapper>, TransportError> {
        // Check if we should attempt upgrade
        let should_attempt = {
            let mut attempts = self.upgrade_attempts.lock().unwrap();
            let attempt = attempts.entry(peer_id.clone()).or_insert_with(|| UpgradeAttempt {
                peer_id: peer_id.clone(),
                peer_addresses: peer_addresses.clone(),
                last_attempt: Instant::now() - config.upgrade_attempt_interval,
                attempt_count: 0,
                max_attempts: 10,
            });

            if attempt.last_attempt.elapsed() >= config.upgrade_attempt_interval && 
               attempt.attempt_count < attempt.max_attempts {
                attempt.last_attempt = Instant::now();
                attempt.attempt_count += 1;
                true
            } else {
                false
            }
        };

        if !should_attempt {
            return Ok(None);
        }

        // Try direct connection to each peer address
        for address in &peer_addresses {
            let ws_url = format!("ws://{}/{}", address, config.subprotocol);
            
            match timeout(
                Duration::from_secs(5), // Shorter timeout for upgrade attempts
                connect_async(&ws_url)
            ).await {
                Ok(Ok((ws_stream, _response))) => {
                    // Successful direct connection
                    self.record_successful_upgrade(&peer_id);
                    return Ok(Some(WebSocketStreamWrapper::MaybeTls(ws_stream)));
                }
                Ok(Err(_)) | Err(_) => {
                    // Connection failed, try next address
                    continue;
                }
            }
        }

        // All direct connection attempts failed
        Ok(None)
    }

    /// Record successful upgrade
    fn record_successful_upgrade(&self, peer_id: &PeerId) {
        if let Ok(mut attempts) = self.upgrade_attempts.lock() {
            attempts.remove(peer_id);
        }
    }

    /// Clean up old upgrade attempts
    pub fn cleanup_old_attempts(&self, max_age: Duration) {
        if let Ok(mut attempts) = self.upgrade_attempts.lock() {
            let cutoff_time = Instant::now() - max_age;
            attempts.retain(|_, attempt| attempt.last_attempt > cutoff_time);
        }
    }

    /// Start periodic cleanup task
    pub fn start_cleanup_task(manager: Arc<ConnectionUpgradeManager>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                manager.cleanup_old_attempts(Duration::from_secs(3600)); // 1 hour
            }
        })
    }
}

/// Enhanced WebSocket connection with upgrade capabilities
impl WebSocketConnection {
    /// Attempt to upgrade relay connection to direct connection
    pub async fn attempt_connection_upgrade(
        &mut self,
        peer_addresses: Vec<SocketAddr>,
        upgrade_manager: &ConnectionUpgradeManager,
    ) -> Result<bool, TransportError> {
        if self.connection_type != ConnectionType::Relay {
            return Ok(false); // Already direct connection
        }

        match upgrade_manager.attempt_upgrade(
            self.info.peer_id.clone(),
            peer_addresses,
            &self.config,
        ).await? {
            Some(new_ws_stream) => {
                // Successfully upgraded to direct connection
                println!("Successfully upgraded relay connection to direct for peer: {}", self.info.peer_id);
                
                // Replace the WebSocket stream
                self.ws_stream = new_ws_stream;
                self.connection_type = ConnectionType::Direct;
                
                // Cancel the upgrade task since we're now direct
                if let Some(upgrade_task) = self.upgrade_task.take() {
                    upgrade_task.abort();
                }
                
                Ok(true)
            }
            None => {
                // Upgrade attempt failed or was skipped
                Ok(false)
            }
        }
    }

    /// Enhanced start upgrade task with upgrade manager
    fn start_upgrade_task_with_manager(&mut self, peer_id: PeerId, peer_addresses: Vec<SocketAddr>) {
        let connected = self.connected.clone();
        let upgrade_interval = self.config.upgrade_attempt_interval;
        let max_upgrade_time = self.config.max_upgrade_time;
        let start_time = Instant::now();
        let upgrade_manager = Arc::new(ConnectionUpgradeManager::new());

        self.upgrade_task = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(upgrade_interval);
            
            while connected.load(Ordering::Relaxed) && start_time.elapsed() < max_upgrade_time {
                interval.tick().await;
                
                // Attempt upgrade (this would need access to the connection)
                // In a real implementation, this would be coordinated through the ConnectionManager
                match upgrade_manager.attempt_upgrade(
                    peer_id.clone(),
                    peer_addresses.clone(),
                    &WebSocketConfig::default(), // Would use actual config
                ).await {
                    Ok(Some(_)) => {
                        println!("Connection upgrade successful for peer: {}", peer_id);
                        break;
                    }
                    Ok(None) => {
                        // No upgrade attempted or failed
                    }
                    Err(e) => {
                        eprintln!("Connection upgrade error for peer {}: {}", peer_id, e);
                    }
                }
            }
        }));
    }
}

/// Relay server implementation for handling relay traffic
#[derive(Debug)]
pub struct RelayServerHandler {
    config: WebSocketConfig,
    active_relays: Arc<std::sync::Mutex<HashMap<String, RelaySession>>>,
    bandwidth_limiter: Arc<BandwidthLimiter>,
}

#[derive(Debug, Clone)]
struct RelaySession {
    relay_id: String,
    source_peer_id: String,
    target_peer_id: String,
    created_at: Instant,
    bytes_relayed: u64,
}

impl RelayServerHandler {
    pub fn new(config: WebSocketConfig) -> Self {
        Self {
            config,
            active_relays: Arc::new(std::sync::Mutex::new(HashMap::new())),
            bandwidth_limiter: Arc::new(BandwidthLimiter::new(1024 * 1024 * 100)), // 100 MB/s default
        }
    }

    /// Handle incoming relay connection request
    pub async fn handle_relay_request(
        &self,
        mut ws_stream: WebSocketStream<TcpStream>,
        remote_addr: SocketAddr,
    ) -> Result<(), TransportError> {
        println!("Handling relay request from: {}", remote_addr);

        // Wait for initial relay request
        match timeout(Duration::from_secs(10), ws_stream.next()).await {
            Ok(Some(Ok(Message::Text(request_text)))) => {
                let request: RelayMessage = serde_json::from_str(&request_text)
                    .map_err(|e| TransportError::Serialization(e.to_string()))?;

                match request {
                    RelayMessage::ConnectRequest { target_peer_id, source_peer_id, .. } => {
                        // Generate relay session ID
                        let relay_id = uuid::Uuid::new_v4().to_string();
                        
                        // Create relay session
                        let session = RelaySession {
                            relay_id: relay_id.clone(),
                            source_peer_id: source_peer_id.clone(),
                            target_peer_id: target_peer_id.clone(),
                            created_at: Instant::now(),
                            bytes_relayed: 0,
                        };

                        // Store session
                        {
                            let mut active_relays = self.active_relays.lock().unwrap();
                            active_relays.insert(relay_id.clone(), session);
                        }

                        // Send success response
                        let response = RelayMessage::ConnectResponse {
                            success: true,
                            error: None,
                            relay_id: Some(relay_id.clone()),
                        };

                        let response_json = serde_json::to_string(&response)
                            .map_err(|e| TransportError::Serialization(e.to_string()))?;

                        ws_stream.send(Message::Text(response_json)).await
                            .map_err(|e| TransportError::WebSocket(format!("Failed to send response: {}", e)))?;

                        // Start relay forwarding loop
                        self.handle_relay_forwarding(ws_stream, relay_id).await?;
                    }
                    _ => {
                        // Invalid request type
                        let response = RelayMessage::ConnectResponse {
                            success: false,
                            error: Some("Invalid request type".to_string()),
                            relay_id: None,
                        };

                        let response_json = serde_json::to_string(&response)
                            .map_err(|e| TransportError::Serialization(e.to_string()))?;

                        ws_stream.send(Message::Text(response_json)).await
                            .map_err(|e| TransportError::WebSocket(format!("Failed to send error response: {}", e)))?;
                    }
                }
            }
            _ => {
                return Err(TransportError::WebSocket("Failed to receive relay request".to_string()));
            }
        }

        Ok(())
    }

    /// Handle relay traffic forwarding
    async fn handle_relay_forwarding(
        &self,
        mut ws_stream: WebSocketStream<TcpStream>,
        relay_id: String,
    ) -> Result<(), TransportError> {
        println!("Starting relay forwarding for session: {}", relay_id);

        while let Some(message_result) = ws_stream.next().await {
            match message_result {
                Ok(Message::Binary(data)) => {
                    // Check bandwidth limits
                    if !self.bandwidth_limiter.check_and_consume(data.len()).await {
                        eprintln!("Bandwidth limit exceeded for relay session: {}", relay_id);
                        break;
                    }

                    // Update session statistics
                    {
                        let mut active_relays = self.active_relays.lock().unwrap();
                        if let Some(session) = active_relays.get_mut(&relay_id) {
                            session.bytes_relayed += data.len() as u64;
                        }
                    }

                    // In a real implementation, this would forward to the target peer
                    // For now, we'll just echo the data back
                    let relay_data = RelayMessage::RelayData { data };
                    let relay_json = serde_json::to_string(&relay_data)
                        .map_err(|e| TransportError::Serialization(e.to_string()))?;

                    if let Err(e) = ws_stream.send(Message::Text(relay_json)).await {
                        eprintln!("Failed to forward relay data: {}", e);
                        break;
                    }
                }
                Ok(Message::Text(text)) => {
                    // Handle control messages
                    if let Ok(message) = serde_json::from_str::<RelayMessage>(&text) {
                        match message {
                            RelayMessage::Ping { timestamp } => {
                                let pong = RelayMessage::Pong { timestamp };
                                let pong_json = serde_json::to_string(&pong)
                                    .map_err(|e| TransportError::Serialization(e.to_string()))?;
                                
                                if let Err(e) = ws_stream.send(Message::Text(pong_json)).await {
                                    eprintln!("Failed to send pong: {}", e);
                                    break;
                                }
                            }
                            RelayMessage::Disconnect { .. } => {
                                println!("Relay session disconnected: {}", relay_id);
                                break;
                            }
                            _ => {
                                // Other message types
                            }
                        }
                    }
                }
                Ok(Message::Close(_)) => {
                    println!("Relay session closed: {}", relay_id);
                    break;
                }
                Err(e) => {
                    eprintln!("Relay forwarding error: {}", e);
                    break;
                }
                _ => {
                    // Other message types (Ping, Pong, etc.)
                }
            }
        }

        // Clean up session
        {
            let mut active_relays = self.active_relays.lock().unwrap();
            active_relays.remove(&relay_id);
        }

        Ok(())
    }

    /// Get relay statistics
    pub fn get_relay_stats(&self) -> RelayStats {
        let active_relays = self.active_relays.lock().unwrap();
        let total_sessions = active_relays.len();
        let total_bytes_relayed = active_relays.values().map(|s| s.bytes_relayed).sum();
        
        RelayStats {
            active_sessions: total_sessions,
            total_bytes_relayed,
            bandwidth_limit: self.bandwidth_limiter.get_limit(),
            current_bandwidth_usage: self.bandwidth_limiter.get_current_usage(),
        }
    }
}

/// Bandwidth limiter for relay servers
#[derive(Debug)]
pub struct BandwidthLimiter {
    limit_bytes_per_second: u64,
    current_usage: Arc<AtomicU64>,
    last_reset: Arc<std::sync::Mutex<Instant>>,
}

impl BandwidthLimiter {
    pub fn new(limit_bytes_per_second: u64) -> Self {
        Self {
            limit_bytes_per_second,
            current_usage: Arc::new(AtomicU64::new(0)),
            last_reset: Arc::new(std::sync::Mutex::new(Instant::now())),
        }
    }

    /// Check if bandwidth is available and consume it
    pub async fn check_and_consume(&self, bytes: usize) -> bool {
        // Reset usage counter every second
        {
            let mut last_reset = self.last_reset.lock().unwrap();
            if last_reset.elapsed() >= Duration::from_secs(1) {
                self.current_usage.store(0, Ordering::Relaxed);
                *last_reset = Instant::now();
            }
        }

        let current = self.current_usage.load(Ordering::Relaxed);
        let new_usage = current + bytes as u64;

        if new_usage <= self.limit_bytes_per_second {
            self.current_usage.store(new_usage, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub fn get_limit(&self) -> u64 {
        self.limit_bytes_per_second
    }

    pub fn get_current_usage(&self) -> u64 {
        self.current_usage.load(Ordering::Relaxed)
    }
}

/// Statistics for relay server
#[derive(Debug, Clone)]
pub struct RelayStats {
    pub active_sessions: usize,
    pub total_bytes_relayed: u64,
    pub bandwidth_limit: u64,
    pub current_bandwidth_usage: u64,
}

#[cfg(test)]
mod relay_tests {
    use super::*;

    #[tokio::test]
    async fn test_relay_manager_health_check() {
        let manager = RelayManager::new();
        // Health check on empty manager should succeed
        assert!(manager.health_check_all_servers().await.is_ok());
    }

    #[test]
    fn test_connection_upgrade_manager() {
        let manager = ConnectionUpgradeManager::new();
        // Should start with no upgrade attempts
        assert_eq!(manager.upgrade_attempts.lock().unwrap().len(), 0);
    }

    #[test]
    fn test_bandwidth_limiter() {
        let limiter = BandwidthLimiter::new(1000); // 1000 bytes per second
        assert_eq!(limiter.get_limit(), 1000);
        assert_eq!(limiter.get_current_usage(), 0);
    }

    #[tokio::test]
    async fn test_bandwidth_limiter_consumption() {
        let limiter = BandwidthLimiter::new(1000);
        
        // Should allow consumption within limit
        assert!(limiter.check_and_consume(500).await);
        assert_eq!(limiter.get_current_usage(), 500);
        
        // Should allow more consumption within limit
        assert!(limiter.check_and_consume(400).await);
        assert_eq!(limiter.get_current_usage(), 900);
        
        // Should reject consumption that exceeds limit
        assert!(!limiter.check_and_consume(200).await);
        assert_eq!(limiter.get_current_usage(), 900);
    }

    #[test]
    fn test_relay_server_handler_creation() {
        let config = WebSocketConfig::default();
        let handler = RelayServerHandler::new(config);
        
        let stats = handler.get_relay_stats();
        assert_eq!(stats.active_sessions, 0);
        assert_eq!(stats.total_bytes_relayed, 0);
    }

    #[test]
    fn test_relay_session() {
        let session = RelaySession {
            relay_id: "test-relay-123".to_string(),
            source_peer_id: "source-peer".to_string(),
            target_peer_id: "target-peer".to_string(),
            created_at: Instant::now(),
            bytes_relayed: 1024,
        };
        
        assert_eq!(session.relay_id, "test-relay-123");
        assert_eq!(session.bytes_relayed, 1024);
    }

    #[test]
    fn test_relay_stats() {
        let stats = RelayStats {
            active_sessions: 5,
            total_bytes_relayed: 1024 * 1024,
            bandwidth_limit: 1000000,
            current_bandwidth_usage: 50000,
        };
        
        assert_eq!(stats.active_sessions, 5);
        assert_eq!(stats.total_bytes_relayed, 1024 * 1024);
    }
}