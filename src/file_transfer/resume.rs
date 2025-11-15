// Resume Token Management Module
//
// Handles resume token generation, validation, and state recovery

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::*,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Resume manager handles resume token lifecycle and validation
#[derive(Clone)]
pub struct ResumeManager {
    /// Active resume tokens indexed by transfer ID
    tokens: Arc<RwLock<HashMap<TransferId, ResumeToken>>>,
    /// Resume token persistence directory
    persistence_dir: PathBuf,
}

impl ResumeManager {
    /// Create a new resume manager with persistence directory
    pub fn new(persistence_dir: PathBuf) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            persistence_dir,
        }
    }

    /// Initialize resume manager and load persisted tokens
    pub async fn initialize(&self) -> Result<()> {
        // Create persistence directory if it doesn't exist
        fs::create_dir_all(&self.persistence_dir)
            .await
            .map_err(|e| FileTransferError::IoError {
                path: self.persistence_dir.clone(),
                source: e,
            })?;

        // Load persisted resume tokens
        self.load_persisted_tokens().await?;

        // Clean up expired tokens
        self.cleanup_expired_tokens().await?;

        Ok(())
    }

    /// Generate a new resume token for a transfer
    pub async fn generate_token(
        &self,
        transfer_id: TransferId,
        session_id: SessionId,
    ) -> Result<ResumeToken> {
        let token = ResumeToken::new(transfer_id, session_id);

        // Store token
        let mut tokens = self.tokens.write().await;
        tokens.insert(transfer_id, token.clone());

        // Persist token
        self.persist_token(&token).await?;

        Ok(token)
    }

    /// Update resume token with progress information
    pub async fn update_token(
        &self,
        transfer_id: TransferId,
        last_completed_file: Option<PathBuf>,
        last_completed_chunk: Option<ChunkId>,
        bytes_completed: u64,
    ) -> Result<()> {
        let mut tokens = self.tokens.write().await;

        if let Some(token) = tokens.get_mut(&transfer_id) {
            token.last_completed_file = last_completed_file;
            token.last_completed_chunk = last_completed_chunk;
            token.bytes_completed = bytes_completed;

            // Persist updated token
            self.persist_token(token).await?;

            Ok(())
        } else {
            Err(FileTransferError::InvalidResumeToken {
                reason: format!("Resume token not found for transfer {}", transfer_id),
            })
        }
    }

    /// Get resume token for a transfer
    pub async fn get_token(&self, transfer_id: TransferId) -> Result<ResumeToken> {
        let tokens = self.tokens.read().await;
        tokens
            .get(&transfer_id)
            .cloned()
            .ok_or_else(|| FileTransferError::InvalidResumeToken {
                reason: format!("Resume token not found for transfer {}", transfer_id),
            })
    }

    /// Validate a resume token
    pub async fn validate_token(&self, token: &ResumeToken) -> Result<bool> {
        // Check if token is expired
        if token.is_expired() {
            return Err(FileTransferError::ResumeTokenExpired);
        }

        // Check if token exists in storage
        let tokens = self.tokens.read().await;
        if !tokens.contains_key(&token.transfer_id) {
            return Err(FileTransferError::InvalidResumeToken {
                reason: "Token not found in storage".to_string(),
            });
        }

        // Verify token integrity
        let stored_token = tokens.get(&token.transfer_id).unwrap();
        if stored_token.session_id != token.session_id {
            return Err(FileTransferError::InvalidResumeToken {
                reason: "Session ID mismatch".to_string(),
            });
        }

        Ok(true)
    }

    /// Check if a transfer can be resumed
    pub async fn can_resume(&self, transfer_id: TransferId) -> Result<bool> {
        let tokens = self.tokens.read().await;

        if let Some(token) = tokens.get(&transfer_id) {
            // Check if token is expired
            if token.is_expired() {
                return Ok(false);
            }

            // Check if there's any progress to resume from
            Ok(token.bytes_completed > 0 || token.last_completed_chunk.is_some())
        } else {
            Ok(false)
        }
    }

    /// Get resume position from token
    pub async fn get_resume_position(
        &self,
        transfer_id: TransferId,
    ) -> Result<ResumePosition> {
        let token = self.get_token(transfer_id).await?;

        // Validate token
        self.validate_token(&token).await?;

        Ok(ResumePosition {
            last_completed_file: token.last_completed_file,
            last_completed_chunk: token.last_completed_chunk,
            bytes_completed: token.bytes_completed,
        })
    }

    /// Remove resume token (after successful completion or cancellation)
    pub async fn remove_token(&self, transfer_id: TransferId) -> Result<()> {
        let mut tokens = self.tokens.write().await;

        if tokens.remove(&transfer_id).is_some() {
            // Remove persisted token file
            self.delete_persisted_token(transfer_id).await?;
            Ok(())
        } else {
            Err(FileTransferError::InvalidResumeToken {
                reason: format!("Resume token not found for transfer {}", transfer_id),
            })
        }
    }

    /// Cleanup expired resume tokens
    pub async fn cleanup_expired_tokens(&self) -> Result<usize> {
        let current_time = current_timestamp();
        let mut tokens = self.tokens.write().await;
        let mut removed_count = 0;

        // Collect expired token IDs
        let expired_ids: Vec<TransferId> = tokens
            .iter()
            .filter(|(_, token)| token.expires_at < current_time)
            .map(|(id, _)| *id)
            .collect();

        // Remove expired tokens
        for transfer_id in expired_ids {
            tokens.remove(&transfer_id);
            self.delete_persisted_token(transfer_id).await.ok();
            removed_count += 1;
        }

        Ok(removed_count)
    }

    /// Get all active resume tokens
    pub async fn get_all_tokens(&self) -> Result<Vec<ResumeToken>> {
        let tokens = self.tokens.read().await;
        Ok(tokens.values().cloned().collect())
    }

    /// Persist resume token to disk
    async fn persist_token(&self, token: &ResumeToken) -> Result<()> {
        let token_file = self.get_token_file_path(token.transfer_id);

        // Serialize token to JSON
        let token_json = serde_json::to_vec_pretty(token).map_err(|e| {
            FileTransferError::InternalError(format!("Failed to serialize resume token: {}", e))
        })?;

        // Write to file
        let mut file = fs::File::create(&token_file).await.map_err(|e| {
            FileTransferError::IoError {
                path: token_file.clone(),
                source: e,
            }
        })?;

        file.write_all(&token_json).await.map_err(|e| {
            FileTransferError::IoError {
                path: token_file.clone(),
                source: e,
            }
        })?;

        file.flush().await.map_err(|e| {
            FileTransferError::IoError {
                path: token_file.clone(),
                source: e,
            }
        })?;

        Ok(())
    }

    /// Load persisted resume tokens from disk
    async fn load_persisted_tokens(&self) -> Result<()> {
        // Read all token files from persistence directory
        let mut entries = fs::read_dir(&self.persistence_dir)
            .await
            .map_err(|e| FileTransferError::IoError {
                path: self.persistence_dir.clone(),
                source: e,
            })?;

        let mut tokens = self.tokens.write().await;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            FileTransferError::IoError {
                path: self.persistence_dir.clone(),
                source: e,
            }
        })? {
            let path = entry.path();

            // Only process .json files
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            // Load and deserialize token
            match self.load_token_from_file(&path).await {
                Ok(token) => {
                    // Only load non-expired tokens
                    if !token.is_expired() {
                        tokens.insert(token.transfer_id, token);
                    } else {
                        // Delete expired token file
                        fs::remove_file(&path).await.ok();
                    }
                }
                Err(e) => {
                    // Log error but continue loading other tokens
                    eprintln!("Failed to load resume token from {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    /// Load a single resume token from file
    async fn load_token_from_file(&self, path: &PathBuf) -> Result<ResumeToken> {
        let mut file = fs::File::open(path).await.map_err(|e| {
            FileTransferError::IoError {
                path: path.clone(),
                source: e,
            }
        })?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await.map_err(|e| {
            FileTransferError::IoError {
                path: path.clone(),
                source: e,
            }
        })?;

        let token: ResumeToken = serde_json::from_slice(&contents).map_err(|e| {
            FileTransferError::InternalError(format!("Failed to deserialize resume token: {}", e))
        })?;

        Ok(token)
    }

    /// Delete persisted resume token file
    async fn delete_persisted_token(&self, transfer_id: TransferId) -> Result<()> {
        let token_file = self.get_token_file_path(transfer_id);

        if token_file.exists() {
            fs::remove_file(&token_file).await.map_err(|e| {
                FileTransferError::IoError {
                    path: token_file,
                    source: e,
                }
            })?;
        }

        Ok(())
    }

    /// Get file path for persisted resume token
    fn get_token_file_path(&self, transfer_id: TransferId) -> PathBuf {
        self.persistence_dir
            .join(format!("resume_{}.json", transfer_id))
    }
}

/// Resume position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumePosition {
    pub last_completed_file: Option<PathBuf>,
    pub last_completed_chunk: Option<ChunkId>,
    pub bytes_completed: u64,
}

impl ResumePosition {
    /// Check if this is a fresh start (no progress)
    pub fn is_fresh_start(&self) -> bool {
        self.bytes_completed == 0 && self.last_completed_chunk.is_none()
    }

    /// Get the next chunk to transfer
    pub fn next_chunk_id(&self) -> ChunkId {
        self.last_completed_chunk.map(|id| id + 1).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_resume_manager() -> (ResumeManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = ResumeManager::new(temp_dir.path().to_path_buf());
        manager.initialize().await.unwrap();
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_generate_token() {
        let (manager, _temp_dir) = create_test_resume_manager().await;
        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let token = manager
            .generate_token(transfer_id, session_id)
            .await
            .unwrap();

        assert_eq!(token.transfer_id, transfer_id);
        assert_eq!(token.session_id, session_id);
        assert!(!token.is_expired());
    }

    #[tokio::test]
    async fn test_get_token() {
        let (manager, _temp_dir) = create_test_resume_manager().await;
        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        manager
            .generate_token(transfer_id, session_id)
            .await
            .unwrap();

        let retrieved = manager.get_token(transfer_id).await.unwrap();
        assert_eq!(retrieved.transfer_id, transfer_id);
    }

    #[tokio::test]
    async fn test_update_token() {
        let (manager, _temp_dir) = create_test_resume_manager().await;
        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        manager
            .generate_token(transfer_id, session_id)
            .await
            .unwrap();

        manager
            .update_token(
                transfer_id,
                Some(PathBuf::from("/test/file.txt")),
                Some(42),
                1024,
            )
            .await
            .unwrap();

        let token = manager.get_token(transfer_id).await.unwrap();
        assert_eq!(token.last_completed_chunk, Some(42));
        assert_eq!(token.bytes_completed, 1024);
    }

    #[tokio::test]
    async fn test_validate_token() {
        let (manager, _temp_dir) = create_test_resume_manager().await;
        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let token = manager
            .generate_token(transfer_id, session_id)
            .await
            .unwrap();

        let is_valid = manager.validate_token(&token).await.unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_expired_token() {
        let (manager, _temp_dir) = create_test_resume_manager().await;
        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let mut token = ResumeToken::new(transfer_id, session_id);
        token.expires_at = current_timestamp() - 1; // Expired 1 second ago

        let mut tokens = manager.tokens.write().await;
        tokens.insert(transfer_id, token.clone());
        drop(tokens);

        let result = manager.validate_token(&token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_can_resume() {
        let (manager, _temp_dir) = create_test_resume_manager().await;
        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        manager
            .generate_token(transfer_id, session_id)
            .await
            .unwrap();

        // Fresh token with no progress
        let can_resume = manager.can_resume(transfer_id).await.unwrap();
        assert!(!can_resume);

        // Update with progress
        manager
            .update_token(transfer_id, None, Some(10), 1024)
            .await
            .unwrap();

        let can_resume = manager.can_resume(transfer_id).await.unwrap();
        assert!(can_resume);
    }

    #[tokio::test]
    async fn test_get_resume_position() {
        let (manager, _temp_dir) = create_test_resume_manager().await;
        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        manager
            .generate_token(transfer_id, session_id)
            .await
            .unwrap();

        manager
            .update_token(
                transfer_id,
                Some(PathBuf::from("/test/file.txt")),
                Some(42),
                2048,
            )
            .await
            .unwrap();

        let position = manager.get_resume_position(transfer_id).await.unwrap();
        assert_eq!(position.last_completed_chunk, Some(42));
        assert_eq!(position.bytes_completed, 2048);
        assert_eq!(position.next_chunk_id(), 43);
    }

    #[tokio::test]
    async fn test_remove_token() {
        let (manager, _temp_dir) = create_test_resume_manager().await;
        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        manager
            .generate_token(transfer_id, session_id)
            .await
            .unwrap();

        manager.remove_token(transfer_id).await.unwrap();

        let result = manager.get_token(transfer_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_token_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let persistence_dir = temp_dir.path().to_path_buf();

        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        {
            let manager = ResumeManager::new(persistence_dir.clone());
            manager.initialize().await.unwrap();

            manager
                .generate_token(transfer_id, session_id)
                .await
                .unwrap();

            manager
                .update_token(transfer_id, None, Some(100), 5000)
                .await
                .unwrap();
        }

        // Create new manager and verify token was loaded
        let manager = ResumeManager::new(persistence_dir);
        manager.initialize().await.unwrap();

        let loaded_token = manager.get_token(transfer_id).await.unwrap();
        assert_eq!(loaded_token.transfer_id, transfer_id);
        assert_eq!(loaded_token.last_completed_chunk, Some(100));
        assert_eq!(loaded_token.bytes_completed, 5000);
    }

    #[tokio::test]
    async fn test_cleanup_expired_tokens() {
        let (manager, _temp_dir) = create_test_resume_manager().await;
        let transfer_id = Uuid::new_v4();
        let session_id = Uuid::new_v4();

        let mut token = ResumeToken::new(transfer_id, session_id);
        token.expires_at = current_timestamp() - 1; // Expired

        let mut tokens = manager.tokens.write().await;
        tokens.insert(transfer_id, token);
        drop(tokens);

        let removed = manager.cleanup_expired_tokens().await.unwrap();
        assert_eq!(removed, 1);

        let result = manager.get_token(transfer_id).await;
        assert!(result.is_err());
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

    #[tokio::test]
    async fn test_resume_position_with_progress() {
        let position = ResumePosition {
            last_completed_file: Some(PathBuf::from("/test/file.txt")),
            last_completed_chunk: Some(50),
            bytes_completed: 3200,
        };

        assert!(!position.is_fresh_start());
        assert_eq!(position.next_chunk_id(), 51);
    }
}
