// Stream recording functionality
//
// Provides local recording of video streams with support for multiple formats,
// configurable quality settings, and recording controls.
//
// Requirements: 5.1, 5.2, 5.4

pub mod recorder;
pub mod storage;
pub mod permissions;

pub use recorder::{StreamRecorder, RecorderImpl};
pub use storage::{StorageManager, RecordingMetadata};
pub use permissions::{PermissionManager, RecordingPermission};

use crate::streaming::{
    StreamResult, StreamError,
    RecordingSession, RecordingConfig, RecordingFile, RecordingStatus,
    VideoStream, RecordingState, SessionId,
};
use async_trait::async_trait;

/// Recording engine implementation
/// 
/// Handles local recording of video streams with support for MP4 and WebM formats,
/// recording controls (start, stop, pause, resume), and configurable quality settings.
/// 
/// Requirements: 5.1, 5.2, 5.4
pub struct RecordingEngineImpl {
    recorder: StreamRecorder,
    storage: StorageManager,
    permissions: PermissionManager,
}

impl RecordingEngineImpl {
    /// Create a new recording engine
    pub fn new(storage_path: std::path::PathBuf) -> StreamResult<Self> {
        Ok(Self {
            recorder: StreamRecorder::new()?,
            storage: StorageManager::new(storage_path)?,
            permissions: PermissionManager::new(),
        })
    }
    
    /// Validate recording configuration
    fn validate_config(&self, config: &RecordingConfig) -> StreamResult<()> {
        // Check output path is valid
        if let Some(parent) = config.output_path.parent() {
            if !parent.exists() {
                return Err(StreamError::configuration(
                    format!("Output directory does not exist: {:?}", parent)
                ));
            }
        }
        
        // Check format is supported
        match config.format {
            crate::streaming::VideoFormat::MP4 | crate::streaming::VideoFormat::WebM => Ok(()),
            _ => Err(StreamError::unsupported(
                format!("Recording format {:?} not supported", config.format)
            )),
        }
    }
}

#[async_trait]
impl crate::streaming::RecordingEngine for RecordingEngineImpl {
    async fn start_recording(
        &self,
        stream: VideoStream,
        config: RecordingConfig,
    ) -> StreamResult<RecordingSession> {
        // Validate configuration
        self.validate_config(&config)?;
        
        // Check storage availability
        self.storage.check_space_available(&config).await?;
        
        // Start recording
        let session = self.recorder.start_recording(stream, config).await?;
        
        // Register with storage manager
        self.storage.register_recording(&session).await?;
        
        Ok(session)
    }
    
    async fn stop_recording(&self, session: RecordingSession) -> StreamResult<RecordingFile> {
        // Stop the recording
        let file = self.recorder.stop_recording(session.clone()).await?;
        
        // Update storage metadata
        self.storage.finalize_recording(&session, &file).await?;
        
        Ok(file)
    }
    
    async fn pause_recording(&self, session: RecordingSession) -> StreamResult<()> {
        self.recorder.pause_recording(session).await
    }
    
    async fn resume_recording(&self, session: RecordingSession) -> StreamResult<()> {
        self.recorder.resume_recording(session).await
    }
    
    async fn get_recording_status(&self, session: RecordingSession) -> StreamResult<RecordingStatus> {
        self.recorder.get_status(session).await
    }
}
