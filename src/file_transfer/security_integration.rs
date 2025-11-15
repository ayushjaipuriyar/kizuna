// Security Integration Module
//
// Integrates file transfer system with security layer for encrypted transfers,
// peer authentication, and trust verification

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::*,
};
use crate::security::{Security, SecurityResult};
use crate::security::encryption::SessionId as SecuritySessionId;
use async_trait::async_trait;
use std::sync::Arc;

/// Security integration for file transfers
pub struct FileTransferSecurity {
    security_system: Arc<dyn Security>,
}

impl FileTransferSecurity {
    /// Create a new file transfer security integration
    pub fn new(security_system: Arc<dyn Security>) -> Self {
        Self { security_system }
    }

    /// Authenticate peer before accepting transfer request
    pub async fn authenticate_peer(&self, peer_id: &PeerId) -> Result<bool> {
        // Convert String PeerId to security::identity::PeerId
        let security_peer_id = crate::security::identity::PeerId::from_hex(peer_id)
            .map_err(|e| FileTransferError::SecurityError(format!("Invalid peer ID: {}", e)))?;
        
        self.security_system
            .is_trusted(&security_peer_id)
            .await
            .map_err(|e| FileTransferError::SecurityError(format!("Authentication failed: {}", e)))
    }

    /// Verify peer trust level for transfer
    pub async fn verify_peer_trust(&self, peer_id: &PeerId) -> Result<()> {
        let is_trusted = self.authenticate_peer(peer_id).await?;
        
        if !is_trusted {
            return Err(FileTransferError::SecurityError(format!(
                "Peer {} is not trusted",
                peer_id
            )));
        }
        
        Ok(())
    }

    /// Establish secure session for file transfer
    pub async fn establish_secure_session(&self, peer_id: &PeerId) -> Result<SecuritySessionId> {
        // Convert String PeerId to security::identity::PeerId
        let security_peer_id = crate::security::identity::PeerId::from_hex(peer_id)
            .map_err(|e| FileTransferError::SecurityError(format!("Invalid peer ID: {}", e)))?;
        
        self.security_system
            .establish_session(&security_peer_id)
            .await
            .map_err(|e| FileTransferError::SecurityError(format!("Failed to establish session: {}", e)))
    }

    /// Encrypt transfer manifest for secure exchange
    pub async fn encrypt_manifest(
        &self,
        session_id: &SecuritySessionId,
        manifest: &TransferManifest,
    ) -> Result<Vec<u8>> {
        // Serialize manifest to JSON
        let manifest_json = serde_json::to_vec(manifest)
            .map_err(|e| FileTransferError::InternalError(format!("Failed to serialize manifest: {}", e)))?;

        // Encrypt the serialized manifest
        self.security_system
            .encrypt_message(session_id, &manifest_json)
            .await
            .map_err(|e| FileTransferError::SecurityError(format!("Failed to encrypt manifest: {}", e)))
    }

    /// Decrypt received transfer manifest
    pub async fn decrypt_manifest(
        &self,
        session_id: &SecuritySessionId,
        encrypted_data: &[u8],
    ) -> Result<TransferManifest> {
        // Decrypt the data
        let decrypted_data = self.security_system
            .decrypt_message(session_id, encrypted_data)
            .await
            .map_err(|e| FileTransferError::SecurityError(format!("Failed to decrypt manifest: {}", e)))?;

        // Deserialize manifest from JSON
        let manifest: TransferManifest = serde_json::from_slice(&decrypted_data)
            .map_err(|e| FileTransferError::InternalError(format!("Failed to deserialize manifest: {}", e)))?;

        Ok(manifest)
    }

    /// Validate manifest integrity and authenticity
    pub async fn validate_manifest(
        &self,
        manifest: &TransferManifest,
        peer_id: &PeerId,
    ) -> Result<()> {
        // Verify the manifest sender matches the peer
        if manifest.sender_id != *peer_id {
            return Err(FileTransferError::SecurityError(format!(
                "Manifest sender {} does not match peer {}",
                manifest.sender_id, peer_id
            )));
        }

        // Verify manifest checksum
        // TODO: Implement actual checksum verification
        // For now, just check that checksum is not all zeros
        if manifest.checksum == [0u8; 32] {
            return Err(FileTransferError::IntegrityError(
                "Manifest checksum is invalid".to_string(),
            ));
        }

        Ok(())
    }

    /// Encrypt chunk data for transmission
    pub async fn encrypt_chunk(
        &self,
        session_id: &SecuritySessionId,
        chunk: &Chunk,
    ) -> Result<Vec<u8>> {
        // Serialize chunk to binary format
        let chunk_data = bincode::serialize(chunk)
            .map_err(|e| FileTransferError::InternalError(format!("Failed to serialize chunk: {}", e)))?;

        // Encrypt the chunk data
        self.security_system
            .encrypt_message(session_id, &chunk_data)
            .await
            .map_err(|e| FileTransferError::SecurityError(format!("Failed to encrypt chunk: {}", e)))
    }

    /// Decrypt received chunk data
    pub async fn decrypt_chunk(
        &self,
        session_id: &SecuritySessionId,
        encrypted_data: &[u8],
    ) -> Result<Chunk> {
        // Decrypt the data
        let decrypted_data = self.security_system
            .decrypt_message(session_id, encrypted_data)
            .await
            .map_err(|e| FileTransferError::SecurityError(format!("Failed to decrypt chunk: {}", e)))?;

        // Deserialize chunk from binary format
        let chunk: Chunk = bincode::deserialize(&decrypted_data)
            .map_err(|e| FileTransferError::InternalError(format!("Failed to deserialize chunk: {}", e)))?;

        Ok(chunk)
    }

    /// Verify chunk integrity after decryption
    pub async fn verify_chunk_integrity(&self, chunk: &Chunk) -> Result<()> {
        // Calculate checksum of chunk data
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&chunk.data);
        let calculated_checksum: [u8; 32] = hasher.finalize().into();

        // Compare with stored checksum
        if calculated_checksum != chunk.checksum {
            return Err(FileTransferError::IntegrityError(format!(
                "Chunk {} integrity check failed",
                chunk.chunk_id
            )));
        }

        Ok(())
    }
}

/// Secure transfer session that wraps a regular transfer session with security
pub struct SecureTransferSession {
    session: TransferSession,
    security_session_id: SecuritySessionId,
    security: Arc<FileTransferSecurity>,
}

impl SecureTransferSession {
    /// Create a new secure transfer session
    pub fn new(
        session: TransferSession,
        security_session_id: SecuritySessionId,
        security: Arc<FileTransferSecurity>,
    ) -> Self {
        Self {
            session,
            security_session_id,
            security,
        }
    }

    /// Get the underlying transfer session
    pub fn session(&self) -> &TransferSession {
        &self.session
    }

    /// Get the security session ID
    pub fn security_session_id(&self) -> &SecuritySessionId {
        &self.security_session_id
    }

    /// Encrypt and send chunk
    pub async fn send_encrypted_chunk(&self, chunk: &Chunk) -> Result<Vec<u8>> {
        self.security.encrypt_chunk(&self.security_session_id, chunk).await
    }

    /// Receive and decrypt chunk
    pub async fn receive_encrypted_chunk(&self, encrypted_data: &[u8]) -> Result<Chunk> {
        let chunk = self.security.decrypt_chunk(&self.security_session_id, encrypted_data).await?;
        self.security.verify_chunk_integrity(&chunk).await?;
        Ok(chunk)
    }

    /// Encrypt manifest for exchange
    pub async fn encrypt_manifest(&self, manifest: &TransferManifest) -> Result<Vec<u8>> {
        self.security.encrypt_manifest(&self.security_session_id, manifest).await
    }

    /// Decrypt received manifest
    pub async fn decrypt_manifest(&self, encrypted_data: &[u8]) -> Result<TransferManifest> {
        let manifest = self.security.decrypt_manifest(&self.security_session_id, encrypted_data).await?;
        self.security.validate_manifest(&manifest, &self.session.peer_id).await?;
        Ok(manifest)
    }
}

/// Trait for secure transfer operations
#[async_trait]
pub trait SecureTransfer: Send + Sync {
    /// Start a secure transfer with peer authentication
    async fn start_secure_transfer(
        &self,
        manifest: TransferManifest,
        peer_id: PeerId,
    ) -> Result<SecureTransferSession>;

    /// Accept incoming secure transfer request
    async fn accept_secure_transfer(
        &self,
        peer_id: PeerId,
        encrypted_manifest: &[u8],
    ) -> Result<SecureTransferSession>;

    /// Reject incoming transfer request
    async fn reject_transfer(&self, peer_id: PeerId, reason: String) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::identity::{DeviceIdentity, PeerId as SecurityPeerId};
    use std::path::PathBuf;

    // Mock security system for testing
    struct MockSecurity {
        trusted_peers: Vec<String>,
    }

    #[async_trait]
    impl Security for MockSecurity {
        async fn get_device_identity(&self) -> SecurityResult<DeviceIdentity> {
            unimplemented!()
        }

        async fn get_peer_id(&self) -> SecurityResult<SecurityPeerId> {
            Ok("test-device".to_string())
        }

        async fn establish_session(&self, _peer_id: &SecurityPeerId) -> SecurityResult<SecuritySessionId> {
            Ok(uuid::Uuid::new_v4())
        }

        async fn encrypt_message(&self, _session_id: &SecuritySessionId, data: &[u8]) -> SecurityResult<Vec<u8>> {
            // Simple XOR encryption for testing
            Ok(data.iter().map(|b| b ^ 0xAA).collect())
        }

        async fn decrypt_message(&self, _session_id: &SecuritySessionId, data: &[u8]) -> SecurityResult<Vec<u8>> {
            // Simple XOR decryption for testing
            Ok(data.iter().map(|b| b ^ 0xAA).collect())
        }

        async fn is_trusted(&self, peer_id: &SecurityPeerId) -> SecurityResult<bool> {
            Ok(self.trusted_peers.contains(peer_id))
        }

        async fn add_trusted_peer(&self, _peer_id: SecurityPeerId, _nickname: String) -> SecurityResult<()> {
            Ok(())
        }
    }

    fn create_test_security() -> Arc<dyn Security> {
        Arc::new(MockSecurity {
            trusted_peers: vec!["trusted-peer".to_string()],
        })
    }

    fn create_test_manifest() -> TransferManifest {
        let mut manifest = TransferManifest::new("test-sender".to_string());
        manifest.checksum = [1u8; 32]; // Non-zero checksum
        manifest
    }

    #[tokio::test]
    async fn test_authenticate_trusted_peer() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);

        let result = ft_security.authenticate_peer(&"trusted-peer".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_authenticate_untrusted_peer() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);

        let result = ft_security.authenticate_peer(&"untrusted-peer".to_string()).await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_verify_peer_trust_success() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);

        let result = ft_security.verify_peer_trust(&"trusted-peer".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_peer_trust_failure() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);

        let result = ft_security.verify_peer_trust(&"untrusted-peer".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_establish_secure_session() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);

        let result = ft_security.establish_secure_session(&"test-peer".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_manifest() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);
        let session_id = uuid::Uuid::new_v4();

        let manifest = create_test_manifest();

        // Encrypt manifest
        let encrypted = ft_security.encrypt_manifest(&session_id, &manifest).await.unwrap();
        assert!(!encrypted.is_empty());

        // Decrypt manifest
        let decrypted = ft_security.decrypt_manifest(&session_id, &encrypted).await.unwrap();
        assert_eq!(decrypted.transfer_id, manifest.transfer_id);
        assert_eq!(decrypted.sender_id, manifest.sender_id);
    }

    #[tokio::test]
    async fn test_validate_manifest_success() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);

        let manifest = create_test_manifest();
        let result = ft_security.validate_manifest(&manifest, &manifest.sender_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_manifest_sender_mismatch() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);

        let manifest = create_test_manifest();
        let result = ft_security.validate_manifest(&manifest, &"different-peer".to_string()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_validate_manifest_invalid_checksum() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);

        let mut manifest = create_test_manifest();
        manifest.checksum = [0u8; 32]; // Invalid checksum
        
        let result = ft_security.validate_manifest(&manifest, &manifest.sender_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_chunk() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);
        let session_id = uuid::Uuid::new_v4();

        // Create test chunk
        let data = vec![1, 2, 3, 4, 5];
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum: [u8; 32] = hasher.finalize().into();

        let chunk = Chunk {
            chunk_id: 0,
            file_path: PathBuf::from("test.txt"),
            offset: 0,
            size: data.len(),
            data: data.clone(),
            checksum,
            compressed: false,
        };

        // Encrypt chunk
        let encrypted = ft_security.encrypt_chunk(&session_id, &chunk).await.unwrap();
        assert!(!encrypted.is_empty());

        // Decrypt chunk
        let decrypted = ft_security.decrypt_chunk(&session_id, &encrypted).await.unwrap();
        assert_eq!(decrypted.chunk_id, chunk.chunk_id);
        assert_eq!(decrypted.data, chunk.data);
    }

    #[tokio::test]
    async fn test_verify_chunk_integrity_success() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);

        let data = vec![1, 2, 3, 4, 5];
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let checksum: [u8; 32] = hasher.finalize().into();

        let chunk = Chunk {
            chunk_id: 0,
            file_path: PathBuf::from("test.txt"),
            offset: 0,
            size: data.len(),
            data,
            checksum,
            compressed: false,
        };

        let result = ft_security.verify_chunk_integrity(&chunk).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_verify_chunk_integrity_failure() {
        let security = create_test_security();
        let ft_security = FileTransferSecurity::new(security);

        let chunk = Chunk {
            chunk_id: 0,
            file_path: PathBuf::from("test.txt"),
            offset: 0,
            size: 5,
            data: vec![1, 2, 3, 4, 5],
            checksum: [0u8; 32], // Wrong checksum
            compressed: false,
        };

        let result = ft_security.verify_chunk_integrity(&chunk).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_secure_transfer_session() {
        let security = create_test_security();
        let ft_security = Arc::new(FileTransferSecurity::new(security));
        let session_id = uuid::Uuid::new_v4();

        let manifest = create_test_manifest();
        let transfer_session = TransferSession::new(
            manifest.clone(),
            "test-peer".to_string(),
            TransportProtocol::Tcp,
        );

        let secure_session = SecureTransferSession::new(
            transfer_session,
            session_id,
            ft_security,
        );

        assert_eq!(secure_session.session().peer_id, "test-peer");
        assert_eq!(secure_session.security_session_id(), &session_id);
    }
}
