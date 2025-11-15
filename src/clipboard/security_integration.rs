//! Security integration for clipboard synchronization
//! 
//! Provides automatic encryption, peer authentication, and trust verification
//! for clipboard operations.

use async_trait::async_trait;
use std::sync::Arc;
use crate::clipboard::{ClipboardContent, ClipboardResult, ClipboardError, PeerId};
use crate::security::{Security, SecuritySystem, SessionId};
use crate::security::identity::PeerId as SecurityPeerId;

/// Security integration for clipboard operations
pub struct ClipboardSecurityIntegration {
    /// Security system for encryption and authentication
    security_system: Arc<SecuritySystem>,
    /// Active sessions by peer ID
    sessions: Arc<tokio::sync::RwLock<std::collections::HashMap<PeerId, SessionId>>>,
}

impl ClipboardSecurityIntegration {
    /// Create new security integration with provided security system
    pub fn new(security_system: Arc<SecuritySystem>) -> Self {
        Self {
            security_system,
            sessions: Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Convert clipboard PeerId (String) to security PeerId
    fn to_security_peer_id(&self, peer_id: &PeerId) -> ClipboardResult<SecurityPeerId> {
        SecurityPeerId::from_string(peer_id)
            .map_err(|e| ClipboardError::security(format!("Invalid peer ID: {}", e)))
    }
    
    /// Get or establish a secure session with a peer
    pub async fn get_or_establish_session(&self, peer_id: &PeerId) -> ClipboardResult<SessionId> {
        // Check if we already have an active session
        {
            let sessions = self.sessions.read().await;
            if let Some(session_id) = sessions.get(peer_id) {
                return Ok(session_id.clone());
            }
        }
        
        // Convert to security peer ID
        let security_peer_id = self.to_security_peer_id(peer_id)?;
        
        // Establish new session
        let session_id = self.security_system
            .establish_session(&security_peer_id)
            .await
            .map_err(|e| ClipboardError::security(format!("Failed to establish session: {}", e)))?;
        
        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(peer_id.clone(), session_id.clone());
        }
        
        Ok(session_id)
    }
    
    /// Verify that a peer is trusted before clipboard operations
    pub async fn verify_peer_trust(&self, peer_id: &PeerId) -> ClipboardResult<bool> {
        let security_peer_id = self.to_security_peer_id(peer_id)?;
        self.security_system
            .is_trusted(&security_peer_id)
            .await
            .map_err(|e| ClipboardError::security(format!("Failed to verify peer trust: {}", e)))
    }
    
    /// Encrypt clipboard content for transmission to a peer
    pub async fn encrypt_content(
        &self,
        peer_id: &PeerId,
        content: &ClipboardContent,
    ) -> ClipboardResult<Vec<u8>> {
        // Verify peer is trusted
        if !self.verify_peer_trust(peer_id).await? {
            return Err(ClipboardError::security(format!(
                "Peer {} is not trusted for clipboard operations",
                peer_id
            )));
        }
        
        // Get or establish session
        let session_id = self.get_or_establish_session(peer_id).await?;
        
        // Serialize content
        let plaintext = serde_json::to_vec(content)
            .map_err(|e| ClipboardError::serialization("clipboard_content", e))?;
        
        // Encrypt content
        let ciphertext = self.security_system
            .encrypt_message(&session_id, &plaintext)
            .await
            .map_err(|e| ClipboardError::security(format!("Failed to encrypt content: {}", e)))?;
        
        Ok(ciphertext)
    }
    
    /// Decrypt clipboard content received from a peer
    pub async fn decrypt_content(
        &self,
        peer_id: &PeerId,
        ciphertext: &[u8],
    ) -> ClipboardResult<ClipboardContent> {
        // Verify peer is trusted
        if !self.verify_peer_trust(peer_id).await? {
            return Err(ClipboardError::security(format!(
                "Peer {} is not trusted for clipboard operations",
                peer_id
            )));
        }
        
        // Get or establish session
        let session_id = self.get_or_establish_session(peer_id).await?;
        
        // Decrypt content
        let plaintext = self.security_system
            .decrypt_message(&session_id, ciphertext)
            .await
            .map_err(|e| ClipboardError::security(format!("Failed to decrypt content: {}", e)))?;
        
        // Deserialize content
        let content = serde_json::from_slice(&plaintext)
            .map_err(|e| ClipboardError::serialization("clipboard_content", e))?;
        
        Ok(content)
    }
    
    /// Add a peer to the trusted list for clipboard operations
    pub async fn add_trusted_peer(&self, peer_id: PeerId, nickname: String) -> ClipboardResult<()> {
        let security_peer_id = self.to_security_peer_id(&peer_id)?;
        self.security_system
            .add_trusted_peer(security_peer_id, nickname)
            .await
            .map_err(|e| ClipboardError::security(format!("Failed to add trusted peer: {}", e)))
    }
    
    /// Remove a peer from the trusted list
    pub async fn remove_trusted_peer(&self, peer_id: &PeerId) -> ClipboardResult<()> {
        // Remove session if exists
        {
            let mut sessions = self.sessions.write().await;
            sessions.remove(peer_id);
        }
        
        let security_peer_id = self.to_security_peer_id(peer_id)?;
        self.security_system
            .remove_trusted_peer(&security_peer_id)
            .await
            .map_err(|e| ClipboardError::security(format!("Failed to remove trusted peer: {}", e)))
    }
    
    /// Get list of all trusted peers
    pub async fn get_trusted_peers(&self) -> ClipboardResult<Vec<PeerId>> {
        let peers = self.security_system
            .get_trusted_peers()
            .await
            .map_err(|e| ClipboardError::security(format!("Failed to get trusted peers: {}", e)))?;
        
        Ok(peers.into_iter().map(|entry| entry.peer_id.to_string()).collect())
    }
    
    /// Clear session for a peer (forces re-establishment on next operation)
    pub async fn clear_session(&self, peer_id: &PeerId) -> ClipboardResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(peer_id);
        Ok(())
    }
    
    /// Clear all sessions
    pub async fn clear_all_sessions(&self) -> ClipboardResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.clear();
        Ok(())
    }
    
    /// Get count of active sessions
    pub async fn active_session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }
    
    /// Cleanup expired sessions
    pub async fn cleanup_expired_sessions(&self) -> ClipboardResult<usize> {
        let count = self.security_system
            .cleanup_expired_sessions()
            .await
            .map_err(|e| ClipboardError::security(format!("Failed to cleanup sessions: {}", e)))?;
        
        // Also clear our local session cache
        // In a real implementation, we'd only clear expired ones
        // For now, we'll just report the count from the security system
        Ok(count)
    }
}

/// Trait for secure clipboard operations
#[async_trait]
pub trait SecureClipboard: Send + Sync {
    /// Encrypt clipboard content for a peer
    async fn encrypt_for_peer(
        &self,
        peer_id: &PeerId,
        content: &ClipboardContent,
    ) -> ClipboardResult<Vec<u8>>;
    
    /// Decrypt clipboard content from a peer
    async fn decrypt_from_peer(
        &self,
        peer_id: &PeerId,
        ciphertext: &[u8],
    ) -> ClipboardResult<ClipboardContent>;
    
    /// Verify peer is trusted
    async fn is_peer_trusted(&self, peer_id: &PeerId) -> ClipboardResult<bool>;
}

#[async_trait]
impl SecureClipboard for ClipboardSecurityIntegration {
    async fn encrypt_for_peer(
        &self,
        peer_id: &PeerId,
        content: &ClipboardContent,
    ) -> ClipboardResult<Vec<u8>> {
        self.encrypt_content(peer_id, content).await
    }
    
    async fn decrypt_from_peer(
        &self,
        peer_id: &PeerId,
        ciphertext: &[u8],
    ) -> ClipboardResult<ClipboardContent> {
        self.decrypt_content(peer_id, ciphertext).await
    }
    
    async fn is_peer_trusted(&self, peer_id: &PeerId) -> ClipboardResult<bool> {
        self.verify_peer_trust(peer_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::{TextContent, TextEncoding, TextFormat};
    use crate::security::identity::DeviceIdentity;
    
    #[tokio::test]
    async fn test_security_integration_creation() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let integration = ClipboardSecurityIntegration::new(security_system);
        
        assert_eq!(integration.active_session_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_peer_trust_verification() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let integration = ClipboardSecurityIntegration::new(security_system.clone());
        
        // Create a test peer
        let test_identity = DeviceIdentity::generate().unwrap();
        let test_peer_id = test_identity.derive_peer_id();
        
        // Initially not trusted
        assert!(!integration.verify_peer_trust(&test_peer_id).await.unwrap());
        
        // Add to trust list
        integration.add_trusted_peer(test_peer_id.clone(), "Test Peer".to_string())
            .await
            .unwrap();
        
        // Now should be trusted
        assert!(integration.verify_peer_trust(&test_peer_id).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_session_management() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let integration = ClipboardSecurityIntegration::new(security_system.clone());
        
        // Create and trust a test peer
        let test_identity = DeviceIdentity::generate().unwrap();
        let test_peer_id = test_identity.derive_peer_id();
        integration.add_trusted_peer(test_peer_id.clone(), "Test Peer".to_string())
            .await
            .unwrap();
        
        // Establish session
        let session_id = integration.get_or_establish_session(&test_peer_id).await.unwrap();
        assert_eq!(integration.active_session_count().await, 1);
        
        // Getting session again should return same session
        let session_id2 = integration.get_or_establish_session(&test_peer_id).await.unwrap();
        assert_eq!(session_id, session_id2);
        assert_eq!(integration.active_session_count().await, 1);
        
        // Clear session
        integration.clear_session(&test_peer_id).await.unwrap();
        assert_eq!(integration.active_session_count().await, 0);
    }
    
    #[tokio::test]
    async fn test_encrypt_decrypt_content() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let integration = ClipboardSecurityIntegration::new(security_system.clone());
        
        // Create and trust a test peer
        let test_identity = DeviceIdentity::generate().unwrap();
        let test_peer_id = test_identity.derive_peer_id();
        integration.add_trusted_peer(test_peer_id.clone(), "Test Peer".to_string())
            .await
            .unwrap();
        
        // Create test content
        let content = ClipboardContent::Text(TextContent {
            text: "Hello, secure clipboard!".to_string(),
            encoding: TextEncoding::Utf8,
            format: TextFormat::Plain,
            size: 24,
        });
        
        // Encrypt content
        let ciphertext = integration.encrypt_content(&test_peer_id, &content).await.unwrap();
        assert!(!ciphertext.is_empty());
        
        // Decrypt content
        let decrypted = integration.decrypt_content(&test_peer_id, &ciphertext).await.unwrap();
        
        // Verify content matches
        match (content, decrypted) {
            (ClipboardContent::Text(original), ClipboardContent::Text(decrypted)) => {
                assert_eq!(original.text, decrypted.text);
            }
            _ => panic!("Content type mismatch"),
        }
    }
    
    #[tokio::test]
    async fn test_untrusted_peer_rejection() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let integration = ClipboardSecurityIntegration::new(security_system.clone());
        
        // Create an untrusted peer
        let test_identity = DeviceIdentity::generate().unwrap();
        let test_peer_id = test_identity.derive_peer_id();
        
        // Create test content
        let content = ClipboardContent::Text(TextContent {
            text: "Test".to_string(),
            encoding: TextEncoding::Utf8,
            format: TextFormat::Plain,
            size: 4,
        });
        
        // Attempt to encrypt for untrusted peer should fail
        let result = integration.encrypt_content(&test_peer_id, &content).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not trusted"));
    }
    
    #[tokio::test]
    async fn test_trusted_peers_list() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let integration = ClipboardSecurityIntegration::new(security_system.clone());
        
        // Initially empty
        let peers = integration.get_trusted_peers().await.unwrap();
        assert_eq!(peers.len(), 0);
        
        // Add some peers
        let peer1 = DeviceIdentity::generate().unwrap().derive_peer_id();
        let peer2 = DeviceIdentity::generate().unwrap().derive_peer_id();
        
        integration.add_trusted_peer(peer1.clone(), "Peer 1".to_string()).await.unwrap();
        integration.add_trusted_peer(peer2.clone(), "Peer 2".to_string()).await.unwrap();
        
        // Get trusted peers
        let peers = integration.get_trusted_peers().await.unwrap();
        assert_eq!(peers.len(), 2);
        assert!(peers.contains(&peer1));
        assert!(peers.contains(&peer2));
    }
}
