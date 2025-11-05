use crate::discovery::{Discovery, DiscoveryError, ServiceRecord};
use async_trait::async_trait;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener as TokioTcpListener, TcpStream as TokioTcpStream};
use tokio::sync::RwLock;


const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_CONCURRENT_SCANS: usize = 10;
const KIZUNA_HELLO: &str = "KIZUNA_HELLO";
const KIZUNA_PEER: &str = "KIZUNA_PEER";

pub struct TcpDiscovery {
    peer_id: String,
    device_name: String,
    port: u16,
    scan_ports: Vec<u16>,
    capabilities: HashMap<String, String>,
    listener: Arc<RwLock<Option<TokioTcpListener>>>,
}

impl TcpDiscovery {
    pub fn new() -> Self {
        let mut capabilities = HashMap::new();
        capabilities.insert("version".to_string(), "1.0.0".to_string());
        capabilities.insert("protocol".to_string(), "tcp".to_string());
        
        Self {
            peer_id: format!("kizuna-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            device_name: "Kizuna Device".to_string(),
            port: 41337,
            scan_ports: vec![41337, 8080, 3000, 3001, 3002, 3003, 3004, 3005],
            capabilities,
            listener: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_config(peer_id: String, device_name: String, port: u16, scan_ports: Vec<u16>) -> Self {
        let mut capabilities = HashMap::new();
        capabilities.insert("version".to_string(), "1.0.0".to_string());
        capabilities.insert("protocol".to_string(), "tcp".to_string());
        
        Self {
            peer_id,
            device_name,
            port,
            scan_ports,
            capabilities,
            listener: Arc::new(RwLock::new(None)),
        }
    }

    /// Scan a specific host and port for Kizuna services
    async fn scan_host_port(&self, host: IpAddr, port: u16) -> Option<ServiceRecord> {
        let addr = SocketAddr::new(host, port);
        
        // Attempt to connect with timeout
        let mut stream = match tokio::time::timeout(HANDSHAKE_TIMEOUT, TokioTcpStream::connect(addr)).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(_)) => return None, // Connection refused or network error
            Err(_) => return None, // Connection timeout
        };
        
        // Perform handshake and handle all possible errors gracefully
        match self.perform_handshake(&mut stream, addr).await {
            Ok(record) => Some(record),
            Err(_) => {
                // Handshake failed - ensure connection is closed
                let _ = stream.shutdown().await;
                None
            }
        }
    }

    /// Perform TCP handshake protocol with a peer
    async fn perform_handshake(&self, stream: &mut TokioTcpStream, peer_addr: SocketAddr) -> Result<ServiceRecord, DiscoveryError> {
        // Send KIZUNA_HELLO message
        let hello_msg = format!("{}|{}|{}\n", KIZUNA_HELLO, "1.0.0", self.peer_id);
        
        // Handle write timeout and errors
        match tokio::time::timeout(HANDSHAKE_TIMEOUT, stream.write_all(hello_msg.as_bytes())).await {
            Ok(Ok(_)) => {}, // Success
            Ok(Err(e)) => return Err(DiscoveryError::Network(e)),
            Err(_) => return Err(DiscoveryError::Timeout { timeout: HANDSHAKE_TIMEOUT }),
        }
        
        // Flush to ensure message is sent
        if let Err(_) = tokio::time::timeout(HANDSHAKE_TIMEOUT, stream.flush()).await {
            return Err(DiscoveryError::Timeout { timeout: HANDSHAKE_TIMEOUT });
        }
        
        // Read response with proper error handling
        let mut buffer = [0u8; 1024];
        let bytes_read = match tokio::time::timeout(HANDSHAKE_TIMEOUT, stream.read(&mut buffer)).await {
            Ok(Ok(0)) => {
                return Err(DiscoveryError::InvalidServiceRecord {
                    reason: "Connection closed by peer during handshake".to_string(),
                });
            }
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(DiscoveryError::Network(e)),
            Err(_) => return Err(DiscoveryError::Timeout { timeout: HANDSHAKE_TIMEOUT }),
        };
        
        let response = String::from_utf8_lossy(&buffer[..bytes_read]);
        
        // Gracefully close the connection
        let _ = stream.shutdown().await;
        
        self.parse_handshake_response(&response, peer_addr)
    }

    /// Parse handshake response and create ServiceRecord
    fn parse_handshake_response(&self, response: &str, peer_addr: SocketAddr) -> Result<ServiceRecord, DiscoveryError> {
        let response = response.trim();
        let parts: Vec<&str> = response.split('|').collect();
        
        if parts.len() < 4 || parts[0] != KIZUNA_PEER {
            return Err(DiscoveryError::InvalidServiceRecord {
                reason: "Invalid handshake response format".to_string(),
            });
        }
        
        let peer_id = parts[1].to_string();
        let name = parts[2].to_string();
        let port: u16 = parts[3].parse()
            .map_err(|_| DiscoveryError::InvalidServiceRecord {
                reason: "Invalid port in handshake response".to_string(),
            })?;
        
        let mut record = ServiceRecord::new(peer_id, name, port);
        record.add_address(peer_addr);
        record.set_discovery_method("tcp".to_string());
        
        // Parse capabilities if present
        if parts.len() > 4 && !parts[4].is_empty() {
            for cap_pair in parts[4].split(',') {
                if let Some((key, value)) = cap_pair.split_once('=') {
                    record.add_capability(key.to_string(), value.to_string());
                }
            }
        }
        
        Ok(record)
    }

    /// Get local network IP addresses for scanning
    fn get_local_network_ips(&self) -> Vec<IpAddr> {
        // For now, scan common local network ranges
        // In a real implementation, you'd get the actual network interfaces
        let mut ips = Vec::new();
        
        // Add localhost
        ips.push(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
        
        // Add a smaller range for testing - just scan first 10 IPs in common ranges
        for i in 1..=10 {
            ips.push(IpAddr::V4(Ipv4Addr::new(192, 168, 1, i)));
        }
        
        ips
    }

    /// Handle incoming TCP handshake connection
    async fn handle_handshake_connection(&self, mut stream: TokioTcpStream) -> Result<(), DiscoveryError> {
        let mut buffer = [0u8; 1024];
        
        // Read the handshake request with timeout
        let bytes_read = match tokio::time::timeout(HANDSHAKE_TIMEOUT, stream.read(&mut buffer)).await {
            Ok(Ok(0)) => {
                // Connection closed immediately
                return Ok(());
            }
            Ok(Ok(n)) => n,
            Ok(Err(_)) | Err(_) => {
                // Network error or timeout - close connection gracefully
                let _ = stream.shutdown().await;
                return Ok(());
            }
        };
        
        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        let request = request.trim();
        
        // Parse and validate KIZUNA_HELLO message
        let parts: Vec<&str> = request.split('|').collect();
        if parts.len() >= 3 && parts[0] == KIZUNA_HELLO {
            // Validate version compatibility (basic check)
            let peer_version = parts[1];
            if !self.is_version_compatible(peer_version) {
                let _ = stream.shutdown().await;
                return Ok(());
            }
            
            // Don't respond to ourselves
            let peer_id = parts[2];
            if peer_id == self.peer_id {
                let _ = stream.shutdown().await;
                return Ok(());
            }
            
            // Send KIZUNA_PEER response
            let capabilities_str = self.capabilities
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(",");
            
            let response = format!("{}|{}|{}|{}|{}\n", 
                KIZUNA_PEER, 
                self.peer_id, 
                self.device_name, 
                self.port,
                capabilities_str
            );
            
            // Send response with timeout and error handling
            match tokio::time::timeout(HANDSHAKE_TIMEOUT, stream.write_all(response.as_bytes())).await {
                Ok(Ok(_)) => {
                    // Flush to ensure response is sent
                    let _ = tokio::time::timeout(HANDSHAKE_TIMEOUT, stream.flush()).await;
                }
                _ => {
                    // Write failed or timed out
                }
            }
        }
        
        // Always close the connection gracefully
        let _ = stream.shutdown().await;
        Ok(())
    }

    /// Check if the peer version is compatible with ours
    fn is_version_compatible(&self, peer_version: &str) -> bool {
        // Simple version compatibility check
        // In a real implementation, you'd have more sophisticated version checking
        match peer_version {
            "1.0.0" | "1.0" | "1" => true,
            _ => false, // Unknown version, assume incompatible
        }
    }

    /// Start TCP beacon server
    async fn start_beacon_server(&self) -> Result<(), DiscoveryError> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), self.port);
        let listener = TokioTcpListener::bind(addr).await
            .map_err(DiscoveryError::Network)?;
        
        // Store the listener
        {
            let mut listener_guard = self.listener.write().await;
            *listener_guard = Some(listener);
        }
        
        // Spawn task to handle incoming connections
        let listener_clone = self.listener.clone();
        let self_clone = TcpDiscovery {
            peer_id: self.peer_id.clone(),
            device_name: self.device_name.clone(),
            port: self.port,
            scan_ports: self.scan_ports.clone(),
            capabilities: self.capabilities.clone(),
            listener: listener_clone,
        };
        
        tokio::spawn(async move {
            loop {
                let listener_guard = self_clone.listener.read().await;
                if let Some(ref listener) = *listener_guard {
                    match listener.accept().await {
                        Ok((stream, _)) => {
                            let handler_clone = TcpDiscovery {
                                peer_id: self_clone.peer_id.clone(),
                                device_name: self_clone.device_name.clone(),
                                port: self_clone.port,
                                scan_ports: self_clone.scan_ports.clone(),
                                capabilities: self_clone.capabilities.clone(),
                                listener: self_clone.listener.clone(),
                            };
                            
                            tokio::spawn(async move {
                                let _ = handler_clone.handle_handshake_connection(stream).await;
                            });
                        }
                        Err(_) => break, // Listener closed
                    }
                } else {
                    break; // No listener
                }
            }
        });
        
        Ok(())
    }
}

#[async_trait]
impl Discovery for TcpDiscovery {
    async fn discover(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        let mut discovered_peers = Vec::new();
        let local_ips = self.get_local_network_ips();
        
        // Create semaphore to limit concurrent connections
        let semaphore = Arc::new(tokio::sync::Semaphore::new(MAX_CONCURRENT_SCANS));
        let mut tasks = Vec::new();
        
        // Scan each IP and port combination
        for ip in local_ips {
            for &port in &self.scan_ports {
                let semaphore_clone = semaphore.clone();
                let self_clone = TcpDiscovery {
                    peer_id: self.peer_id.clone(),
                    device_name: self.device_name.clone(),
                    port: self.port,
                    scan_ports: self.scan_ports.clone(),
                    capabilities: self.capabilities.clone(),
                    listener: self.listener.clone(),
                };
                
                let task = tokio::spawn(async move {
                    let _permit = semaphore_clone.acquire().await.unwrap();
                    self_clone.scan_host_port(ip, port).await
                });
                
                tasks.push(task);
            }
        }
        
        // Wait for all scans to complete with timeout
        let scan_results = tokio::time::timeout(timeout, futures::future::join_all(tasks)).await
            .map_err(|_| DiscoveryError::Timeout { timeout })?;
        
        // Collect successful discoveries
        for result in scan_results {
            if let Ok(Some(record)) = result {
                // Don't discover ourselves
                if record.peer_id != self.peer_id {
                    discovered_peers.push(record);
                }
            }
        }
        
        Ok(discovered_peers)
    }

    async fn announce(&self) -> Result<(), DiscoveryError> {
        // Check if already announcing
        {
            let listener_guard = self.listener.read().await;
            if listener_guard.is_some() {
                return Ok(()); // Already announcing
            }
        }
        
        self.start_beacon_server().await
    }

    async fn stop_announce(&self) -> Result<(), DiscoveryError> {
        let mut listener_guard = self.listener.write().await;
        if let Some(listener) = listener_guard.take() {
            drop(listener); // Dropping the listener will close it
        }
        Ok(())
    }

    fn strategy_name(&self) -> &'static str {
        "tcp"
    }

    fn is_available(&self) -> bool {
        // TCP should be available on all platforms
        true
    }

    fn priority(&self) -> u8 {
        // Lower priority - more intrusive than other methods
        30
    }
}

impl Default for TcpDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_tcp_discovery_creation() {
        let discovery = TcpDiscovery::new();
        
        assert_eq!(discovery.strategy_name(), "tcp");
        assert!(discovery.is_available());
        assert_eq!(discovery.priority(), 30);
        assert_eq!(discovery.port, 41337);
        assert!(!discovery.scan_ports.is_empty());
        assert!(discovery.capabilities.contains_key("version"));
        assert!(discovery.capabilities.contains_key("protocol"));
    }

    #[tokio::test]
    async fn test_tcp_discovery_with_config() {
        let peer_id = "test-peer-123".to_string();
        let device_name = "Test Device".to_string();
        let port = 8080;
        let scan_ports = vec![8080, 9090];
        
        let discovery = TcpDiscovery::with_config(peer_id.clone(), device_name.clone(), port, scan_ports.clone());
        
        assert_eq!(discovery.peer_id, peer_id);
        assert_eq!(discovery.device_name, device_name);
        assert_eq!(discovery.port, port);
        assert_eq!(discovery.scan_ports, scan_ports);
    }

    #[tokio::test]
    async fn test_handshake_response_parsing() {
        let discovery = TcpDiscovery::new();
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        
        // Test valid response
        let response = "KIZUNA_PEER|peer-123|Test Device|8080|version=1.0.0,protocol=tcp";
        let result = discovery.parse_handshake_response(response, peer_addr);
        
        assert!(result.is_ok());
        let record = result.unwrap();
        assert_eq!(record.peer_id, "peer-123");
        assert_eq!(record.name, "Test Device");
        assert_eq!(record.port, 8080);
        assert_eq!(record.discovery_method, "tcp");
        assert_eq!(record.addresses.len(), 1);
        assert_eq!(record.addresses[0], peer_addr);
        assert!(record.has_capability("version"));
        assert!(record.has_capability("protocol"));
    }

    #[tokio::test]
    async fn test_handshake_response_parsing_invalid() {
        let discovery = TcpDiscovery::new();
        let peer_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        
        // Test invalid response format
        let response = "INVALID_RESPONSE|peer-123";
        let result = discovery.parse_handshake_response(response, peer_addr);
        assert!(result.is_err());
        
        // Test invalid port
        let response = "KIZUNA_PEER|peer-123|Test Device|invalid_port|";
        let result = discovery.parse_handshake_response(response, peer_addr);
        assert!(result.is_err());
        
        // Test missing parts
        let response = "KIZUNA_PEER|peer-123";
        let result = discovery.parse_handshake_response(response, peer_addr);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_version_compatibility() {
        let discovery = TcpDiscovery::new();
        
        // Test compatible versions
        assert!(discovery.is_version_compatible("1.0.0"));
        assert!(discovery.is_version_compatible("1.0"));
        assert!(discovery.is_version_compatible("1"));
        
        // Test incompatible versions
        assert!(!discovery.is_version_compatible("2.0.0"));
        assert!(!discovery.is_version_compatible("0.9.0"));
        assert!(!discovery.is_version_compatible("invalid"));
    }

    #[tokio::test]
    async fn test_announce_and_stop_announce() {
        let discovery = TcpDiscovery::with_config(
            "test-peer".to_string(),
            "Test Device".to_string(),
            0, // Use port 0 to let OS assign a free port
            vec![8080],
        );
        
        // Test announce
        let result = discovery.announce().await;
        assert!(result.is_ok());
        
        // Verify listener is active
        {
            let listener_guard = discovery.listener.read().await;
            assert!(listener_guard.is_some());
        }
        
        // Test stop announce
        let result = discovery.stop_announce().await;
        assert!(result.is_ok());
        
        // Verify listener is stopped
        {
            let listener_guard = discovery.listener.read().await;
            assert!(listener_guard.is_none());
        }
    }

    #[tokio::test]
    async fn test_announce_idempotent() {
        let discovery = TcpDiscovery::with_config(
            "test-peer".to_string(),
            "Test Device".to_string(),
            0, // Use port 0 to let OS assign a free port
            vec![8080],
        );
        
        // First announce should succeed
        let result1 = discovery.announce().await;
        assert!(result1.is_ok());
        
        // Second announce should also succeed (idempotent)
        let result2 = discovery.announce().await;
        assert!(result2.is_ok());
        
        // Clean up
        let _ = discovery.stop_announce().await;
    }

    #[tokio::test]
    async fn test_scan_nonexistent_host() {
        let discovery = TcpDiscovery::new();
        
        // Scan a non-existent host/port combination
        let result = discovery.scan_host_port(
            IpAddr::V4(Ipv4Addr::new(192, 168, 255, 254)), 
            9999
        ).await;
        
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_discover_empty_network() {
        let discovery = TcpDiscovery::with_config(
            "test-peer".to_string(),
            "Test Device".to_string(),
            41337,
            vec![9999], // Use a port that's unlikely to be in use
        );
        
        // Override get_local_network_ips to return just localhost for faster testing
        let local_ips = vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))];
        
        // Discovery should complete without errors but find no peers
        let timeout_duration = Duration::from_secs(3);
        let result = discovery.discover(timeout_duration).await;
        
        assert!(result.is_ok());
        let peers = result.unwrap();
        assert!(peers.is_empty());
    }

    #[tokio::test]
    async fn test_handshake_protocol_integration() {
        // Wrap the entire test in a timeout
        let test_future = async {
            // Create two discovery instances
            let discovery1 = TcpDiscovery::with_config(
                "peer-1".to_string(),
                "Device 1".to_string(),
                0, // Let OS assign port
                vec![],
            );
            
            let discovery2 = TcpDiscovery::with_config(
                "peer-2".to_string(),
                "Device 2".to_string(),
                0, // Let OS assign port
                vec![],
            );
            
            // Start discovery1 as a beacon
            discovery1.announce().await.expect("Failed to start beacon");
            
            // Give the server a moment to start
            sleep(Duration::from_millis(50)).await;
            
            // Get the actual port that was assigned
            let actual_port = {
                let listener_guard = discovery1.listener.read().await;
                if let Some(ref listener) = *listener_guard {
                    listener.local_addr().unwrap().port()
                } else {
                    panic!("Listener not started");
                }
            };
            
            // Test handshake by connecting directly
            let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), actual_port);
            
            // Connect and perform handshake
            let mut stream = TokioTcpStream::connect(addr).await.expect("Failed to connect");
            let result = discovery2.perform_handshake(&mut stream, addr).await;
            assert!(result.is_ok());
            
            let record = result.unwrap();
            assert_eq!(record.peer_id, "peer-1");
            assert_eq!(record.name, "Device 1");
            assert_eq!(record.discovery_method, "tcp");
            
            // Clean up
            discovery1.stop_announce().await.expect("Failed to stop beacon");
        };
        
        // Run the test with a 5-second timeout
        tokio::time::timeout(Duration::from_secs(5), test_future)
            .await
            .expect("Test timed out");
    }
}