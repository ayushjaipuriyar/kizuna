//! Shared Clipboard System
//! 
//! Provides cross-device clipboard synchronization with privacy controls and platform abstraction.

pub mod monitor;
pub mod sync;
pub mod privacy;
pub mod history;
pub mod content;
pub mod platform;
pub mod notification;
pub mod error;
pub mod security_integration;
pub mod transport_integration;
pub mod api;

use async_trait::async_trait;
use std::time::SystemTime;
use uuid::Uuid;

pub use error::{ClipboardError, ClipboardResult};
pub use security_integration::{ClipboardSecurityIntegration, SecureClipboard};
pub use transport_integration::{ClipboardTransportIntegration, ClipboardTransport, ClipboardMessage};
pub use api::{ClipboardSystem, ClipboardSystemConfig, ClipboardSystemBuilder, ClipboardSystemStatus};

/// Unique identifier for clipboard events
pub type EventId = Uuid;

/// Unique identifier for history entries
pub type HistoryId = Uuid;

/// Unique identifier for devices
pub type DeviceId = String;

/// Unique identifier for peers
pub type PeerId = String;

/// Timestamp type for clipboard operations
pub type Timestamp = SystemTime;

/// Main clipboard content types supported by the system
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ClipboardContent {
    Text(TextContent),
    Image(ImageContent),
    Files(Vec<String>),
    Custom { mime_type: String, data: Vec<u8> },
}

/// Text clipboard content with formatting information
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextContent {
    pub text: String,
    pub encoding: TextEncoding,
    pub format: TextFormat,
    pub size: usize,
}

/// Image clipboard content with metadata
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImageContent {
    pub data: Vec<u8>,
    pub format: ImageFormat,
    pub width: u32,
    pub height: u32,
    pub compressed: bool,
}

/// Text encoding types
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TextEncoding {
    Utf8,
    Utf16,
    Ascii,
}

/// Text format types
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TextFormat {
    Plain,
    Rtf,
    Html,
    Markdown,
}

/// Image format types
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImageFormat {
    Png,
    Jpeg,
    Bmp,
    Gif,
    Tiff,
}

/// Clipboard change events
#[derive(Debug, Clone)]
pub struct ClipboardEvent {
    pub event_id: EventId,
    pub event_type: ClipboardEventType,
    pub content: Option<ClipboardContent>,
    pub source: ContentSource,
    pub timestamp: Timestamp,
}

/// Types of clipboard events
#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardEventType {
    ContentChanged,
    ContentReceived,
    SyncStarted,
    SyncCompleted,
    SyncFailed,
}

/// Source of clipboard content
#[derive(Debug, Clone, PartialEq)]
pub enum ContentSource {
    Local,
    Remote(PeerId),
    History(HistoryId),
}

/// Main clipboard trait for unified operations
#[async_trait]
pub trait Clipboard: Send + Sync {
    /// Get current clipboard content
    async fn get_content(&self) -> ClipboardResult<Option<ClipboardContent>>;
    
    /// Set clipboard content
    async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()>;
    
    /// Start monitoring clipboard changes
    async fn start_monitoring(&self) -> ClipboardResult<()>;
    
    /// Stop monitoring clipboard changes
    async fn stop_monitoring(&self) -> ClipboardResult<()>;
    
    /// Check if monitoring is active
    fn is_monitoring(&self) -> bool;
}

/// Device sync status information
#[derive(Debug, Clone)]
pub struct DeviceSyncStatus {
    pub device_id: DeviceId,
    pub device_name: String,
    pub sync_enabled: bool,
    pub last_sync: Option<Timestamp>,
    pub sync_count: u64,
    pub connection_status: ConnectionStatus,
}

/// Connection status with remote devices
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Connecting,
    Error(String),
}

/// Sync policy configuration
#[derive(Debug, Clone)]
pub struct SyncPolicy {
    pub auto_sync_enabled: bool,
    pub max_content_size: usize,
    pub image_compression_threshold: usize,
    pub privacy_filter_enabled: bool,
    pub notification_enabled: bool,
    pub history_retention_days: u32,
    pub allowed_content_types: Vec<ContentType>,
}

/// Content types for filtering
#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    Text,
    Image,
    Files,
    Custom(String),
}

impl Default for SyncPolicy {
    fn default() -> Self {
        Self {
            auto_sync_enabled: true,
            max_content_size: 1024 * 1024, // 1MB
            image_compression_threshold: 5 * 1024 * 1024, // 5MB
            privacy_filter_enabled: true,
            notification_enabled: true,
            history_retention_days: 30,
            allowed_content_types: vec![ContentType::Text, ContentType::Image],
        }
    }
}

impl ClipboardContent {
    /// Get the size of the clipboard content in bytes
    pub fn size(&self) -> usize {
        match self {
            ClipboardContent::Text(text) => text.size,
            ClipboardContent::Image(image) => image.data.len(),
            ClipboardContent::Files(files) => files.iter().map(|f| f.len()).sum(),
            ClipboardContent::Custom { data, .. } => data.len(),
        }
    }
    
    /// Get the content type
    pub fn content_type(&self) -> ContentType {
        match self {
            ClipboardContent::Text(_) => ContentType::Text,
            ClipboardContent::Image(_) => ContentType::Image,
            ClipboardContent::Files(_) => ContentType::Files,
            ClipboardContent::Custom { mime_type, .. } => ContentType::Custom(mime_type.clone()),
        }
    }
}

impl TextContent {
    /// Create new text content
    pub fn new(text: String) -> Self {
        let size = text.len();
        Self {
            text,
            encoding: TextEncoding::Utf8,
            format: TextFormat::Plain,
            size,
        }
    }
}

impl ImageContent {
    /// Create new image content
    pub fn new(data: Vec<u8>, format: ImageFormat, width: u32, height: u32) -> Self {
        Self {
            data,
            format,
            width,
            height,
            compressed: false,
        }
    }
}