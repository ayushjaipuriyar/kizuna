// Incoming Transfer Management Module
//
// Handles incoming file transfer requests with user prompts and controls

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::*,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::sync::RwLock;

/// Incoming transfer request details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncomingTransferRequest {
    /// Request identifier
    pub request_id: TransferId,
    /// Sender peer ID
    pub sender_id: PeerId,
    /// Transfer manifest
    pub manifest: TransferManifest,
    /// Timestamp when request was received
    pub received_at: Timestamp,
    /// Request state
    pub state: IncomingRequestState,
}

/// State of an incoming transfer request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncomingRequestState {
    Pending,
    Accepted,
    Rejected,
    Expired,
}

/// User response to incoming transfer request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResponse {
    /// Whether to accept the transfer
    pub accept: bool,
    /// Download location (if accepted)
    pub download_location: Option<PathBuf>,
    /// Rejection reason (if rejected)
    pub rejection_reason: Option<String>,
}

/// Incoming transfer manager handles incoming transfer requests
pub struct IncomingTransferManager {
    /// Pending incoming requests
    pending_requests: Arc<RwLock<Vec<IncomingTransferRequest>>>,
    /// Request timeout in seconds (default: 5 minutes)
    request_timeout: u64,
}

impl IncomingTransferManager {
    /// Create a new incoming transfer manager
    pub fn new() -> Self {
        Self {
            pending_requests: Arc::new(RwLock::new(Vec::new())),
            request_timeout: 300, // 5 minutes
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(timeout_seconds: u64) -> Self {
        Self {
            pending_requests: Arc::new(RwLock::new(Vec::new())),
            request_timeout: timeout_seconds,
        }
    }

    /// Receive an incoming transfer request
    pub async fn receive_request(
        &self,
        sender_id: PeerId,
        manifest: TransferManifest,
    ) -> Result<IncomingTransferRequest> {
        let request = IncomingTransferRequest {
            request_id: manifest.transfer_id,
            sender_id,
            manifest,
            received_at: current_timestamp(),
            state: IncomingRequestState::Pending,
        };

        // Add to pending requests
        let mut pending = self.pending_requests.write().await;
        pending.push(request.clone());

        Ok(request)
    }

    /// Get all pending incoming requests
    pub async fn get_pending_requests(&self) -> Result<Vec<IncomingTransferRequest>> {
        let pending = self.pending_requests.read().await;
        Ok(pending
            .iter()
            .filter(|r| r.state == IncomingRequestState::Pending)
            .cloned()
            .collect())
    }

    /// Get a specific incoming request
    pub async fn get_request(&self, request_id: TransferId) -> Result<IncomingTransferRequest> {
        let pending = self.pending_requests.read().await;
        pending
            .iter()
            .find(|r| r.request_id == request_id)
            .cloned()
            .ok_or_else(|| FileTransferError::InternalError(
                format!("Incoming request not found: {}", request_id)
            ))
    }

    /// Accept an incoming transfer request
    pub async fn accept_request(
        &self,
        request_id: TransferId,
        download_location: PathBuf,
    ) -> Result<TransferManifest> {
        // Validate download location
        self.validate_download_location(&download_location).await?;

        // Check available disk space
        let request = self.get_request(request_id).await?;
        self.check_disk_space(&download_location, request.manifest.total_size).await?;

        // Update request state
        let mut pending = self.pending_requests.write().await;
        if let Some(req) = pending.iter_mut().find(|r| r.request_id == request_id) {
            req.state = IncomingRequestState::Accepted;
            Ok(req.manifest.clone())
        } else {
            Err(FileTransferError::InternalError(
                format!("Incoming request not found: {}", request_id)
            ))
        }
    }

    /// Reject an incoming transfer request
    pub async fn reject_request(
        &self,
        request_id: TransferId,
        reason: Option<String>,
    ) -> Result<()> {
        let mut pending = self.pending_requests.write().await;
        if let Some(req) = pending.iter_mut().find(|r| r.request_id == request_id) {
            req.state = IncomingRequestState::Rejected;
            Ok(())
        } else {
            Err(FileTransferError::InternalError(
                format!("Incoming request not found: {}", request_id)
            ))
        }
    }

    /// Defer an incoming transfer request (keep it pending)
    pub async fn defer_request(&self, request_id: TransferId) -> Result<()> {
        // Request remains in pending state
        let pending = self.pending_requests.read().await;
        if pending.iter().any(|r| r.request_id == request_id) {
            Ok(())
        } else {
            Err(FileTransferError::InternalError(
                format!("Incoming request not found: {}", request_id)
            ))
        }
    }

    /// Cleanup expired requests
    pub async fn cleanup_expired_requests(&self) -> Result<usize> {
        let current_time = current_timestamp();
        let mut pending = self.pending_requests.write().await;
        
        let mut removed_count = 0;
        pending.retain(|req| {
            let is_expired = req.state == IncomingRequestState::Pending
                && (current_time - req.received_at) > self.request_timeout;
            
            if is_expired {
                removed_count += 1;
                false
            } else {
                true
            }
        });

        Ok(removed_count)
    }

    /// Validate download location
    async fn validate_download_location(&self, location: &PathBuf) -> Result<()> {
        // Check if parent directory exists
        if let Some(parent) = location.parent() {
            if !parent.exists() {
                return Err(FileTransferError::IoError {
                    path: parent.to_path_buf(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Parent directory does not exist",
                    ),
                });
            }

            // Check if parent is writable
            let metadata = fs::metadata(parent).await.map_err(|e| {
                FileTransferError::IoError {
                    path: parent.to_path_buf(),
                    source: e,
                }
            })?;

            if metadata.permissions().readonly() {
                return Err(FileTransferError::IoError {
                    path: parent.to_path_buf(),
                    source: std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "Directory is read-only",
                    ),
                });
            }
        }

        Ok(())
    }

    /// Check available disk space
    async fn check_disk_space(&self, location: &PathBuf, required_bytes: u64) -> Result<()> {
        // Get the parent directory to check
        let check_path = if location.exists() {
            location.clone()
        } else if let Some(parent) = location.parent() {
            parent.to_path_buf()
        } else {
            return Err(FileTransferError::InternalError(
                "Cannot determine path for disk space check".to_string()
            ));
        };

        // Use statvfs on Unix or similar on other platforms
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = fs::metadata(&check_path).await.map_err(|e| {
                FileTransferError::IoError {
                    path: check_path.clone(),
                    source: e,
                }
            })?;

            // This is a simplified check - in production, you'd use statvfs
            // For now, we'll just check if the path is accessible
            if !check_path.exists() {
                return Err(FileTransferError::InsufficientDiskSpace {
                    required: required_bytes,
                    available: 0,
                });
            }
        }

        #[cfg(not(unix))]
        {
            // On non-Unix platforms, we'll do a basic check
            if !check_path.exists() {
                return Err(FileTransferError::InsufficientDiskSpace {
                    required: required_bytes,
                    available: 0,
                });
            }
        }

        Ok(())
    }

    /// Get transfer request details for display
    pub async fn get_request_details(&self, request_id: TransferId) -> Result<TransferRequestDetails> {
        let request = self.get_request(request_id).await?;
        
        Ok(TransferRequestDetails {
            request_id,
            sender_id: request.sender_id,
            file_count: request.manifest.file_count,
            total_size: request.manifest.total_size,
            file_names: request
                .manifest
                .files
                .iter()
                .map(|f| f.path.clone())
                .collect(),
            received_at: request.received_at,
        })
    }
}

impl Default for IncomingTransferManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Transfer request details for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequestDetails {
    pub request_id: TransferId,
    pub sender_id: PeerId,
    pub file_count: usize,
    pub total_size: u64,
    pub file_names: Vec<PathBuf>,
    pub received_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manifest() -> TransferManifest {
        let mut manifest = TransferManifest::new("test-sender".to_string());
        manifest.total_size = 1000;
        manifest.file_count = 1;
        manifest
    }

    #[tokio::test]
    async fn test_incoming_transfer_manager_creation() {
        let manager = IncomingTransferManager::new();
        let pending = manager.get_pending_requests().await.unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_receive_request() {
        let manager = IncomingTransferManager::new();
        let manifest = create_test_manifest();
        
        let request = manager
            .receive_request("test-peer".to_string(), manifest)
            .await
            .unwrap();
        
        assert_eq!(request.state, IncomingRequestState::Pending);
        assert_eq!(request.sender_id, "test-peer");
    }

    #[tokio::test]
    async fn test_get_pending_requests() {
        let manager = IncomingTransferManager::new();
        let manifest = create_test_manifest();
        
        manager
            .receive_request("test-peer".to_string(), manifest)
            .await
            .unwrap();
        
        let pending = manager.get_pending_requests().await.unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_accept_request() {
        let manager = IncomingTransferManager::new();
        let manifest = create_test_manifest();
        let temp_dir = TempDir::new().unwrap();
        
        let request = manager
            .receive_request("test-peer".to_string(), manifest)
            .await
            .unwrap();
        
        let download_location = temp_dir.path().join("download");
        let result = manager
            .accept_request(request.request_id, download_location)
            .await;
        
        assert!(result.is_ok());
        
        let updated_request = manager.get_request(request.request_id).await.unwrap();
        assert_eq!(updated_request.state, IncomingRequestState::Accepted);
    }

    #[tokio::test]
    async fn test_reject_request() {
        let manager = IncomingTransferManager::new();
        let manifest = create_test_manifest();
        
        let request = manager
            .receive_request("test-peer".to_string(), manifest)
            .await
            .unwrap();
        
        manager
            .reject_request(request.request_id, Some("Not interested".to_string()))
            .await
            .unwrap();
        
        let updated_request = manager.get_request(request.request_id).await.unwrap();
        assert_eq!(updated_request.state, IncomingRequestState::Rejected);
    }

    #[tokio::test]
    async fn test_defer_request() {
        let manager = IncomingTransferManager::new();
        let manifest = create_test_manifest();
        
        let request = manager
            .receive_request("test-peer".to_string(), manifest)
            .await
            .unwrap();
        
        manager.defer_request(request.request_id).await.unwrap();
        
        let updated_request = manager.get_request(request.request_id).await.unwrap();
        assert_eq!(updated_request.state, IncomingRequestState::Pending);
    }

    #[tokio::test]
    async fn test_get_request_details() {
        let manager = IncomingTransferManager::new();
        let manifest = create_test_manifest();
        
        let request = manager
            .receive_request("test-peer".to_string(), manifest)
            .await
            .unwrap();
        
        let details = manager
            .get_request_details(request.request_id)
            .await
            .unwrap();
        
        assert_eq!(details.sender_id, "test-peer");
        assert_eq!(details.file_count, 1);
        assert_eq!(details.total_size, 1000);
    }

    #[tokio::test]
    async fn test_validate_download_location() {
        let manager = IncomingTransferManager::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Valid location
        let valid_location = temp_dir.path().join("download");
        let result = manager.validate_download_location(&valid_location).await;
        assert!(result.is_ok());
        
        // Invalid location (non-existent parent)
        let invalid_location = PathBuf::from("/nonexistent/path/download");
        let result = manager.validate_download_location(&invalid_location).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_expired_requests() {
        let manager = IncomingTransferManager::with_timeout(0); // Immediate expiration
        let manifest = create_test_manifest();
        
        manager
            .receive_request("test-peer".to_string(), manifest)
            .await
            .unwrap();
        
        // Wait a bit to ensure expiration
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        let removed = manager.cleanup_expired_requests().await.unwrap();
        assert_eq!(removed, 1);
        
        let pending = manager.get_pending_requests().await.unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_requests() {
        let manager = IncomingTransferManager::new();
        
        for i in 0..3 {
            let manifest = create_test_manifest();
            manager
                .receive_request(format!("peer-{}", i), manifest)
                .await
                .unwrap();
        }
        
        let pending = manager.get_pending_requests().await.unwrap();
        assert_eq!(pending.len(), 3);
    }
}
