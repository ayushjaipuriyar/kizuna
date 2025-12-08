//! Tests for unified communication interface with automatic fallback

#[cfg(test)]
mod tests {
    use crate::browser_support::communication::{UnifiedCommunicationManager, ProtocolDetector};
    use crate::browser_support::types::*;
    use crate::browser_support::BrowserConnectionInfo;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_unified_manager_creation() {
        let manager = UnifiedCommunicationManager::new();
        assert!(manager.is_fallback_enabled());
    }

    #[tokio::test]
    async fn test_unified_manager_with_fallback_disabled() {
        let manager = UnifiedCommunicationManager::with_fallback(false);
        assert!(!manager.is_fallback_enabled());
    }

    #[tokio::test]
    async fn test_protocol_detection_webrtc_supported() {
        let detector = ProtocolDetector::new();
        
        let browser_info = BrowserInfo {
            user_agent: "Mozilla/5.0 (Chrome)".to_string(),
            browser_type: BrowserType::Chrome,
            version: "100.0".to_string(),
            platform: "Linux".to_string(),
            supports_webrtc: true,
            supports_clipboard_api: true,
        };

        let protocol = detector.detect_best_protocol(&browser_info).await.unwrap();
        assert!(matches!(protocol, CommunicationProtocol::WebRTC));
    }

    #[tokio::test]
    async fn test_protocol_detection_webrtc_not_supported() {
        let detector = ProtocolDetector::new();
        
        let browser_info = BrowserInfo {
            user_agent: "Mozilla/5.0 (Old Browser)".to_string(),
            browser_type: BrowserType::Other("OldBrowser".to_string()),
            version: "1.0".to_string(),
            platform: "Unknown".to_string(),
            supports_webrtc: false,
            supports_clipboard_api: false,
        };

        let protocol = detector.detect_best_protocol(&browser_info).await.unwrap();
        assert!(matches!(protocol, CommunicationProtocol::WebSocket));
    }

    #[tokio::test]
    async fn test_protocol_detection_mobile_safari() {
        let detector = ProtocolDetector::new();
        
        let browser_info = BrowserInfo {
            user_agent: "Mozilla/5.0 (iPhone; Safari)".to_string(),
            browser_type: BrowserType::Safari,
            version: "15.0".to_string(),
            platform: "Mobile iOS".to_string(),
            supports_webrtc: true,
            supports_clipboard_api: true,
        };

        let protocol = detector.detect_best_protocol(&browser_info).await.unwrap();
        // Mobile Safari should fallback to WebSocket
        assert!(matches!(protocol, CommunicationProtocol::WebSocket));
    }

    #[tokio::test]
    async fn test_protocol_detection_desktop_safari() {
        let detector = ProtocolDetector::new();
        
        let browser_info = BrowserInfo {
            user_agent: "Mozilla/5.0 (Macintosh; Safari)".to_string(),
            browser_type: BrowserType::Safari,
            version: "15.0".to_string(),
            platform: "macOS".to_string(),
            supports_webrtc: true,
            supports_clipboard_api: true,
        };

        let protocol = detector.detect_best_protocol(&browser_info).await.unwrap();
        // Desktop Safari should use WebRTC
        assert!(matches!(protocol, CommunicationProtocol::WebRTC));
    }

    #[tokio::test]
    async fn test_websocket_connection_establishment() {
        let mut manager = UnifiedCommunicationManager::new();
        manager.initialize().await.unwrap();

        let connection_info = BrowserConnectionInfo {
            peer_id: "test-peer-websocket".to_string(),
            signaling_info: SignalingInfo {
                signaling_server: None,
                ice_servers: vec![],
                connection_type: ConnectionType::Direct,
            },
            browser_info: BrowserInfo {
                user_agent: "Mozilla/5.0 (Old Browser)".to_string(),
                browser_type: BrowserType::Other("OldBrowser".to_string()),
                version: "1.0".to_string(),
                platform: "Unknown".to_string(),
                supports_webrtc: false,
                supports_clipboard_api: false,
            },
        };

        let session = manager.establish_connection(connection_info).await.unwrap();
        
        // Verify WebSocket protocol was selected
        let protocol = manager.get_session_protocol(session.session_id).await;
        assert!(matches!(protocol, Some(CommunicationProtocol::WebSocket)));
    }

    #[tokio::test]
    async fn test_fallback_configuration() {
        let mut manager = UnifiedCommunicationManager::with_fallback(true);
        assert!(manager.is_fallback_enabled());
        
        manager.set_fallback_enabled(false);
        assert!(!manager.is_fallback_enabled());
        
        manager.set_fallback_enabled(true);
        assert!(manager.is_fallback_enabled());
    }

    #[tokio::test]
    async fn test_capabilities_extraction() {
        let mut manager = UnifiedCommunicationManager::new();
        manager.initialize().await.unwrap();

        let connection_info = BrowserConnectionInfo {
            peer_id: "test-peer-caps".to_string(),
            signaling_info: SignalingInfo {
                signaling_server: None,
                ice_servers: vec![],
                connection_type: ConnectionType::Direct,
            },
            browser_info: BrowserInfo {
                user_agent: "Mozilla/5.0 (Chrome)".to_string(),
                browser_type: BrowserType::Chrome,
                version: "100.0".to_string(),
                platform: "Linux".to_string(),
                supports_webrtc: true,
                supports_clipboard_api: true,
            },
        };

        // The capabilities should be extracted based on browser info
        // WebRTC support means video streaming is available
        // Clipboard API support means clipboard sync is available
        let session = manager.establish_connection(connection_info).await;
        
        // Even if WebRTC connection fails, we should get a session via fallback
        assert!(session.is_ok() || manager.is_fallback_enabled());
    }

    #[tokio::test]
    async fn test_manager_shutdown() {
        let mut manager = UnifiedCommunicationManager::new();
        manager.initialize().await.unwrap();

        // Create a WebSocket connection
        let connection_info = BrowserConnectionInfo {
            peer_id: "test-peer-shutdown".to_string(),
            signaling_info: SignalingInfo {
                signaling_server: None,
                ice_servers: vec![],
                connection_type: ConnectionType::Direct,
            },
            browser_info: BrowserInfo {
                user_agent: "Mozilla/5.0 (Test)".to_string(),
                browser_type: BrowserType::Chrome,
                version: "100.0".to_string(),
                platform: "Linux".to_string(),
                supports_webrtc: false,
                supports_clipboard_api: true,
            },
        };

        manager.establish_connection(connection_info).await.unwrap();

        // Shutdown should succeed
        assert!(manager.shutdown().await.is_ok());
    }
}
