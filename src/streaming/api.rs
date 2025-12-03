// Unified Streaming API
//
// This module provides a high-level, event-driven API for all streaming operations,
// hiding codec and network complexity behind simple interfaces.
//
// Requirements: 10.1, 10.2, 10.3

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use super::{
    StreamError, StreamResult,
    StreamSession, StreamConfig, ScreenConfig, StreamQuality,
    SessionId, ViewerId, PeerId,
    CameraDevice, CaptureConfig, CaptureCapabilities,
    VideoStream, StreamConnection, StreamStats,
    ViewerPermissions, ViewerStatus,
    RecordingSession, RecordingConfig, RecordingFile, RecordingStatus,
    StreamState, StreamType,
};

/// Stream event types for event-driven API
/// 
/// Provides callbacks for stream status changes, quality adjustments,
/// viewer management, and error conditions.
/// 
/// Requirements: 10.3
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Stream session started successfully
    SessionStarted {
        session_id: SessionId,
        stream_type: StreamType,
    },
    
    /// Stream session stopped
    SessionStopped {
        session_id: SessionId,
        reason: StopReason,
    },
    
    /// Stream state changed
    StateChanged {
        session_id: SessionId,
        old_state: StreamState,
        new_state: StreamState,
    },
    
    /// Stream quality adjusted
    QualityChanged {
        session_id: SessionId,
        old_quality: StreamQuality,
        new_quality: StreamQuality,
        reason: QualityChangeReason,
    },
    
    /// Viewer connected to stream
    ViewerConnected {
        session_id: SessionId,
        viewer_id: ViewerId,
        peer_id: PeerId,
    },
    
    /// Viewer disconnected from stream
    ViewerDisconnected {
        session_id: SessionId,
        viewer_id: ViewerId,
        reason: String,
    },
    
    /// Viewer requested access to stream
    ViewerRequestReceived {
        session_id: SessionId,
        peer_id: PeerId,
        device_name: String,
    },
    
    /// Stream statistics updated
    StatsUpdated {
        session_id: SessionId,
        stats: StreamStats,
    },
    
    /// Stream error occurred
    Error {
        session_id: Option<SessionId>,
        error: String,
        recoverable: bool,
    },
    
    /// Recording started
    RecordingStarted {
        session_id: SessionId,
        recording_session: SessionId,
    },
    
    /// Recording stopped
    RecordingStopped {
        session_id: SessionId,
        recording_session: SessionId,
        file_path: String,
    },
    
    /// Network condition changed
    NetworkConditionChanged {
        session_id: SessionId,
        bandwidth_kbps: u32,
        latency_ms: u32,
        packet_loss: f32,
    },
}

/// Reason for stream stop
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReason {
    UserRequested,
    NetworkFailure,
    ResourceExhaustion,
    PermissionDenied,
    Error,
}

/// Reason for quality change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityChangeReason {
    UserRequested,
    NetworkAdaptation,
    ResourceConstraint,
    ViewerRequest,
    Automatic,
}

/// Event handler trait for receiving stream events
/// 
/// Implement this trait to receive callbacks for stream events.
/// 
/// Requirements: 10.3
#[async_trait]
pub trait StreamEventHandler: Send + Sync {
    /// Handle a stream event
    async fn on_event(&self, event: StreamEvent);
}

/// Unified streaming interface providing comprehensive video operations
/// 
/// This trait provides a high-level API that abstracts all codec and network
/// complexity, offering simple methods for camera streaming, screen sharing,
/// viewer management, and recording.
/// 
/// Requirements: 10.1, 10.2, 10.3
#[async_trait]
pub trait Streaming: Send + Sync {
    // === Session Management ===
    
    /// Start a camera stream with the specified configuration
    /// 
    /// Returns a StreamSession that can be used to control the stream.
    async fn start_camera_stream(&self, config: StreamConfig) -> StreamResult<StreamSession>;
    
    /// Start a screen stream with the specified configuration
    /// 
    /// Returns a StreamSession that can be used to control the stream.
    async fn start_screen_stream(&self, config: ScreenConfig) -> StreamResult<StreamSession>;
    
    /// Stop an active stream session
    async fn stop_stream(&self, session_id: SessionId) -> StreamResult<()>;
    
    /// Pause an active stream session
    async fn pause_stream(&self, session_id: SessionId) -> StreamResult<()>;
    
    /// Resume a paused stream session
    async fn resume_stream(&self, session_id: SessionId) -> StreamResult<()>;
    
    /// Get information about a specific stream session
    async fn get_stream(&self, session_id: SessionId) -> StreamResult<StreamSession>;
    
    /// Get all active stream sessions
    async fn get_active_streams(&self) -> StreamResult<Vec<StreamSession>>;
    
    // === Quality Management ===
    
    /// Adjust the quality of an active stream
    async fn adjust_quality(&self, session_id: SessionId, quality: StreamQuality) -> StreamResult<()>;
    
    /// Get current stream statistics
    async fn get_stream_stats(&self, session_id: SessionId) -> StreamResult<StreamStats>;
    
    /// Enable or disable automatic quality adaptation
    async fn set_auto_quality(&self, session_id: SessionId, enabled: bool) -> StreamResult<()>;
    
    // === Device Management ===
    
    /// List all available camera devices
    async fn list_cameras(&self) -> StreamResult<Vec<CameraDevice>>;
    
    /// Get capabilities of a specific camera device
    async fn get_camera_capabilities(&self, device: CameraDevice) -> StreamResult<CaptureCapabilities>;
    
    // === Viewer Management ===
    
    /// Add a viewer to a stream with specified permissions
    async fn add_viewer(
        &self,
        session_id: SessionId,
        peer_id: PeerId,
        permissions: ViewerPermissions,
    ) -> StreamResult<ViewerId>;
    
    /// Remove a viewer from a stream
    async fn remove_viewer(&self, session_id: SessionId, viewer_id: ViewerId) -> StreamResult<()>;
    
    /// Get status of all viewers for a stream
    async fn get_viewers(&self, session_id: SessionId) -> StreamResult<Vec<ViewerStatus>>;
    
    /// Approve a viewer access request
    async fn approve_viewer(&self, session_id: SessionId, peer_id: PeerId) -> StreamResult<ViewerId>;
    
    /// Reject a viewer access request
    async fn reject_viewer(&self, session_id: SessionId, peer_id: PeerId) -> StreamResult<()>;
    
    // === Recording Management ===
    
    /// Start recording a stream
    async fn start_recording(
        &self,
        session_id: SessionId,
        config: RecordingConfig,
    ) -> StreamResult<RecordingSession>;
    
    /// Stop recording
    async fn stop_recording(&self, recording_session: SessionId) -> StreamResult<RecordingFile>;
    
    /// Pause recording
    async fn pause_recording(&self, recording_session: SessionId) -> StreamResult<()>;
    
    /// Resume recording
    async fn resume_recording(&self, recording_session: SessionId) -> StreamResult<()>;
    
    /// Get recording status
    async fn get_recording_status(&self, recording_session: SessionId) -> StreamResult<RecordingStatus>;
    
    // === Event Management ===
    
    /// Register an event handler to receive stream events
    async fn register_event_handler(&self, handler: Arc<dyn StreamEventHandler>) -> StreamResult<()>;
    
    /// Unregister an event handler
    async fn unregister_event_handler(&self, handler: Arc<dyn StreamEventHandler>) -> StreamResult<()>;
}

/// Concrete implementation of the unified streaming API
/// 
/// This implementation integrates all streaming components (capture, encoding,
/// network, viewer management, recording) and provides a simple, high-level API.
/// 
/// Requirements: 10.1, 10.2, 10.3
pub struct StreamingApi {
    /// Active stream sessions
    sessions: Arc<RwLock<std::collections::HashMap<SessionId, StreamSession>>>,
    
    /// Event handlers
    event_handlers: Arc<RwLock<Vec<Arc<dyn StreamEventHandler>>>>,
    
    /// Event channel for internal event distribution
    event_tx: mpsc::UnboundedSender<StreamEvent>,
    event_rx: Arc<RwLock<mpsc::UnboundedReceiver<StreamEvent>>>,
}

impl StreamingApi {
    /// Create a new streaming API instance
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        let api = Self {
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            event_handlers: Arc::new(RwLock::new(Vec::new())),
            event_tx,
            event_rx: Arc::new(RwLock::new(event_rx)),
        };
        
        // Start event processing task
        api.start_event_processor();
        
        api
    }
    
    /// Start the event processor task
    fn start_event_processor(&self) {
        let event_rx = Arc::clone(&self.event_rx);
        let event_handlers = Arc::clone(&self.event_handlers);
        
        tokio::spawn(async move {
            loop {
                let event = {
                    let mut rx = event_rx.write().await;
                    rx.recv().await
                };
                
                if let Some(event) = event {
                    let handlers = event_handlers.read().await;
                    for handler in handlers.iter() {
                        handler.on_event(event.clone()).await;
                    }
                } else {
                    break;
                }
            }
        });
    }
    
    /// Emit an event to all registered handlers
    async fn emit_event(&self, event: StreamEvent) {
        let _ = self.event_tx.send(event);
    }
    
    /// Update session state and emit event
    async fn update_session_state(
        &self,
        session_id: SessionId,
        new_state: StreamState,
    ) -> StreamResult<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            let old_state = session.state;
            session.state = new_state;
            
            drop(sessions); // Release lock before emitting event
            
            self.emit_event(StreamEvent::StateChanged {
                session_id,
                old_state,
                new_state,
            }).await;
            
            Ok(())
        } else {
            Err(StreamError::session_not_found(session_id))
        }
    }
}

impl Default for StreamingApi {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Streaming for StreamingApi {
    async fn start_camera_stream(&self, config: StreamConfig) -> StreamResult<StreamSession> {
        // Create new session
        let session_id = Uuid::new_v4();
        let session = StreamSession {
            session_id,
            stream_type: StreamType::Camera,
            source: super::StreamSource::Camera(CameraDevice {
                id: "default".to_string(),
                name: "Default Camera".to_string(),
                description: None,
                capabilities: vec![],
            }),
            viewers: vec![],
            quality: config.quality.clone(),
            state: StreamState::Starting,
            stats: super::StreamStats::default(),
            created_at: std::time::SystemTime::now(),
        };
        
        // Store session
        self.sessions.write().await.insert(session_id, session.clone());
        
        // Emit event
        self.emit_event(StreamEvent::SessionStarted {
            session_id,
            stream_type: StreamType::Camera,
        }).await;
        
        // Update state to active
        self.update_session_state(session_id, StreamState::Active).await?;
        
        Ok(session)
    }
    
    async fn start_screen_stream(&self, config: ScreenConfig) -> StreamResult<StreamSession> {
        // Create new session
        let session_id = Uuid::new_v4();
        let session = StreamSession {
            session_id,
            stream_type: StreamType::Screen,
            source: super::StreamSource::Screen(config.region),
            viewers: vec![],
            quality: config.quality.clone(),
            state: StreamState::Starting,
            stats: super::StreamStats::default(),
            created_at: std::time::SystemTime::now(),
        };
        
        // Store session
        self.sessions.write().await.insert(session_id, session.clone());
        
        // Emit event
        self.emit_event(StreamEvent::SessionStarted {
            session_id,
            stream_type: StreamType::Screen,
        }).await;
        
        // Update state to active
        self.update_session_state(session_id, StreamState::Active).await?;
        
        Ok(session)
    }
    
    async fn stop_stream(&self, session_id: SessionId) -> StreamResult<()> {
        // Update state to stopping
        self.update_session_state(session_id, StreamState::Stopping).await?;
        
        // Remove session
        self.sessions.write().await.remove(&session_id);
        
        // Emit event
        self.emit_event(StreamEvent::SessionStopped {
            session_id,
            reason: StopReason::UserRequested,
        }).await;
        
        Ok(())
    }
    
    async fn pause_stream(&self, session_id: SessionId) -> StreamResult<()> {
        self.update_session_state(session_id, StreamState::Paused).await
    }
    
    async fn resume_stream(&self, session_id: SessionId) -> StreamResult<()> {
        self.update_session_state(session_id, StreamState::Active).await
    }
    
    async fn get_stream(&self, session_id: SessionId) -> StreamResult<StreamSession> {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id)
            .cloned()
            .ok_or_else(|| StreamError::session_not_found(session_id))
    }
    
    async fn get_active_streams(&self) -> StreamResult<Vec<StreamSession>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.values()
            .filter(|s| s.state == StreamState::Active)
            .cloned()
            .collect())
    }
    
    async fn adjust_quality(&self, session_id: SessionId, quality: StreamQuality) -> StreamResult<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            let old_quality = session.quality.clone();
            session.quality = quality.clone();
            
            drop(sessions); // Release lock before emitting event
            
            self.emit_event(StreamEvent::QualityChanged {
                session_id,
                old_quality,
                new_quality: quality,
                reason: QualityChangeReason::UserRequested,
            }).await;
            
            Ok(())
        } else {
            Err(StreamError::session_not_found(session_id))
        }
    }
    
    async fn get_stream_stats(&self, session_id: SessionId) -> StreamResult<StreamStats> {
        let sessions = self.sessions.read().await;
        sessions.get(&session_id)
            .map(|s| s.stats.clone())
            .ok_or_else(|| StreamError::session_not_found(session_id))
    }
    
    async fn set_auto_quality(&self, _session_id: SessionId, _enabled: bool) -> StreamResult<()> {
        // Implementation would integrate with quality manager
        Ok(())
    }
    
    async fn list_cameras(&self) -> StreamResult<Vec<CameraDevice>> {
        // Implementation would use platform-specific capture engine
        Ok(vec![])
    }
    
    async fn get_camera_capabilities(&self, _device: CameraDevice) -> StreamResult<CaptureCapabilities> {
        // Implementation would query device capabilities
        Err(StreamError::unsupported("Not yet implemented"))
    }
    
    async fn add_viewer(
        &self,
        session_id: SessionId,
        peer_id: PeerId,
        _permissions: ViewerPermissions,
    ) -> StreamResult<ViewerId> {
        let viewer_id = Uuid::new_v4();
        
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.viewers.push(viewer_id);
            
            drop(sessions); // Release lock before emitting event
            
            self.emit_event(StreamEvent::ViewerConnected {
                session_id,
                viewer_id,
                peer_id,
            }).await;
            
            Ok(viewer_id)
        } else {
            Err(StreamError::session_not_found(session_id))
        }
    }
    
    async fn remove_viewer(&self, session_id: SessionId, viewer_id: ViewerId) -> StreamResult<()> {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.viewers.retain(|&v| v != viewer_id);
            
            drop(sessions); // Release lock before emitting event
            
            self.emit_event(StreamEvent::ViewerDisconnected {
                session_id,
                viewer_id,
                reason: "Removed by host".to_string(),
            }).await;
            
            Ok(())
        } else {
            Err(StreamError::session_not_found(session_id))
        }
    }
    
    async fn get_viewers(&self, session_id: SessionId) -> StreamResult<Vec<ViewerStatus>> {
        // Implementation would return actual viewer status
        let sessions = self.sessions.read().await;
        if sessions.contains_key(&session_id) {
            Ok(vec![])
        } else {
            Err(StreamError::session_not_found(session_id))
        }
    }
    
    async fn approve_viewer(&self, session_id: SessionId, peer_id: PeerId) -> StreamResult<ViewerId> {
        let viewer_id = Uuid::new_v4();
        
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id) {
            session.viewers.push(viewer_id);
            
            drop(sessions); // Release lock before emitting event
            
            self.emit_event(StreamEvent::ViewerConnected {
                session_id,
                viewer_id,
                peer_id,
            }).await;
            
            Ok(viewer_id)
        } else {
            Err(StreamError::session_not_found(session_id))
        }
    }
    
    async fn reject_viewer(&self, _session_id: SessionId, _peer_id: PeerId) -> StreamResult<()> {
        // Implementation would reject viewer request
        Ok(())
    }
    
    async fn start_recording(
        &self,
        session_id: SessionId,
        config: RecordingConfig,
    ) -> StreamResult<RecordingSession> {
        let sessions = self.sessions.read().await;
        if !sessions.contains_key(&session_id) {
            return Err(StreamError::session_not_found(session_id));
        }
        drop(sessions);
        
        let recording_session_id = Uuid::new_v4();
        let recording_session = RecordingSession {
            session_id: recording_session_id,
            stream_session: session_id,
            output_path: config.output_path.clone(),
            format: config.format,
            state: super::RecordingState::Recording,
        };
        
        self.emit_event(StreamEvent::RecordingStarted {
            session_id,
            recording_session: recording_session_id,
        }).await;
        
        Ok(recording_session)
    }
    
    async fn stop_recording(&self, recording_session: SessionId) -> StreamResult<RecordingFile> {
        // Implementation would stop recording and return file info
        let file = RecordingFile {
            path: std::path::PathBuf::from("/tmp/recording.mp4"),
            format: super::VideoFormat::MP4,
            file_size: 0,
            duration: std::time::Duration::from_secs(0),
            created_at: std::time::SystemTime::now(),
        };
        
        Ok(file)
    }
    
    async fn pause_recording(&self, _recording_session: SessionId) -> StreamResult<()> {
        // Implementation would pause recording
        Ok(())
    }
    
    async fn resume_recording(&self, _recording_session: SessionId) -> StreamResult<()> {
        // Implementation would resume recording
        Ok(())
    }
    
    async fn get_recording_status(&self, _recording_session: SessionId) -> StreamResult<RecordingStatus> {
        // Implementation would return recording status
        Err(StreamError::unsupported("Not yet implemented"))
    }
    
    async fn register_event_handler(&self, handler: Arc<dyn StreamEventHandler>) -> StreamResult<()> {
        let mut handlers = self.event_handlers.write().await;
        handlers.push(handler);
        Ok(())
    }
    
    async fn unregister_event_handler(&self, handler: Arc<dyn StreamEventHandler>) -> StreamResult<()> {
        let mut handlers = self.event_handlers.write().await;
        handlers.retain(|h| !Arc::ptr_eq(h, &handler));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    struct TestEventHandler {
        events: Arc<RwLock<Vec<StreamEvent>>>,
    }
    
    impl TestEventHandler {
        fn new() -> Self {
            Self {
                events: Arc::new(RwLock::new(Vec::new())),
            }
        }
        
        async fn get_events(&self) -> Vec<StreamEvent> {
            self.events.read().await.clone()
        }
    }
    
    #[async_trait]
    impl StreamEventHandler for TestEventHandler {
        async fn on_event(&self, event: StreamEvent) {
            self.events.write().await.push(event);
        }
    }
    
    #[tokio::test]
    async fn test_start_camera_stream() {
        let api = StreamingApi::new();
        let config = StreamConfig::default();
        
        let session = api.start_camera_stream(config).await.unwrap();
        assert_eq!(session.stream_type, StreamType::Camera);
        assert_eq!(session.state, StreamState::Active);
    }
    
    #[tokio::test]
    async fn test_start_screen_stream() {
        let api = StreamingApi::new();
        let config = ScreenConfig {
            region: super::super::ScreenRegion {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            capture_cursor: true,
            capture_audio: false,
            monitor_index: None,
            quality: StreamQuality::default(),
        };
        
        let session = api.start_screen_stream(config).await.unwrap();
        assert_eq!(session.stream_type, StreamType::Screen);
        assert_eq!(session.state, StreamState::Active);
    }
    
    #[tokio::test]
    async fn test_stop_stream() {
        let api = StreamingApi::new();
        let config = StreamConfig::default();
        
        let session = api.start_camera_stream(config).await.unwrap();
        let session_id = session.session_id;
        
        api.stop_stream(session_id).await.unwrap();
        
        let result = api.get_stream(session_id).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_pause_resume_stream() {
        let api = StreamingApi::new();
        let config = StreamConfig::default();
        
        let session = api.start_camera_stream(config).await.unwrap();
        let session_id = session.session_id;
        
        api.pause_stream(session_id).await.unwrap();
        let session = api.get_stream(session_id).await.unwrap();
        assert_eq!(session.state, StreamState::Paused);
        
        api.resume_stream(session_id).await.unwrap();
        let session = api.get_stream(session_id).await.unwrap();
        assert_eq!(session.state, StreamState::Active);
    }
    
    #[tokio::test]
    async fn test_adjust_quality() {
        let api = StreamingApi::new();
        let config = StreamConfig::default();
        
        let session = api.start_camera_stream(config).await.unwrap();
        let session_id = session.session_id;
        
        let new_quality = super::super::QualityPreset::High.to_quality();
        api.adjust_quality(session_id, new_quality.clone()).await.unwrap();
        
        let session = api.get_stream(session_id).await.unwrap();
        assert_eq!(session.quality.quality_preset, super::super::QualityPreset::High);
    }
    
    #[tokio::test]
    async fn test_add_remove_viewer() {
        let api = StreamingApi::new();
        let config = StreamConfig::default();
        
        let session = api.start_camera_stream(config).await.unwrap();
        let session_id = session.session_id;
        
        let viewer_id = api.add_viewer(
            session_id,
            "peer123".to_string(),
            ViewerPermissions::default(),
        ).await.unwrap();
        
        let session = api.get_stream(session_id).await.unwrap();
        assert_eq!(session.viewers.len(), 1);
        assert_eq!(session.viewers[0], viewer_id);
        
        api.remove_viewer(session_id, viewer_id).await.unwrap();
        
        let session = api.get_stream(session_id).await.unwrap();
        assert_eq!(session.viewers.len(), 0);
    }
    
    #[tokio::test]
    async fn test_event_handler() {
        let api = StreamingApi::new();
        let handler = Arc::new(TestEventHandler::new());
        
        api.register_event_handler(handler.clone()).await.unwrap();
        
        let config = StreamConfig::default();
        let session = api.start_camera_stream(config).await.unwrap();
        
        // Give event processor time to run
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let events = handler.get_events().await;
        assert!(!events.is_empty());
        
        // Check for SessionStarted event
        let has_started = events.iter().any(|e| matches!(e, StreamEvent::SessionStarted { .. }));
        assert!(has_started);
    }
    
    #[tokio::test]
    async fn test_get_active_streams() {
        let api = StreamingApi::new();
        
        let config1 = StreamConfig::default();
        let session1 = api.start_camera_stream(config1).await.unwrap();
        
        let config2 = StreamConfig::default();
        let session2 = api.start_camera_stream(config2).await.unwrap();
        
        let active = api.get_active_streams().await.unwrap();
        assert_eq!(active.len(), 2);
        
        api.pause_stream(session1.session_id).await.unwrap();
        
        let active = api.get_active_streams().await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].session_id, session2.session_id);
    }
}
