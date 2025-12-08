// Transfer Notification Module
//
// Provides comprehensive transfer status reporting and notifications for UI integration

use crate::file_transfer::{
    error::Result,
    types::*,
    progress::TransferEvent,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Transfer notification types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransferNotification {
    /// Transfer has started
    TransferStarted {
        session_id: SessionId,
        peer_id: PeerId,
        file_count: usize,
        total_size: u64,
    },
    /// Transfer progress update
    TransferProgress {
        session_id: SessionId,
        progress: TransferProgress,
    },
    /// Individual file completed
    FileCompleted {
        session_id: SessionId,
        file_path: std::path::PathBuf,
        file_size: u64,
    },
    /// Transfer completed successfully
    TransferCompleted {
        session_id: SessionId,
        total_bytes: u64,
        duration: Duration,
        average_speed: u64,
    },
    /// Transfer failed with error
    TransferFailed {
        session_id: SessionId,
        error: String,
        bytes_transferred: u64,
    },
    /// Transfer was cancelled
    TransferCancelled {
        session_id: SessionId,
        bytes_transferred: u64,
    },
    /// Transfer was paused
    TransferPaused {
        session_id: SessionId,
        bytes_transferred: u64,
    },
    /// Transfer was resumed
    TransferResumed {
        session_id: SessionId,
        bytes_remaining: u64,
    },
}

/// Notification callback function type
pub type NotificationCallback = Arc<dyn Fn(TransferNotification) + Send + Sync>;

/// Notification manager handles transfer notifications
pub struct NotificationManager {
    /// Registered notification callbacks
    callbacks: Arc<RwLock<Vec<NotificationCallback>>>,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new() -> Self {
        Self {
            callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a notification callback
    pub async fn register_callback(&self, callback: NotificationCallback) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(callback);
    }

    /// Send a notification to all registered callbacks
    pub async fn notify(&self, notification: TransferNotification) {
        let callbacks = self.callbacks.read().await;
        for callback in callbacks.iter() {
            callback(notification.clone());
        }
    }

    /// Convert transfer event to notification and send
    pub async fn notify_from_event(&self, event: TransferEvent) {
        let notification = match event {
            TransferEvent::Started { session_id, manifest } => {
                TransferNotification::TransferStarted {
                    session_id,
                    peer_id: manifest.sender_id,
                    file_count: manifest.file_count,
                    total_size: manifest.total_size,
                }
            }
            TransferEvent::Progress { session_id, progress } => {
                TransferNotification::TransferProgress {
                    session_id,
                    progress,
                }
            }
            TransferEvent::FileCompleted { session_id, file_path } => {
                TransferNotification::FileCompleted {
                    session_id,
                    file_path,
                    file_size: 0, // TODO: Get actual file size from manifest
                }
            }
            TransferEvent::Completed { session_id, total_bytes, duration } => {
                let average_speed = if duration.as_secs() > 0 {
                    total_bytes / duration.as_secs()
                } else {
                    0
                };
                TransferNotification::TransferCompleted {
                    session_id,
                    total_bytes,
                    duration,
                    average_speed,
                }
            }
            TransferEvent::Failed { session_id, error } => {
                TransferNotification::TransferFailed {
                    session_id,
                    error,
                    bytes_transferred: 0, // TODO: Get actual bytes transferred
                }
            }
            TransferEvent::Cancelled { session_id } => {
                TransferNotification::TransferCancelled {
                    session_id,
                    bytes_transferred: 0, // TODO: Get actual bytes transferred
                }
            }
        };

        self.notify(notification).await;
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Comprehensive transfer status for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStatus {
    /// Session identifier
    pub session_id: SessionId,
    /// Peer identifier
    pub peer_id: PeerId,
    /// Current transfer state
    pub state: TransferState,
    /// Transfer progress
    pub progress: TransferProgress,
    /// Transport protocol being used
    pub transport: TransportProtocol,
    /// Bandwidth limit (if any)
    pub bandwidth_limit: Option<u64>,
    /// Number of parallel streams
    pub parallel_streams: usize,
    /// File list with individual status
    pub files: Vec<FileStatus>,
    /// Transfer start time
    pub started_at: Timestamp,
    /// Transfer completion time (if completed)
    pub completed_at: Option<Timestamp>,
    /// Error message (if failed)
    pub error_message: Option<String>,
}

/// Individual file status within a transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileStatus {
    /// File path
    pub path: std::path::PathBuf,
    /// File size
    pub size: u64,
    /// Bytes transferred for this file
    pub bytes_transferred: u64,
    /// File transfer state
    pub state: FileTransferState,
    /// Checksum verification status
    pub checksum_verified: bool,
}

/// File-level transfer state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileTransferState {
    Pending,
    Transferring,
    Completed,
    Failed,
    Skipped,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_notification_manager_creation() {
        let manager = NotificationManager::new();
        // Should be able to create without errors
        assert!(true);
    }

    #[tokio::test]
    async fn test_register_callback() {
        let manager = NotificationManager::new();
        let callback_count = Arc::new(AtomicUsize::new(0));

        let callback_count_clone = Arc::clone(&callback_count);
        let callback: NotificationCallback = Arc::new(move |_notification| {
            callback_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        manager.register_callback(callback).await;

        // Send a notification
        let notification = TransferNotification::TransferStarted {
            session_id: uuid::Uuid::new_v4(),
            peer_id: "test-peer".to_string(),
            file_count: 1,
            total_size: 1000,
        };

        manager.notify(notification).await;

        // Give callback time to execute
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(callback_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_multiple_callbacks() {
        let manager = NotificationManager::new();
        let callback_count = Arc::new(AtomicUsize::new(0));

        // Register multiple callbacks
        for _ in 0..3 {
            let callback_count_clone = Arc::clone(&callback_count);
            let callback: NotificationCallback = Arc::new(move |_notification| {
                callback_count_clone.fetch_add(1, Ordering::SeqCst);
            });
            manager.register_callback(callback).await;
        }

        // Send a notification
        let notification = TransferNotification::TransferStarted {
            session_id: uuid::Uuid::new_v4(),
            peer_id: "test-peer".to_string(),
            file_count: 1,
            total_size: 1000,
        };

        manager.notify(notification).await;

        // Give callbacks time to execute
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(callback_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_notify_from_event() {
        let manager = NotificationManager::new();
        let notification_received = Arc::new(AtomicUsize::new(0));

        let notification_received_clone = Arc::clone(&notification_received);
        let callback: NotificationCallback = Arc::new(move |notification| {
            match notification {
                TransferNotification::TransferStarted { .. } => {
                    notification_received_clone.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
        });

        manager.register_callback(callback).await;

        // Create a transfer event
        let manifest = TransferManifest::new("test-sender".to_string());
        let event = TransferEvent::Started {
            session_id: uuid::Uuid::new_v4(),
            manifest,
        };

        manager.notify_from_event(event).await;

        // Give callback time to execute
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(notification_received.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_transfer_completed_notification() {
        let manager = NotificationManager::new();
        let completed_received = Arc::new(AtomicUsize::new(0));

        let completed_received_clone = Arc::clone(&completed_received);
        let callback: NotificationCallback = Arc::new(move |notification| {
            match notification {
                TransferNotification::TransferCompleted { .. } => {
                    completed_received_clone.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
        });

        manager.register_callback(callback).await;

        let notification = TransferNotification::TransferCompleted {
            session_id: uuid::Uuid::new_v4(),
            total_bytes: 1000,
            duration: Duration::from_secs(10),
            average_speed: 100,
        };

        manager.notify(notification).await;

        // Give callback time to execute
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(completed_received.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_transfer_failed_notification() {
        let manager = NotificationManager::new();
        let failed_received = Arc::new(AtomicUsize::new(0));

        let failed_received_clone = Arc::clone(&failed_received);
        let callback: NotificationCallback = Arc::new(move |notification| {
            match notification {
                TransferNotification::TransferFailed { .. } => {
                    failed_received_clone.fetch_add(1, Ordering::SeqCst);
                }
                _ => {}
            }
        });

        manager.register_callback(callback).await;

        let notification = TransferNotification::TransferFailed {
            session_id: uuid::Uuid::new_v4(),
            error: "Test error".to_string(),
            bytes_transferred: 500,
        };

        manager.notify(notification).await;

        // Give callback time to execute
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert_eq!(failed_received.load(Ordering::SeqCst), 1);
    }
}
