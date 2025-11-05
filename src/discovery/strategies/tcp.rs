use crate::discovery::{Discovery, DiscoveryError, ServiceRecord};
use async_trait::async_trait;
use std::time::Duration;

pub struct TcpDiscovery {
    peer_id: String,
    device_name: String,
    port: u16,
    scan_ports: Vec<u16>,
}

impl TcpDiscovery {
    pub fn new() -> Self {
        Self {
            peer_id: format!("kizuna-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            device_name: "Kizuna Device".to_string(),
            port: 41337,
            scan_ports: vec![41337, 8080, 3000, 3001, 3002, 3003, 3004, 3005],
        }
    }

    pub fn with_config(peer_id: String, device_name: String, port: u16, scan_ports: Vec<u16>) -> Self {
        Self {
            peer_id,
            device_name,
            port,
            scan_ports,
        }
    }
}

#[async_trait]
impl Discovery for TcpDiscovery {
    async fn discover(&self, _timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        // TODO: Implement TCP handshake beacon discovery
        // This would scan common ports and perform handshakes
        Err(DiscoveryError::StrategyUnavailable {
            strategy: "tcp".to_string(),
        })
    }

    async fn announce(&self) -> Result<(), DiscoveryError> {
        // TODO: Implement TCP beacon server
        Ok(())
    }

    async fn stop_announce(&self) -> Result<(), DiscoveryError> {
        // TODO: Stop TCP beacon server
        Ok(())
    }

    fn strategy_name(&self) -> &'static str {
        "tcp"
    }

    fn is_available(&self) -> bool {
        // TCP should be available on all platforms
        false // Set to false until implemented
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