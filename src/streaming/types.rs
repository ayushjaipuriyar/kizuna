// Core streaming data structures and types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

use super::{SessionId, ViewerId, PeerId};

/// Stream session representing an active streaming operation
/// 
/// Requirements: 10.1, 10.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamSession {
    pub session_id: SessionId,
    pub stream_type: StreamType,
    pub source: StreamSource,
    pub viewers: Vec<ViewerId>,
    pub quality: StreamQuality,
    pub state: StreamState,
    pub stats: StreamStats,
    pub created_at: SystemTime,
}

/// Type of stream being transmitted
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamType {
    Camera,
    Screen,
    Audio,
    Combined,
}

/// Source of the stream content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamSource {
    Camera(CameraDevice),
    Screen(ScreenRegion),
    File(PathBuf),
}

/// Current state of a stream
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamState {
    Starting,
    Active,
    Paused,
    Stopping,
    Stopped,
    Error,
}

/// Stream quality configuration
/// 
/// Requirements: 7.1, 7.2
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamQuality {
    pub resolution: Resolution,
    pub framerate: u32,
    pub bitrate: u32,
    pub quality_preset: QualityPreset,
    pub hardware_acceleration: bool,
}

impl Default for StreamQuality {
    fn default() -> Self {
        Self {
            resolution: Resolution { width: 1280, height: 720 },
            framerate: 30,
            bitrate: 1_500_000,
            quality_preset: QualityPreset::Medium,
            hardware_acceleration: true,
        }
    }
}

/// Quality presets for easy configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityPreset {
    Low,    // 480p, 15fps, 500kbps
    Medium, // 720p, 30fps, 1.5Mbps
    High,   // 1080p, 30fps, 3Mbps
    Ultra,  // 1080p, 60fps, 6Mbps
    Custom,
}

impl QualityPreset {
    /// Get the default quality settings for this preset
    pub fn to_quality(&self) -> StreamQuality {
        match self {
            QualityPreset::Low => StreamQuality {
                resolution: Resolution { width: 854, height: 480 },
                framerate: 15,
                bitrate: 500_000,
                quality_preset: *self,
                hardware_acceleration: true,
            },
            QualityPreset::Medium => StreamQuality {
                resolution: Resolution { width: 1280, height: 720 },
                framerate: 30,
                bitrate: 1_500_000,
                quality_preset: *self,
                hardware_acceleration: true,
            },
            QualityPreset::High => StreamQuality {
                resolution: Resolution { width: 1920, height: 1080 },
                framerate: 30,
                bitrate: 3_000_000,
                quality_preset: *self,
                hardware_acceleration: true,
            },
            QualityPreset::Ultra => StreamQuality {
                resolution: Resolution { width: 1920, height: 1080 },
                framerate: 60,
                bitrate: 6_000_000,
                quality_preset: *self,
                hardware_acceleration: true,
            },
            QualityPreset::Custom => StreamQuality::default(),
        }
    }
}

/// Video resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

/// Stream statistics for monitoring
/// 
/// Requirements: 7.3, 9.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub frames_encoded: u64,
    pub frames_decoded: u64,
    pub frames_dropped: u64,
    pub current_bitrate: u32,
    pub average_bitrate: u32,
    pub latency_ms: u32,
    pub jitter_ms: u32,
    pub packet_loss_rate: f32,
    pub last_updated: SystemTime,
}

impl Default for StreamStats {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            frames_encoded: 0,
            frames_decoded: 0,
            frames_dropped: 0,
            current_bitrate: 0,
            average_bitrate: 0,
            latency_ms: 0,
            jitter_ms: 0,
            packet_loss_rate: 0.0,
            last_updated: SystemTime::now(),
        }
    }
}

/// Camera device information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDevice {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub capabilities: Vec<CameraCapability>,
}

/// Camera capability (resolution, framerate, format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraCapability {
    pub resolution: Resolution,
    pub framerate: u32,
    pub pixel_format: PixelFormat,
}

/// Pixel format for video frames
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    RGB24,
    RGBA32,
    YUV420,
    NV12,
    MJPEG,
}

/// Screen region for screen capture
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScreenRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Stream configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamConfig {
    pub quality: StreamQuality,
    pub enable_audio: bool,
    pub enable_recording: bool,
    pub max_viewers: u32,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            quality: StreamQuality::default(),
            enable_audio: false,
            enable_recording: false,
            max_viewers: 10,
        }
    }
}

/// Screen capture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenConfig {
    pub region: ScreenRegion,
    pub capture_cursor: bool,
    pub capture_audio: bool,
    pub monitor_index: Option<u32>,
    pub quality: StreamQuality,
}

/// Capture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub resolution: Resolution,
    pub framerate: u32,
    pub pixel_format: PixelFormat,
    pub buffer_count: u32,
    pub auto_exposure: bool,
    pub auto_focus: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            resolution: Resolution { width: 1280, height: 720 },
            framerate: 30,
            pixel_format: PixelFormat::YUV420,
            buffer_count: 4,
            auto_exposure: true,
            auto_focus: true,
        }
    }
}

/// Capture capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureCapabilities {
    pub supported_resolutions: Vec<Resolution>,
    pub supported_framerates: Vec<u32>,
    pub supported_formats: Vec<PixelFormat>,
    pub has_auto_exposure: bool,
    pub has_auto_focus: bool,
}

/// Capture stream handle
#[derive(Debug, Clone)]
pub struct CaptureStream {
    pub id: Uuid,
    pub device: String,
    pub config: CaptureConfig,
}

/// Video frame data
#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub timestamp: SystemTime,
}

/// Encoded video frame
#[derive(Debug, Clone)]
pub struct EncodedFrame {
    pub data: Vec<u8>,
    pub timestamp: SystemTime,
    pub is_keyframe: bool,
}

/// Encoding quality settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodingQuality {
    pub bitrate: u32,
    pub quality_factor: u32, // 0-100
    pub keyframe_interval: u32,
}

/// Encoder configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderConfig {
    pub codec: VideoCodecType,
    pub resolution: Resolution,
    pub framerate: u32,
    pub bitrate: u32,
    pub hardware_acceleration: bool,
}

/// Video codec type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodecType {
    H264,
    H265,
    VP8,
    VP9,
    AV1,
}

/// Encoder capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderCapabilities {
    pub supported_codecs: Vec<VideoCodecType>,
    pub hardware_acceleration_available: bool,
    pub max_resolution: Resolution,
    pub max_framerate: u32,
}

/// Video stream handle
#[derive(Debug, Clone)]
pub struct VideoStream {
    pub id: Uuid,
    pub source: StreamSource,
    pub quality: StreamQuality,
}

/// Stream connection handle
#[derive(Debug, Clone)]
pub struct StreamConnection {
    pub id: Uuid,
    pub peer_id: PeerId,
    pub stream_id: Uuid,
}

/// Viewer permissions
/// 
/// Requirements: 6.4, 8.3, 8.4
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerPermissions {
    pub can_view: bool,
    pub can_record: bool,
    pub can_control_quality: bool,
    pub max_quality: QualityPreset,
}

impl Default for ViewerPermissions {
    fn default() -> Self {
        Self {
            can_view: true,
            can_record: false,
            can_control_quality: false,
            max_quality: QualityPreset::Medium,
        }
    }
}

/// Viewer status information
/// 
/// Requirements: 6.3, 8.5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewerStatus {
    pub viewer_id: ViewerId,
    pub peer_id: PeerId,
    pub device_name: String,
    pub connection_quality: ConnectionQuality,
    pub permissions: ViewerPermissions,
    pub connected_at: SystemTime,
    pub bytes_sent: u64,
    pub current_quality: StreamQuality,
}

/// Connection quality indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionQuality {
    Excellent,
    Good,
    Fair,
    Poor,
    Disconnected,
}

/// Recording session handle
#[derive(Debug, Clone)]
pub struct RecordingSession {
    pub session_id: SessionId,
    pub stream_session: SessionId,
    pub output_path: PathBuf,
    pub format: VideoFormat,
    pub state: RecordingState,
}

/// Recording state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordingState {
    Recording,
    Paused,
    Stopped,
    Error,
}

/// Video file format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFormat {
    MP4,
    WebM,
    AVI,
    MOV,
}

/// Recording configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingConfig {
    pub output_path: PathBuf,
    pub format: VideoFormat,
    pub quality: StreamQuality,
    pub max_file_size: Option<u64>,
    pub max_duration: Option<Duration>,
}

/// Recording file information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingFile {
    pub path: PathBuf,
    pub format: VideoFormat,
    pub file_size: u64,
    pub duration: Duration,
    pub created_at: SystemTime,
}

/// Recording status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingStatus {
    pub session_id: SessionId,
    pub state: RecordingState,
    pub file_size: u64,
    pub duration: Duration,
    pub frames_recorded: u64,
}
