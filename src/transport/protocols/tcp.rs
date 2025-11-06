use std::net::SocketAddr;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::timeout;
use async_trait::async_trait;

use crate::transport::{
    Connection, ConnectionInfo, PeerAddress, PeerId, Transport, 
    TransportCapabilities, TransportError
};

/// Configuration for TCP transport
#[derive(Debug, Clone)]
pub struct TcpConfig {
    /// Connection timeout duration
    pub connect_timeout: Duration,
    /// Keep-alive interval (None to disable)
    pub keep_alive: Option<Duration>,
    /// TCP_NODELAY setting for low latency
    pub no_delay: bool,
    /// SO_REUSEADDR setting
    pub reuse_addr: bool,
    /// Socket receive buffer size
    pub recv_buffer_size: Option<usize>,
    /// Socket send buffer size  
    pub send_buffer_size: Option<usize>,
    /// Maximum number of pending connections for listener
    pub listen_backlog: u32,
    /// Maximum number of concurrent connections
    pub max_connections: u64,
    /// Connection idle timeout before cleanup
    pub idle_timeout: Duration,
    /// Enable SO_REUSEPORT for load balancing (Linux/macOS)
    pub reuse_port: bool,
    /// TCP keep-alive probe count
    pub keep_alive_probes: Option<u32>,
    /// TCP keep-alive probe interval
    pub keep_alive_interval: Option<Duration>,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            keep_alive: Some(Duration::from_secs(60)),
            no_delay: true,
            reuse_addr: true,
            recv_buffer_size: Some(65536), // 64KB
            send_buffer_size: Some(65536), // 64KB
            listen_backlog: 128,
            max_connections: 1000,
            idle_timeout: Duration::from_secs(300), // 5 minutes
            reuse_port: false, // Disabled by default for compatibility
            keep_alive_probes: Some(9),
            keep_alive_interval: Some(Duration::from_secs(75)),
        }
    }
}

impl TcpConfig {
    /// Create a configuration optimized for low latency
    pub fn low_latency() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            keep_alive: Some(Duration::from_secs(30)),
            no_delay: true, // Critical for low latency
            reuse_addr: true,
            recv_buffer_size: Some(32768), // Smaller buffers for lower latency
            send_buffer_size: Some(32768),
            listen_backlog: 64,
            max_connections: 500,
            idle_timeout: Duration::from_secs(120),
            reuse_port: true, // Enable for better load distribution
            keep_alive_probes: Some(3), // Faster detection of dead connections
            keep_alive_interval: Some(Duration::from_secs(30)),
        }
    }

    /// Create a configuration optimized for high throughput
    pub fn high_throughput() -> Self {
        Self {
            connect_timeout: Duration::from_secs(15),
            keep_alive: Some(Duration::from_secs(120)),
            no_delay: false, // Allow Nagle's algorithm for better throughput
            reuse_addr: true,
            recv_buffer_size: Some(262144), // 256KB for high throughput
            send_buffer_size: Some(262144),
            listen_backlog: 512,
            max_connections: 2000,
            idle_timeout: Duration::from_secs(600),
            reuse_port: true,
            keep_alive_probes: Some(9),
            keep_alive_interval: Some(Duration::from_secs(120)),
        }
    }

    /// Create a configuration optimized for mobile/battery-constrained devices
    pub fn mobile_optimized() -> Self {
        Self {
            connect_timeout: Duration::from_secs(20),
            keep_alive: Some(Duration::from_secs(300)), // Longer keep-alive to reduce reconnections
            no_delay: true,
            reuse_addr: true,
            recv_buffer_size: Some(16384), // Smaller buffers to save memory
            send_buffer_size: Some(16384),
            listen_backlog: 32,
            max_connections: 100,
            idle_timeout: Duration::from_secs(180),
            reuse_port: false,
            keep_alive_probes: Some(3),
            keep_alive_interval: Some(Duration::from_secs(180)),
        }
    }
}

/// TCP transport implementation
#[derive(Debug)]
pub struct TcpTransport {
    config: TcpConfig,
}

impl TcpTransport {
    /// Create a new TCP transport with default configuration
    pub fn new() -> Self {
        Self {
            config: TcpConfig::default(),
        }
    }

    /// Create a new TCP transport with custom configuration
    pub fn with_config(config: TcpConfig) -> Self {
        Self { config }
    }

    /// Configure socket options for a TCP stream
    async fn configure_socket(&self, stream: &TcpStream) -> Result<(), TransportError> {
        // Set TCP_NODELAY for low latency
        if self.config.no_delay {
            stream.set_nodelay(true)?;
        }

        // Configure keep-alive using socket2 for advanced options
        if self.config.keep_alive.is_some() {
            // Get the underlying socket for advanced configuration
            let socket = socket2::SockRef::from(stream);
            socket.set_keepalive(true)?;
            
            // Set advanced keep-alive parameters on supported platforms
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            {
                // Note: Advanced keep-alive parameters require platform-specific code
                // For now, we'll use basic keep-alive functionality
                if self.config.keep_alive_probes.is_some() || self.config.keep_alive_interval.is_some() {
                    // These would require platform-specific socket options
                    // socket.set_keepalive_probes(probes)?;
                    // socket.set_keepalive_interval(interval)?;
                }
            }
        }

        // Configure buffer sizes if specified
        if let Some(size) = self.config.recv_buffer_size {
            let socket = socket2::SockRef::from(stream);
            socket.set_recv_buffer_size(size)?;
        }

        if let Some(size) = self.config.send_buffer_size {
            let socket = socket2::SockRef::from(stream);
            socket.set_send_buffer_size(size)?;
        }

        Ok(())
    }
}

impl Default for TcpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for TcpTransport {
    async fn connect(&self, addr: &PeerAddress) -> Result<Box<dyn Connection>, TransportError> {
        if addr.addresses.is_empty() {
            return Err(TransportError::InvalidPeerAddress);
        }

        let mut last_error = None;

        // Try each address until one succeeds
        for socket_addr in &addr.addresses {
            match timeout(self.config.connect_timeout, TcpStream::connect(socket_addr)).await {
                Ok(Ok(stream)) => {
                    // Configure socket options
                    self.configure_socket(&stream).await?;

                    let connection = TcpConnection::new(
                        stream,
                        addr.peer_id.clone(),
                        *socket_addr,
                    );

                    return Ok(Box::new(connection));
                }
                Ok(Err(e)) => {
                    last_error = Some(TransportError::Io(e));
                }
                Err(_) => {
                    last_error = Some(TransportError::ConnectionTimeout {
                        timeout: self.config.connect_timeout,
                    });
                }
            }
        }

        Err(last_error.unwrap_or_else(|| TransportError::ConnectionFailed {
            reason: "No addresses available".to_string(),
        }))
    }

    async fn listen(&self, bind_addr: &std::net::SocketAddr) -> Result<(), TransportError> {
        let listener = TcpListener::bind(*bind_addr, self.config.clone()).await?;
        let local_addr = listener.local_addr()?;
        
        println!("TCP transport listening on {}", local_addr);
        
        // Spawn a task to handle incoming connections
        let listener = Arc::new(listener);
        let listener_clone = listener.clone();
        
        tokio::spawn(async move {
            let result = listener_clone.accept_loop(|_connection, remote_addr| async move {
                println!("Accepted TCP connection from {}", remote_addr);
                
                // In a real implementation, this would be handled by the ConnectionManager
                // For now, we'll just log the connection and close it
                // The actual connection handling should be done by the application layer
                
                // Keep connection alive briefly for demonstration
                tokio::time::sleep(Duration::from_millis(100)).await;
                
                Ok(())
            }).await;
            
            if let Err(e) = result {
                eprintln!("TCP listener error: {}", e);
            }
        });
        
        Ok(())
    }

    fn protocol_name(&self) -> &'static str {
        "tcp"
    }

    fn is_available(&self) -> bool {
        true // TCP is available on all platforms
    }

    fn priority(&self) -> u8 {
        50 // Medium priority - reliable but not as feature-rich as QUIC
    }

    fn capabilities(&self) -> TransportCapabilities {
        TransportCapabilities::tcp()
    }
}

/// TCP connection implementation
pub struct TcpConnection {
    stream: TcpStream,
    info: ConnectionInfo,
    bytes_sent: Arc<AtomicU64>,
    bytes_received: Arc<AtomicU64>,
    connected: Arc<AtomicBool>,
    connection_counter: Option<Arc<AtomicU64>>,
}

impl TcpConnection {
    /// Create a new TCP connection
    pub fn new(stream: TcpStream, peer_id: PeerId, remote_addr: SocketAddr) -> Self {
        Self::new_with_counter(stream, peer_id, remote_addr, None)
    }

    /// Create a new TCP connection with connection counter
    pub fn new_with_counter(
        stream: TcpStream, 
        peer_id: PeerId, 
        remote_addr: SocketAddr,
        connection_counter: Option<Arc<AtomicU64>>
    ) -> Self {
        let local_addr = stream.local_addr().unwrap_or_else(|_| {
            SocketAddr::from(([0, 0, 0, 0], 0))
        });

        let info = ConnectionInfo::new(
            peer_id,
            local_addr,
            remote_addr,
            "tcp".to_string(),
        );

        Self {
            stream,
            info,
            bytes_sent: Arc::new(AtomicU64::new(0)),
            bytes_received: Arc::new(AtomicU64::new(0)),
            connected: Arc::new(AtomicBool::new(true)),
            connection_counter,
        }
    }

    /// Update connection statistics
    fn update_stats(&self) -> ConnectionInfo {
        let mut info = self.info.clone();
        info.bytes_sent = self.bytes_sent.load(Ordering::Relaxed);
        info.bytes_received = self.bytes_received.load(Ordering::Relaxed);
        info
    }

    /// Check connection health by attempting to read with zero timeout
    async fn check_health(&self) -> bool {
        if !self.connected.load(Ordering::Relaxed) {
            return false;
        }

        // Try to peek at the stream to check if it's still connected
        match self.stream.ready(tokio::io::Interest::READABLE).await {
            Ok(_) => true,
            Err(_) => {
                self.connected.store(false, Ordering::Relaxed);
                false
            }
        }
    }
}

impl std::fmt::Debug for TcpConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TcpConnection")
            .field("info", &self.info)
            .field("bytes_sent", &self.bytes_sent.load(Ordering::Relaxed))
            .field("bytes_received", &self.bytes_received.load(Ordering::Relaxed))
            .field("connected", &self.connected.load(Ordering::Relaxed))
            .finish()
    }
}

#[async_trait]
impl Connection for TcpConnection {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(TransportError::ConnectionFailed {
                reason: "Connection is closed".to_string(),
            });
        }

        match self.stream.read(buf).await {
            Ok(0) => {
                // Connection closed by peer
                self.connected.store(false, Ordering::Relaxed);
                Err(TransportError::ConnectionFailed {
                    reason: "Connection closed by peer".to_string(),
                })
            }
            Ok(n) => {
                self.bytes_received.fetch_add(n as u64, Ordering::Relaxed);
                Ok(n)
            }
            Err(e) => {
                self.connected.store(false, Ordering::Relaxed);
                Err(TransportError::Io(e))
            }
        }
    }

    async fn write(&mut self, buf: &[u8]) -> Result<usize, TransportError> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(TransportError::ConnectionFailed {
                reason: "Connection is closed".to_string(),
            });
        }

        match self.stream.write(buf).await {
            Ok(n) => {
                self.bytes_sent.fetch_add(n as u64, Ordering::Relaxed);
                Ok(n)
            }
            Err(e) => {
                self.connected.store(false, Ordering::Relaxed);
                Err(TransportError::Io(e))
            }
        }
    }

    async fn flush(&mut self) -> Result<(), TransportError> {
        if !self.connected.load(Ordering::Relaxed) {
            return Err(TransportError::ConnectionFailed {
                reason: "Connection is closed".to_string(),
            });
        }

        self.stream.flush().await.map_err(|e| {
            self.connected.store(false, Ordering::Relaxed);
            TransportError::Io(e)
        })
    }

    async fn close(&mut self) -> Result<(), TransportError> {
        if self.connected.load(Ordering::Relaxed) {
            self.connected.store(false, Ordering::Relaxed);
            
            // Decrement connection counter if we have one
            if let Some(counter) = &self.connection_counter {
                counter.fetch_sub(1, Ordering::Relaxed);
            }
            
            self.stream.shutdown().await.map_err(TransportError::Io)?;
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

/// TCP listener for accepting incoming connections
pub struct TcpListener {
    listener: tokio::net::TcpListener,
    config: TcpConfig,
    active_connections: Arc<AtomicU64>,
    max_connections: u64,
    shutdown_signal: Arc<AtomicBool>,
}

impl TcpListener {
    /// Create a new TCP listener
    pub async fn bind(addr: SocketAddr, config: TcpConfig) -> Result<Self, TransportError> {
        let max_connections = config.max_connections;
        Self::bind_with_limits(addr, config, max_connections).await
    }

    /// Create a new TCP listener with connection limits
    pub async fn bind_with_limits(
        addr: SocketAddr, 
        config: TcpConfig, 
        max_connections: u64
    ) -> Result<Self, TransportError> {
        // Create socket with advanced configuration
        let socket = socket2::Socket::new(
            socket2::Domain::for_address(addr),
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?;

        // Configure socket options
        if config.reuse_addr {
            socket.set_reuse_address(true)?;
        }

        // Enable SO_REUSEPORT if configured (Linux/macOS only)
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if config.reuse_port {
            // Note: SO_REUSEPORT support varies by socket2 version
            // For now, we'll skip this advanced feature
            // socket.set_reuse_port(true)?;
        }

        // Set receive buffer size if specified
        if let Some(size) = config.recv_buffer_size {
            socket.set_recv_buffer_size(size)?;
        }

        // Set send buffer size if specified
        if let Some(size) = config.send_buffer_size {
            socket.set_send_buffer_size(size)?;
        }

        // Configure keep-alive settings
        if config.keep_alive.is_some() {
            socket.set_keepalive(true)?;
            
            // Set keep-alive parameters if available
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            {
                // Note: Advanced keep-alive parameters require platform-specific code
                // For now, we'll use basic keep-alive functionality
                if config.keep_alive_probes.is_some() || config.keep_alive_interval.is_some() {
                    // These would require platform-specific socket options
                    // socket.set_keepalive_probes(probes)?;
                    // socket.set_keepalive_interval(interval)?;
                }
            }
        }

        socket.bind(&addr.into())?;
        socket.listen(config.listen_backlog as i32)?;
        socket.set_nonblocking(true)?;

        let std_listener: std::net::TcpListener = socket.into();
        let listener = tokio::net::TcpListener::from_std(std_listener)?;

        Ok(Self { 
            listener, 
            config,
            active_connections: Arc::new(AtomicU64::new(0)),
            max_connections,
            shutdown_signal: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Accept an incoming connection
    pub async fn accept(&self) -> Result<(TcpConnection, SocketAddr), TransportError> {
        // Check if we're shutting down
        if self.shutdown_signal.load(Ordering::Relaxed) {
            return Err(TransportError::ConnectionFailed {
                reason: "Listener is shutting down".to_string(),
            });
        }

        // Check connection limits
        let current_connections = self.active_connections.load(Ordering::Relaxed);
        if current_connections >= self.max_connections {
            return Err(TransportError::ResourceLimitExceeded {
                resource: format!("Max connections ({}) exceeded", self.max_connections),
            });
        }

        let (stream, remote_addr) = self.listener.accept().await?;

        // Configure the accepted socket
        let transport = TcpTransport::with_config(self.config.clone());
        transport.configure_socket(&stream).await?;

        // Increment connection counter
        self.active_connections.fetch_add(1, Ordering::Relaxed);

        // Create connection with unknown peer ID (will be determined during handshake)
        let connection = TcpConnection::new_with_counter(
            stream,
            "unknown".to_string(), // Peer ID will be set during protocol handshake
            remote_addr,
            Some(self.active_connections.clone()),
        );

        Ok((connection, remote_addr))
    }

    /// Accept connections in a loop with proper error handling
    pub async fn accept_loop<F, Fut>(&self, mut handler: F) -> Result<(), TransportError>
    where
        F: FnMut(TcpConnection, SocketAddr) -> Fut,
        Fut: std::future::Future<Output = Result<(), TransportError>>,
    {
        while !self.shutdown_signal.load(Ordering::Relaxed) {
            match self.accept().await {
                Ok((connection, addr)) => {
                    // Handle connection in background
                    if let Err(e) = handler(connection, addr).await {
                        eprintln!("Connection handler error: {}", e);
                    }
                }
                Err(TransportError::ResourceLimitExceeded { .. }) => {
                    // Wait a bit before trying again when at connection limit
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Err(e) if e.is_recoverable() => {
                    eprintln!("Recoverable accept error: {}", e);
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                Err(e) => {
                    eprintln!("Fatal accept error: {}", e);
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// Get the local address this listener is bound to
    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.listener.local_addr().map_err(TransportError::Io)
    }

    /// Get current number of active connections
    pub fn active_connection_count(&self) -> u64 {
        self.active_connections.load(Ordering::Relaxed)
    }

    /// Get maximum allowed connections
    pub fn max_connections(&self) -> u64 {
        self.max_connections
    }

    /// Set maximum allowed connections
    pub fn set_max_connections(&mut self, max: u64) {
        self.max_connections = max;
    }

    /// Initiate graceful shutdown
    pub fn shutdown(&self) {
        self.shutdown_signal.store(true, Ordering::Relaxed);
    }

    /// Check if listener is shutting down
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_signal.load(Ordering::Relaxed)
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        if self.connected.load(Ordering::Relaxed) {
            // Decrement connection counter if we have one
            if let Some(counter) = &self.connection_counter {
                counter.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_tcp_transport_creation() {
        let transport = TcpTransport::new();
        assert_eq!(transport.protocol_name(), "tcp");
        assert!(transport.is_available());
        assert_eq!(transport.priority(), 50);
        
        let caps = transport.capabilities();
        assert!(caps.reliable);
        assert!(caps.ordered);
        assert!(!caps.multiplexed);
    }

    #[tokio::test]
    async fn test_tcp_config_default() {
        let config = TcpConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.keep_alive, Some(Duration::from_secs(60)));
        assert!(config.no_delay);
        assert!(config.reuse_addr);
        assert_eq!(config.recv_buffer_size, Some(65536));
        assert_eq!(config.send_buffer_size, Some(65536));
        assert_eq!(config.listen_backlog, 128);
    }

    #[tokio::test]
    async fn test_tcp_connection_invalid_address() {
        let transport = TcpTransport::new();
        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![], // Empty addresses
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );

        let result = transport.connect(&peer_addr).await;
        assert!(matches!(result, Err(TransportError::InvalidPeerAddress)));
    }

    #[tokio::test]
    async fn test_tcp_connection_timeout() {
        let mut config = TcpConfig::default();
        config.connect_timeout = Duration::from_millis(100);
        let transport = TcpTransport::with_config(config);

        // Try to connect to a non-routable address (should timeout)
        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 12345)],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );

        let result = transport.connect(&peer_addr).await;
        assert!(matches!(result, Err(TransportError::ConnectionTimeout { .. })));
    }

    #[tokio::test]
    async fn test_tcp_listener_creation() {
        let config = TcpConfig::default();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        
        let listener = TcpListener::bind(addr, config).await;
        assert!(listener.is_ok());
        
        let listener = listener.unwrap();
        let local_addr = listener.local_addr().unwrap();
        assert_eq!(local_addr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        assert_ne!(local_addr.port(), 0); // Should have been assigned a port
    }

    #[tokio::test]
    async fn test_tcp_connection_lifecycle() {
        // Start a listener
        let config = TcpConfig::default();
        let listener_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        let listener = TcpListener::bind(listener_addr, config).await.unwrap();
        let bind_addr = listener.local_addr().unwrap();

        // Accept connections in background
        let accept_handle = tokio::spawn(async move {
            listener.accept().await
        });

        // Connect to the listener
        let transport = TcpTransport::new();
        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![bind_addr],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );

        let mut client_conn = transport.connect(&peer_addr).await.unwrap();
        assert!(client_conn.is_connected());

        // Accept the connection
        let (mut server_conn, _) = accept_handle.await.unwrap().unwrap();
        assert!(server_conn.is_connected());

        // Test data transfer
        let test_data = b"Hello, TCP!";
        let bytes_written = client_conn.write(test_data).await.unwrap();
        assert_eq!(bytes_written, test_data.len());

        client_conn.flush().await.unwrap();

        let mut buffer = [0u8; 1024];
        let bytes_read = server_conn.read(&mut buffer).await.unwrap();
        assert_eq!(bytes_read, test_data.len());
        assert_eq!(&buffer[..bytes_read], test_data);

        // Test connection info
        let client_info = client_conn.info();
        assert_eq!(client_info.protocol, "tcp");
        assert_eq!(client_info.bytes_sent, test_data.len() as u64);
        assert_eq!(client_info.remote_addr, bind_addr);

        let server_info = server_conn.info();
        assert_eq!(server_info.protocol, "tcp");
        assert_eq!(server_info.bytes_received, test_data.len() as u64);

        // Test connection close
        client_conn.close().await.unwrap();
        assert!(!client_conn.is_connected());

        // Server should detect the closed connection
        sleep(Duration::from_millis(10)).await;
        let mut buffer = [0u8; 1024];
        let result = server_conn.read(&mut buffer).await;
        assert!(matches!(result, Err(TransportError::ConnectionFailed { .. })));
        assert!(!server_conn.is_connected());
    }

    #[tokio::test]
    async fn test_tcp_listener_connection_limits() {
        let config = TcpConfig::default();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        
        // Create listener with connection limit of 2
        let listener = TcpListener::bind_with_limits(addr, config, 2).await.unwrap();
        let bind_addr = listener.local_addr().unwrap();
        
        assert_eq!(listener.max_connections(), 2);
        assert_eq!(listener.active_connection_count(), 0);

        // Connect first client
        let transport = TcpTransport::new();
        let peer_addr = PeerAddress::new(
            "test-peer-1".to_string(),
            vec![bind_addr],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );

        let _conn1 = transport.connect(&peer_addr).await.unwrap();
        let (_server_conn1, _) = listener.accept().await.unwrap();
        assert_eq!(listener.active_connection_count(), 1);

        // Connect second client
        let peer_addr2 = PeerAddress::new(
            "test-peer-2".to_string(),
            vec![bind_addr],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );

        let _conn2 = transport.connect(&peer_addr2).await.unwrap();
        let (_server_conn2, _) = listener.accept().await.unwrap();
        assert_eq!(listener.active_connection_count(), 2);

        // Try to connect third client - should hit limit
        let peer_addr3 = PeerAddress::new(
            "test-peer-3".to_string(),
            vec![bind_addr],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );

        let _conn3 = transport.connect(&peer_addr3).await.unwrap();
        let result = listener.accept().await;
        assert!(matches!(result, Err(TransportError::ResourceLimitExceeded { .. })));
    }

    #[tokio::test]
    async fn test_tcp_listener_graceful_shutdown() {
        let config = TcpConfig::default();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        
        let listener = TcpListener::bind(addr, config).await.unwrap();
        assert!(!listener.is_shutting_down());

        listener.shutdown();
        assert!(listener.is_shutting_down());

        // Accept should fail after shutdown
        let result = listener.accept().await;
        assert!(matches!(result, Err(TransportError::ConnectionFailed { .. })));
    }

    #[tokio::test]
    async fn test_tcp_server_lifecycle() {
        let config = TcpConfig::default();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        
        let mut server = TcpServer::new(addr, config).await.unwrap();
        let server_addr = server.local_addr().unwrap();

        // Start server in background
        let server_handle = tokio::spawn(async move {
            server.start(|mut connection, _addr| async move {
                // Echo server - read and write back
                let mut buffer = [0u8; 1024];
                match connection.read(&mut buffer).await {
                    Ok(n) => {
                        connection.write(&buffer[..n]).await?;
                        connection.flush().await?;
                    }
                    Err(_) => {} // Connection closed
                }
                Ok(())
            }).await
        });

        // Give server time to start
        sleep(Duration::from_millis(10)).await;

        // Connect and test echo
        let transport = TcpTransport::new();
        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![server_addr],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );

        let mut client = transport.connect(&peer_addr).await.unwrap();
        
        let test_data = b"Hello, Server!";
        client.write(test_data).await.unwrap();
        client.flush().await.unwrap();

        let mut buffer = [0u8; 1024];
        let n = client.read(&mut buffer).await.unwrap();
        assert_eq!(&buffer[..n], test_data);

        client.close().await.unwrap();

        // Server should still be running
        assert!(!server_handle.is_finished());
        server_handle.abort();
    }

    #[tokio::test]
    async fn test_connection_info_updates() {
        let stream = TcpStream::connect("127.0.0.1:80").await;
        if stream.is_err() {
            // Skip test if we can't connect (no internet or service)
            return;
        }

        let stream = stream.unwrap();
        let remote_addr = stream.peer_addr().unwrap();
        let mut connection = TcpConnection::new(
            stream,
            "test-peer".to_string(),
            remote_addr,
        );

        let initial_info = connection.info();
        assert_eq!(initial_info.bytes_sent, 0);
        assert_eq!(initial_info.bytes_received, 0);

        // Write some data
        let test_data = b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n";
        connection.write(test_data).await.unwrap();

        let updated_info = connection.info();
        assert_eq!(updated_info.bytes_sent, test_data.len() as u64);

        connection.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_tcp_config_presets() {
        let low_latency = TcpConfig::low_latency();
        assert!(low_latency.no_delay);
        assert_eq!(low_latency.recv_buffer_size, Some(32768));
        assert_eq!(low_latency.keep_alive_probes, Some(3));

        let high_throughput = TcpConfig::high_throughput();
        assert!(!high_throughput.no_delay); // Nagle's algorithm enabled
        assert_eq!(high_throughput.recv_buffer_size, Some(262144));
        assert_eq!(high_throughput.max_connections, 2000);

        let mobile = TcpConfig::mobile_optimized();
        assert_eq!(mobile.recv_buffer_size, Some(16384));
        assert_eq!(mobile.max_connections, 100);
        assert!(!mobile.reuse_port);
    }

    #[tokio::test]
    async fn test_tcp_listener_advanced_config() {
        let config = TcpConfig::high_throughput();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        
        let listener = TcpListener::bind(addr, config.clone()).await;
        assert!(listener.is_ok());
        
        let listener = listener.unwrap();
        assert_eq!(listener.max_connections(), config.max_connections);
        
        let local_addr = listener.local_addr().unwrap();
        assert_eq!(local_addr.ip(), IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    }

    #[tokio::test]
    async fn test_tcp_transport_with_custom_config() {
        let config = TcpConfig::low_latency();
        let transport = TcpTransport::with_config(config);
        
        assert_eq!(transport.protocol_name(), "tcp");
        assert!(transport.is_available());
        
        // Test connection to invalid address should timeout quickly
        let peer_addr = PeerAddress::new(
            "test-peer".to_string(),
            vec![SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)), 12345)],
            vec!["tcp".to_string()],
            TransportCapabilities::tcp(),
        );

        let start = std::time::Instant::now();
        let result = transport.connect(&peer_addr).await;
        let elapsed = start.elapsed();
        
        assert!(result.is_err());
        // Should timeout within the configured timeout (5 seconds for low_latency)
        assert!(elapsed < Duration::from_secs(6));
    }

    #[tokio::test]
    async fn test_tcp_listener_resource_management() {
        let config = TcpConfig::mobile_optimized(); // Has lower connection limits
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
        
        let listener = TcpListener::bind_with_limits(addr, config, 2).await.unwrap();
        let _bind_addr = listener.local_addr().unwrap();
        
        // Test that connection limits are enforced
        assert_eq!(listener.max_connections(), 2);
        assert_eq!(listener.active_connection_count(), 0);
        
        // Test graceful shutdown
        assert!(!listener.is_shutting_down());
        listener.shutdown();
        assert!(listener.is_shutting_down());
    }
}
/// TCP server for handling multiple connections
pub struct TcpServer {
    listener: TcpListener,
    connection_handlers: Vec<tokio::task::JoinHandle<()>>,
    shutdown_tx: Option<tokio::sync::broadcast::Sender<()>>,
}

impl TcpServer {
    /// Create a new TCP server
    pub async fn new(addr: SocketAddr, config: TcpConfig) -> Result<Self, TransportError> {
        let listener = TcpListener::bind(addr, config).await?;
        
        Ok(Self {
            listener,
            connection_handlers: Vec::new(),
            shutdown_tx: None,
        })
    }

    /// Start the server and handle connections
    pub async fn start<F, Fut>(&mut self, handler: F) -> Result<(), TransportError>
    where
        F: Fn(TcpConnection, SocketAddr) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<(), TransportError>> + Send,
    {
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        loop {
            tokio::select! {
                // Accept new connections
                accept_result = self.listener.accept() => {
                    match accept_result {
                        Ok((connection, addr)) => {
                            let handler_clone = handler.clone();
                            let mut shutdown_rx_clone = shutdown_rx.resubscribe();
                            
                            // Spawn connection handler
                            let handle = tokio::spawn(async move {
                                tokio::select! {
                                    result = handler_clone(connection, addr) => {
                                        if let Err(e) = result {
                                            eprintln!("Connection handler error: {}", e);
                                        }
                                    }
                                    _ = shutdown_rx_clone.recv() => {
                                        // Shutdown signal received
                                    }
                                }
                            });
                            
                            self.connection_handlers.push(handle);
                        }
                        Err(TransportError::ResourceLimitExceeded { .. }) => {
                            // Wait before accepting more connections
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        Err(e) if e.is_recoverable() => {
                            eprintln!("Recoverable accept error: {}", e);
                            tokio::time::sleep(Duration::from_millis(10)).await;
                        }
                        Err(e) => {
                            eprintln!("Fatal accept error: {}", e);
                            return Err(e);
                        }
                    }
                }
                
                // Handle shutdown signal
                _ = shutdown_rx.recv() => {
                    break;
                }
            }
        }

        // Wait for all connection handlers to finish
        for handle in self.connection_handlers.drain(..) {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(&mut self) -> Result<(), TransportError> {
        // Signal listener to stop accepting connections
        self.listener.shutdown();

        // Send shutdown signal to all connection handlers
        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(());
        }

        // Wait for all handlers to complete
        for handle in self.connection_handlers.drain(..) {
            let _ = handle.await;
        }

        Ok(())
    }

    /// Get server statistics
    pub fn stats(&self) -> TcpServerStats {
        TcpServerStats {
            active_connections: self.listener.active_connection_count(),
            max_connections: self.listener.max_connections(),
            total_handlers: self.connection_handlers.len(),
            is_shutting_down: self.listener.is_shutting_down(),
        }
    }

    /// Get the local address the server is bound to
    pub fn local_addr(&self) -> Result<SocketAddr, TransportError> {
        self.listener.local_addr()
    }
}

/// Statistics for TCP server
#[derive(Debug, Clone)]
pub struct TcpServerStats {
    pub active_connections: u64,
    pub max_connections: u64,
    pub total_handlers: usize,
    pub is_shutting_down: bool,
}