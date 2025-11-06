//! Browser Discovery and Connection Setup
//! 
//! This module handles QR code generation, URL-based connection setup,
//! automatic peer discovery, and connection status reporting for browser clients.

use crate::browser_support::{BrowserResult, BrowserSupportError};
use crate::browser_support::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Connection setup information for browsers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionSetup {
    pub setup_id: Uuid,
    pub connection_url: String,
    pub qr_code_data: String,
    pub peer_info: PeerInfo,
    pub expires_at: std::time::SystemTime,
    pub ice_servers: Vec<IceServer>,
}

/// Peer information for discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub name: String,
    pub device_type: String,
    pub capabilities: Vec<String>,
    pub network_addresses: Vec<String>,
    pub last_seen: std::time::SystemTime,
}

/// Connection status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatus {
    pub session_id: Uuid,
    pub peer_id: String,
    pub connection_state: ConnectionState,
    pub ice_connection_state: IceConnectionState,
    pub data_channels: HashMap<ChannelType, DataChannelStatus>,
    pub connection_quality: ConnectionQuality,
    pub established_at: Option<std::time::SystemTime>,
    pub last_activity: std::time::SystemTime,
}

/// Data channel status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChannelStatus {
    pub state: DataChannelState,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
}

/// Connection quality metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionQuality {
    pub signal_strength: Option<f64>,
    pub round_trip_time: Option<f64>,
    pub bandwidth_estimate: Option<u64>,
    pub packet_loss_rate: Option<f64>,
    pub jitter: Option<f64>,
}

/// Browser discovery manager
pub struct BrowserDiscovery {
    active_setups: Arc<RwLock<HashMap<Uuid, ConnectionSetup>>>,
    discovered_peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    connection_statuses: Arc<RwLock<HashMap<Uuid, ConnectionStatus>>>,
    local_peer_info: RwLock<PeerInfo>,
    server_address: RwLock<Option<SocketAddr>>,
}

impl BrowserDiscovery {
    /// Create a new browser discovery manager
    pub fn new(peer_id: String, device_name: String) -> Self {
        let local_peer_info = PeerInfo {
            peer_id: peer_id.clone(),
            name: device_name,
            device_type: "kizuna-native".to_string(),
            capabilities: vec![
                "file_transfer".to_string(),
                "clipboard_sync".to_string(),
                "command_execution".to_string(),
                "camera_streaming".to_string(),
            ],
            network_addresses: Vec::new(),
            last_seen: std::time::SystemTime::now(),
        };

        Self {
            active_setups: Arc::new(RwLock::new(HashMap::new())),
            discovered_peers: Arc::new(RwLock::new(HashMap::new())),
            connection_statuses: Arc::new(RwLock::new(HashMap::new())),
            local_peer_info: RwLock::new(local_peer_info),
            server_address: RwLock::new(None),
        }
    }

    /// Initialize the discovery manager
    pub async fn initialize(&self, server_address: SocketAddr) -> BrowserResult<()> {
        *self.server_address.write().await = Some(server_address);
        
        // Update local peer info with network addresses
        let mut addresses = Vec::new();
        addresses.push(server_address.to_string());
        
        // Add local network interfaces
        if let Ok(interfaces) = local_ip_address::list_afinet_netifas() {
            for (_, ip) in interfaces {
                if !ip.is_loopback() {
                    addresses.push(format!("{}:{}", ip, server_address.port()));
                }
            }
        }
        
        let mut local_peer = self.local_peer_info.write().await;
        local_peer.network_addresses = addresses;
        local_peer.last_seen = std::time::SystemTime::now();
        
        Ok(())
    }

    /// Create a new connection setup for browser clients
    pub async fn create_connection_setup(&self) -> BrowserResult<ConnectionSetup> {
        let setup_id = Uuid::new_v4();
        let server_addr = self.server_address.read().await.ok_or_else(|| {
            BrowserSupportError::ConfigurationError {
                parameter: "server_address".to_string(),
                issue: "Not initialized".to_string(),
            }
        })?;

        // Create connection URL
        let connection_url = format!(
            "https://{}:{}/connect?setup_id={}",
            server_addr.ip(),
            server_addr.port(),
            setup_id
        );

        // Generate QR code data (URL for QR code)
        let qr_code_data = connection_url.clone();

        // Create ICE servers configuration
        let ice_servers = vec![
            IceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_string()],
                username: None,
                credential: None,
            },
            IceServer {
                urls: vec!["stun:stun1.l.google.com:19302".to_string()],
                username: None,
                credential: None,
            },
        ];

        let local_peer = self.local_peer_info.read().await.clone();
        let setup = ConnectionSetup {
            setup_id,
            connection_url,
            qr_code_data,
            peer_info: local_peer,
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(300), // 5 minutes
            ice_servers,
        };

        // Store the setup
        self.active_setups.write().await.insert(setup_id, setup.clone());

        Ok(setup)
    }

    /// Get connection setup by ID
    pub async fn get_connection_setup(&self, setup_id: Uuid) -> BrowserResult<ConnectionSetup> {
        let setups = self.active_setups.read().await;
        let setup = setups.get(&setup_id).ok_or_else(|| {
            BrowserSupportError::SessionError {
                session_id: setup_id.to_string(),
                error: "Connection setup not found or expired".to_string(),
            }
        })?;

        // Check if setup has expired
        if setup.expires_at < std::time::SystemTime::now() {
            return Err(BrowserSupportError::SessionError {
                session_id: setup_id.to_string(),
                error: "Connection setup has expired".to_string(),
            });
        }

        Ok(setup.clone())
    }

    /// Remove expired connection setups
    pub async fn cleanup_expired_setups(&self) -> BrowserResult<()> {
        let mut setups = self.active_setups.write().await;
        let now = std::time::SystemTime::now();
        
        setups.retain(|_, setup| setup.expires_at > now);
        
        Ok(())
    }

    /// Add discovered peer
    pub async fn add_discovered_peer(&self, peer_info: PeerInfo) -> BrowserResult<()> {
        self.discovered_peers.write().await.insert(peer_info.peer_id.clone(), peer_info);
        Ok(())
    }

    /// Get all discovered peers
    pub async fn get_discovered_peers(&self) -> BrowserResult<Vec<PeerInfo>> {
        let peers = self.discovered_peers.read().await;
        Ok(peers.values().cloned().collect())
    }

    /// Update connection status
    pub async fn update_connection_status(&self, status: ConnectionStatus) -> BrowserResult<()> {
        self.connection_statuses.write().await.insert(status.session_id, status);
        Ok(())
    }

    /// Get connection status
    pub async fn get_connection_status(&self, session_id: Uuid) -> BrowserResult<ConnectionStatus> {
        let statuses = self.connection_statuses.read().await;
        statuses.get(&session_id).cloned().ok_or_else(|| {
            BrowserSupportError::SessionError {
                session_id: session_id.to_string(),
                error: "Connection status not found".to_string(),
            }
        })
    }

    /// Get all connection statuses
    pub async fn get_all_connection_statuses(&self) -> BrowserResult<Vec<ConnectionStatus>> {
        let statuses = self.connection_statuses.read().await;
        Ok(statuses.values().cloned().collect())
    }

    /// Remove connection status
    pub async fn remove_connection_status(&self, session_id: Uuid) -> BrowserResult<()> {
        self.connection_statuses.write().await.remove(&session_id);
        Ok(())
    }

    /// Get local peer information
    pub async fn get_local_peer_info(&self) -> PeerInfo {
        self.local_peer_info.read().await.clone()
    }

    /// Update local peer capabilities
    pub async fn update_local_capabilities(&self, capabilities: Vec<String>) {
        let mut local_peer = self.local_peer_info.write().await;
        local_peer.capabilities = capabilities;
        local_peer.last_seen = std::time::SystemTime::now();
    }

    /// Generate QR code SVG data
    pub fn generate_qr_code_svg(&self, data: &str) -> BrowserResult<String> {
        // For now, return a placeholder SVG
        // In a real implementation, you would use a QR code library like 'qrcode'
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="200" viewBox="0 0 200 200">
                <rect width="200" height="200" fill="white"/>
                <text x="100" y="100" text-anchor="middle" font-family="Arial" font-size="12">
                    QR Code for: {}
                </text>
            </svg>"#,
            data
        );
        Ok(svg)
    }

    /// Create connection info for browser client
    pub async fn create_browser_connection_info(
        &self,
        setup_id: Uuid,
        browser_info: BrowserInfo,
    ) -> BrowserResult<BrowserConnectionInfo> {
        let setup = self.get_connection_setup(setup_id).await?;
        
        let server_addr = self.server_address.read().await.ok_or_else(|| {
            BrowserSupportError::ConfigurationError {
                parameter: "server_address".to_string(),
                issue: "Not initialized".to_string(),
            }
        })?;
        
        let signaling_info = SignalingInfo {
            signaling_server: Some(format!("ws://{}:{}/ws", 
                server_addr.ip(),
                server_addr.port()
            )),
            ice_servers: setup.ice_servers,
            connection_type: ConnectionType::Hybrid,
        };

        Ok(BrowserConnectionInfo {
            peer_id: setup.peer_info.peer_id,
            signaling_info,
            browser_info,
        })
    }
}

// Helper function to get local IP addresses
mod local_ip_address {
    use std::net::{IpAddr, Ipv4Addr};
    
    pub fn list_afinet_netifas() -> Result<Vec<(String, IpAddr)>, std::io::Error> {
        // Simplified implementation - in a real scenario you'd use a proper network interface library
        Ok(vec![
            ("eth0".to_string(), IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))),
            ("wlan0".to_string(), IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101))),
        ])
    }
}