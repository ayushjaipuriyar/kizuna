// Core File Transfer Data Structures

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

// Type aliases for clarity
pub type TransferId = Uuid;
pub type SessionId = Uuid;
pub type PeerId = String;
pub type QueueId = Uuid;
pub type ChunkId = u64;
pub type Timestamp = u64;

/// Get current timestamp in seconds since UNIX epoch
pub fn current_timestamp() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Transfer manifest containing metadata for files to be transferred
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferManifest {
    pub transfer_id: TransferId,
    pub sender_id: PeerId,
    pub created_at: Timestamp,
    pub total_size: u64,
    pub file_count: usize,
    pub files: Vec<FileEntry>,
    pub directories: Vec<DirectoryEntry>,
    pub checksum: [u8; 32], // SHA-256 of entire manifest
}

impl TransferManifest {
    pub fn new(sender_id: PeerId) -> Self {
        Self {
            transfer_id: Uuid::new_v4(),
            sender_id,
            created_at: current_timestamp(),
            total_size: 0,
            file_count: 0,
            files: Vec::new(),
            directories: Vec::new(),
            checksum: [0u8; 32],
        }
    }
}

/// File entry in transfer manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub checksum: [u8; 32], // SHA-256
    pub permissions: FilePermissions,
    pub modified_at: Timestamp,
    pub chunk_count: usize,
}

/// Directory entry in transfer manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub path: PathBuf,
    pub permissions: FilePermissions,
    pub created_at: Timestamp,
}

/// File permissions (cross-platform representation)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FilePermissions {
    pub readonly: bool,
    pub executable: bool,
    #[cfg(unix)]
    pub mode: u32,
}

impl Default for FilePermissions {
    fn default() -> Self {
        Self {
            readonly: false,
            executable: false,
            #[cfg(unix)]
            mode: 0o644,
        }
    }
}

/// Transfer session representing an active file transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSession {
    pub session_id: SessionId,
    pub manifest: TransferManifest,
    pub peer_id: PeerId,
    pub transport: TransportProtocol,
    pub state: TransferState,
    pub progress: TransferProgress,
    pub bandwidth_limit: Option<u64>,
    pub parallel_streams: usize,
    pub resume_token: Option<ResumeToken>,
    pub created_at: Timestamp,
}

impl TransferSession {
    pub fn new(manifest: TransferManifest, peer_id: PeerId, transport: TransportProtocol) -> Self {
        Self {
            session_id: Uuid::new_v4(),
            manifest,
            peer_id,
            transport,
            state: TransferState::Pending,
            progress: TransferProgress::default(),
            bandwidth_limit: None,
            parallel_streams: 1,
            resume_token: None,
            created_at: current_timestamp(),
        }
    }
}

/// Transfer state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferState {
    Pending,
    Negotiating,
    Transferring,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Transfer progress tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferProgress {
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub files_completed: usize,
    pub total_files: usize,
    pub current_speed: u64,  // bytes per second
    pub average_speed: u64,  // bytes per second
    pub eta_seconds: Option<u64>,
    pub last_update: Timestamp,
}

impl Default for TransferProgress {
    fn default() -> Self {
        Self {
            bytes_transferred: 0,
            total_bytes: 0,
            files_completed: 0,
            total_files: 0,
            current_speed: 0,
            average_speed: 0,
            eta_seconds: None,
            last_update: current_timestamp(),
        }
    }
}

impl TransferProgress {
    /// Calculate progress percentage (0-100)
    pub fn percentage(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.bytes_transferred as f64 / self.total_bytes as f64) * 100.0
        }
    }

    /// Update ETA based on current speed
    pub fn update_eta(&mut self) {
        if self.current_speed > 0 {
            let remaining_bytes = self.total_bytes.saturating_sub(self.bytes_transferred);
            self.eta_seconds = Some(remaining_bytes / self.current_speed);
        } else {
            self.eta_seconds = None;
        }
    }
}

/// Resume token for interrupted transfers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeToken {
    pub transfer_id: TransferId,
    pub session_id: SessionId,
    pub last_completed_file: Option<PathBuf>,
    pub last_completed_chunk: Option<ChunkId>,
    pub bytes_completed: u64,
    pub created_at: Timestamp,
    pub expires_at: Timestamp,
}

impl ResumeToken {
    pub fn new(transfer_id: TransferId, session_id: SessionId) -> Self {
        let created_at = current_timestamp();
        let expires_at = created_at + (24 * 60 * 60); // 24 hours
        
        Self {
            transfer_id,
            session_id,
            last_completed_file: None,
            last_completed_chunk: None,
            bytes_completed: 0,
            created_at,
            expires_at,
        }
    }

    /// Check if resume token is expired
    pub fn is_expired(&self) -> bool {
        current_timestamp() > self.expires_at
    }
}

/// File chunk for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub chunk_id: ChunkId,
    pub file_path: PathBuf,
    pub offset: u64,
    pub size: usize,
    pub data: Vec<u8>,
    pub checksum: [u8; 32], // SHA-256
    pub compressed: bool,
}

impl Chunk {
    pub const DEFAULT_SIZE: usize = 64 * 1024; // 64KB
}

/// Chunk metadata for transmission (without data payload)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub chunk_id: ChunkId,
    pub file_path: PathBuf,
    pub offset: u64,
    pub size: usize,
    pub checksum: [u8; 32],
    pub compressed: bool,
}

/// Transport protocol options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransportProtocol {
    Quic,
    Tcp,
    WebRtc,
}

impl TransportProtocol {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransportProtocol::Quic => "QUIC",
            TransportProtocol::Tcp => "TCP",
            TransportProtocol::WebRtc => "WebRTC",
        }
    }
}

/// Transport capabilities of a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportCapabilities {
    pub supports_quic: bool,
    pub supports_tcp: bool,
    pub supports_webrtc: bool,
    pub max_parallel_streams: usize,
    pub max_bandwidth: Option<u64>,
}

impl Default for TransportCapabilities {
    fn default() -> Self {
        Self {
            supports_quic: true,
            supports_tcp: true,
            supports_webrtc: false,
            max_parallel_streams: 4,
            max_bandwidth: None,
        }
    }
}

/// Performance metrics for transport benchmarking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub latency_ms: u64,
    pub throughput_bps: u64,
    pub packet_loss: f64,
    pub jitter_ms: u64,
}

/// Transfer request for queue management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    pub manifest: TransferManifest,
    pub peer_id: PeerId,
    pub transport_preference: Option<TransportProtocol>,
    pub bandwidth_limit: Option<u64>,
}

/// Queue item in transfer queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub queue_id: QueueId,
    pub transfer_request: TransferRequest,
    pub priority: Priority,
    pub estimated_start: Option<Timestamp>,
    pub state: QueueState,
    pub created_at: Timestamp,
}

/// Priority levels for queue items
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
}

/// Queue state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueState {
    Pending,
    Scheduled,
    Paused,
    Cancelled,
}
