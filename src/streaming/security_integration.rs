// Security integration for encrypted streaming
//
// Provides end-to-end encryption, peer authentication, and trust verification
// for video streams.
//
// Requirements: 8.1, 8.2, 10.4

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::security::{Security, SecurityResult, PeerId as SecurityPeerId, SessionId as SecuritySessionId};
use crate::streaming::{StreamError, StreamResult, PeerId, SessionId};

/// Security integration for streaming system
/// 
/// Manages encrypted video streams with peer authentication and trust verification.
/// 
/// Requirements: 8.1, 8.2
pub struct StreamSecurityManager {
    security_system: Arc<dyn Security>,
    active_sessions: Arc<RwLock<HashMap<SessionId, SecuritySessionId>>>,
    trusted_peers: Arc<RwLock<HashMap<PeerId, PeerTrustInfo>>>,
}

/// Trust information for a peer
#[derive(Debug, Clone)]
pub struct PeerTrustInfo {
    pub peer_id: PeerId,
    pub security_peer_id: SecurityPeerId,
    pub is_trusted: bool,
    pub nickname: Option<String>,
    pub verified_at: std::time::SystemTime,
}

impl StreamSecurityManager {
    /// Create a new stream security manager
    pub fn new(security_system: Arc<dyn Security>) -> Self {
        Self {
            security_system,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            trusted_peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Verify peer authentication and trust for stream access
    /// 
    /// Requirements: 8.2
    pub async fn verify_peer_access(&self, peer_id: &PeerId) -> StreamResult<bool> {
        // Check if peer is in trusted cache
        {
            let trusted = self.trusted_peers.read().await;
            if let Some(info) = trusted.get(peer_id) {
                return Ok(info.is_trusted);
            }
        }

        // Convert peer_id to security peer_id format
        let security_peer_id = crate::security::PeerId::from_hex(peer_id)
            .unwrap_or_else(|_| {
                // If not a valid hex, create a hash of the peer_id string
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(peer_id.as_bytes());
                let fingerprint: [u8; 32] = hasher.finalize().into();
                crate::security::PeerId::from_fingerprint(fingerprint)
            });

        // Check with security system
        let is_trusted = self.security_system
            .is_trusted(&security_peer_id)
            .await
            .map_err(|e| StreamError::internal(format!("Trust verification failed: {}", e)))?;

        // Cache the result
        let trust_info = PeerTrustInfo {
            peer_id: peer_id.clone(),
            security_peer_id,
            is_trusted,
            nickname: None,
            verified_at: std::time::SystemTime::now(),
        };

        self.trusted_peers.write().await.insert(peer_id.clone(), trust_info);

        Ok(is_trusted)
    }

    /// Establish a secure session for streaming
    /// 
    /// Requirements: 8.1, 10.4
    pub async fn establish_secure_session(&self, peer_id: &PeerId, stream_session_id: SessionId) -> StreamResult<SecuritySessionId> {
        // Verify peer is trusted first
        if !self.verify_peer_access(peer_id).await? {
            return Err(StreamError::permission(format!("Peer {} is not trusted", peer_id)));
        }

        // Convert peer_id to security format
        let security_peer_id = crate::security::PeerId::from_hex(peer_id)
            .unwrap_or_else(|_| {
                // If not a valid hex, create a hash of the peer_id string
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(peer_id.as_bytes());
                let fingerprint: [u8; 32] = hasher.finalize().into();
                crate::security::PeerId::from_fingerprint(fingerprint)
            });

        // Establish secure session with security system
        let security_session = self.security_system
            .establish_session(&security_peer_id)
            .await
            .map_err(|e| StreamError::internal(format!("Failed to establish secure session: {}", e)))?;

        // Store the mapping
        self.active_sessions.write().await.insert(stream_session_id, security_session.clone());

        Ok(security_session)
    }

    /// Encrypt video stream data
    /// 
    /// Requirements: 8.1
    pub async fn encrypt_stream_data(&self, stream_session_id: &SessionId, data: &[u8]) -> StreamResult<Vec<u8>> {
        // Get the security session
        let security_session = {
            let sessions = self.active_sessions.read().await;
            sessions.get(stream_session_id)
                .cloned()
                .ok_or_else(|| StreamError::session_not_found(format!("No security session for stream {}", stream_session_id)))?
        };

        // Encrypt the data
        self.security_system
            .encrypt_message(&security_session, data)
            .await
            .map_err(|e| StreamError::internal(format!("Encryption failed: {}", e)))
    }

    /// Decrypt video stream data
    /// 
    /// Requirements: 8.1
    pub async fn decrypt_stream_data(&self, stream_session_id: &SessionId, data: &[u8]) -> StreamResult<Vec<u8>> {
        // Get the security session
        let security_session = {
            let sessions = self.active_sessions.read().await;
            sessions.get(stream_session_id)
                .cloned()
                .ok_or_else(|| StreamError::session_not_found(format!("No security session for stream {}", stream_session_id)))?
        };

        // Decrypt the data
        self.security_system
            .decrypt_message(&security_session, data)
            .await
            .map_err(|e| StreamError::internal(format!("Decryption failed: {}", e)))
    }

    /// Close a secure session
    pub async fn close_secure_session(&self, stream_session_id: &SessionId) -> StreamResult<()> {
        self.active_sessions.write().await.remove(stream_session_id);
        Ok(())
    }

    /// Add a trusted peer for streaming
    /// 
    /// Requirements: 8.2
    pub async fn add_trusted_peer(&self, peer_id: PeerId, nickname: Option<String>) -> StreamResult<()> {
        let security_peer_id = crate::security::PeerId::from_hex(&peer_id)
            .unwrap_or_else(|_| {
                // If not a valid hex, create a hash of the peer_id string
                use sha2::{Sha256, Digest};
                let mut hasher = Sha256::new();
                hasher.update(peer_id.as_bytes());
                let fingerprint: [u8; 32] = hasher.finalize().into();
                crate::security::PeerId::from_fingerprint(fingerprint)
            });

        // Add to security system
        self.security_system
            .add_trusted_peer(security_peer_id.clone(), nickname.clone().unwrap_or_default())
            .await
            .map_err(|e| StreamError::internal(format!("Failed to add trusted peer: {}", e)))?;

        // Update cache
        let trust_info = PeerTrustInfo {
            peer_id: peer_id.clone(),
            security_peer_id,
            is_trusted: true,
            nickname,
            verified_at: std::time::SystemTime::now(),
        };

        self.trusted_peers.write().await.insert(peer_id, trust_info);

        Ok(())
    }

    /// Get trust information for a peer
    pub async fn get_peer_trust_info(&self, peer_id: &PeerId) -> StreamResult<Option<PeerTrustInfo>> {
        Ok(self.trusted_peers.read().await.get(peer_id).cloned())
    }

    /// Clear trust cache (forces re-verification)
    pub async fn clear_trust_cache(&self) {
        self.trusted_peers.write().await.clear();
    }
}

/// Secure stream wrapper that automatically encrypts/decrypts data
/// 
/// Requirements: 8.1, 10.4
pub struct SecureStreamWrapper {
    security_manager: Arc<StreamSecurityManager>,
    stream_session_id: SessionId,
}

impl SecureStreamWrapper {
    /// Create a new secure stream wrapper
    pub fn new(security_manager: Arc<StreamSecurityManager>, stream_session_id: SessionId) -> Self {
        Self {
            security_manager,
            stream_session_id,
        }
    }

    /// Encrypt frame data before transmission
    pub async fn encrypt_frame(&self, frame_data: &[u8]) -> StreamResult<Vec<u8>> {
        self.security_manager.encrypt_stream_data(&self.stream_session_id, frame_data).await
    }

    /// Decrypt received frame data
    pub async fn decrypt_frame(&self, encrypted_data: &[u8]) -> StreamResult<Vec<u8>> {
        self.security_manager.decrypt_stream_data(&self.stream_session_id, encrypted_data).await
    }
}

/// Stream access control manager
/// 
/// Manages viewer approval and rejection workflow.
/// 
/// Requirements: 8.3, 8.4
pub struct StreamAccessControl {
    security_manager: Arc<StreamSecurityManager>,
    pending_requests: Arc<RwLock<HashMap<PeerId, AccessRequest>>>,
    approved_viewers: Arc<RwLock<HashMap<PeerId, ViewerAccess>>>,
}

/// Access request from a peer
#[derive(Debug, Clone)]
pub struct AccessRequest {
    pub peer_id: PeerId,
    pub requested_at: std::time::SystemTime,
    pub device_name: Option<String>,
}

/// Approved viewer access
#[derive(Debug, Clone)]
pub struct ViewerAccess {
    pub peer_id: PeerId,
    pub approved_at: std::time::SystemTime,
    pub permissions: crate::streaming::ViewerPermissions,
}

impl StreamAccessControl {
    /// Create a new stream access control manager
    pub fn new(security_manager: Arc<StreamSecurityManager>) -> Self {
        Self {
            security_manager,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            approved_viewers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Request access to a stream
    /// 
    /// Requirements: 8.3
    pub async fn request_access(&self, peer_id: PeerId, device_name: Option<String>) -> StreamResult<()> {
        // Verify peer is trusted
        if !self.security_manager.verify_peer_access(&peer_id).await? {
            return Err(StreamError::permission(format!("Peer {} is not trusted", peer_id)));
        }

        let request = AccessRequest {
            peer_id: peer_id.clone(),
            requested_at: std::time::SystemTime::now(),
            device_name,
        };

        self.pending_requests.write().await.insert(peer_id, request);

        Ok(())
    }

    /// Approve a viewer request
    /// 
    /// Requirements: 8.3
    pub async fn approve_viewer(&self, peer_id: &PeerId, permissions: crate::streaming::ViewerPermissions) -> StreamResult<()> {
        // Remove from pending
        self.pending_requests.write().await.remove(peer_id);

        // Add to approved
        let access = ViewerAccess {
            peer_id: peer_id.clone(),
            approved_at: std::time::SystemTime::now(),
            permissions,
        };

        self.approved_viewers.write().await.insert(peer_id.clone(), access);

        Ok(())
    }

    /// Reject a viewer request
    /// 
    /// Requirements: 8.3
    pub async fn reject_viewer(&self, peer_id: &PeerId) -> StreamResult<()> {
        self.pending_requests.write().await.remove(peer_id);
        Ok(())
    }

    /// Revoke viewer access
    /// 
    /// Requirements: 8.5
    pub async fn revoke_access(&self, peer_id: &PeerId) -> StreamResult<()> {
        self.approved_viewers.write().await.remove(peer_id);
        Ok(())
    }

    /// Check if a peer has access
    pub async fn has_access(&self, peer_id: &PeerId) -> bool {
        self.approved_viewers.read().await.contains_key(peer_id)
    }

    /// Get pending access requests
    pub async fn get_pending_requests(&self) -> Vec<AccessRequest> {
        self.pending_requests.read().await.values().cloned().collect()
    }

    /// Get approved viewers
    pub async fn get_approved_viewers(&self) -> Vec<ViewerAccess> {
        self.approved_viewers.read().await.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests require a mock security system
    // These tests verify the structure and basic logic

    #[test]
    fn test_peer_trust_info_creation() {
        let info = PeerTrustInfo {
            peer_id: "test-peer".to_string(),
            security_peer_id: SecurityPeerId::from("test-peer".to_string()),
            is_trusted: true,
            nickname: Some("Test Device".to_string()),
            verified_at: std::time::SystemTime::now(),
        };

        assert_eq!(info.peer_id, "test-peer");
        assert!(info.is_trusted);
        assert_eq!(info.nickname, Some("Test Device".to_string()));
    }

    #[test]
    fn test_access_request_creation() {
        let request = AccessRequest {
            peer_id: "test-peer".to_string(),
            requested_at: std::time::SystemTime::now(),
            device_name: Some("Test Device".to_string()),
        };

        assert_eq!(request.peer_id, "test-peer");
        assert_eq!(request.device_name, Some("Test Device".to_string()));
    }
}
