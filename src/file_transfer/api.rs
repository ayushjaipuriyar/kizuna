// Unified File Transfer API
//
// High-level API that integrates all file transfer components and provides
// a simple interface for applications

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::*,
    security_integration::{FileTransferSecurity, SecureTransferSession, SecureTransfer},
    transport_integration::FileTransferTransport,
    progress::{ProgressTracker, ProgressCallback, EventCallback, TransferEvent},
    session::SessionManager,
    transport::TransportNegotiatorImpl,
    TransportNegotiator,
    FileTransfer, TransferManager,
};
use crate::security::Security;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

/// Unified file transfer system
pub struct FileTransferSystem {
    /// Security integration
    security: Arc<FileTransferSecurity>,
    /// Transport integration
    transport: Arc<FileTransferTransport>,
    /// Session manager
    session_manager: Arc<SessionManager>,
    /// Transport negotiator
    transport_negotiator: Arc<TransportNegotiatorImpl>,
    /// Progress tracker
    progress_tracker: Arc<ProgressTracker>,
    /// Global bandwidth limit
    bandwidth_limit: Arc<tokio::sync::RwLock<Option<u64>>>,
}

impl FileTransferSystem {
    /// Create a new file transfer system
    pub fn new(
        security_system: Arc<dyn Security>,
        session_persistence_dir: PathBuf,
    ) -> Self {
        let security = Arc::new(FileTransferSecurity::new(security_system));
        let transport = Arc::new(FileTransferTransport::new());
        let session_manager = Arc::new(SessionManager::new(session_persistence_dir));
        let transport_negotiator = Arc::new(TransportNegotiatorImpl::new());
        let progress_tracker = Arc::new(ProgressTracker::new());

        Self {
            security,
            transport,
            session_manager,
            transport_negotiator,
            progress_tracker,
            bandwidth_limit: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Get transport integration for adding connections
    pub fn transport(&self) -> &Arc<FileTransferTransport> {
        &self.transport
    }

    /// Initialize the file transfer system
    pub async fn initialize(&self) -> Result<()> {
        self.session_manager.initialize().await?;
        Ok(())
    }

    /// Register a progress callback
    pub async fn on_progress(&self, callback: ProgressCallback) {
        self.progress_tracker.register_progress_callback(callback).await;
    }

    /// Register an event callback
    pub async fn on_event(&self, callback: EventCallback) {
        self.progress_tracker.register_event_callback(callback).await;
    }

    /// Send a file to a peer
    pub async fn send_file(
        &self,
        file_path: PathBuf,
        peer_id: PeerId,
    ) -> Result<TransferSession> {
        // Build manifest for single file
        let manifest = self.build_file_manifest(file_path).await?;
        
        // Start transfer
        self.start_transfer(manifest, peer_id).await
    }

    /// Send multiple files to a peer
    pub async fn send_files(
        &self,
        file_paths: Vec<PathBuf>,
        peer_id: PeerId,
    ) -> Result<TransferSession> {
        // Build manifest for multiple files
        let manifest = self.build_multi_file_manifest(file_paths).await?;
        
        // Start transfer
        self.start_transfer(manifest, peer_id).await
    }

    /// Send a folder to a peer
    pub async fn send_folder(
        &self,
        folder_path: PathBuf,
        peer_id: PeerId,
        recursive: bool,
    ) -> Result<TransferSession> {
        // Build manifest for folder
        let manifest = self.build_folder_manifest(folder_path, recursive).await?;
        
        // Start transfer
        self.start_transfer(manifest, peer_id).await
    }

    /// Build manifest for a single file
    async fn build_file_manifest(&self, _file_path: PathBuf) -> Result<TransferManifest> {
        // TODO: Implement actual manifest building
        // For now, return a placeholder
        Ok(TransferManifest::new("local-peer".to_string()))
    }

    /// Build manifest for multiple files
    async fn build_multi_file_manifest(&self, _file_paths: Vec<PathBuf>) -> Result<TransferManifest> {
        // TODO: Implement actual manifest building
        // For now, return a placeholder
        Ok(TransferManifest::new("local-peer".to_string()))
    }

    /// Build manifest for a folder
    async fn build_folder_manifest(&self, _folder_path: PathBuf, _recursive: bool) -> Result<TransferManifest> {
        // TODO: Implement actual manifest building
        // For now, return a placeholder
        Ok(TransferManifest::new("local-peer".to_string()))
    }

    /// Get detailed transfer statistics
    pub async fn get_transfer_stats(&self, session_id: SessionId) -> Result<TransferStats> {
        let session = self.session_manager.get_session(session_id).await?;
        let progress = self.progress_tracker.get_progress(session_id).await?;

        Ok(TransferStats {
            session_id,
            peer_id: session.peer_id,
            transport: session.transport,
            state: session.state,
            progress,
            bandwidth_limit: session.bandwidth_limit,
            parallel_streams: session.parallel_streams,
        })
    }

    /// Get all active transfers
    pub async fn get_all_transfers(&self) -> Result<Vec<TransferStats>> {
        let sessions = self.session_manager.get_active_sessions().await?;
        let mut stats = Vec::new();

        for session in sessions {
            if let Ok(transfer_stats) = self.get_transfer_stats(session.session_id).await {
                stats.push(transfer_stats);
            }
        }

        Ok(stats)
    }

    /// Pause a transfer
    pub async fn pause_transfer(&self, session_id: SessionId) -> Result<()> {
        self.session_manager
            .update_session_state(session_id, TransferState::Paused)
            .await
    }

    /// Resume a paused transfer
    pub async fn resume_paused_transfer(&self, session_id: SessionId) -> Result<()> {
        self.session_manager
            .update_session_state(session_id, TransferState::Transferring)
            .await
    }
}

#[async_trait]
impl FileTransfer for FileTransferSystem {
    async fn start_transfer(
        &self,
        manifest: TransferManifest,
        peer_id: PeerId,
    ) -> Result<TransferSession> {
        // Verify peer trust
        self.security.verify_peer_trust(&peer_id).await?;

        // Negotiate transport protocol
        let protocol = self
            .transport_negotiator
            .negotiate_transport(peer_id.clone(), manifest.total_size)
            .await?;

        // Create transfer session
        let session = self
            .session_manager
            .create_session(manifest.clone(), peer_id.clone(), protocol)
            .await?;

        // Start progress tracking
        self.progress_tracker
            .start_session(session.session_id, manifest)
            .await;

        // Notify event
        self.progress_tracker
            .notify_event(TransferEvent::Started {
                session_id: session.session_id,
                manifest: session.manifest.clone(),
            })
            .await;

        Ok(session)
    }

    async fn resume_transfer(&self, resume_token: ResumeToken) -> Result<TransferSession> {
        // Validate resume token
        if resume_token.is_expired() {
            return Err(FileTransferError::ResumeTokenExpired);
        }

        // Get existing session
        let session = self
            .session_manager
            .get_session(resume_token.session_id)
            .await?;

        // Update session state
        self.session_manager
            .update_session_state(session.session_id, TransferState::Transferring)
            .await?;

        // Resume progress tracking
        self.progress_tracker
            .update_progress(session.session_id, resume_token.bytes_completed)
            .await?;

        Ok(session)
    }

    async fn cancel_transfer(&self, session_id: SessionId) -> Result<()> {
        // Update session state
        self.session_manager
            .update_session_state(session_id, TransferState::Cancelled)
            .await?;

        // Cancel progress tracking
        self.progress_tracker.cancel_session(session_id).await?;

        Ok(())
    }

    async fn set_bandwidth_limit(&self, limit: Option<u64>) -> Result<()> {
        let mut bandwidth_limit = self.bandwidth_limit.write().await;
        *bandwidth_limit = limit;
        Ok(())
    }

    async fn get_active_transfers(&self) -> Result<Vec<TransferSession>> {
        self.session_manager.get_active_sessions().await
    }

    async fn get_transfer_progress(&self, session_id: SessionId) -> Result<TransferProgress> {
        self.progress_tracker.get_progress(session_id).await
    }
}

#[async_trait]
impl SecureTransfer for FileTransferSystem {
    async fn start_secure_transfer(
        &self,
        manifest: TransferManifest,
        peer_id: PeerId,
    ) -> Result<SecureTransferSession> {
        // Start regular transfer
        let session = self.start_transfer(manifest.clone(), peer_id.clone()).await?;

        // Establish secure session
        let security_session_id = self.security.establish_secure_session(&peer_id).await?;

        // Create secure transfer session
        Ok(SecureTransferSession::new(
            session,
            security_session_id,
            Arc::clone(&self.security),
        ))
    }

    async fn accept_secure_transfer(
        &self,
        peer_id: PeerId,
        encrypted_manifest: &[u8],
    ) -> Result<SecureTransferSession> {
        // Verify peer trust
        self.security.verify_peer_trust(&peer_id).await?;

        // Establish secure session
        let security_session_id = self.security.establish_secure_session(&peer_id).await?;

        // Decrypt manifest
        let manifest = self
            .security
            .decrypt_manifest(&security_session_id, encrypted_manifest)
            .await?;

        // Validate manifest
        self.security
            .validate_manifest(&manifest, &peer_id)
            .await?;

        // Negotiate transport
        let protocol = self
            .transport_negotiator
            .negotiate_transport(peer_id.clone(), manifest.total_size)
            .await?;

        // Create transfer session
        let session = self
            .session_manager
            .create_session(manifest.clone(), peer_id, protocol)
            .await?;

        // Start progress tracking
        self.progress_tracker
            .start_session(session.session_id, manifest)
            .await;

        // Create secure transfer session
        Ok(SecureTransferSession::new(
            session,
            security_session_id,
            Arc::clone(&self.security),
        ))
    }

    async fn reject_transfer(&self, _peer_id: PeerId, _reason: String) -> Result<()> {
        // TODO: Implement transfer rejection notification
        Ok(())
    }
}

/// Detailed transfer statistics
#[derive(Debug, Clone)]
pub struct TransferStats {
    pub session_id: SessionId,
    pub peer_id: PeerId,
    pub transport: TransportProtocol,
    pub state: TransferState,
    pub progress: TransferProgress,
    pub bandwidth_limit: Option<u64>,
    pub parallel_streams: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::identity::{DeviceIdentity, PeerId as SecurityPeerId};
    use crate::security::encryption::SessionId as SecuritySessionId;
    use crate::security::SecurityResult;
    use crate::transport::{Connection, ConnectionInfo, TransportError};
    use std::net::SocketAddr;
    use tempfile::TempDir;

    // Mock security system
    struct MockSecurity;

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
            Ok(data.to_vec())
        }

        async fn decrypt_message(&self, _session_id: &SecuritySessionId, data: &[u8]) -> SecurityResult<Vec<u8>> {
            Ok(data.to_vec())
        }

        async fn is_trusted(&self, _peer_id: &SecurityPeerId) -> SecurityResult<bool> {
            Ok(true)
        }

        async fn add_trusted_peer(&self, _peer_id: SecurityPeerId, _nickname: String) -> SecurityResult<()> {
            Ok(())
        }
    }

    async fn create_test_system() -> (FileTransferSystem, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let security = Arc::new(MockSecurity);
        
        let system = FileTransferSystem::new(
            security,
            temp_dir.path().to_path_buf(),
        );
        
        system.initialize().await.unwrap();
        
        (system, temp_dir)
    }

    #[tokio::test]
    async fn test_file_transfer_system_creation() {
        let (system, _temp_dir) = create_test_system().await;
        let transfers = system.get_active_transfers().await.unwrap();
        assert_eq!(transfers.len(), 0);
    }

    #[tokio::test]
    async fn test_start_transfer() {
        let (system, _temp_dir) = create_test_system().await;
        let manifest = TransferManifest::new("test-sender".to_string());
        let peer_id = "test-peer".to_string();

        let session = system.start_transfer(manifest, peer_id).await.unwrap();
        assert_eq!(session.state, TransferState::Pending);
    }

    #[tokio::test]
    async fn test_cancel_transfer() {
        let (system, _temp_dir) = create_test_system().await;
        let manifest = TransferManifest::new("test-sender".to_string());
        let peer_id = "test-peer".to_string();

        let session = system.start_transfer(manifest, peer_id).await.unwrap();
        let result = system.cancel_transfer(session.session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_bandwidth_limit() {
        let (system, _temp_dir) = create_test_system().await;
        let result = system.set_bandwidth_limit(Some(1_000_000)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_transfer_progress() {
        let (system, _temp_dir) = create_test_system().await;
        let manifest = TransferManifest::new("test-sender".to_string());
        let peer_id = "test-peer".to_string();

        let session = system.start_transfer(manifest, peer_id).await.unwrap();
        let progress = system.get_transfer_progress(session.session_id).await.unwrap();
        
        assert_eq!(progress.bytes_transferred, 0);
    }

    #[tokio::test]
    async fn test_get_transfer_stats() {
        let (system, _temp_dir) = create_test_system().await;
        let manifest = TransferManifest::new("test-sender".to_string());
        let peer_id = "test-peer".to_string();

        let session = system.start_transfer(manifest, peer_id.clone()).await.unwrap();
        let stats = system.get_transfer_stats(session.session_id).await.unwrap();
        
        assert_eq!(stats.session_id, session.session_id);
        assert_eq!(stats.peer_id, peer_id);
    }

    #[tokio::test]
    async fn test_pause_and_resume_transfer() {
        let (system, _temp_dir) = create_test_system().await;
        let manifest = TransferManifest::new("test-sender".to_string());
        let peer_id = "test-peer".to_string();

        let session = system.start_transfer(manifest, peer_id).await.unwrap();
        
        // Pause transfer
        system.pause_transfer(session.session_id).await.unwrap();
        let paused_session = system.session_manager.get_session(session.session_id).await.unwrap();
        assert_eq!(paused_session.state, TransferState::Paused);
        
        // Resume transfer
        system.resume_paused_transfer(session.session_id).await.unwrap();
        let resumed_session = system.session_manager.get_session(session.session_id).await.unwrap();
        assert_eq!(resumed_session.state, TransferState::Transferring);
    }
}
