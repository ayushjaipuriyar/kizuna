use crate::discovery::{Discovery, DiscoveryError, ServiceRecord};
use async_trait::async_trait;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;

pub struct UdpDiscovery {
    port: u16,
    peer_id: String,
    device_name: String,
}

impl UdpDiscovery {
    pub fn new() -> Self {
        Self {
            port: 41337, // Updated to match design spec
            peer_id: format!("kizuna-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            device_name: "Kizuna Device".to_string(),
        }
    }

    pub fn with_config(port: u16, peer_id: String, device_name: String) -> Self {
        Self {
            port,
            peer_id,
            device_name,
        }
    }

    async fn send_discovery_broadcast(&self) -> Result<(), DiscoveryError> {
        // Create a socket for sending broadcasts
        let socket = UdpSocket::bind("0.0.0.0:0").await
            .map_err(DiscoveryError::Network)?;
        
        socket.set_broadcast(true)
            .map_err(DiscoveryError::Network)?;

        let broadcast_addr: SocketAddr = format!("255.255.255.255:{}", self.port)
            .parse()
            .map_err(|e| DiscoveryError::Parse(format!("Invalid broadcast address: {}", e)))?;

        let message = format!("DISCOVER_KIZUNA|{}|{}|{}", 
            self.peer_id, self.device_name, self.port);

        socket.send_to(message.as_bytes(), broadcast_addr).await
            .map_err(DiscoveryError::Network)?;

        Ok(())
    }

    async fn listen_for_responses(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        let bind_addr = format!("0.0.0.0:{}", self.port);
        let socket = UdpSocket::bind(&bind_addr).await
            .map_err(DiscoveryError::Network)?;

        let mut peers = Vec::new();
        let mut buf = [0u8; 1024];

        let deadline = tokio::time::Instant::now() + timeout;

        while tokio::time::Instant::now() < deadline {
            let remaining = deadline - tokio::time::Instant::now();
            
            match tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await {
                Ok(Ok((n, addr))) => {
                    let message = String::from_utf8_lossy(&buf[..n]);
                    
                    if let Some(peer) = self.parse_peer_response(&message, addr) {
                        peers.push(peer);
                    }
                }
                Ok(Err(e)) => {
                    return Err(DiscoveryError::Network(e));
                }
                Err(_) => {
                    // Timeout - continue or break based on remaining time
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
            let parts: Vec<&str> = message.split('|').collect();
            if parts.len() >= 4 {
                let peer_id = parts[1].to_string();
                let name = parts[2].to_string();
                let port: u16 = parts[3].parse().unwrap_or(self.port);
                
                let mut record = ServiceRecord::new(peer_id, name, port);
                record.add_address(SocketAddr::new(addr.ip(), port));
                record.set_discovery_method("udp".to_string());
                
                // Parse capabilities if present
                if parts.len() > 5 {
                    let capabilities_str = parts[5];
                    for cap in capabilities_str.split(',') {
                        if let Some((key, value)) = cap.split_once('=') {
                            record.add_capability(key.to_string(), value.to_string());
                        }
                    }
                }
                
                return Some(record);
            }
        } else if message.starts_with("DISCOVER_KIZUNA|") {
            // This is a discovery request, respond to it
            let parts: Vec<&str> = message.split('|').collect();
            if parts.len() >= 3 {
                let peer_id = parts[1].to_string();
                let name = parts[2].to_string();
                let port: u16 = parts.get(3).and_then(|p| p.parse().ok()).unwrap_or(self.port);
                
                let mut record = ServiceRecord::new(peer_id, name, port);
                record.add_address(SocketAddr::new(addr.ip(), port));
                record.set_discovery_method("udp".to_string());
                
                return Some(record);
            }
        }
        
        None
    }

    async fn respond_to_discovery(&self, requester_addr: SocketAddr) -> Result<(), DiscoveryError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await
            .map_err(DiscoveryError::Network)?;

        let response = format!("KIZUNA_PEER|{}|{}|{}||version=0.1.0", 
            self.peer_id, self.device_name, self.port);

        socket.send_to(response.as_bytes(), requester_addr).await
            .map_err(DiscoveryError::Network)?;

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