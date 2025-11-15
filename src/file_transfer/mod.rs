// File Transfer Module
//
// This module provides reliable, efficient, and secure file transfer capabilities
// between Kizuna peers with support for resumability, compression, and intelligent
// transport negotiation.

pub mod manifest;
pub mod chunk;
pub mod queue;
pub mod transport;
pub mod error;
pub mod types;
pub mod session;
pub mod resume;
pub mod recovery;
pub mod compression;
pub mod bandwidth;
pub mod parallel;
pub mod security_integration;
pub mod transport_integration;
pub mod progress;
pub mod api;

pub use error::{FileTransferError, Result};
pub use types::*;
pub use api::{FileTransferSystem, TransferStats};
pub use progress::{ProgressTracker, ProgressCallback, EventCallback, TransferEvent};
pub use security_integration::{FileTransferSecurity, SecureTransferSession, SecureTransfer};
pub use transport_integration::{FileTransferTransport, ProtocolConfig, ConnectionPoolStats};

use async_trait::async_trait;
use std::path::PathBuf;
use uuid::Uuid;

/// Core file transfer trait providing unified interface for all transfer operations
#[async_trait]
pub trait FileTransfer: Send + Sync {
    /// Start a new file transfer with the given manifest to the specified peer
    async fn start_transfer(
        &self,
        manifest: TransferManifest,
        peer_id: PeerId,
    ) -> Result<TransferSession>;

    /// Resume an interrupted transfer using a resume token
    async fn resume_transfer(&self, resume_token: ResumeToken) -> Result<TransferSession>;

    /// Cancel an active transfer
    async fn cancel_transfer(&self, session_id: SessionId) -> Result<()>;

    /// Set bandwidth limit for transfers (None for unlimited)
    async fn set_bandwidth_limit(&self, limit: Option<u64>) -> Result<()>;

    /// Get all active transfer sessions
    async fn get_active_transfers(&self) -> Result<Vec<TransferSession>>;

    /// Get transfer progress for a specific session
    async fn get_transfer_progress(&self, session_id: SessionId) -> Result<TransferProgress>;
}

/// Transfer manager orchestrates file transfer operations and session management
#[async_trait]
pub trait TransferManager: Send + Sync {
    /// Start a new transfer session
    async fn start_transfer(
        &self,
        manifest: TransferManifest,
        peer_id: PeerId,
    ) -> Result<TransferSession>;

    /// Resume an interrupted transfer
    async fn resume_transfer(&self, resume_token: ResumeToken) -> Result<TransferSession>;

    /// Cancel a transfer
    async fn cancel_transfer(&self, session_id: SessionId) -> Result<()>;

    /// Set bandwidth limit
    async fn set_bandwidth_limit(&self, limit: Option<u64>) -> Result<()>;

    /// Get active transfers
    async fn get_active_transfers(&self) -> Result<Vec<TransferSession>>;
}

/// Queue manager handles transfer queue, scheduling, and prioritization
#[async_trait]
pub trait QueueManager: Send + Sync {
    /// Enqueue a transfer request with priority
    async fn enqueue_transfer(
        &self,
        request: TransferRequest,
        priority: Priority,
    ) -> Result<QueueId>;

    /// Reorder queue item to new position
    async fn reorder_queue(&self, queue_id: QueueId, new_position: usize) -> Result<()>;

    /// Pause a queued transfer
    async fn pause_queue_item(&self, queue_id: QueueId) -> Result<()>;

    /// Cancel a queued transfer
    async fn cancel_queue_item(&self, queue_id: QueueId) -> Result<()>;

    /// Get current queue status
    async fn get_queue_status(&self) -> Result<Vec<QueueItem>>;
}

/// Manifest builder creates transfer manifests with file metadata and structure
#[async_trait]
pub trait ManifestBuilder: Send + Sync {
    /// Build manifest for a single file
    async fn build_file_manifest(&self, path: PathBuf) -> Result<TransferManifest>;

    /// Build manifest for multiple files
    async fn build_multi_file_manifest(&self, paths: Vec<PathBuf>) -> Result<TransferManifest>;

    /// Build manifest for a folder (recursive)
    async fn build_folder_manifest(&self, path: PathBuf, recursive: bool)
        -> Result<TransferManifest>;

    /// Verify manifest integrity
    async fn verify_manifest(&self, manifest: &TransferManifest) -> Result<bool>;
}

/// Chunk engine handles file chunking, streaming, and verification
#[async_trait]
pub trait ChunkEngine: Send + Sync {
    /// Create chunks from a file
    async fn create_chunks(&self, file_path: PathBuf) -> Result<Vec<Chunk>>;

    /// Stream a chunk over the connection
    async fn stream_chunk(&self, chunk: Chunk, stream: &mut dyn ChunkStream) -> Result<()>;

    /// Receive a chunk from the connection
    async fn receive_chunk(&self, stream: &mut dyn ChunkStream) -> Result<Chunk>;

    /// Verify chunk integrity
    async fn verify_chunk(&self, chunk: &Chunk) -> Result<bool>;

    /// Reassemble file from chunks
    async fn reassemble_file(&self, chunks: Vec<Chunk>, output_path: PathBuf) -> Result<()>;
}

/// Chunk stream trait for abstracting transport layer
#[async_trait]
pub trait ChunkStream: Send + Sync {
    /// Send bytes over the stream
    async fn send(&mut self, data: &[u8]) -> Result<()>;

    /// Receive bytes from the stream
    async fn receive(&mut self, buffer: &mut [u8]) -> Result<usize>;

    /// Flush the stream
    async fn flush(&mut self) -> Result<()>;
}

/// Transport negotiator selects optimal transport protocol for file transfers
#[async_trait]
pub trait TransportNegotiator: Send + Sync {
    /// Negotiate transport protocol with peer
    async fn negotiate_transport(
        &self,
        peer_id: PeerId,
        file_size: u64,
    ) -> Result<TransportProtocol>;

    /// Get peer transport capabilities
    async fn get_peer_capabilities(&self, peer_id: PeerId) -> Result<TransportCapabilities>;

    /// Benchmark transport performance
    async fn benchmark_transport(
        &self,
        protocol: TransportProtocol,
        peer_id: PeerId,
    ) -> Result<PerformanceMetrics>;

    /// Get fallback transport if current fails
    async fn fallback_transport(
        &self,
        current: TransportProtocol,
    ) -> Result<Option<TransportProtocol>>;
}
