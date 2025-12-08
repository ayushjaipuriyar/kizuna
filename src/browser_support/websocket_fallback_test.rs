//! Tests for WebSocket fallback functionality

#[cfg(test)]
mod tests {
    use crate::browser_support::websocket_fallback::WebSocketFallbackManager;
    use crate::browser_support::types::*;
    use crate::browser_support::BrowserConnectionInfo;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_websocket_manager_creation() {
        let manager = WebSocketFallbackManager::new();
        assert!(manager.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_websocket_connection_establishment() {
        let mut manager = WebSocketFallbackManager::new();
        manager.initialize().await.unwrap();

        let connection_info = BrowserConnectionInfo {
            peer_id: "test-peer-123".to_string(),
            signaling_info: SignalingInfo {
                signaling_server: None,
                ice_servers: vec![],
                connection_type: ConnectionType::Direct,
            },
            browser_info: BrowserInfo {
                user_agent: "Mozilla/5.0 (Test Browser)".to_string(),
                browser_type: BrowserType::Chrome,
                version: "100.0".to_string(),
                platform: "Linux".to_string(),
                supports_webrtc: false,
                supports_clipboard_api: true,
            },
        };

        let session = manager.establish_connection(connection_info).await.unwrap();
        assert_eq!(session.browser_info.browser_type, BrowserType::Chrome);
        assert!(!session.browser_info.supports_webrtc);
    }

    #[tokio::test]
    async fn test_websocket_message_sending() {
        let mut manager = WebSocketFallbackManager::new();
        manager.initialize().await.unwrap();

        let connection_info = BrowserConnectionInfo {
            peer_id: "test-peer-456".to_string(),
            signaling_info: SignalingInfo {
                signaling_server: None,
                ice_servers: vec![],
                connection_type: ConnectionType::Direct,
            },
            browser_info: BrowserInfo {
                user_agent: "Mozilla/5.0 (Test Browser)".to_string(),
                browser_type: BrowserType::Firefox,
                version: "90.0".to_string(),
                platform: "Windows".to_string(),
                supports_webrtc: false,
                supports_clipboard_api: false,
            },
        };

        let session = manager.establish_connection(connection_info).await.unwrap();
        
        let message = crate::browser_support::BrowserMessage {
            message_id: Uuid::new_v4(),
            message_type: BrowserMessageType::StatusUpdate,
            payload: serde_json::json!({"status": "connected"}),
            timestamp: std::time::SystemTime::now(),
            session_id: session.session_id,
        };

        // Note: This will fail without an actual WebSocket connection
        // In a real test, we'd need to set up a WebSocket server
        let result = manager.send_message(session.session_id, message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_websocket_connection_stats() {
        let mut manager = WebSocketFallbackManager::new();
        manager.initialize().await.unwrap();

        let connection_info = BrowserConnectionInfo {
            peer_id: "test-peer-789".to_string(),
            signaling_info: SignalingInfo {
                signaling_server: None,
                ice_servers: vec![],
                connection_type: ConnectionType::Direct,
            },
            browser_info: BrowserInfo {
                user_agent: "Mozilla/5.0 (Test Browser)".to_string(),
                browser_type: BrowserType::Safari,
                version: "15.0".to_string(),
                platform: "macOS".to_string(),
                supports_webrtc: false,
                supports_clipboard_api: true,
            },
        };

        let session = manager.establish_connection(connection_info).await.unwrap();
        
        let stats = manager.get_connection_stats(session.session_id).await.unwrap();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.packets_sent, 0);
        assert_eq!(stats.packets_received, 0);
    }

    #[tokio::test]
    async fn test_websocket_connection_close() {
        let mut manager = WebSocketFallbackManager::new();
        manager.initialize().await.unwrap();

        let connection_info = BrowserConnectionInfo {
            peer_id: "test-peer-close".to_string(),
            signaling_info: SignalingInfo {
                signaling_server: None,
                ice_servers: vec![],
                connection_type: ConnectionType::Direct,
            },
            browser_info: BrowserInfo {
                user_agent: "Mozilla/5.0 (Test Browser)".to_string(),
                browser_type: BrowserType::Edge,
                version: "95.0".to_string(),
                platform: "Windows".to_string(),
                supports_webrtc: false,
                supports_clipboard_api: true,
            },
        };

        let session = manager.establish_connection(connection_info).await.unwrap();
        
        // Close the connection
        assert!(manager.close_connection(session.session_id).await.is_ok());
        
        // Verify connection is closed
        let is_connected = manager.is_connected(session.session_id).await.unwrap();
        assert!(!is_connected);
    }

    #[tokio::test]
    async fn test_websocket_manager_shutdown() {
        let mut manager = WebSocketFallbackManager::new();
        manager.initialize().await.unwrap();

        // Create multiple connections
        for i in 0..3 {
            let connection_info = BrowserConnectionInfo {
                peer_id: format!("test-peer-{}", i),
                signaling_info: SignalingInfo {
                    signaling_server: None,
                    ice_servers: vec![],
                    connection_type: ConnectionType::Direct,
                },
                browser_info: BrowserInfo {
                    user_agent: "Mozilla/5.0 (Test Browser)".to_string(),
                    browser_type: BrowserType::Chrome,
                    version: "100.0".to_string(),
                    platform: "Linux".to_string(),
                    supports_webrtc: false,
                    supports_clipboard_api: true,
                },
            };
            manager.establish_connection(connection_info).await.unwrap();
        }

        // Shutdown should close all connections
        assert!(manager.shutdown().await.is_ok());
    }
}
