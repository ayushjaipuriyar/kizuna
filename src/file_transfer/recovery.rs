// Transfer Recovery Module
//
// Handles interrupted transfer detection, chunk verification, and recovery

use crate::file_transfer::{
    chunk::ChunkEngineImpl,
    error::{FileTransferError, Result},
    resume::{ResumeManager, ResumePosition},
    session::SessionManager,
    types::*,
    ChunkEngine,
};
use std::collections::HashSet;
use std::path::PathBuf;
use tokio::fs;

/// Transfer recovery manager handles interrupted transfer detection and recovery
pub struct RecoveryManager {
    session_manager: SessionManager,
    resume_manager: ResumeManager,
    chunk_engine: ChunkEngineImpl,
}

impl RecoveryManager {
    /// Create a new recovery manager
    pub fn new(
        session_manager: SessionManager,
        resume_manager: ResumeManager,
    ) -> Self {
        Self {
            session_manager,
            resume_manager,
            chunk_engine: ChunkEngineImpl::new(),
        }
    }

    /// Detect interrupted transfers on reconnection
    pub async fn detect_interrupted_transfers(
        &self,
        peer_id: &PeerId,
    ) -> Result<Vec<TransferSession>> {
        // Get all active sessions for this peer
        let all_sessions = self.session_manager.get_active_sessions().await?;
        
        // Filter sessions that are in transferring or paused state for this peer
        let interrupted_sessions: Vec<TransferSession> = all_sessions
            .into_iter()
            .filter(|session| {
                session.peer_id == *peer_id
                    && matches!(
                        session.state,
                        TransferState::Transferring | TransferState::Paused
                    )
            })
            .collect();

        Ok(interrupted_sessions)
    }

    /// Check if a transfer can be resumed
    pub async fn can_resume_transfer(&self, transfer_id: TransferId) -> Result<bool> {
        // Check if resume token exists and is valid
        let can_resume = self.resume_manager.can_resume(transfer_id).await?;
        
        if !can_resume {
            return Ok(false);
        }

        // Get resume token
        let token = self.resume_manager.get_token(transfer_id).await?;
        
        // Validate token
        self.resume_manager.validate_token(&token).await?;

        Ok(true)
    }

    /// Verify chunks for resume operation
    pub async fn verify_chunks_for_resume(
        &self,
        manifest: &TransferManifest,
        resume_position: &ResumePosition,
        temp_dir: &PathBuf,
    ) -> Result<ChunkVerificationResult> {
        let mut verified_chunks = HashSet::new();
        let mut corrupted_chunks = Vec::new();
        let mut missing_chunks = Vec::new();

        // If fresh start, no chunks to verify
        if resume_position.is_fresh_start() {
            return Ok(ChunkVerificationResult {
                verified_chunks,
                corrupted_chunks,
                missing_chunks,
                total_verified_bytes: 0,
            });
        }

        let mut total_verified_bytes = 0u64;

        // Iterate through files in manifest
        for file_entry in &manifest.files {
            // Determine which chunks to verify for this file
            let chunks_to_verify = self.get_chunks_to_verify(
                file_entry,
                resume_position,
            )?;

            if chunks_to_verify.is_empty() {
                continue;
            }

            // Get temporary file path
            let temp_file_path = self.get_temp_file_path(temp_dir, &file_entry.path);

            // Check if temp file exists
            if !temp_file_path.exists() {
                // File doesn't exist, mark all chunks as missing
                for chunk_id in chunks_to_verify {
                    missing_chunks.push((file_entry.path.clone(), chunk_id));
                }
                continue;
            }

            // Verify each chunk
            for chunk_id in chunks_to_verify {
                match self.verify_chunk_in_file(
                    &temp_file_path,
                    &file_entry.path,
                    chunk_id,
                    file_entry.size,
                ).await {
                    Ok(true) => {
                        verified_chunks.insert((file_entry.path.clone(), chunk_id));
                        total_verified_bytes += self.calculate_chunk_size(
                            chunk_id,
                            file_entry.size,
                        ) as u64;
                    }
                    Ok(false) => {
                        corrupted_chunks.push((file_entry.path.clone(), chunk_id));
                    }
                    Err(_) => {
                        missing_chunks.push((file_entry.path.clone(), chunk_id));
                    }
                }
            }
        }

        Ok(ChunkVerificationResult {
            verified_chunks,
            corrupted_chunks,
            missing_chunks,
            total_verified_bytes,
        })
    }

    /// Detect gaps in chunk sequence
    pub async fn detect_chunk_gaps(
        &self,
        manifest: &TransferManifest,
        verification_result: &ChunkVerificationResult,
    ) -> Result<Vec<ChunkGap>> {
        let mut gaps = Vec::new();

        for file_entry in &manifest.files {
            let mut file_gaps = self.detect_file_chunk_gaps(
                file_entry,
                verification_result,
            )?;
            gaps.append(&mut file_gaps);
        }

        Ok(gaps)
    }

    /// Create recovery plan for interrupted transfer
    pub async fn create_recovery_plan(
        &self,
        manifest: &TransferManifest,
        resume_position: &ResumePosition,
        temp_dir: &PathBuf,
    ) -> Result<RecoveryPlan> {
        // Verify existing chunks
        let verification_result = self
            .verify_chunks_for_resume(manifest, resume_position, temp_dir)
            .await?;

        // Detect gaps
        let gaps = self.detect_chunk_gaps(manifest, &verification_result).await?;

        // Calculate chunks to transfer
        let chunks_to_transfer = self.calculate_chunks_to_transfer(
            manifest,
            &verification_result,
            &gaps,
        )?;

        // Calculate bytes remaining
        let bytes_remaining = manifest.total_size - verification_result.total_verified_bytes;

        Ok(RecoveryPlan {
            verification_result,
            gaps,
            chunks_to_transfer,
            bytes_remaining,
            resume_from_chunk: resume_position.next_chunk_id(),
        })
    }

    /// Resume transfer from last valid chunk position
    pub async fn resume_from_position(
        &self,
        session_id: SessionId,
        recovery_plan: &RecoveryPlan,
    ) -> Result<()> {
        // Update session state to transferring
        self.session_manager
            .update_session_state(session_id, TransferState::Transferring)
            .await?;

        // Update session progress with verified bytes
        let session = self.session_manager.get_session(session_id).await?;
        let mut progress = session.progress;
        progress.bytes_transferred = recovery_plan.verification_result.total_verified_bytes;
        progress.update_eta();

        self.session_manager
            .update_session_progress(session_id, progress)
            .await?;

        Ok(())
    }

    /// Get chunks to verify for a file based on resume position
    fn get_chunks_to_verify(
        &self,
        file_entry: &FileEntry,
        resume_position: &ResumePosition,
    ) -> Result<Vec<ChunkId>> {
        // If no progress, no chunks to verify
        if resume_position.is_fresh_start() {
            return Ok(Vec::new());
        }

        // If this file hasn't been started, no chunks to verify
        if let Some(ref last_file) = resume_position.last_completed_file {
            if file_entry.path > *last_file {
                return Ok(Vec::new());
            }
        }

        // Determine last chunk to verify
        let last_chunk = if let Some(ref last_file) = resume_position.last_completed_file {
            if file_entry.path == *last_file {
                // This is the file we were working on
                resume_position.last_completed_chunk.unwrap_or(0)
            } else {
                // This file was completed, verify all chunks
                file_entry.chunk_count as ChunkId - 1
            }
        } else {
            // No file completed yet, use chunk ID
            resume_position.last_completed_chunk.unwrap_or(0)
        };

        // Generate list of chunk IDs to verify (0 to last_chunk inclusive)
        Ok((0..=last_chunk).collect())
    }

    /// Verify a specific chunk in a file
    async fn verify_chunk_in_file(
        &self,
        file_path: &PathBuf,
        original_path: &PathBuf,
        chunk_id: ChunkId,
        file_size: u64,
    ) -> Result<bool> {
        // Calculate chunk offset and size
        let offset = chunk_id * Chunk::DEFAULT_SIZE as u64;
        let chunk_size = self.calculate_chunk_size(chunk_id, file_size);

        // Check if offset is within file bounds
        if offset >= file_size {
            return Ok(false);
        }

        // Read chunk data from file
        let mut file = fs::File::open(file_path).await.map_err(|e| {
            FileTransferError::IoError {
                path: file_path.clone(),
                source: e,
            }
        })?;

        use tokio::io::{AsyncReadExt, AsyncSeekExt};
        file.seek(std::io::SeekFrom::Start(offset))
            .await
            .map_err(|e| FileTransferError::IoError {
                path: file_path.clone(),
                source: e,
            })?;

        let mut buffer = vec![0u8; chunk_size];
        let bytes_read = file.read_exact(&mut buffer).await.map_err(|e| {
            FileTransferError::IoError {
                path: file_path.clone(),
                source: e,
            }
        })?;

        if bytes_read != chunk_size {
            return Ok(false);
        }

        // Create chunk for verification
        let chunk = Chunk {
            chunk_id,
            file_path: original_path.clone(),
            offset,
            size: chunk_size,
            data: buffer,
            checksum: [0u8; 32], // Will be calculated during verification
            compressed: false,
        };

        // Verify chunk integrity
        self.chunk_engine.verify_chunk(&chunk).await
    }

    /// Calculate chunk size for a given chunk ID and file size
    fn calculate_chunk_size(&self, chunk_id: ChunkId, file_size: u64) -> usize {
        let offset = chunk_id * Chunk::DEFAULT_SIZE as u64;
        let remaining = file_size - offset;
        
        if remaining >= Chunk::DEFAULT_SIZE as u64 {
            Chunk::DEFAULT_SIZE
        } else {
            remaining as usize
        }
    }

    /// Detect chunk gaps for a specific file
    fn detect_file_chunk_gaps(
        &self,
        file_entry: &FileEntry,
        verification_result: &ChunkVerificationResult,
    ) -> Result<Vec<ChunkGap>> {
        let mut gaps = Vec::new();
        let total_chunks = file_entry.chunk_count as ChunkId;

        let mut current_chunk = 0u64;
        let mut gap_start: Option<ChunkId> = None;

        while current_chunk < total_chunks {
            let is_verified = verification_result
                .verified_chunks
                .contains(&(file_entry.path.clone(), current_chunk));

            if !is_verified {
                // Start or continue a gap
                if gap_start.is_none() {
                    gap_start = Some(current_chunk);
                }
            } else if let Some(start) = gap_start {
                // End of gap
                gaps.push(ChunkGap {
                    file_path: file_entry.path.clone(),
                    start_chunk: start,
                    end_chunk: current_chunk - 1,
                });
                gap_start = None;
            }

            current_chunk += 1;
        }

        // Handle gap extending to end of file
        if let Some(start) = gap_start {
            gaps.push(ChunkGap {
                file_path: file_entry.path.clone(),
                start_chunk: start,
                end_chunk: total_chunks - 1,
            });
        }

        Ok(gaps)
    }

    /// Calculate chunks that need to be transferred
    fn calculate_chunks_to_transfer(
        &self,
        manifest: &TransferManifest,
        verification_result: &ChunkVerificationResult,
        gaps: &[ChunkGap],
    ) -> Result<Vec<ChunkToTransfer>> {
        let mut chunks_to_transfer = Vec::new();

        // Add all chunks from gaps
        for gap in gaps {
            for chunk_id in gap.start_chunk..=gap.end_chunk {
                chunks_to_transfer.push(ChunkToTransfer {
                    file_path: gap.file_path.clone(),
                    chunk_id,
                    priority: ChunkPriority::Gap,
                });
            }
        }

        // Add corrupted chunks with high priority
        for (file_path, chunk_id) in &verification_result.corrupted_chunks {
            chunks_to_transfer.push(ChunkToTransfer {
                file_path: file_path.clone(),
                chunk_id: *chunk_id,
                priority: ChunkPriority::Corrupted,
            });
        }

        // Add missing chunks
        for (file_path, chunk_id) in &verification_result.missing_chunks {
            chunks_to_transfer.push(ChunkToTransfer {
                file_path: file_path.clone(),
                chunk_id: *chunk_id,
                priority: ChunkPriority::Missing,
            });
        }

        // Sort by priority (corrupted first, then missing, then gaps)
        chunks_to_transfer.sort_by_key(|c| c.priority);

        Ok(chunks_to_transfer)
    }

    /// Get temporary file path for a file entry
    fn get_temp_file_path(&self, temp_dir: &PathBuf, file_path: &PathBuf) -> PathBuf {
        // Create a safe filename from the original path
        let file_name = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        temp_dir.join(format!("{}.partial", file_name))
    }
}

/// Result of chunk verification
#[derive(Debug, Clone)]
pub struct ChunkVerificationResult {
    pub verified_chunks: HashSet<(PathBuf, ChunkId)>,
    pub corrupted_chunks: Vec<(PathBuf, ChunkId)>,
    pub missing_chunks: Vec<(PathBuf, ChunkId)>,
    pub total_verified_bytes: u64,
}

/// Chunk gap in file
#[derive(Debug, Clone)]
pub struct ChunkGap {
    pub file_path: PathBuf,
    pub start_chunk: ChunkId,
    pub end_chunk: ChunkId,
}

impl ChunkGap {
    /// Get number of chunks in this gap
    pub fn chunk_count(&self) -> usize {
        (self.end_chunk - self.start_chunk + 1) as usize
    }
}

/// Chunk to transfer with priority
#[derive(Debug, Clone)]
pub struct ChunkToTransfer {
    pub file_path: PathBuf,
    pub chunk_id: ChunkId,
    pub priority: ChunkPriority,
}

/// Priority for chunk transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChunkPriority {
    Corrupted = 0,  // Highest priority
    Missing = 1,
    Gap = 2,        // Lowest priority
}

/// Recovery plan for interrupted transfer
#[derive(Debug, Clone)]
pub struct RecoveryPlan {
    pub verification_result: ChunkVerificationResult,
    pub gaps: Vec<ChunkGap>,
    pub chunks_to_transfer: Vec<ChunkToTransfer>,
    pub bytes_remaining: u64,
    pub resume_from_chunk: ChunkId,
}

impl RecoveryPlan {
    /// Get total number of chunks to transfer
    pub fn total_chunks_to_transfer(&self) -> usize {
        self.chunks_to_transfer.len()
    }

    /// Check if recovery is needed
    pub fn needs_recovery(&self) -> bool {
        !self.chunks_to_transfer.is_empty()
    }

    /// Get progress percentage of verified data
    pub fn verified_percentage(&self, total_size: u64) -> f64 {
        if total_size == 0 {
            0.0
        } else {
            (self.verification_result.total_verified_bytes as f64 / total_size as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;

    async fn create_test_managers() -> (SessionManager, ResumeManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let session_dir = temp_dir.path().join("sessions");
        let resume_dir = temp_dir.path().join("resume");

        let session_manager = SessionManager::new(session_dir);
        session_manager.initialize().await.unwrap();

        let resume_manager = ResumeManager::new(resume_dir);
        resume_manager.initialize().await.unwrap();

        (session_manager, resume_manager, temp_dir)
    }

    fn create_test_manifest() -> TransferManifest {
        let mut manifest = TransferManifest::new("test_peer".to_string());
        manifest.total_size = 1024 * 1024; // 1MB
        manifest.file_count = 1;
        manifest.files.push(FileEntry {
            path: PathBuf::from("/test/file.txt"),
            size: 1024 * 1024,
            checksum: [0u8; 32],
            permissions: FilePermissions::default(),
            modified_at: current_timestamp(),
            chunk_count: 16, // 1MB / 64KB
        });
        manifest
    }

    #[tokio::test]
    async fn test_detect_interrupted_transfers() {
        let (session_manager, resume_manager, _temp_dir) = create_test_managers().await;
        let recovery_manager = RecoveryManager::new(session_manager.clone(), resume_manager);

        let manifest = create_test_manifest();
        let peer_id = "peer1".to_string();

        // Create a session in transferring state
        let session = session_manager
            .create_session(manifest, peer_id.clone(), TransportProtocol::Tcp)
            .await
            .unwrap();

        session_manager
            .update_session_state(session.session_id, TransferState::Negotiating)
            .await
            .unwrap();

        session_manager
            .update_session_state(session.session_id, TransferState::Transferring)
            .await
            .unwrap();

        // Detect interrupted transfers
        let interrupted = recovery_manager
            .detect_interrupted_transfers(&peer_id)
            .await
            .unwrap();

        assert_eq!(interrupted.len(), 1);
        assert_eq!(interrupted[0].session_id, session.session_id);
    }

    #[tokio::test]
    async fn test_can_resume_transfer() {
        let (session_manager, resume_manager, _temp_dir) = create_test_managers().await;
        let recovery_manager = RecoveryManager::new(session_manager, resume_manager.clone());

        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        // Generate resume token with progress
        resume_manager
            .generate_token(transfer_id, session_id)
            .await
            .unwrap();

        resume_manager
            .update_token(transfer_id, None, Some(10), 1024)
            .await
            .unwrap();

        let can_resume = recovery_manager
            .can_resume_transfer(transfer_id)
            .await
            .unwrap();

        assert!(can_resume);
    }

    #[tokio::test]
    async fn test_chunk_gap_detection() {
        let (session_manager, resume_manager, _temp_dir) = create_test_managers().await;
        let recovery_manager = RecoveryManager::new(session_manager, resume_manager);

        let manifest = create_test_manifest();
        let file_entry = &manifest.files[0];

        // Create verification result with some verified chunks
        let mut verified_chunks = HashSet::new();
        verified_chunks.insert((file_entry.path.clone(), 0));
        verified_chunks.insert((file_entry.path.clone(), 1));
        verified_chunks.insert((file_entry.path.clone(), 2));
        // Gap: chunks 3-5
        verified_chunks.insert((file_entry.path.clone(), 6));
        verified_chunks.insert((file_entry.path.clone(), 7));
        // Gap: chunks 8-15

        let verification_result = ChunkVerificationResult {
            verified_chunks,
            corrupted_chunks: Vec::new(),
            missing_chunks: Vec::new(),
            total_verified_bytes: 0,
        };

        let gaps = recovery_manager
            .detect_file_chunk_gaps(file_entry, &verification_result)
            .unwrap();

        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0].start_chunk, 3);
        assert_eq!(gaps[0].end_chunk, 5);
        assert_eq!(gaps[1].start_chunk, 8);
        assert_eq!(gaps[1].end_chunk, 15);
    }

    #[tokio::test]
    async fn test_recovery_plan_creation() {
        let (session_manager, resume_manager, temp_dir) = create_test_managers().await;
        let recovery_manager = RecoveryManager::new(session_manager, resume_manager);

        let manifest = create_test_manifest();
        let resume_position = ResumePosition {
            last_completed_file: None,
            last_completed_chunk: Some(5),
            bytes_completed: 6 * Chunk::DEFAULT_SIZE as u64,
        };

        let temp_transfer_dir = temp_dir.path().join("transfers");
        fs::create_dir_all(&temp_transfer_dir).await.unwrap();

        let recovery_plan = recovery_manager
            .create_recovery_plan(&manifest, &resume_position, &temp_transfer_dir)
            .await
            .unwrap();

        assert_eq!(recovery_plan.resume_from_chunk, 6);
        assert!(recovery_plan.needs_recovery());
    }

    #[tokio::test]
    async fn test_chunk_priority_ordering() {
        let chunks = vec![
            ChunkToTransfer {
                file_path: PathBuf::from("/test/file.txt"),
                chunk_id: 1,
                priority: ChunkPriority::Gap,
            },
            ChunkToTransfer {
                file_path: PathBuf::from("/test/file.txt"),
                chunk_id: 2,
                priority: ChunkPriority::Corrupted,
            },
            ChunkToTransfer {
                file_path: PathBuf::from("/test/file.txt"),
                chunk_id: 3,
                priority: ChunkPriority::Missing,
            },
        ];

        let mut sorted_chunks = chunks.clone();
        sorted_chunks.sort_by_key(|c| c.priority);

        assert_eq!(sorted_chunks[0].priority, ChunkPriority::Corrupted);
        assert_eq!(sorted_chunks[1].priority, ChunkPriority::Missing);
        assert_eq!(sorted_chunks[2].priority, ChunkPriority::Gap);
    }

    #[tokio::test]
    async fn test_resume_position_fresh_start() {
        let position = ResumePosition {
            last_completed_file: None,
            last_completed_chunk: None,
            bytes_completed: 0,
        };

        assert!(position.is_fresh_start());
        assert_eq!(position.next_chunk_id(), 0);
    }
}
