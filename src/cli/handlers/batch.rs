// Batch operation support for CLI
//
// Implements multiple file selection and batch transfer operations,
// batch command execution with parallel processing, and batch operation
// progress tracking and error handling.
//
// Requirements: 2.4, 10.4

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::handlers::{SendArgs, TransferHandler, TransferResult};
use crate::cli::types::{OperationState, OperationStatus, OperationType, ProgressInfo};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Batch operation arguments
#[derive(Debug, Clone)]
pub struct BatchOperationArgs {
    pub files: Vec<PathBuf>,
    pub peers: Vec<String>,
    pub compression: Option<bool>,
    pub encryption: Option<bool>,
    pub parallel: bool,
    pub max_concurrent: Option<usize>,
}

/// Batch operation result
#[derive(Debug, Clone)]
pub struct BatchOperationResult {
    pub batch_id: Uuid,
    pub operations: Vec<BatchOperationItem>,
    pub total_operations: usize,
    pub successful: usize,
    pub failed: usize,
}

/// Individual batch operation item
#[derive(Debug, Clone)]
pub struct BatchOperationItem {
    pub operation_id: Uuid,
    pub file: PathBuf,
    pub peer: String,
    pub status: OperationState,
    pub error: Option<String>,
}

/// Batch operation handler
pub struct BatchOperationHandler {
    transfer_handler: Arc<TransferHandler>,
    active_batches: Arc<Mutex<Vec<BatchOperationStatus>>>,
}

/// Batch operation status tracking
#[derive(Debug, Clone)]
pub struct BatchOperationStatus {
    pub batch_id: Uuid,
    pub operations: Vec<BatchOperationItem>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl BatchOperationHandler {
    /// Create a new batch operation handler
    pub fn new(transfer_handler: Arc<TransferHandler>) -> Self {
        Self {
            transfer_handler,
            active_batches: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Execute batch file transfer operation
    pub async fn execute_batch_transfer(
        &self,
        args: BatchOperationArgs,
    ) -> CLIResult<BatchOperationResult> {
        let batch_id = Uuid::new_v4();
        let mut operations = Vec::new();

        // Create operation items for each file-peer combination
        for file in &args.files {
            for peer in &args.peers {
                operations.push(BatchOperationItem {
                    operation_id: Uuid::new_v4(),
                    file: file.clone(),
                    peer: peer.clone(),
                    status: OperationState::Starting,
                    error: None,
                });
            }
        }

        // Store batch status
        let batch_status = BatchOperationStatus {
            batch_id,
            operations: operations.clone(),
            started_at: chrono::Utc::now(),
            completed_at: None,
        };

        self.active_batches.lock().await.push(batch_status);

        // Execute operations
        let total_operations = operations.len();
        let results = if args.parallel {
            self.execute_parallel(operations, args.max_concurrent, &args)
                .await?
        } else {
            self.execute_sequential(operations, &args).await?
        };

        // Count successes and failures
        let successful = results
            .iter()
            .filter(|op| matches!(op.status, OperationState::Completed))
            .count();
        let failed = results
            .iter()
            .filter(|op| matches!(op.status, OperationState::Failed(_)))
            .count();

        // Update batch status
        let mut batches = self.active_batches.lock().await;
        if let Some(batch) = batches.iter_mut().find(|b| b.batch_id == batch_id) {
            batch.operations = results.clone();
            batch.completed_at = Some(chrono::Utc::now());
        }

        Ok(BatchOperationResult {
            batch_id,
            operations: results,
            total_operations,
            successful,
            failed,
        })
    }

    /// Execute operations sequentially
    async fn execute_sequential(
        &self,
        mut operations: Vec<BatchOperationItem>,
        args: &BatchOperationArgs,
    ) -> CLIResult<Vec<BatchOperationItem>> {
        for operation in &mut operations {
            let send_args = SendArgs {
                files: vec![operation.file.clone()],
                peer: operation.peer.clone(),
                compression: args.compression,
                encryption: args.encryption,
            };

            match self.transfer_handler.handle_send(send_args).await {
                Ok(_) => {
                    operation.status = OperationState::Completed;
                }
                Err(e) => {
                    operation.status = OperationState::Failed(e.to_string());
                    operation.error = Some(e.to_string());
                }
            }
        }

        Ok(operations)
    }

    /// Execute operations in parallel
    async fn execute_parallel(
        &self,
        mut operations: Vec<BatchOperationItem>,
        max_concurrent: Option<usize>,
        args: &BatchOperationArgs,
    ) -> CLIResult<Vec<BatchOperationItem>> {
        let max_concurrent = max_concurrent.unwrap_or(4);
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

        let mut tasks = Vec::new();

        for (idx, operation) in operations.iter().enumerate() {
            let transfer_handler = self.transfer_handler.clone();
            let semaphore = semaphore.clone();
            let send_args = SendArgs {
                files: vec![operation.file.clone()],
                peer: operation.peer.clone(),
                compression: args.compression,
                encryption: args.encryption,
            };

            let task = tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                let result = transfer_handler.handle_send(send_args).await;
                (idx, result)
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        for task in tasks {
            let (idx, result) = task.await.map_err(|e| {
                CLIError::batch_operation(format!("Task execution failed: {}", e))
            })?;

            match result {
                Ok(_) => {
                    operations[idx].status = OperationState::Completed;
                }
                Err(e) => {
                    operations[idx].status = OperationState::Failed(e.to_string());
                    operations[idx].error = Some(e.to_string());
                }
            }
        }

        Ok(operations)
    }

    /// Get batch operation status
    pub async fn get_batch_status(&self, batch_id: Uuid) -> CLIResult<BatchOperationStatus> {
        let batches = self.active_batches.lock().await;
        batches
            .iter()
            .find(|b| b.batch_id == batch_id)
            .cloned()
            .ok_or_else(|| CLIError::batch_operation(format!("Batch {} not found", batch_id)))
    }

    /// Get all active batch operations
    pub async fn get_active_batches(&self) -> CLIResult<Vec<BatchOperationStatus>> {
        Ok(self.active_batches.lock().await.clone())
    }

    /// Get batch progress summary
    pub async fn get_batch_progress(&self, batch_id: Uuid) -> CLIResult<BatchProgressInfo> {
        let batch = self.get_batch_status(batch_id).await?;

        let total = batch.operations.len();
        let completed = batch
            .operations
            .iter()
            .filter(|op| matches!(op.status, OperationState::Completed))
            .count();
        let failed = batch
            .operations
            .iter()
            .filter(|op| matches!(op.status, OperationState::Failed(_)))
            .count();
        let in_progress = batch
            .operations
            .iter()
            .filter(|op| {
                matches!(
                    op.status,
                    OperationState::Starting | OperationState::InProgress
                )
            })
            .count();

        Ok(BatchProgressInfo {
            batch_id,
            total_operations: total,
            completed_operations: completed,
            failed_operations: failed,
            in_progress_operations: in_progress,
            overall_progress: (completed as f64 / total as f64) * 100.0,
        })
    }

    /// Cancel a batch operation
    pub async fn cancel_batch(&self, batch_id: Uuid) -> CLIResult<()> {
        let mut batches = self.active_batches.lock().await;
        if let Some(batch) = batches.iter_mut().find(|b| b.batch_id == batch_id) {
            for operation in &mut batch.operations {
                if matches!(
                    operation.status,
                    OperationState::Starting | OperationState::InProgress
                ) {
                    operation.status = OperationState::Cancelled;
                }
            }
            batch.completed_at = Some(chrono::Utc::now());
            Ok(())
        } else {
            Err(CLIError::batch_operation(format!(
                "Batch {} not found",
                batch_id
            )))
        }
    }
}

/// Batch progress information
#[derive(Debug, Clone)]
pub struct BatchProgressInfo {
    pub batch_id: Uuid,
    pub total_operations: usize,
    pub completed_operations: usize,
    pub failed_operations: usize,
    pub in_progress_operations: usize,
    pub overall_progress: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_transfer::api::FileTransferSystem;
    use crate::security::SecuritySystem;
    use tempfile::TempDir;

    fn create_test_handler() -> (BatchOperationHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let transfer_handler = Arc::new(TransferHandler::new(
            security_system,
            temp_dir.path().to_path_buf(),
        ));
        let handler = BatchOperationHandler::new(transfer_handler);
        (handler, temp_dir)
    }

    #[tokio::test]
    async fn test_batch_handler_creation() {
        let (handler, _temp_dir) = create_test_handler();
        let batches = handler.get_active_batches().await.unwrap();
        assert_eq!(batches.len(), 0);
    }

    #[tokio::test]
    async fn test_batch_transfer_sequential() {
        let (handler, temp_dir) = create_test_handler();

        // Create test files
        let file1 = temp_dir.path().join("test1.txt");
        let file2 = temp_dir.path().join("test2.txt");
        std::fs::write(&file1, b"test content 1").unwrap();
        std::fs::write(&file2, b"test content 2").unwrap();

        let args = BatchOperationArgs {
            files: vec![file1, file2],
            peers: vec!["peer1".to_string()],
            compression: Some(true),
            encryption: Some(true),
            parallel: false,
            max_concurrent: None,
        };

        let result = handler.execute_batch_transfer(args).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.total_operations, 2);
    }

    #[tokio::test]
    async fn test_batch_transfer_parallel() {
        let (handler, temp_dir) = create_test_handler();

        // Create test files
        let file1 = temp_dir.path().join("test1.txt");
        let file2 = temp_dir.path().join("test2.txt");
        std::fs::write(&file1, b"test content 1").unwrap();
        std::fs::write(&file2, b"test content 2").unwrap();

        let args = BatchOperationArgs {
            files: vec![file1, file2],
            peers: vec!["peer1".to_string(), "peer2".to_string()],
            compression: Some(true),
            encryption: Some(true),
            parallel: true,
            max_concurrent: Some(2),
        };

        let result = handler.execute_batch_transfer(args).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.total_operations, 4); // 2 files × 2 peers
    }

    #[tokio::test]
    async fn test_get_batch_progress() {
        let (handler, temp_dir) = create_test_handler();

        let file1 = temp_dir.path().join("test1.txt");
        std::fs::write(&file1, b"test content").unwrap();

        let args = BatchOperationArgs {
            files: vec![file1],
            peers: vec!["peer1".to_string()],
            compression: Some(true),
            encryption: Some(true),
            parallel: false,
            max_concurrent: None,
        };

        let result = handler.execute_batch_transfer(args).await.unwrap();
        let progress = handler.get_batch_progress(result.batch_id).await;
        assert!(progress.is_ok());
    }

    #[tokio::test]
    async fn test_cancel_batch() {
        let (handler, temp_dir) = create_test_handler();

        let file1 = temp_dir.path().join("test1.txt");
        std::fs::write(&file1, b"test content").unwrap();

        let args = BatchOperationArgs {
            files: vec![file1],
            peers: vec!["peer1".to_string()],
            compression: Some(true),
            encryption: Some(true),
            parallel: false,
            max_concurrent: None,
        };

        let result = handler.execute_batch_transfer(args).await.unwrap();
        let cancel_result = handler.cancel_batch(result.batch_id).await;
        assert!(cancel_result.is_ok());
    }
}
