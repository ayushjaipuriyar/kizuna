//! Tests for browser discovery and connection setup

#[cfg(test)]
mod tests {
    use super::*;
    use crate::browser_support::discovery::BrowserDiscovery;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_browser_discovery_creation() {
        let discovery = BrowserDiscovery::new(
            "test-peer-123".to_string(),
            "Test Device".to_string(),
        );
        
        let local_peer = discovery.get_local_peer_info().await;
        assert_eq!(local_peer.peer_id, "test-peer-123");
        assert_eq!(local_peer.name, "Test Device");
        assert_eq!(local_peer.device_type, "kizuna-native");
        assert!(local_peer.capabilities.contains(&"file_transfer".to_string()));
    }

    #[tokio::test]
    async fn test_connection_setup_creation() {
        let discovery = BrowserDiscovery::new(
            "test-peer-456".to_string(),
            "Test Device 2".to_string(),
        );
        
        // Initialize with a test address
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        discovery.initialize(addr).await.unwrap();
        
        // Create connection setup
        let setup = discovery.create_connection_setup().await.unwrap();
        
        assert!(setup.connection_url.contains("127.0.0.1:8080"));
        assert!(setup.connection_url.contains(&setup.setup_id.to_string()));
        assert_eq!(setup.peer_info.peer_id, "test-peer-456");
        assert!(!setup.ice_servers.is_empty());
    }

    #[tokio::test]
    async fn test_connection_setup_retrieval() {
        let discovery = BrowserDiscovery::new(
            "test-peer-789".to_string(),
            "Test Device 3".to_string(),
        );
        
        let addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();
        discovery.initialize(addr).await.unwrap();
        
        // Create and retrieve setup
        let setup = discovery.create_connection_setup().await.unwrap();
        let retrieved_setup = discovery.get_connection_setup(setup.setup_id).await.unwrap();
        
        assert_eq!(setup.setup_id, retrieved_setup.setup_id);
        assert_eq!(setup.connection_url, retrieved_setup.connection_url);
        assert_eq!(setup.peer_info.peer_id, retrieved_setup.peer_info.peer_id);
    }

    #[tokio::test]
    async fn test_qr_code_generation() {
        let discovery = BrowserDiscovery::new(
            "test-peer-qr".to_string(),
            "QR Test Device".to_string(),
        );
        
        let qr_svg = discovery.generate_qr_code_svg("https://example.com/connect?id=123").unwrap();
        
        assert!(qr_svg.contains("<svg"));
        assert!(qr_svg.contains("https://example.com/connect?id=123"));
    }

    #[tokio::test]
    async fn test_peer_discovery() {
        let discovery = BrowserDiscovery::new(
            "test-peer-discovery".to_string(),
            "Discovery Test Device".to_string(),
        );
        
        // Add a discovered peer
        let peer_info = crate::browser_support::discovery::PeerInfo {
            peer_id: "remote-peer-123".to_string(),
            name: "Remote Device".to_string(),
            device_type: "mobile".to_string(),
            capabilities: vec!["file_transfer".to_string(), "clipboard_sync".to_string()],
            network_addresses: vec!["192.168.1.100:8080".to_string()],
            last_seen: std::time::SystemTime::now(),
        };
        
        discovery.add_discovered_peer(peer_info.clone()).await.unwrap();
        
        let discovered_peers = discovery.get_discovered_peers().await.unwrap();
        assert_eq!(discovered_peers.len(), 1);
        assert_eq!(discovered_peers[0].peer_id, "remote-peer-123");
        assert_eq!(discovered_peers[0].name, "Remote Device");
    }
}