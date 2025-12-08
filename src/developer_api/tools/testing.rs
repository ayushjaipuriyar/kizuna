/// Testing utilities and mock framework
use crate::developer_api::core::events::PeerInfo;

/// Mock framework for testing
pub struct MockFramework {
    mock_peers: Vec<MockPeer>,
}

impl MockFramework {
    /// Creates a new mock framework
    pub fn new() -> Self {
        Self {
            mock_peers: Vec::new(),
        }
    }
    
    /// Creates a mock peer
    pub fn create_mock_peer(&mut self, config: MockPeerConfig) -> MockPeer {
        let peer = MockPeer::new(config);
        self.mock_peers.push(peer.clone());
        peer
    }
}

impl Default for MockFramework {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for mock peers
#[derive(Debug, Clone)]
pub struct MockPeerConfig {
    /// Peer name
    pub name: String,
    
    /// Peer capabilities
    pub capabilities: Vec<String>,
}

/// Mock peer for testing
#[derive(Debug, Clone)]
pub struct MockPeer {
    config: MockPeerConfig,
}

impl MockPeer {
    /// Creates a new mock peer
    pub fn new(config: MockPeerConfig) -> Self {
        Self { config }
    }
    
    /// Converts to PeerInfo
    pub fn to_peer_info(&self) -> PeerInfo {
        PeerInfo {
            peer_id: format!("mock-{}", self.config.name).into(),
            name: self.config.name.clone(),
            addresses: vec!["127.0.0.1:0".parse().unwrap()],
        }
    }
}
