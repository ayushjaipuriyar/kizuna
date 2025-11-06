use crate::discovery::{Discovery, DiscoveryError, ServiceRecord};
use async_trait::async_trait;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;

pub struct UdpDiscovery {
    port: u16,
    peer_id: String,
    device_name: String,
    last_broadcast: Arc<RwLock<Option<Instant>>>,
    rate_limit_duration: Duration,
}

impl UdpDiscovery {
    pub fn new() -> Self {
        Self {
            port: 41337, // Updated to match design spec
            peer_id: format!("kizuna-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            device_name: "Kizuna Device".to_string(),
            last_broadcast: Arc::new(RwLock::new(None)),
            rate_limit_duration: Duration::from_secs(5), // Rate limit: max 1 broadcast per 5 seconds
        }
    }

    pub fn with_config(port: u16, peer_id: String, device_name: String) -> Self {
        Self {
            port,
            peer_id,
            device_name,
            last_broadcast: Arc::new(RwLock::new(None)),
            rate_limit_duration: Duration::from_secs(5),
        }
    }

    /// Check if we can send a broadcast (rate limiting)
    async fn can_broadcast(&self) -> bool {
        let last_broadcast = self.last_broadcast.read().await;
        match *last_broadcast {
            Some(last_time) => last_time.elapsed() >= self.rate_limit_duration,
            None => true,
        }
    }

    /// Update the last broadcast time
    async fn update_broadcast_time(&self) {
        let mut last_broadcast = self.last_broadcast.write().await;
        *last_broadcast = Some(Instant::now());
    }

    async fn send_discovery_broadcast(&self) -> Result<(), DiscoveryError> {
        // Check rate limiting
        if !self.can_broadcast().await {
            return Err(DiscoveryError::Network(
                "Rate limit exceeded for UDP broadcast".to_string()
            ));
        }

        // Create a socket for sending broadcasts
        let socket = UdpSocket::bind("0.0.0.0:0").await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;
        
        socket.set_broadcast(true)
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;

        // Try to broadcast on multiple interfaces
        let broadcast_addresses = vec![
            format!("255.255.255.255:{}", self.port), // Global broadcast
            format!("192.168.255.255:{}", self.port), // Common LAN broadcast
            format!("10.255.255.255:{}", self.port),  // Common private network broadcast
        ];

        let message = format!("DISCOVER_KIZUNA|{}|{}|{}", 
            self.peer_id, self.device_name, self.port);

        let mut success = false;
        for addr_str in broadcast_addresses {
            if let Ok(broadcast_addr) = addr_str.parse::<SocketAddr>() {
                if socket.send_to(message.as_bytes(), broadcast_addr).await.is_ok() {
                    success = true;
                }
            }
        }

        if success {
            self.update_broadcast_time().await;
            Ok(())
        } else {
            Err(DiscoveryError::Network(
                "Failed to send broadcast on any interface".to_string()
            ))
        }
    }

    async fn listen_for_responses(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        let bind_addr = format!("0.0.0.0:{}", self.port);
        let socket = UdpSocket::bind(&bind_addr).await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;

        let mut peers = Vec::new();
        let mut buf = [0u8; 2048]; // Increased buffer size for larger messages
        let mut seen_peer_ids = std::collections::HashSet::new();

        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            let remaining = deadline - tokio::time::Instant::now();
            
            // Break if we have very little time left
            if remaining < Duration::from_millis(100) {
                break;
            }
            
            match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                Ok(Ok((n, addr))) => {
                    // Validate message size
                    if n == 0 || n >= buf.len() {
                        continue;
                    }

                    let message = String::from_utf8_lossy(&buf[..n]);
                    
                    // Skip our own messages
                    if message.contains(&self.peer_id) {
                        continue;
                    }
                    
                    if let Some(peer) = self.parse_peer_response(&message, addr) {
                        // Avoid duplicate peers in the same discovery session
                        if !seen_peer_ids.contains(&peer.peer_id) {
                            seen_peer_ids.insert(peer.peer_id.clone());
                            peers.push(peer);
                        }
                    }
                }
                Ok(Err(e)) => {
                    // Log the error but continue listening
                    eprintln!("UDP receive error: {}", e);
                    continue;
                }
                Err(_) => {
                    // Timeout - break out of the loop
                    break;
                }
            }
        }

        Ok(peers)
    }

    fn parse_peer_response(&self, message: &str, addr: SocketAddr) -> Option<ServiceRecord> {
        // Handle different message formats:
        // "KIZUNA_PEER|<peer_id>|<name>|<port>|<addresses>|<capabilities>"
        // "DISCOVER_KIZUNA|<peer_id>|<name>|<port>" (discovery request)
        
        if message.starts_with("KIZUNA_PEER|") {
            self.parse_peer_message(message, addr)
        } else if message.starts_with("DISCOVER_KIZUNA|") {
            // This is a discovery request, we should respond to it and also record the peer
            if let Some(record) = self.parse_discovery_request(message, addr) {
                // Spawn a task to respond to the discovery request
                let self_peer_id = self.peer_id.clone();
                let self_device_name = self.device_name.clone();
                let self_port = self.port;
                tokio::spawn(async move {
                    if let Err(e) = Self::send_peer_response(self_peer_id, self_device_name, self_port, addr).await {
                        eprintln!("Failed to respond to discovery request: {}", e);
                    }
                });
                Some(record)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Parse a KIZUNA_PEER message
    fn parse_peer_message(&self, message: &str, addr: SocketAddr) -> Option<ServiceRecord> {
        let parts: Vec<&str> = message.split('|').collect();
        if parts.len() < 4 {
            return None;
        }

        let peer_id = parts[1].to_string();
        let name = parts[2].to_string();
        let port: u16 = parts[3].parse().ok()?;
        
        let mut record = ServiceRecord::new(peer_id, name, port);
        record.set_discovery_method("udp".to_string());
        
        // Parse addresses if present (part 4)
        if parts.len() > 4 && !parts[4].is_empty() {
            for addr_str in parts[4].split(',') {
                if let Ok(parsed_addr) = addr_str.parse::<SocketAddr>() {
                    record.add_address(parsed_addr);
                }
            }
        }
        
        // If no addresses were parsed, use the sender's address
        if record.addresses.is_empty() {
            record.add_address(SocketAddr::new(addr.ip(), port));
        }
        
        // Parse capabilities if present (part 5)
        if parts.len() > 5 && !parts[5].is_empty() {
            for cap in parts[5].split(',') {
                if let Some((key, value)) = cap.split_once('=') {
                    record.add_capability(key.to_string(), value.to_string());
                }
            }
        }
        
        Some(record)
    }

    /// Parse a DISCOVER_KIZUNA message
    fn parse_discovery_request(&self, message: &str, addr: SocketAddr) -> Option<ServiceRecord> {
        let parts: Vec<&str> = message.split('|').collect();
        if parts.len() < 3 {
            return None;
        }

        let peer_id = parts[1].to_string();
        let name = parts[2].to_string();
        let port: u16 = parts.get(3).and_then(|p| p.parse().ok()).unwrap_or(self.port);
        
        let mut record = ServiceRecord::new(peer_id, name, port);
        record.add_address(SocketAddr::new(addr.ip(), port));
        record.set_discovery_method("udp".to_string());
        
        Some(record)
    }

    /// Send a peer response message
    async fn send_peer_response(peer_id: String, device_name: String, port: u16, target_addr: SocketAddr) -> Result<(), DiscoveryError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;

        // Create response message with our peer information
        let response = format!("KIZUNA_PEER|{}|{}|{}||version=0.1.0,protocol=udp", 
            peer_id, device_name, port);

        socket.send_to(response.as_bytes(), target_addr).await
            .map_err(|e| DiscoveryError::Network(e.to_string()))?;

        Ok(())
    }


}

#[async_trait]
impl Discovery for UdpDiscovery {
    async fn discover(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        // Send broadcast discovery message
        self.send_discovery_broadcast().await?;
        
        // Listen for responses
        self.listen_for_responses(timeout).await
    }

    async fn announce(&self) -> Result<(), DiscoveryError> {
        // For UDP, we don't maintain persistent announcements
        // Instead, we respond to discovery requests when they come in
        // This is a no-op for UDP strategy
        Ok(())
    }

    async fn stop_announce(&self) -> Result<(), DiscoveryError> {
        // No persistent announcement to stop for UDP
        Ok(())
    }

    fn strategy_name(&self) -> &'static str {
        "udp"
    }

    fn is_available(&self) -> bool {
        // UDP should be available on all platforms
        true
    }

    fn priority(&self) -> u8 {
        // Medium priority - works everywhere but not as elegant as mDNS
        50
    }
}

impl Default for UdpDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

// UdpDiscovery is already public, no need to re-export
#[cfg(
test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_udp_discovery_creation() {
        let discovery = UdpDiscovery::new();
        assert_eq!(discovery.port, 41337);
        assert!(discovery.peer_id.starts_with("kizuna-"));
        assert_eq!(discovery.device_name, "Kizuna Device");
        assert_eq!(discovery.rate_limit_duration, Duration::from_secs(5));
    }

    #[test]
    fn test_udp_discovery_with_config() {
        let discovery = UdpDiscovery::with_config(
            8080,
            "test-peer-123".to_string(),
            "Test Device".to_string(),
        );
        assert_eq!(discovery.port, 8080);
        assert_eq!(discovery.peer_id, "test-peer-123");
        assert_eq!(discovery.device_name, "Test Device");
    }

    #[test]
    fn test_discovery_trait_methods() {
        let discovery = UdpDiscovery::new();
        assert_eq!(discovery.strategy_name(), "udp");
        assert!(discovery.is_available());
        assert_eq!(discovery.priority(), 50);
    }

    #[tokio::test]
    async fn test_rate_limiting() {
        let discovery = UdpDiscovery::new();
        
        // First broadcast should be allowed
        assert!(discovery.can_broadcast().await);
        
        // Update broadcast time
        discovery.update_broadcast_time().await;
        
        // Immediate second broadcast should be blocked
        assert!(!discovery.can_broadcast().await);
        
        // After waiting, should be allowed again
        // Note: In real tests, we'd wait the full duration, but for unit tests
        // we can test the logic without actually waiting
    }

    #[test]
    fn test_parse_peer_message() {
        let discovery = UdpDiscovery::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        
        // Test valid KIZUNA_PEER message
        let message = "KIZUNA_PEER|peer-123|Test Device|8080|192.168.1.100:8080|version=1.0.0,features=chat";
        let record = discovery.parse_peer_message(message, addr).unwrap();
        
        assert_eq!(record.peer_id, "peer-123");
        assert_eq!(record.name, "Test Device");
        assert_eq!(record.port, 8080);
        assert_eq!(record.discovery_method, "udp");
        assert_eq!(record.addresses.len(), 1);
        assert_eq!(record.addresses[0], "192.168.1.100:8080".parse().unwrap());
        assert!(record.capabilities.contains_key("version"));
        assert_eq!(record.capabilities.get("version"), Some(&"1.0.0".to_string()));
        assert!(record.capabilities.contains_key("features"));
        assert_eq!(record.capabilities.get("features"), Some(&"chat".to_string()));
    }

    #[test]
    fn test_parse_peer_message_minimal() {
        let discovery = UdpDiscovery::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        
        // Test minimal KIZUNA_PEER message
        let message = "KIZUNA_PEER|peer-456|Simple Device|9000";
        let record = discovery.parse_peer_message(message, addr).unwrap();
        
        assert_eq!(record.peer_id, "peer-456");
        assert_eq!(record.name, "Simple Device");
        assert_eq!(record.port, 9000);
        assert_eq!(record.discovery_method, "udp");
        assert_eq!(record.addresses.len(), 1);
        assert_eq!(record.addresses[0], SocketAddr::new(addr.ip(), 9000));
        assert!(record.capabilities.is_empty());
    }

    #[test]
    fn test_parse_discovery_request() {
        let discovery = UdpDiscovery::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        
        // Test DISCOVER_KIZUNA message
        let message = "DISCOVER_KIZUNA|peer-789|Requesting Device|7000";
        let record = discovery.parse_discovery_request(message, addr).unwrap();
        
        assert_eq!(record.peer_id, "peer-789");
        assert_eq!(record.name, "Requesting Device");
        assert_eq!(record.port, 7000);
        assert_eq!(record.discovery_method, "udp");
        assert_eq!(record.addresses.len(), 1);
        assert_eq!(record.addresses[0], SocketAddr::new(addr.ip(), 7000));
    }

    #[test]
    fn test_parse_discovery_request_minimal() {
        let discovery = UdpDiscovery::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        
        // Test minimal DISCOVER_KIZUNA message (no port)
        let message = "DISCOVER_KIZUNA|peer-999|Minimal Device";
        let record = discovery.parse_discovery_request(message, addr).unwrap();
        
        assert_eq!(record.peer_id, "peer-999");
        assert_eq!(record.name, "Minimal Device");
        assert_eq!(record.port, discovery.port); // Should use default port
        assert_eq!(record.addresses.len(), 1);
        assert_eq!(record.addresses[0], SocketAddr::new(addr.ip(), discovery.port));
    }

    #[test]
    fn test_parse_invalid_messages() {
        let discovery = UdpDiscovery::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        
        // Test invalid messages
        assert!(discovery.parse_peer_message("INVALID_MESSAGE", addr).is_none());
        assert!(discovery.parse_peer_message("KIZUNA_PEER|incomplete", addr).is_none());
        assert!(discovery.parse_discovery_request("DISCOVER_KIZUNA|incomplete", addr).is_none());
        
        // Test parse_peer_response with invalid messages
        assert!(discovery.parse_peer_response("INVALID_MESSAGE", addr).is_none());
        assert!(discovery.parse_peer_response("WRONG_PREFIX|peer|name|port", addr).is_none());
    }

    #[test]
    fn test_parse_peer_message_with_multiple_addresses() {
        let discovery = UdpDiscovery::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        
        // Test KIZUNA_PEER message with multiple addresses
        let message = "KIZUNA_PEER|peer-multi|Multi Device|8080|192.168.1.100:8080,10.0.0.50:8080|type=server";
        let record = discovery.parse_peer_message(message, addr).unwrap();
        
        assert_eq!(record.peer_id, "peer-multi");
        assert_eq!(record.addresses.len(), 2);
        assert!(record.addresses.contains(&"192.168.1.100:8080".parse().unwrap()));
        assert!(record.addresses.contains(&"10.0.0.50:8080".parse().unwrap()));
        assert_eq!(record.capabilities.get("type"), Some(&"server".to_string()));
    }

    #[test]
    fn test_parse_peer_message_invalid_port() {
        let discovery = UdpDiscovery::new();
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        
        // Test KIZUNA_PEER message with invalid port
        let message = "KIZUNA_PEER|peer-bad|Bad Device|invalid_port";
        let record = discovery.parse_peer_message(message, addr);
        
        assert!(record.is_none());
    }

    #[tokio::test]
    async fn test_announce_and_stop_announce() {
        let discovery = UdpDiscovery::new();
        
        // These should not fail for UDP (they're no-ops)
        assert!(discovery.announce().await.is_ok());
        assert!(discovery.stop_announce().await.is_ok());
    }

    #[tokio::test]
    async fn test_send_peer_response() {
        // Test the static method for sending peer responses
        let target_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 12345);
        
        // This test mainly checks that the method doesn't panic
        // In a real network test, we'd set up a listener to verify the message
        let result = UdpDiscovery::send_peer_response(
            "test-peer".to_string(),
            "Test Device".to_string(),
            8080,
            target_addr,
        ).await;
        
        // The result might be an error if the target address is not reachable,
        // but the method should not panic
        match result {
            Ok(_) => {
                // Success - message was sent
            }
            Err(DiscoveryError::Network(_)) => {
                // Expected if target is not reachable
            }
            Err(e) => {
                panic!("Unexpected error type: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_discovery_with_timeout() {
        let discovery = UdpDiscovery::new();
        let timeout = Duration::from_millis(100);
        
        // This test verifies that discovery completes within the timeout
        // and doesn't hang indefinitely
        let start = tokio::time::Instant::now();
        let result = discovery.discover(timeout).await;
        let elapsed = start.elapsed();
        
        // Should complete within reasonable time (timeout + some buffer)
        assert!(elapsed <= timeout + Duration::from_millis(50));
        
        // Result should be Ok (empty list is fine for this test)
        assert!(result.is_ok());
    }
}