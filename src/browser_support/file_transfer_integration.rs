//! File Transfer Integration for Browser Support
//!
//! Integrates browser file transfer with the existing file transfer system,
//! enabling seamless file transfers between browser clients and native peers.

use async_trait::async_trait;
use std::sync::Arc;
use std::path::PathBuf;
use tokio::sync::RwLock;
use std::collections::HashMap;

use crate::browser_support::{BrowserResult, BrowserSupportError, BrowserSession};
use crate::file_transfer::{
    FileTransfer, FileTransferSystem, TransferManifest, TransferSession,
    TransferProgress, PeerId, SessionId, ResumeToken, FileEntry,
};
use crate::browser_support::webrtc::data_channel::DataChannelManager;

/// Browser file transfer integration
pub struct BrowserFileTransferIntegration {
    /// Core file transfer system
    file_transfer_system: Arc<FileTransferSystem>,
    /// Data channel manager for WebRTC communication
    data_channel_manager: Arc<RwLock<DataChannelManager>>,
    /// Active browser transfer sessions
    browser_sessions: Arc<RwLock<HashMap<SessionId, BrowserTransferSession>>>,
}

/// Browser-specific transfer session information
#[derive(Debug, Clone)]
pub struct BrowserTransferSession {
    /// Session ID
    pub session_id: SessionId,
    /// Browser session ID
    pub browser_session_id: String,
    /// Peer ID
    pub peer_id: PeerId,
    /// Transfer direction
    pub direction: TransferDirection,
    /// Transfer manifest
    pub manifest: TransferManifest,
    /// Current progress
    pub progress: TransferProgress,
}

/// Transfer direction
#[derive(Debug, Clone, PartialEq)]
pub enum TransferDirection {
    /// Upload from browser to peer
    Upload,
    /// Download from peer to browser
    Download,
}

impl BrowserFileTransferIntegration {
    /// Create a new browser file transfer integration
    pub fn new(
        file_transfer_system: Arc<FileTransferSystem>,
        data_channel_manager: Arc<RwLock<DataChannelManager>>,
    ) -> Self {
        Self {
            file_transfer_system,
            data_channel_manager,
            browser_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Start file upload from browser to peer
    pub async fn start_browser_upload(
        &self,
        browser_session: &BrowserSession,
        file_name: String,
        file_size: u64,
        mime_type: String,
        peer_id: PeerId,
    ) -> BrowserResult<SessionId> {
        // Create transfer manifest for browser upload
        let manifest = TransferManifest {
            transfer_id: uuid::Uuid::new_v4(),
            sender_id: browser_session.session_id.to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            total_size: file_size,
            file_count: 1,
            files: vec![FileEntry {
                path: PathBuf::from(&file_name),
                size: file_size,
                checksum: [0u8; 32], // Will be computed during transfer
                permissions: crate::file_transfer::FilePermissions::default(),
                modified_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                chunk_count: ((file_size + 65535) / 65536) as usize,
            }],
            directories: vec![],
            checksum: [0u8; 32],
        };

        // Start transfer through file transfer system
        let session = self.file_transfer_system
            .start_transfer(manifest.clone(), peer_id.clone())
            .await
            .map_err(|e| BrowserSupportError::integration("file_transfer", format!("Failed to start transfer: {}", e)))?;

        let session_id = session.session_id;

        // Create browser transfer session
        let browser_transfer = BrowserTransferSession {
            session_id,
            browser_session_id: browser_session.session_id.to_string(),
            peer_id,
            direction: TransferDirection::Upload,
            manifest,
            progress: TransferProgress {
                bytes_transferred: 0,
                total_bytes: file_size,
                files_completed: 0,
                total_files: 1,
                current_speed: 0,
                average_speed: 0,
                eta_seconds: None,
                last_update: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            },
        };

        // Store browser session
        {
            let mut sessions = self.browser_sessions.write().await;
            sessions.insert(session_id, browser_transfer);
        }

        Ok(session_id)
    }

    /// Start file download from peer to browser
    pub async fn start_browser_download(
        &self,
        browser_session: &BrowserSession,
        file_path: PathBuf,
        peer_id: PeerId,
    ) -> BrowserResult<SessionId> {
        // Get file metadata from peer
        // For now, create a basic manifest
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("download")
            .to_string();

        let manifest = TransferManifest {
            transfer_id: uuid::Uuid::new_v4(),
            sender_id: peer_id.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            total_size: 0,
            file_count: 1,
            files: vec![FileEntry {
                path: file_path.clone(),
                size: 0, // Will be updated when we get actual file info
                checksum: [0u8; 32],
                permissions: crate::file_transfer::FilePermissions::default(),
                modified_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                chunk_count: 0,
            }],
            directories: vec![],
            checksum: [0u8; 32],
        };

        // Start transfer through file transfer system
        let session = self.file_transfer_system
            .start_transfer(manifest.clone(), peer_id.clone())
            .await
            .map_err(|e| BrowserSupportError::integration("file_transfer", format!("Failed to start transfer: {}", e)))?;

        let session_id = session.session_id;

        // Create browser transfer session
        let browser_transfer = BrowserTransferSession {
            session_id,
            browser_session_id: browser_session.session_id.to_string(),
            peer_id,
            direction: TransferDirection::Download,
            manifest,
            progress: TransferProgress {
                bytes_transferred: 0,
                total_bytes: 0,
                files_completed: 0,
                total_files: 1,
                current_speed: 0,
                average_speed: 0,
                eta_seconds: None,
                last_update: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            },
        };

        // Store browser session
        {
            let mut sessions = self.browser_sessions.write().await;
            sessions.insert(session_id, browser_transfer);
        }

        Ok(session_id)
    }

    /// Cancel a browser transfer
    pub async fn cancel_browser_transfer(&self, session_id: SessionId) -> BrowserResult<()> {
        // Cancel through file transfer system
        self.file_transfer_system
            .cancel_transfer(session_id)
            .await
            .map_err(|e| BrowserSupportError::integration("file_transfer", format!("Failed to cancel transfer: {}", e)))?;

        // Remove browser session
        {
            let mut sessions = self.browser_sessions.write().await;
            sessions.remove(&session_id);
        }

        Ok(())
    }

    /// Get transfer progress for a browser session
    pub async fn get_browser_transfer_progress(&self, session_id: SessionId) -> BrowserResult<TransferProgress> {
        // Get progress from file transfer system
        let progress = self.file_transfer_system
            .get_transfer_progress(session_id)
            .await
            .map_err(|e| BrowserSupportError::integration("file_transfer", format!("Failed to get progress: {}", e)))?;

        // Update browser session progress
        {
            let mut sessions = self.browser_sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id) {
                session.progress = progress.clone();
            }
        }

        Ok(progress)
    }

    /// Get all active browser transfers
    pub async fn get_active_browser_transfers(&self) -> Vec<BrowserTransferSession> {
        let sessions = self.browser_sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Get browser transfers for a specific browser session
    pub async fn get_browser_session_transfers(&self, browser_session_id: &str) -> Vec<BrowserTransferSession> {
        let sessions = self.browser_sessions.read().await;
        sessions.values()
            .filter(|s| s.browser_session_id == browser_session_id)
            .cloned()
            .collect()
    }

    /// Resume a browser transfer
    pub async fn resume_browser_transfer(&self, resume_token: ResumeToken) -> BrowserResult<SessionId> {
        // Resume through file transfer system
        let session = self.file_transfer_system
            .resume_transfer(resume_token)
            .await
            .map_err(|e| BrowserSupportError::integration("file_transfer", format!("Failed to resume transfer: {}", e)))?;

        Ok(session.session_id)
    }

    /// Handle file chunk from browser
    pub async fn handle_browser_file_chunk(
        &self,
        session_id: SessionId,
        chunk_data: Vec<u8>,
        chunk_index: u64,
    ) -> BrowserResult<()> {
        // Forward chunk to file transfer system through data channel
        let data_channel_mgr = self.data_channel_manager.read().await;
        
        // Send chunk through appropriate data channel
        // This would integrate with the actual data channel implementation
        // For now, we'll just validate the session exists
        let sessions = self.browser_sessions.read().await;
        if !sessions.contains_key(&session_id) {
            return Err(BrowserSupportError::session_not_found(session_id.to_string()));
        }

        // In a real implementation, this would:
        // 1. Write chunk to temporary storage
        // 2. Verify chunk integrity
        // 3. Update transfer progress
        // 4. Notify file transfer system

        Ok(())
    }

    /// Send file chunk to browser
    pub async fn send_file_chunk_to_browser(
        &self,
        session_id: SessionId,
        chunk_data: Vec<u8>,
        chunk_index: u64,
    ) -> BrowserResult<()> {
        // Send chunk through data channel to browser
        let data_channel_mgr = self.data_channel_manager.read().await;
        
        // Verify session exists
        let sessions = self.browser_sessions.read().await;
        if !sessions.contains_key(&session_id) {
            return Err(BrowserSupportError::session_not_found(session_id.to_string()));
        }

        // In a real implementation, this would:
        // 1. Read chunk from file transfer system
        // 2. Send through WebRTC data channel
        // 3. Wait for acknowledgment
        // 4. Update progress

        Ok(())
    }

    /// Clean up completed browser transfers
    pub async fn cleanup_completed_transfers(&self) -> usize {
        let mut sessions = self.browser_sessions.write().await;
        let initial_count = sessions.len();

        // Get active transfers from file transfer system
        let active_transfers = match self.file_transfer_system.get_active_transfers().await {
            Ok(transfers) => transfers.into_iter().map(|t| t.session_id).collect::<Vec<_>>(),
            Err(_) => return 0,
        };

        // Remove sessions that are no longer active
        sessions.retain(|session_id, _| active_transfers.contains(session_id));

        initial_count - sessions.len()
    }
}

/// Trait for browser file transfer operations
#[async_trait]
pub trait BrowserFileTransfer: Send + Sync {
    /// Start upload from browser
    async fn upload_from_browser(
        &self,
        browser_session: &BrowserSession,
        file_name: String,
        file_size: u64,
        mime_type: String,
        peer_id: PeerId,
    ) -> BrowserResult<SessionId>;

    /// Start download to browser
    async fn download_to_browser(
        &self,
        browser_session: &BrowserSession,
        file_path: PathBuf,
        peer_id: PeerId,
    ) -> BrowserResult<SessionId>;

    /// Cancel transfer
    async fn cancel_transfer(&self, session_id: SessionId) -> BrowserResult<()>;

    /// Get transfer progress
    async fn get_progress(&self, session_id: SessionId) -> BrowserResult<TransferProgress>;
}

#[async_trait]
impl BrowserFileTransfer for BrowserFileTransferIntegration {
    async fn upload_from_browser(
        &self,
        browser_session: &BrowserSession,
        file_name: String,
        file_size: u64,
        mime_type: String,
        peer_id: PeerId,
    ) -> BrowserResult<SessionId> {
        self.start_browser_upload(browser_session, file_name, file_size, mime_type, peer_id).await
    }

    async fn download_to_browser(
        &self,
        browser_session: &BrowserSession,
        file_path: PathBuf,
        peer_id: PeerId,
    ) -> BrowserResult<SessionId> {
        self.start_browser_download(browser_session, file_path, peer_id).await
    }

    async fn cancel_transfer(&self, session_id: SessionId) -> BrowserResult<()> {
        self.cancel_browser_transfer(session_id).await
    }

    async fn get_progress(&self, session_id: SessionId) -> BrowserResult<TransferProgress> {
        self.get_browser_transfer_progress(session_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_transfer_direction() {
        assert_eq!(TransferDirection::Upload, TransferDirection::Upload);
        assert_eq!(TransferDirection::Download, TransferDirection::Download);
        assert_ne!(TransferDirection::Upload, TransferDirection::Download);
    }

    #[tokio::test]
    async fn test_browser_transfer_session_creation() {
        let session_id = uuid::Uuid::new_v4();
        let manifest = TransferManifest {
            transfer_id: uuid::Uuid::new_v4(),
            sender_id: "test-sender".to_string(),
            created_at: 0,
            total_size: 0,
            file_count: 0,
            files: vec![],
            directories: vec![],
            checksum: [0u8; 32],
        };

        let session = BrowserTransferSession {
            session_id,
            browser_session_id: "test-browser-session".to_string(),
            peer_id: "test-peer".to_string(),
            direction: TransferDirection::Upload,
            manifest: manifest.clone(),
            progress: TransferProgress {
                bytes_transferred: 0,
                total_bytes: 1000,
                files_completed: 0,
                total_files: 1,
                current_speed: 0,
                average_speed: 0,
                eta_seconds: None,
                last_update: 0,
            },
        };

        assert_eq!(session.session_id, session_id);
        assert_eq!(session.browser_session_id, "test-browser-session");
        assert_eq!(session.direction, TransferDirection::Upload);
    }
}
