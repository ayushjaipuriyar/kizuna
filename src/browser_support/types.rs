//! Browser Support Data Types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Browser connection information for WebRTC establishment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConnectionInfo {
    pub peer_id: String,
    pub signaling_info: SignalingInfo,
    pub browser_info: BrowserInfo,
}

/// WebRTC signaling information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalingInfo {
    pub signaling_server: Option<String>,
    pub ice_servers: Vec<IceServer>,
    pub connection_type: ConnectionType,
}

/// ICE server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceServer {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

/// Connection type for WebRTC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Direct,
    Relay,
    Hybrid,
}

/// Browser information and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserInfo {
    pub user_agent: String,
    pub browser_type: BrowserType,
    pub version: String,
    pub platform: String,
    pub supports_webrtc: bool,
    pub supports_clipboard_api: bool,
}

/// Browser type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserType {
    Chrome,
    Firefox,
    Safari,
    Edge,
    Other(String),
}

/// Browser session representing an active connection
#[derive(Debug)]
pub struct BrowserSession {
    pub session_id: Uuid,
    pub browser_info: BrowserInfo,
    pub webrtc_connection: WebRTCConnection,
    pub permissions: BrowserPermissions,
    pub created_at: std::time::SystemTime,
    pub last_activity: std::time::SystemTime,
}

/// WebRTC connection state
#[derive(Debug)]
pub struct WebRTCConnection {
    pub connection_id: Uuid,
    pub peer_id: String,
    pub data_channels: HashMap<ChannelType, DataChannelInfo>,
    pub connection_state: ConnectionState,
    pub ice_connection_state: IceConnectionState,
}

/// Data channel types for different services
#[derive(Debug, Clone, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ChannelType {
    FileTransfer,
    Clipboard,
    Command,
    Video,
    Control,
}

/// Data channel information
#[derive(Debug)]
pub struct DataChannelInfo {
    pub channel_type: ChannelType,
    pub ready_state: DataChannelState,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

/// WebRTC connection states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionState {
    New,
    Connecting,
    Connected,
    Disconnected,
    Failed,
    Closed,
}

/// ICE connection states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IceConnectionState {
    New,
    Checking,
    Connected,
    Completed,
    Failed,
    Disconnected,
    Closed,
}

/// Data channel states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DataChannelState {
    Connecting,
    Open,
    Closing,
    Closed,
}

/// Browser permissions for different operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserPermissions {
    pub file_transfer: bool,
    pub clipboard_sync: bool,
    pub command_execution: bool,
    pub camera_streaming: bool,
    pub system_info: bool,
}

impl Default for BrowserPermissions {
    fn default() -> Self {
        Self {
            file_transfer: false,
            clipboard_sync: false,
            command_execution: false,
            camera_streaming: false,
            system_info: false,
        }
    }
}

/// Browser message types for communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserMessage {
    pub message_id: Uuid,
    pub message_type: BrowserMessageType,
    pub payload: serde_json::Value,
    pub timestamp: std::time::SystemTime,
    pub session_id: Uuid,
}

/// Browser message type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserMessageType {
    FileTransferRequest,
    ClipboardSync,
    CommandExecution,
    VideoStreamRequest,
    PeerDiscovery,
    StatusUpdate,
    Error,
    // WebSocket fallback specific messages
    WebSocketHandshake,
    ProtocolNegotiation,
    FallbackActivated,
}

/// Communication protocol type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationProtocol {
    WebRTC,
    WebSocket,
}

/// Protocol capabilities for feature detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolCapabilities {
    pub supports_webrtc: bool,
    pub supports_websocket: bool,
    pub supports_file_transfer: bool,
    pub supports_clipboard: bool,
    pub supports_video_streaming: bool,
    pub supports_command_execution: bool,
}

/// Unified connection interface
#[derive(Debug, Clone)]
pub struct UnifiedConnection {
    pub connection_id: Uuid,
    pub protocol: CommunicationProtocol,
    pub session_id: Uuid,
    pub capabilities: ProtocolCapabilities,
    pub created_at: std::time::SystemTime,
    pub last_activity: std::time::SystemTime,
}

/// WebSocket connection state
#[derive(Debug, Clone)]
pub struct WebSocketConnection {
    pub connection_id: Uuid,
    pub peer_id: String,
    pub connection_state: ConnectionState,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub created_at: std::time::SystemTime,
}

/// Web file representation for browser file transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFile {
    pub name: String,
    pub size: u64,
    pub mime_type: String,
    pub last_modified: u64, // Unix timestamp
    pub data: FileData,
}

/// File data representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileData {
    Base64(String),
    Chunks(Vec<FileChunk>),
    Stream(String), // Stream identifier
}

/// File chunk for streaming transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub chunk_id: u32,
    pub data: String, // Base64 encoded
    pub checksum: String,
}

/// PWA manifest configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub start_url: String,
    pub display: DisplayMode,
    pub theme_color: String,
    pub background_color: String,
    pub icons: Vec<AppIcon>,
    pub categories: Vec<String>,
}

/// PWA display modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisplayMode {
    #[serde(rename = "fullscreen")]
    Fullscreen,
    #[serde(rename = "standalone")]
    Standalone,
    #[serde(rename = "minimal-ui")]
    MinimalUI,
    #[serde(rename = "browser")]
    Browser,
}

/// PWA app icon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIcon {
    pub src: String,
    pub sizes: String,
    #[serde(rename = "type")]
    pub icon_type: String,
    pub purpose: Option<String>,
}