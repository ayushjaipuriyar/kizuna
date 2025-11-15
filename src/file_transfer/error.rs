// File Transfer Error Types

use thiserror::Error;
use std::io;
use std::path::PathBuf;

pub type Result<T> = std::result::Result<T, FileTransferError>;

/// Comprehensive error types for file transfer operations
#[derive(Error, Debug)]
pub enum FileTransferError {
    // Manifest errors
    #[error("Failed to scan file or directory: {path}")]
    ScanError { path: PathBuf, source: io::Error },

    #[error("Failed to calculate checksum for: {path}")]
    ChecksumError { path: PathBuf, source: io::Error },

    #[error("Invalid manifest: {reason}")]
    InvalidManifest { reason: String },

    #[error("Manifest verification failed: {reason}")]
    ManifestVerificationFailed { reason: String },

    // Transport errors
    #[error("Transport negotiation failed: {reason}")]
    TransportNegotiationFailed { reason: String },

    #[error("Transport protocol not supported: {protocol}")]
    UnsupportedTransport { protocol: String },

    #[error("Network connection failed: {reason}")]
    NetworkError { reason: String },

    #[error("Transport error: {0}")]
    TransportError(String),

    // Storage errors
    #[error("Insufficient disk space: required {required} bytes, available {available} bytes")]
    InsufficientDiskSpace { required: u64, available: u64 },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: PathBuf },

    #[error("File I/O error: {path}")]
    IoError { path: PathBuf, source: io::Error },

    #[error("Invalid path: {path}")]
    InvalidPath { path: PathBuf },

    // Integrity errors
    #[error("Checksum mismatch for: {path}")]
    ChecksumMismatch { path: PathBuf },

    #[error("Chunk verification failed: chunk {chunk_id}")]
    ChunkVerificationFailed { chunk_id: u64 },

    #[error("Data corruption detected: {reason}")]
    CorruptionDetected { reason: String },

    #[error("Integrity error: {0}")]
    IntegrityError(String),

    // Security errors
    #[error("Security error: {0}")]
    SecurityError(String),

    #[error("Peer authentication failed: {peer_id}")]
    AuthenticationFailed { peer_id: String },

    #[error("Peer not trusted: {peer_id}")]
    PeerNotTrusted { peer_id: String },

    #[error("Encryption failed: {reason}")]
    EncryptionFailed { reason: String },

    #[error("Decryption failed: {reason}")]
    DecryptionFailed { reason: String },

    // Resume errors
    #[error("Invalid resume token: {reason}")]
    InvalidResumeToken { reason: String },

    #[error("Resume token expired")]
    ResumeTokenExpired,

    #[error("Cannot resume transfer: {reason}")]
    ResumeError { reason: String },

    // Session errors
    #[error("Transfer session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Transfer already in progress: {session_id}")]
    TransferInProgress { session_id: String },

    #[error("Transfer cancelled by user")]
    TransferCancelled,

    #[error("Transfer timeout")]
    TransferTimeout,

    // Queue errors
    #[error("Queue item not found: {queue_id}")]
    QueueItemNotFound { queue_id: String },

    #[error("Invalid queue operation: {reason}")]
    InvalidQueueOperation { reason: String },

    // Compression errors
    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Decompression error: {0}")]
    DecompressionError(String),

    // General errors
    #[error("Invalid configuration: {reason}")]
    InvalidConfiguration { reason: String },

    #[error("Operation not supported: {operation}")]
    UnsupportedOperation { operation: String },

    #[error("Internal error: {0}")]
    InternalError(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl FileTransferError {
    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            FileTransferError::NetworkError { .. }
                | FileTransferError::TransportError(_)
                | FileTransferError::TransferTimeout
                | FileTransferError::ChunkVerificationFailed { .. }
        )
    }

    /// Check if error should trigger transport fallback
    pub fn should_fallback(&self) -> bool {
        matches!(
            self,
            FileTransferError::TransportNegotiationFailed { .. }
                | FileTransferError::UnsupportedTransport { .. }
                | FileTransferError::NetworkError { .. }
        )
    }
}
