// Camera/Media Streaming System
//
// This module provides real-time video streaming capabilities with adaptive quality,
// multi-viewer support, and cross-platform compatibility.

pub mod capture;
pub mod encode;
pub mod network;
pub mod viewer;
pub mod recording;
pub mod error;
pub mod types;
pub mod security_integration;
pub mod api;

pub use error::{StreamError, StreamResult};
pub use types::*;
pub use capture::screen::{
    ScreenCaptureOptimizer, RegionSelector, CursorCapture,
    ResolutionChangeDetector, CaptureConfigOptimizer,
};
pub use recording::{
    RecordingEngineImpl, StreamRecorder, StorageManager, RecordingMetadata,
    PermissionManager, RecordingPermission,
};
pub use security_integration::{
    StreamSecurityManager, PeerTrustInfo, SecureStreamWrapper,
    StreamAccessControl, AccessRequest, ViewerAccess,
};
pub use api::{
    Streaming, StreamingApi, StreamEvent, StreamEventHandler,
    StopReason, QualityChangeReason,
};

use async_trait::async_trait;
use uuid::Uuid;

/// Unified streaming interface for all video operations
/// 
/// This trait abstracts video codec and network protocol details behind simple APIs,
/// providing a consistent interface for camera streaming, screen sharing, and media playback.
/// 
/// Requirements: 10.1, 10.2
#[async_trait]
pub trait StreamManager: Send + Sync {
    /// Start a camera stream with the specified configuration
    async fn start_camera_stream(&self, config: StreamConfig) -> StreamResult<StreamSession>;
    
    /// Start a screen stream with the specified configuration
    async fn start_screen_stream(&self, config: ScreenConfig) -> StreamResult<StreamSession>;
    
    /// Stop an active stream
    async fn stop_stream(&self, session_id: SessionId) -> StreamResult<()>;
    
    /// Adjust the quality of an active stream
    async fn adjust_quality(&self, session_id: SessionId, quality: StreamQuality) -> StreamResult<()>;
    
    /// Get all active streaming sessions
    async fn get_active_streams(&self) -> StreamResult<Vec<StreamSession>>;
}

/// Capture engine interface for video and audio capture
/// 
/// Provides platform-agnostic access to camera hardware, screen capture,
/// and audio input devices.
/// 
/// Requirements: 1.1, 1.2
#[async_trait]
pub trait CaptureEngine: Send + Sync {
    /// List all available camera devices
    async fn list_cameras(&self) -> StreamResult<Vec<CameraDevice>>;
    
    /// Start capturing from a camera device
    async fn start_camera_capture(
        &self,
        device: CameraDevice,
        config: CaptureConfig,
    ) -> StreamResult<CaptureStream>;
    
    /// Start capturing screen content
    async fn start_screen_capture(
        &self,
        region: ScreenRegion,
        config: CaptureConfig,
    ) -> StreamResult<CaptureStream>;
    
    /// Stop an active capture stream
    async fn stop_capture(&self, stream: CaptureStream) -> StreamResult<()>;
    
    /// Get the capabilities of a camera device
    async fn get_capture_capabilities(&self, device: CameraDevice) -> StreamResult<CaptureCapabilities>;
}

/// Video codec interface for encoding and decoding
/// 
/// Handles video encoding and decoding with hardware acceleration support,
/// providing efficient compression for streaming.
/// 
/// Requirements: 1.2, 2.1, 9.1
#[async_trait]
pub trait VideoCodec: Send + Sync {
    /// Encode a video frame
    async fn encode_frame(&self, frame: VideoFrame, quality: EncodingQuality) -> StreamResult<EncodedFrame>;
    
    /// Decode an encoded frame
    async fn decode_frame(&self, data: &[u8]) -> StreamResult<VideoFrame>;
    
    /// Configure the encoder with specific settings
    async fn configure_encoder(&self, config: EncoderConfig) -> StreamResult<()>;
    
    /// Get encoder capabilities (hardware acceleration, supported formats, etc.)
    async fn get_encoder_capabilities(&self) -> StreamResult<EncoderCapabilities>;
    
    /// Enable hardware acceleration if available
    async fn enable_hardware_acceleration(&self) -> StreamResult<bool>;
}

/// Network streaming interface for video transmission
/// 
/// Manages network transmission of video streams with adaptive bitrate control
/// and support for multiple transport protocols.
/// 
/// Requirements: 1.3, 2.2, 4.1, 4.2
#[async_trait]
pub trait NetworkStreamer: Send + Sync {
    /// Start streaming to a peer
    async fn start_streaming(&self, peer_id: String, stream: VideoStream) -> StreamResult<StreamConnection>;
    
    /// Receive a stream from a peer
    async fn receive_stream(&self, peer_id: String) -> StreamResult<VideoStream>;
    
    /// Adjust the bitrate of an active stream
    async fn adjust_bitrate(&self, connection: StreamConnection, bitrate: u32) -> StreamResult<()>;
    
    /// Get statistics for a stream connection
    async fn get_stream_stats(&self, connection: StreamConnection) -> StreamResult<StreamStats>;
    
    /// Close a stream connection
    async fn close_stream(&self, connection: StreamConnection) -> StreamResult<()>;
}

/// Viewer management interface for multi-viewer broadcasting
/// 
/// Manages multiple viewers and broadcast scenarios, including viewer authentication,
/// connection management, and resource allocation.
/// 
/// Requirements: 6.1, 6.3, 8.3
#[async_trait]
pub trait ViewerManager: Send + Sync {
    /// Add a viewer to the broadcast
    async fn add_viewer(&self, peer_id: String, permissions: ViewerPermissions) -> StreamResult<ViewerId>;
    
    /// Remove a viewer from the broadcast
    async fn remove_viewer(&self, viewer_id: ViewerId) -> StreamResult<()>;
    
    /// Broadcast a stream to all connected viewers
    async fn broadcast_to_viewers(&self, stream: VideoStream) -> StreamResult<()>;
    
    /// Get the status of all viewers
    async fn get_viewer_status(&self) -> StreamResult<Vec<ViewerStatus>>;
    
    /// Approve a viewer request
    async fn approve_viewer_request(&self, peer_id: String) -> StreamResult<ViewerId>;
}

/// Recording engine interface for local stream recording
/// 
/// Handles local recording of video streams with support for multiple formats
/// and storage management.
/// 
/// Requirements: 5.1, 5.2
#[async_trait]
pub trait RecordingEngine: Send + Sync {
    /// Start recording a stream
    async fn start_recording(&self, stream: VideoStream, config: RecordingConfig) -> StreamResult<RecordingSession>;
    
    /// Stop recording and finalize the file
    async fn stop_recording(&self, session: RecordingSession) -> StreamResult<RecordingFile>;
    
    /// Pause an active recording
    async fn pause_recording(&self, session: RecordingSession) -> StreamResult<()>;
    
    /// Resume a paused recording
    async fn resume_recording(&self, session: RecordingSession) -> StreamResult<()>;
    
    /// Get the status of a recording session
    async fn get_recording_status(&self, session: RecordingSession) -> StreamResult<RecordingStatus>;
}

// Type aliases for convenience
pub type SessionId = Uuid;
pub type ViewerId = Uuid;
pub type PeerId = String;
