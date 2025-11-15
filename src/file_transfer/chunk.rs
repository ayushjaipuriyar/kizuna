// Chunk Engine Module
//
// Handles file chunking, streaming, and reassembly

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::*,
    ChunkEngine, ChunkStream,
};
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Chunk engine implementation for file streaming
pub struct ChunkEngineImpl {
    chunk_size: usize,
}

impl ChunkEngineImpl {
    /// Create a new chunk engine with default 64KB chunk size
    pub fn new() -> Self {
        Self {
            chunk_size: Chunk::DEFAULT_SIZE,
        }
    }

    /// Create a new chunk engine with custom chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        Self { chunk_size }
    }

    /// Calculate SHA-256 checksum for data
    fn calculate_checksum(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut checksum = [0u8; 32];
        checksum.copy_from_slice(&result);
        checksum
    }
}

impl Default for ChunkEngineImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ChunkEngine for ChunkEngineImpl {
    /// Create chunks from a file
    /// Reads file in 64KB segments and creates chunk metadata with checksums
    async fn create_chunks(&self, file_path: PathBuf) -> Result<Vec<Chunk>> {
        // Open the file
        let mut file = File::open(&file_path).await.map_err(|e| {
            FileTransferError::IoError {
                path: file_path.clone(),
                source: e,
            }
        })?;

        // Get file metadata
        let metadata = file.metadata().await.map_err(|e| {
            FileTransferError::IoError {
                path: file_path.clone(),
                source: e,
            }
        })?;

        let file_size = metadata.len();
        let mut chunks = Vec::new();
        let mut offset = 0u64;
        let mut chunk_id = 0u64;

        // Read file in chunks
        loop {
            let mut buffer = vec![0u8; self.chunk_size];
            let bytes_read = file.read(&mut buffer).await.map_err(|e| {
                FileTransferError::IoError {
                    path: file_path.clone(),
                    source: e,
                }
            })?;

            if bytes_read == 0 {
                break; // End of file
            }

            // Truncate buffer to actual bytes read
            buffer.truncate(bytes_read);

            // Calculate checksum for this chunk
            let checksum = Self::calculate_checksum(&buffer);

            // Create chunk with metadata
            let chunk = Chunk {
                chunk_id,
                file_path: file_path.clone(),
                offset,
                size: bytes_read,
                data: buffer,
                checksum,
                compressed: false,
            };

            chunks.push(chunk);

            offset += bytes_read as u64;
            chunk_id += 1;

            // Safety check
            if offset >= file_size {
                break;
            }
        }

        Ok(chunks)
    }

    /// Stream a chunk over the connection
    /// Sends chunk metadata followed by chunk data with flow control
    async fn stream_chunk(&self, chunk: Chunk, stream: &mut dyn ChunkStream) -> Result<()> {
        // Serialize chunk metadata (without data) for transmission
        let metadata = ChunkMetadata {
            chunk_id: chunk.chunk_id,
            file_path: chunk.file_path.clone(),
            offset: chunk.offset,
            size: chunk.size,
            checksum: chunk.checksum,
            compressed: chunk.compressed,
        };

        // Serialize metadata to JSON
        let metadata_json = serde_json::to_vec(&metadata).map_err(|e| {
            FileTransferError::InternalError(format!("Failed to serialize chunk metadata: {}", e))
        })?;

        // Send metadata length (4 bytes, big-endian)
        let metadata_len = metadata_json.len() as u32;
        stream.send(&metadata_len.to_be_bytes()).await?;

        // Send metadata
        stream.send(&metadata_json).await?;

        // Send chunk data
        stream.send(&chunk.data).await?;

        // Flush to ensure data is sent
        stream.flush().await?;

        Ok(())
    }

    /// Receive a chunk from the connection
    /// Receives chunk metadata and data, then verifies integrity
    async fn receive_chunk(&self, stream: &mut dyn ChunkStream) -> Result<Chunk> {
        // Read metadata length (4 bytes)
        let mut len_buf = [0u8; 4];
        let bytes_read = stream.receive(&mut len_buf).await?;
        if bytes_read != 4 {
            return Err(FileTransferError::TransportError(
                "Failed to read metadata length".to_string(),
            ));
        }
        let metadata_len = u32::from_be_bytes(len_buf) as usize;

        // Validate metadata length (prevent excessive allocation)
        if metadata_len > 1024 * 1024 {
            // 1MB max for metadata
            return Err(FileTransferError::TransportError(
                "Metadata length exceeds maximum".to_string(),
            ));
        }

        // Read metadata
        let mut metadata_buf = vec![0u8; metadata_len];
        let mut total_read = 0;
        while total_read < metadata_len {
            let bytes_read = stream
                .receive(&mut metadata_buf[total_read..])
                .await?;
            if bytes_read == 0 {
                return Err(FileTransferError::TransportError(
                    "Connection closed while reading metadata".to_string(),
                ));
            }
            total_read += bytes_read;
        }

        // Deserialize metadata
        let metadata: ChunkMetadata = serde_json::from_slice(&metadata_buf).map_err(|e| {
            FileTransferError::InternalError(format!("Failed to deserialize chunk metadata: {}", e))
        })?;

        // Read chunk data
        let mut data = vec![0u8; metadata.size];
        let mut total_read = 0;
        while total_read < metadata.size {
            let bytes_read = stream.receive(&mut data[total_read..]).await?;
            if bytes_read == 0 {
                return Err(FileTransferError::TransportError(
                    "Connection closed while reading chunk data".to_string(),
                ));
            }
            total_read += bytes_read;
        }

        // Create chunk from received data
        let chunk = Chunk {
            chunk_id: metadata.chunk_id,
            file_path: metadata.file_path,
            offset: metadata.offset,
            size: metadata.size,
            data,
            checksum: metadata.checksum,
            compressed: metadata.compressed,
        };

        // Verify chunk integrity
        if !self.verify_chunk(&chunk).await? {
            return Err(FileTransferError::ChunkVerificationFailed {
                chunk_id: chunk.chunk_id,
            });
        }

        Ok(chunk)
    }

    /// Verify chunk integrity by recalculating checksum
    async fn verify_chunk(&self, chunk: &Chunk) -> Result<bool> {
        let calculated_checksum = Self::calculate_checksum(&chunk.data);
        Ok(calculated_checksum == chunk.checksum)
    }

    /// Reassemble file from chunks
    /// Orders chunks, detects gaps, writes to file, and verifies final integrity
    async fn reassemble_file(&self, mut chunks: Vec<Chunk>, output_path: PathBuf) -> Result<()> {
        if chunks.is_empty() {
            return Err(FileTransferError::InternalError(
                "Cannot reassemble file from empty chunk list".to_string(),
            ));
        }

        // Sort chunks by chunk_id to ensure correct order
        chunks.sort_by_key(|c| c.chunk_id);

        // Detect gaps in chunk sequence
        for i in 0..chunks.len() {
            if chunks[i].chunk_id != i as u64 {
                return Err(FileTransferError::InternalError(format!(
                    "Missing chunk in sequence: expected chunk_id {}, found {}",
                    i, chunks[i].chunk_id
                )));
            }
        }

        // Verify all chunks have the same file path
        let expected_path = &chunks[0].file_path;
        for chunk in &chunks {
            if chunk.file_path != *expected_path {
                return Err(FileTransferError::InternalError(
                    "Chunks belong to different files".to_string(),
                ));
            }
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = output_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                FileTransferError::IoError {
                    path: parent.to_path_buf(),
                    source: e,
                }
            })?;
        }

        // Create output file
        let mut output_file = File::create(&output_path).await.map_err(|e| {
            FileTransferError::IoError {
                path: output_path.clone(),
                source: e,
            }
        })?;

        // Write chunks to file in order
        let mut expected_offset = 0u64;

        for chunk in &chunks {
            // Verify chunk offset matches expected position
            if chunk.offset != expected_offset {
                return Err(FileTransferError::InternalError(format!(
                    "Chunk offset mismatch: expected {}, found {}",
                    expected_offset, chunk.offset
                )));
            }

            // Verify chunk integrity before writing
            if !self.verify_chunk(chunk).await? {
                return Err(FileTransferError::ChunkVerificationFailed {
                    chunk_id: chunk.chunk_id,
                });
            }

            // Write chunk data to file
            output_file.write_all(&chunk.data).await.map_err(|e| {
                FileTransferError::IoError {
                    path: output_path.clone(),
                    source: e,
                }
            })?;

            expected_offset += chunk.size as u64;
        }

        // Flush and sync to ensure all data is written
        output_file.flush().await.map_err(|e| {
            FileTransferError::IoError {
                path: output_path.clone(),
                source: e,
            }
        })?;

        output_file.sync_all().await.map_err(|e| {
            FileTransferError::IoError {
                path: output_path.clone(),
                source: e,
            }
        })?;

        // Close the file
        drop(output_file);

        // Verify final file integrity by calculating checksum
        let final_checksum = self.calculate_file_checksum(&output_path).await?;

        // Calculate expected checksum from all chunk data
        let mut hasher = Sha256::new();
        for chunk in &chunks {
            hasher.update(&chunk.data);
        }
        let expected_checksum = hasher.finalize();
        let mut expected_checksum_array = [0u8; 32];
        expected_checksum_array.copy_from_slice(&expected_checksum);

        // Verify checksums match
        if final_checksum != expected_checksum_array {
            return Err(FileTransferError::ChecksumMismatch {
                path: output_path,
            });
        }

        Ok(())
    }

}

impl ChunkEngineImpl {
    /// Calculate checksum for an entire file (private helper method)
    async fn calculate_file_checksum(&self, file_path: &PathBuf) -> Result<[u8; 32]> {
        let mut file = File::open(file_path).await.map_err(|e| {
            FileTransferError::IoError {
                path: file_path.clone(),
                source: e,
            }
        })?;

        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; self.chunk_size];

        loop {
            let bytes_read = file.read(&mut buffer).await.map_err(|e| {
                FileTransferError::IoError {
                    path: file_path.clone(),
                    source: e,
                }
            })?;

            if bytes_read == 0 {
                break;
            }

            hasher.update(&buffer[..bytes_read]);
        }

        let result = hasher.finalize();
        let mut checksum = [0u8; 32];
        checksum.copy_from_slice(&result);
        Ok(checksum)
    }
}
