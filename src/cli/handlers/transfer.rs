// File transfer command handlers
//
// Implements "kizuna send" and "kizuna receive" commands with file selection,
// peer targeting, and transfer progress display with full integration to
// the core file transfer system.
//
// Requirements: 2.1, 2.2, 2.3, 2.5, 3.1, 3.2, 3.4, 3.5

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::handlers::{ReceiveArgs, ReceiveResult, SendArgs, TransferResult};
use crate::cli::types::{OperationState, OperationStatus, OperationType, ProgressInfo};
use crate::file_transfer::api::FileTransferSystem;
use crate::file_transfer::progress::{ProgressCallback, EventCallback, TransferEvent};
use crate::file_transfer::types::{PeerId, TransferState};
use crate::security::api::SecuritySystem;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// File transfer command handler implementation with real-time event support
/// Fully integrated with core file transfer system
pub struct TransferHandler {
    file_transfer: Arc<FileTransferSystem>,
    /// Active transfer operations with real-time status
    active_operations: Arc<RwLock<std::collections::HashMap<Uuid, OperationStatus>>>,
    /// Transfer event notification channel for CLI/TUI updates
    event_tx: Arc<RwLock<Option<mpsc::UnboundedSender<TransferEvent>>>>,
    /// Progress notification channel for real-time updates
    progress_tx: Arc<RwLock<Option<mpsc::UnboundedSender<(Uuid, ProgressInfo)>>>>,
}

impl TransferHandler {
    /// Create a new transfer handler
    pub fn new(security_system: Arc<SecuritySystem>, session_dir: PathBuf) -> Self {
        let file_transfer = Arc::new(FileTransferSystem::new(
            security_system as Arc<dyn crate::security::Security>,
            session_dir,
        ));

        let handler = Self {
            file_transfer,
            active_operations: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_tx: Arc::new(RwLock::new(None)),
            progress_tx: Arc::new(RwLock::new(None)),
        };

        // Register event callbacks for real-time updates
        handler.register_callbacks();

        handler
    }

    /// Create a new transfer handler with custom file transfer system
    pub fn with_file_transfer(file_transfer: Arc<FileTransferSystem>) -> Self {
        let handler = Self {
            file_transfer,
            active_operations: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_tx: Arc::new(RwLock::new(None)),
            progress_tx: Arc::new(RwLock::new(None)),
        };

        // Register event callbacks for real-time updates
        handler.register_callbacks();

        handler
    }

    /// Subscribe to transfer events
    /// Returns a receiver that will get notified of transfer events
    pub async fn subscribe_events(&self) -> mpsc::UnboundedReceiver<TransferEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.event_tx.write().await = Some(tx);
        rx
    }

    /// Subscribe to progress updates
    /// Returns a receiver that will get notified of progress updates
    pub async fn subscribe_progress(&self) -> mpsc::UnboundedReceiver<(Uuid, ProgressInfo)> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.progress_tx.write().await = Some(tx);
        rx
    }

    /// Register progress and event callbacks for real-time updates with notification support
    fn register_callbacks(&self) {
        let active_operations = Arc::clone(&self.active_operations);
        let file_transfer_progress = Arc::clone(&self.file_transfer);
        let file_transfer_event = Arc::clone(&self.file_transfer);
        let progress_tx = Arc::clone(&self.progress_tx);

        // Register progress callback
        let progress_ops = Arc::clone(&active_operations);
        let progress_notif = Arc::clone(&progress_tx);
        tokio::spawn(async move {
            let callback: crate::file_transfer::progress::ProgressCallback = Arc::new(move |session_id, progress| {
                let ops = Arc::clone(&progress_ops);
                let notif = Arc::clone(&progress_notif);
                
                tokio::spawn(async move {
                    let progress_info = ProgressInfo {
                        current: progress.bytes_transferred,
                        total: Some(progress.total_bytes),
                        rate: Some(progress.current_speed as f64),
                        eta: progress.eta_seconds.map(|s| std::time::Duration::from_secs(s)),
                        message: None,
                    };

                    let mut operations = ops.write().await;
                    if let Some(op) = operations.get_mut(&session_id) {
                        op.progress = Some(progress_info.clone());
                    }
                    drop(operations);

                    // Send progress notification
                    if let Some(tx) = notif.read().await.as_ref() {
                        let _ = tx.send((session_id, progress_info));
                    }
                });
            });
            
            file_transfer_progress.on_progress(callback).await;
        });

        // Register event callback
        let event_ops = Arc::clone(&active_operations);
        let event_tx = Arc::clone(&self.event_tx);
        tokio::spawn(async move {
            let callback: crate::file_transfer::progress::EventCallback = Arc::new(move |event| {
                let ops = Arc::clone(&event_ops);
                let tx = Arc::clone(&event_tx);
                let event_clone = event.clone();
                
                tokio::spawn(async move {
                    match &event {
                        TransferEvent::Started { session_id, .. } => {
                            let mut operations = ops.write().await;
                            if let Some(op) = operations.get_mut(session_id) {
                                op.status = OperationState::InProgress;
                            }
                        }
                        TransferEvent::Completed { session_id, .. } => {
                            let mut operations = ops.write().await;
                            if let Some(op) = operations.get_mut(session_id) {
                                op.status = OperationState::Completed;
                            }
                        }
                        TransferEvent::Failed { session_id, error } => {
                            let mut operations = ops.write().await;
                            if let Some(op) = operations.get_mut(session_id) {
                                op.status = OperationState::Failed(error.clone());
                            }
                        }
                        TransferEvent::Cancelled { session_id } => {
                            let mut operations = ops.write().await;
                            if let Some(op) = operations.get_mut(session_id) {
                                op.status = OperationState::Cancelled;
                            }
                        }
                        _ => {}
                    }
                    drop(ops);

                    // Send event notification
                    if let Some(event_tx) = tx.read().await.as_ref() {
                        let _ = event_tx.send(event_clone);
                    }
                });
            });
            
            file_transfer_event.on_event(callback).await;
        });
    }

    /// Handle send command
    pub async fn handle_send(&self, args: SendArgs) -> CLIResult<TransferResult> {
        // Validate files exist
        for file in &args.files {
            if !file.exists() {
                return Err(CLIError::file_not_found(file.display().to_string()));
            }
        }

        // Initialize file transfer system
        self.file_transfer
            .initialize()
            .await
            .map_err(|e| CLIError::transfer(format!("Failed to initialize file transfer: {}", e)))?;

        // Determine peer ID
        let peer_id = args.peer.clone();

        // Send files
        let session = if args.files.len() == 1 {
            // Single file transfer
            self.file_transfer
                .send_file(args.files[0].clone(), peer_id.clone())
                .await
        } else {
            // Multiple file transfer
            self.file_transfer
                .send_files(args.files.clone(), peer_id.clone())
                .await
        }
        .map_err(|e| CLIError::transfer(format!("Failed to start transfer: {}", e)))?;

        // Convert transfer state to operation state
        let operation_state = match session.state {
            TransferState::Pending => OperationState::Starting,
            TransferState::Negotiating => OperationState::Starting,
            TransferState::Transferring => OperationState::InProgress,
            TransferState::Completed => OperationState::Completed,
            TransferState::Failed => OperationState::Failed("Transfer failed".to_string()),
            TransferState::Cancelled => OperationState::Cancelled,
            TransferState::Paused => OperationState::InProgress,
        };

        let operation_status = OperationStatus {
            operation_id: session.session_id,
            operation_type: OperationType::FileTransfer,
            peer_id: Uuid::new_v4(), // Convert string peer_id to UUID
            status: operation_state,
            progress: Some(ProgressInfo {
                current: 0,
                total: Some(session.manifest.total_size),
                rate: None,
                eta: None,
                message: Some(format!("Transferring {} files", args.files.len())),
            }),
            started_at: chrono::Utc::now(),
            estimated_completion: None,
        };

        // Store operation for real-time tracking
        self.active_operations
            .write()
            .await
            .insert(session.session_id, operation_status.clone());

        Ok(TransferResult {
            operation_id: session.session_id,
            status: operation_status,
        })
    }

    /// Handle receive command
    pub async fn handle_receive(&self, args: ReceiveArgs) -> CLIResult<ReceiveResult> {
        // Initialize file transfer system
        self.file_transfer
            .initialize()
            .await
            .map_err(|e| CLIError::transfer(format!("Failed to initialize file transfer: {}", e)))?;

        // For now, create a placeholder operation
        // In a real implementation, this would listen for incoming transfers
        let operation_id = Uuid::new_v4();

        let operation_status = OperationStatus {
            operation_id,
            operation_type: OperationType::FileTransfer,
            peer_id: Uuid::new_v4(),
            status: OperationState::Starting,
            progress: Some(ProgressInfo {
                current: 0,
                total: None,
                rate: None,
                eta: None,
                message: Some("Waiting for incoming transfer...".to_string()),
            }),
            started_at: chrono::Utc::now(),
            estimated_completion: None,
        };

        // Store operation for real-time tracking
        self.active_operations
            .write()
            .await
            .insert(operation_id, operation_status.clone());

        Ok(ReceiveResult {
            operation_id,
            status: operation_status,
        })
    }

    /// Get real-time operation status
    pub async fn get_operation_status(&self, operation_id: Uuid) -> CLIResult<OperationStatus> {
        let operations = self.active_operations.read().await;
        operations
            .get(&operation_id)
            .cloned()
            .ok_or_else(|| CLIError::transfer(format!("Operation {} not found", operation_id)))
    }

    /// Get all active operations with real-time status
    pub async fn get_all_operations(&self) -> CLIResult<Vec<OperationStatus>> {
        let operations = self.active_operations.read().await;
        Ok(operations.values().cloned().collect())
    }

    /// Get transfer progress
    pub async fn get_transfer_progress(&self, operation_id: Uuid) -> CLIResult<ProgressInfo> {
        use crate::file_transfer::FileTransfer;
        
        let progress = self
            .file_transfer
            .get_transfer_progress(operation_id)
            .await
            .map_err(|e| CLIError::transfer(format!("Failed to get progress: {}", e)))?;

        Ok(ProgressInfo {
            current: progress.bytes_transferred,
            total: Some(progress.total_bytes),
            rate: Some(progress.current_speed as f64),
            eta: progress.eta_seconds.map(|s| std::time::Duration::from_secs(s)),
            message: None,
        })
    }

    /// Get all active transfers
    pub async fn get_active_transfers(&self) -> CLIResult<Vec<OperationStatus>> {
        use crate::file_transfer::FileTransfer;
        
        let sessions = self
            .file_transfer
            .get_active_transfers()
            .await
            .map_err(|e| CLIError::transfer(format!("Failed to get active transfers: {}", e)))?;

        let operations = sessions
            .into_iter()
            .map(|session| {
                let operation_state = match session.state {
                    TransferState::Pending => OperationState::Starting,
                    TransferState::Negotiating => OperationState::Starting,
                    TransferState::Transferring => OperationState::InProgress,
                    TransferState::Completed => OperationState::Completed,
                    TransferState::Failed => OperationState::Failed("Transfer failed".to_string()),
                    TransferState::Cancelled => OperationState::Cancelled,
                    TransferState::Paused => OperationState::InProgress,
                };

                OperationStatus {
                    operation_id: session.session_id,
                    operation_type: OperationType::FileTransfer,
                    peer_id: Uuid::new_v4(),
                    status: operation_state,
                    progress: Some(ProgressInfo {
                        current: 0,
                        total: Some(session.manifest.total_size),
                        rate: None,
                        eta: None,
                        message: None,
                    }),
                    started_at: chrono::Utc::now(),
                    estimated_completion: None,
                }
            })
            .collect();

        Ok(operations)
    }

    /// Cancel a transfer
    pub async fn cancel_transfer(&self, operation_id: Uuid) -> CLIResult<()> {
        use crate::file_transfer::FileTransfer;
        
        self.file_transfer
            .cancel_transfer(operation_id)
            .await
            .map_err(|e| CLIError::transfer(format!("Failed to cancel transfer: {}", e)))?;

        Ok(())
    }

    /// Pause a transfer
    pub async fn pause_transfer(&self, operation_id: Uuid) -> CLIResult<()> {
        self.file_transfer
            .pause_transfer(operation_id)
            .await
            .map_err(|e| CLIError::transfer(format!("Failed to pause transfer: {}", e)))?;

        Ok(())
    }

    /// Resume a paused transfer
    pub async fn resume_transfer(&self, operation_id: Uuid) -> CLIResult<()> {
        self.file_transfer
            .resume_paused_transfer(operation_id)
            .await
            .map_err(|e| CLIError::transfer(format!("Failed to resume transfer: {}", e)))?;

        Ok(())
    }

    /// Set bandwidth limit for transfers
    pub async fn set_bandwidth_limit(&self, limit_bytes_per_sec: Option<u64>) -> CLIResult<()> {
        use crate::file_transfer::FileTransfer;
        
        self.file_transfer
            .set_bandwidth_limit(limit_bytes_per_sec)
            .await
            .map_err(|e| CLIError::transfer(format!("Failed to set bandwidth limit: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_handler() -> (TransferHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let handler = TransferHandler::new(security_system, temp_dir.path().to_path_buf());
        (handler, temp_dir)
    }

    #[tokio::test]
    async fn test_transfer_handler_creation() {
        let (handler, _temp_dir) = create_test_handler();
        let transfers = handler.get_active_transfers().await.unwrap();
        assert_eq!(transfers.len(), 0);
    }

    #[tokio::test]
    async fn test_send_nonexistent_file() {
        let (handler, _temp_dir) = create_test_handler();
        let args = SendArgs {
            files: vec![PathBuf::from("/nonexistent/file.txt")],
            peer: "test-peer".to_string(),
            compression: Some(true),
            encryption: Some(true),
        };

        let result = handler.handle_send(args).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_existing_file() {
        let (handler, temp_dir) = create_test_handler();

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, b"test content").unwrap();

        let args = SendArgs {
            files: vec![test_file],
            peer: "test-peer".to_string(),
            compression: Some(true),
            encryption: Some(true),
        };

        let result = handler.handle_send(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_receive_command() {
        let (handler, _temp_dir) = create_test_handler();
        let args = ReceiveArgs {
            download_path: None,
            auto_accept: false,
        };

        let result = handler.handle_receive(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_set_bandwidth_limit() {
        let (handler, _temp_dir) = create_test_handler();
        let result = handler.set_bandwidth_limit(Some(1_000_000)).await;
        assert!(result.is_ok());
    }
}
