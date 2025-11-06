use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{PeerId, TransportError};

/// Trait for transport connections providing unified interface
#[async_trait]
pub trait Connection: Send + Sync + std::fmt::Debug {
    /// Read data from the connection
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, TransportError>;
    
    /// Write data to the connection
    async fn write(&mut self, buf: &[u8]) -> Result<usize, TransportError>;
    
    /// Flush any buffered data
    async fn flush(&mut self) -> Result<(), TransportError>;
    
    /// Close the connection gracefully
    async fn close(&mut self) -> Result<(), TransportError>;
    
    /// Get connection metadata and statistics
    fn info(&self) -> ConnectionInfo;
    
    /// Check if connection is still active
    fn is_connected(&self) -> bool;
}

/// Metadata and statistics about an active connection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConnectionInfo {
    /// ID of the connected peer
    pub peer_id: PeerId,
    /// Local socket address
    pub local_addr: SocketAddr,
    /// Remote socket address
    pub remote_addr: SocketAddr,
    /// Transport protocol name
    pub protocol: String,
    /// When the connection was established
    pub established_at: SystemTime,
    /// Total bytes sent over this connection
    pub bytes_sent: u64,
    /// Total bytes received over this connection
    pub bytes_received: u64,
    /// Round-trip time if available
    pub rtt: Option<Duration>,
    /// Current bandwidth estimate in bytes per second
    pub bandwidth: Option<u64>,
}

impl ConnectionInfo {
    /// Create new connection info
    pub fn new(
        peer_id: PeerId,
        local_addr: SocketAddr,
        remote_addr: SocketAddr,
        protocol: String,
    ) -> Self {
        Self {
            peer_id,
            local_addr,
            remote_addr,
            protocol,
            established_at: SystemTime::now(),
            bytes_sent: 0,
            bytes_received: 0,
            rtt: None,
            bandwidth: None,
        }
    }

    /// Update bytes sent counter
    pub fn add_bytes_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
    }

    /// Update bytes received counter
    pub fn add_bytes_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
    }

    /// Update round-trip time measurement
    pub fn update_rtt(&mut self, rtt: Duration) {
        self.rtt = Some(rtt);
    }

    /// Update bandwidth estimate
    pub fn update_bandwidth(&mut self, bandwidth: u64) {
        self.bandwidth = Some(bandwidth);
    }

    /// Get connection duration
    pub fn duration(&self) -> Duration {
        SystemTime::now()
            .duration_since(self.established_at)
            .unwrap_or_default()
    }

    /// Calculate total bytes transferred
    pub fn total_bytes(&self) -> u64 {
        self.bytes_sent + self.bytes_received
    }

    /// Get connection quality score (0-100)
    pub fn quality_score(&self) -> u8 {
        let mut score = 100u8;
        
        // Penalize high latency
        if let Some(rtt) = self.rtt {
            if rtt > Duration::from_millis(500) {
                score = score.saturating_sub(30);
            } else if rtt > Duration::from_millis(100) {
                score = score.saturating_sub(10);
            }
        }
        
        // Reward high bandwidth
        if let Some(bandwidth) = self.bandwidth {
            if bandwidth < 1024 * 1024 { // Less than 1MB/s
                score = score.saturating_sub(20);
            }
        }
        
        score
    }
}