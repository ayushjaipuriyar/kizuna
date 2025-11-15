// Progress Tracking Module
//
// Provides real-time progress tracking, speed monitoring, and ETA estimation

use crate::file_transfer::{
    error::Result,
    types::*,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Progress callback function type
pub type ProgressCallback = Arc<dyn Fn(SessionId, TransferProgress) + Send + Sync>;

/// Transfer event types
#[derive(Debug, Clone)]
pub enum TransferEvent {
    Started {
        session_id: SessionId,
        manifest: TransferManifest,
    },
    Progress {
        session_id: SessionId,
        progress: TransferProgress,
    },
    FileCompleted {
        session_id: SessionId,
        file_path: std::path::PathBuf,
    },
    Completed {
        session_id: SessionId,
        total_bytes: u64,
        duration: Duration,
    },
    Failed {
        session_id: SessionId,
        error: String,
    },
    Cancelled {
        session_id: SessionId,
    },
}

/// Event callback function type
pub type EventCallback = Arc<dyn Fn(TransferEvent) + Send + Sync>;

/// Progress tracker for monitoring transfer progress
pub struct ProgressTracker {
    /// Progress data for each session
    sessions: Arc<RwLock<HashMap<SessionId, SessionProgress>>>,
    /// Progress callbacks
    progress_callbacks: Arc<RwLock<Vec<ProgressCallback>>>,
    /// Event callbacks
    event_callbacks: Arc<RwLock<Vec<EventCallback>>>,
}

/// Session progress tracking data
struct SessionProgress {
    progress: TransferProgress,
    start_time: Instant,
    last_update: Instant,
    speed_samples: Vec<SpeedSample>,
}

/// Speed sample for calculating average speed
struct SpeedSample {
    timestamp: Instant,
    bytes_transferred: u64,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            progress_callbacks: Arc::new(RwLock::new(Vec::new())),
            event_callbacks: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a progress callback
    pub async fn register_progress_callback(&self, callback: ProgressCallback) {
        let mut callbacks = self.progress_callbacks.write().await;
        callbacks.push(callback);
    }

    /// Register an event callback
    pub async fn register_event_callback(&self, callback: EventCallback) {
        let mut callbacks = self.event_callbacks.write().await;
        callbacks.push(callback);
    }

    /// Start tracking a new session
    pub async fn start_session(&self, session_id: SessionId, manifest: TransferManifest) {
        let session_progress = SessionProgress {
            progress: TransferProgress {
                total_bytes: manifest.total_size,
                total_files: manifest.file_count,
                ..Default::default()
            },
            start_time: Instant::now(),
            last_update: Instant::now(),
            speed_samples: Vec::new(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session_progress);

        // Notify event callbacks
        self.notify_event(TransferEvent::Started {
            session_id,
            manifest,
        })
        .await;
    }

    /// Update progress for a session
    pub async fn update_progress(
        &self,
        session_id: SessionId,
        bytes_transferred: u64,
    ) -> Result<TransferProgress> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            let now = Instant::now();
            let elapsed = now.duration_since(session.last_update);

            // Update bytes transferred
            session.progress.bytes_transferred = bytes_transferred;

            // Calculate current speed
            if elapsed.as_secs() > 0 {
                let bytes_since_last = bytes_transferred.saturating_sub(
                    session.speed_samples.last().map(|s| s.bytes_transferred).unwrap_or(0)
                );
                session.progress.current_speed = (bytes_since_last as f64 / elapsed.as_secs_f64()) as u64;
            }

            // Add speed sample
            session.speed_samples.push(SpeedSample {
                timestamp: now,
                bytes_transferred,
            });

            // Keep only last 10 samples
            if session.speed_samples.len() > 10 {
                session.speed_samples.remove(0);
            }

            // Calculate average speed
            if let (Some(first), Some(last)) = (session.speed_samples.first(), session.speed_samples.last()) {
                let duration = last.timestamp.duration_since(first.timestamp);
                if duration.as_secs() > 0 {
                    let bytes_diff = last.bytes_transferred - first.bytes_transferred;
                    session.progress.average_speed = (bytes_diff as f64 / duration.as_secs_f64()) as u64;
                }
            }

            // Update ETA
            session.progress.update_eta();
            session.progress.last_update = current_timestamp();
            session.last_update = now;

            let progress = session.progress.clone();

            // Notify callbacks
            drop(sessions); // Release lock before callbacks
            self.notify_progress(session_id, progress.clone()).await;
            self.notify_event(TransferEvent::Progress {
                session_id,
                progress: progress.clone(),
            })
            .await;

            Ok(progress)
        } else {
            Err(crate::file_transfer::error::FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Mark a file as completed
    pub async fn file_completed(&self, session_id: SessionId, file_path: std::path::PathBuf) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.progress.files_completed += 1;
            
            // Notify event callbacks
            drop(sessions);
            self.notify_event(TransferEvent::FileCompleted {
                session_id,
                file_path,
            })
            .await;
            
            Ok(())
        } else {
            Err(crate::file_transfer::error::FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Mark session as completed
    pub async fn complete_session(&self, session_id: SessionId) -> Result<()> {
        let sessions = self.sessions.read().await;
        
        if let Some(session) = sessions.get(&session_id) {
            let duration = session.start_time.elapsed();
            let total_bytes = session.progress.total_bytes;
            
            drop(sessions);
            
            // Notify event callbacks
            self.notify_event(TransferEvent::Completed {
                session_id,
                total_bytes,
                duration,
            })
            .await;
            
            // Remove session after a delay to allow final progress queries
            tokio::spawn({
                let sessions = Arc::clone(&self.sessions);
                async move {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    let mut sessions = sessions.write().await;
                    sessions.remove(&session_id);
                }
            });
            
            Ok(())
        } else {
            Err(crate::file_transfer::error::FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Mark session as failed
    pub async fn fail_session(&self, session_id: SessionId, error: String) -> Result<()> {
        self.notify_event(TransferEvent::Failed {
            session_id,
            error,
        })
        .await;
        
        // Remove session
        let mut sessions = self.sessions.write().await;
        sessions.remove(&session_id);
        
        Ok(())
    }

    /// Mark session as cancelled
    pub async fn cancel_session(&self, session_id: SessionId) -> Result<()> {
        self.notify_event(TransferEvent::Cancelled { session_id })
            .await;
        
        // Remove session
        let mut sessions = self.sessions.write().await;
        sessions.remove(&session_id);
        
        Ok(())
    }

    /// Get current progress for a session
    pub async fn get_progress(&self, session_id: SessionId) -> Result<TransferProgress> {
        let sessions = self.sessions.read().await;
        
        sessions
            .get(&session_id)
            .map(|s| s.progress.clone())
            .ok_or_else(|| crate::file_transfer::error::FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
    }

    /// Get all active sessions
    pub async fn get_active_sessions(&self) -> Vec<SessionId> {
        let sessions = self.sessions.read().await;
        sessions.keys().copied().collect()
    }

    /// Notify progress callbacks
    async fn notify_progress(&self, session_id: SessionId, progress: TransferProgress) {
        let callbacks = self.progress_callbacks.read().await;
        for callback in callbacks.iter() {
            callback(session_id, progress.clone());
        }
    }

    /// Notify event callbacks
    pub async fn notify_event(&self, event: TransferEvent) {
        let callbacks = self.event_callbacks.read().await;
        for callback in callbacks.iter() {
            callback(event.clone());
        }
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_progress_tracker_creation() {
        let tracker = ProgressTracker::new();
        let sessions = tracker.get_active_sessions().await;
        assert_eq!(sessions.len(), 0);
    }

    #[tokio::test]
    async fn test_start_session() {
        let tracker = ProgressTracker::new();
        let session_id = uuid::Uuid::new_v4();
        let manifest = TransferManifest::new("test-sender".to_string());

        tracker.start_session(session_id, manifest).await;

        let sessions = tracker.get_active_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert!(sessions.contains(&session_id));
    }

    #[tokio::test]
    async fn test_update_progress() {
        let tracker = ProgressTracker::new();
        let session_id = uuid::Uuid::new_v4();
        let mut manifest = TransferManifest::new("test-sender".to_string());
        manifest.total_size = 1000;

        tracker.start_session(session_id, manifest).await;

        let progress = tracker.update_progress(session_id, 500).await.unwrap();
        assert_eq!(progress.bytes_transferred, 500);
        assert_eq!(progress.percentage(), 50.0);
    }

    #[tokio::test]
    async fn test_file_completed() {
        let tracker = ProgressTracker::new();
        let session_id = uuid::Uuid::new_v4();
        let manifest = TransferManifest::new("test-sender".to_string());

        tracker.start_session(session_id, manifest).await;

        tracker
            .file_completed(session_id, std::path::PathBuf::from("test.txt"))
            .await
            .unwrap();

        let progress = tracker.get_progress(session_id).await.unwrap();
        assert_eq!(progress.files_completed, 1);
    }

    #[tokio::test]
    async fn test_complete_session() {
        let tracker = ProgressTracker::new();
        let session_id = uuid::Uuid::new_v4();
        let manifest = TransferManifest::new("test-sender".to_string());

        tracker.start_session(session_id, manifest).await;
        tracker.complete_session(session_id).await.unwrap();

        // Session should still exist briefly
        let result = tracker.get_progress(session_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_progress_callback() {
        let tracker = ProgressTracker::new();
        let callback_count = Arc::new(AtomicUsize::new(0));

        let callback_count_clone = Arc::clone(&callback_count);
        let callback: ProgressCallback = Arc::new(move |_session_id, _progress| {
            callback_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        tracker.register_progress_callback(callback).await;

        let session_id = uuid::Uuid::new_v4();
        let mut manifest = TransferManifest::new("test-sender".to_string());
        manifest.total_size = 1000;

        tracker.start_session(session_id, manifest).await;
        tracker.update_progress(session_id, 500).await.unwrap();

        // Give callbacks time to execute
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert!(callback_count.load(Ordering::SeqCst) > 0);
    }

    #[tokio::test]
    async fn test_event_callback() {
        let tracker = ProgressTracker::new();
        let event_count = Arc::new(AtomicUsize::new(0));

        let event_count_clone = Arc::clone(&event_count);
        let callback: EventCallback = Arc::new(move |_event| {
            event_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        tracker.register_event_callback(callback).await;

        let session_id = uuid::Uuid::new_v4();
        let manifest = TransferManifest::new("test-sender".to_string());

        tracker.start_session(session_id, manifest).await;

        // Give callbacks time to execute
        tokio::time::sleep(Duration::from_millis(10)).await;

        assert!(event_count.load(Ordering::SeqCst) > 0);
    }

    #[tokio::test]
    async fn test_speed_calculation() {
        let tracker = ProgressTracker::new();
        let session_id = uuid::Uuid::new_v4();
        let mut manifest = TransferManifest::new("test-sender".to_string());
        manifest.total_size = 10000;

        tracker.start_session(session_id, manifest).await;

        // Simulate progress updates
        tracker.update_progress(session_id, 1000).await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        tracker.update_progress(session_id, 2000).await.unwrap();

        let progress = tracker.get_progress(session_id).await.unwrap();
        assert!(progress.current_speed > 0);
    }

    #[tokio::test]
    async fn test_eta_calculation() {
        let tracker = ProgressTracker::new();
        let session_id = uuid::Uuid::new_v4();
        let mut manifest = TransferManifest::new("test-sender".to_string());
        manifest.total_size = 10000;

        tracker.start_session(session_id, manifest).await;

        // Simulate progress with speed
        tracker.update_progress(session_id, 5000).await.unwrap();

        let progress = tracker.get_progress(session_id).await.unwrap();
        // ETA should be calculated if speed > 0
        if progress.current_speed > 0 {
            assert!(progress.eta_seconds.is_some());
        }
    }
}
