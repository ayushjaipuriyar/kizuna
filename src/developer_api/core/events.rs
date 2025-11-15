/// Event system for the Developer API
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Events emitted by the Kizuna API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KizunaEvent {
    /// A peer was discovered
    PeerDiscovered(PeerInfo),
    
    /// A peer connection was established
    PeerConnected(PeerId),
    
    /// A peer connection was closed
    PeerDisconnected(PeerId),
    
    /// A file transfer started
    TransferStarted(TransferInfo),
    
    /// File transfer progress update
    TransferProgress(TransferProgress),
    
    /// A file transfer completed
    TransferCompleted(TransferResult),
    
    /// A media stream started
    StreamStarted(StreamInfo),
    
    /// A media stream ended
    StreamEnded(StreamId),
    
    /// A command was executed
    CommandExecuted(CommandResult),
    
    /// An error occurred
    Error(ErrorEvent),
}

/// Peer identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub String);

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for PeerId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for PeerId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Information about a discovered peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer identifier
    pub id: PeerId,
    
    /// Peer name
    pub name: String,
    
    /// Peer addresses
    pub addresses: Vec<String>,
    
    /// Peer capabilities
    pub capabilities: Vec<String>,
    
    /// Discovery method
    pub discovery_method: String,
}

/// Transfer identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransferId(pub Uuid);

impl fmt::Display for TransferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TransferId {
    /// Creates a new transfer ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TransferId {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a file transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferInfo {
    /// Transfer identifier
    pub id: TransferId,
    
    /// File name
    pub file_name: String,
    
    /// File size in bytes
    pub file_size: u64,
    
    /// Peer ID
    pub peer_id: PeerId,
    
    /// Transfer direction
    pub direction: TransferDirection,
}

/// Transfer direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferDirection {
    /// Sending to peer
    Send,
    
    /// Receiving from peer
    Receive,
}

/// Transfer progress information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    /// Transfer identifier
    pub id: TransferId,
    
    /// Bytes transferred
    pub bytes_transferred: u64,
    
    /// Total bytes
    pub total_bytes: u64,
    
    /// Transfer speed in bytes per second
    pub speed_bps: u64,
}

impl TransferProgress {
    /// Calculates the progress percentage
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.bytes_transferred as f64 / self.total_bytes as f64) * 100.0
        }
    }
}

/// Transfer result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    /// Transfer identifier
    pub id: TransferId,
    
    /// Whether the transfer was successful
    pub success: bool,
    
    /// Error message if failed
    pub error: Option<String>,
    
    /// Total bytes transferred
    pub bytes_transferred: u64,
    
    /// Transfer duration in milliseconds
    pub duration_ms: u64,
}

/// Stream identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StreamId(pub Uuid);

impl fmt::Display for StreamId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StreamId {
    /// Creates a new stream ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for StreamId {
    fn default() -> Self {
        Self::new()
    }
}

/// Information about a media stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamInfo {
    /// Stream identifier
    pub id: StreamId,
    
    /// Stream type
    pub stream_type: StreamType,
    
    /// Peer ID
    pub peer_id: PeerId,
}

/// Type of media stream
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamType {
    /// Camera video stream
    Camera,
    
    /// Screen sharing stream
    Screen,
    
    /// Audio stream
    Audio,
}

/// Command result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    /// Command that was executed
    pub command: String,
    
    /// Peer ID where command was executed
    pub peer_id: PeerId,
    
    /// Exit code
    pub exit_code: i32,
    
    /// Standard output
    pub stdout: String,
    
    /// Standard error
    pub stderr: String,
}

/// Error event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    /// Error message
    pub message: String,
    
    /// Error code
    pub code: Option<String>,
    
    /// Additional context
    pub context: std::collections::HashMap<String, String>,
}

/// Event listener trait
#[async_trait::async_trait]
pub trait EventListener: Send + Sync {
    /// Called when an event is emitted
    async fn on_event(&self, event: KizunaEvent);
}

/// Event emitter for publishing events
pub struct EventEmitter {
    listeners: Vec<Box<dyn EventListener>>,
}

impl EventEmitter {
    /// Creates a new event emitter
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
        }
    }
    
    /// Adds an event listener
    pub fn add_listener(&mut self, listener: Box<dyn EventListener>) {
        self.listeners.push(listener);
    }
    
    /// Emits an event to all listeners
    pub async fn emit(&self, event: KizunaEvent) {
        for listener in &self.listeners {
            listener.on_event(event.clone()).await;
        }
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}
