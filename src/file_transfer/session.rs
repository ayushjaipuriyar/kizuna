// Transfer Session Management Module
//
// Handles transfer session lifecycle, state management, and persistence

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

/// Session manager handles transfer session lifecycle and state management
#[derive(Clone)]
pub struct SessionManager {
    /// Active sessions indexed by session ID
    sessions: Arc<RwLock<HashMap<SessionId, TransferSession>>>,
    /// Session persistence directory
    persistence_dir: PathBuf,
}

impl SessionManager {
    /// Create a new session manager with persistence directory
    pub fn new(persistence_dir: PathBuf) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            persistence_dir,
        }
    }

    /// Initialize session manager and load persisted sessions
    pub async fn initialize(&self) -> Result<()> {
        // Create persistence directory if it doesn't exist
        fs::create_dir_all(&self.persistence_dir)
            .await
            .map_err(|e| FileTransferError::IoError {
                path: self.persistence_dir.clone(),
                source: e,
            })?;

        // Load persisted sessions
        self.load_persisted_sessions().await?;

        Ok(())
    }

    /// Create a new transfer session
    pub async fn create_session(
        &self,
        manifest: TransferManifest,
        peer_id: PeerId,
        transport: TransportProtocol,
    ) -> Result<TransferSession> {
        let mut session = TransferSession::new(manifest.clone(), peer_id, transport);
        
        // Initialize progress with manifest data
        session.progress.total_bytes = manifest.total_size;
        session.progress.total_files = manifest.file_count;

        // Store session
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id, session.clone());

        // Persist session
        self.persist_session(&session).await?;

        Ok(session)
    }

    /// Get a session by ID
    pub async fn get_session(&self, session_id: SessionId) -> Result<TransferSession> {
        let sessions = self.sessions.read().await;
        sessions
            .get(&session_id)
            .cloned()
            .ok_or_else(|| FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
    }

    /// Update session state
    pub async fn update_session_state(
        &self,
        session_id: SessionId,
        new_state: TransferState,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            // Validate state transition
            self.validate_state_transition(session.state, new_state)?;
            
            session.state = new_state;
            
            // Persist updated session
            self.persist_session(session).await?;
            
            Ok(())
        } else {
            Err(FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Update session progress
    pub async fn update_session_progress(
        &self,
        session_id: SessionId,
        progress: TransferProgress,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.progress = progress;
            session.progress.last_update = current_timestamp();
            
            // Persist updated session
            self.persist_session(session).await?;
            
            Ok(())
        } else {
            Err(FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Set resume token for a session
    pub async fn set_resume_token(
        &self,
        session_id: SessionId,
        resume_token: ResumeToken,
    ) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&session_id) {
            session.resume_token = Some(resume_token);
            
            // Persist updated session
            self.persist_session(session).await?;
            
            Ok(())
        } else {
            Err(FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Get all active sessions
    pub async fn get_active_sessions(&self) -> Result<Vec<TransferSession>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .values()
            .filter(|s| matches!(
                s.state,
                TransferState::Pending
                    | TransferState::Negotiating
                    | TransferState::Transferring
                    | TransferState::Paused
            ))
            .cloned()
            .collect())
    }

    /// Get all sessions in a specific state
    pub async fn get_sessions_by_state(&self, state: TransferState) -> Result<Vec<TransferSession>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .values()
            .filter(|s| s.state == state)
            .cloned()
            .collect())
    }

    /// Remove a session (cleanup)
    pub async fn remove_session(&self, session_id: SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if sessions.remove(&session_id).is_some() {
            // Remove persisted session file
            self.delete_persisted_session(session_id).await?;
            Ok(())
        } else {
            Err(FileTransferError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Cleanup completed or failed sessions older than specified duration
    pub async fn cleanup_old_sessions(&self, max_age_seconds: u64) -> Result<usize> {
        let current_time = current_timestamp();
        let cutoff_time = current_time.saturating_sub(max_age_seconds);
        
        let mut sessions = self.sessions.write().await;
        let mut removed_count = 0;
        
        // Collect session IDs to remove
        let sessions_to_remove: Vec<SessionId> = sessions
            .iter()
            .filter(|(_, session)| {
                matches!(
                    session.state,
                    TransferState::Completed | TransferState::Failed | TransferState::Cancelled
                ) && session.created_at < cutoff_time
            })
            .map(|(id, _)| *id)
            .collect();
        
        // Remove sessions
        for session_id in sessions_to_remove {
            sessions.remove(&session_id);
            self.delete_persisted_session(session_id).await.ok();
            removed_count += 1;
        }
        
        Ok(removed_count)
    }

    /// Validate state transition
    fn validate_state_transition(
        &self,
        current: TransferState,
        new: TransferState,
    ) -> Result<()> {
        // Define valid state transitions
        let valid = match (current, new) {
            // From Pending
            (TransferState::Pending, TransferState::Negotiating) => true,
            (TransferState::Pending, TransferState::Cancelled) => true,
            
            // From Negotiating
            (TransferState::Negotiating, TransferState::Transferring) => true,
            (TransferState::Negotiating, TransferState::Failed) => true,
            (TransferState::Negotiating, TransferState::Cancelled) => true,
            
            // From Transferring
            (TransferState::Transferring, TransferState::Paused) => true,
            (TransferState::Transferring, TransferState::Completed) => true,
            (TransferState::Transferring, TransferState::Failed) => true,
            (TransferState::Transferring, TransferState::Cancelled) => true,
            
            // From Paused
            (TransferState::Paused, TransferState::Transferring) => true,
            (TransferState::Paused, TransferState::Cancelled) => true,
            
            // Terminal states cannot transition
            (TransferState::Completed, _) => false,
            (TransferState::Failed, _) => false,
            (TransferState::Cancelled, _) => false,
            
            // Same state is always valid (no-op)
            (a, b) if a == b => true,
            
            // All other transitions are invalid
            _ => false,
        };

        if valid {
            Ok(())
        } else {
            Err(FileTransferError::InternalError(format!(
                "Invalid state transition from {:?} to {:?}",
                current, new
            )))
        }
    }

    /// Persist session to disk
    async fn persist_session(&self, session: &TransferSession) -> Result<()> {
        let session_file = self.get_session_file_path(session.session_id);
        
        // Serialize session to JSON
        let session_json = serde_json::to_vec_pretty(session).map_err(|e| {
            FileTransferError::InternalError(format!("Failed to serialize session: {}", e))
        })?;
        
        // Write to file
        let mut file = fs::File::create(&session_file).await.map_err(|e| {
            FileTransferError::IoError {
                path: session_file.clone(),
                source: e,
            }
        })?;
        
        file.write_all(&session_json).await.map_err(|e| {
            FileTransferError::IoError {
                path: session_file.clone(),
                source: e,
            }
        })?;
        
        file.flush().await.map_err(|e| {
            FileTransferError::IoError {
                path: session_file.clone(),
                source: e,
            }
        })?;
        
        Ok(())
    }

    /// Load persisted sessions from disk
    async fn load_persisted_sessions(&self) -> Result<()> {
        // Read all session files from persistence directory
        let mut entries = fs::read_dir(&self.persistence_dir)
            .await
            .map_err(|e| FileTransferError::IoError {
                path: self.persistence_dir.clone(),
                source: e,
            })?;
        
        let mut sessions = self.sessions.write().await;
        
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
            
            // Load and deserialize session
            match self.load_session_from_file(&path).await {
                Ok(session) => {
                    sessions.insert(session.session_id, session);
                }
                Err(e) => {
                    // Log error but continue loading other sessions
                    eprintln!("Failed to load session from {:?}: {}", path, e);
                }
            }
        }
        
        Ok(())
    }

    /// Load a single session from file
    async fn load_session_from_file(&self, path: &PathBuf) -> Result<TransferSession> {
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
        
        let session: TransferSession = serde_json::from_slice(&contents).map_err(|e| {
            FileTransferError::InternalError(format!("Failed to deserialize session: {}", e))
        })?;
        
        Ok(session)
    }

    /// Delete persisted session file
    async fn delete_persisted_session(&self, session_id: SessionId) -> Result<()> {
        let session_file = self.get_session_file_path(session_id);
        
        if session_file.exists() {
            fs::remove_file(&session_file).await.map_err(|e| {
                FileTransferError::IoError {
                    path: session_file,
                    source: e,
                }
            })?;
        }
        
        Ok(())
    }

    /// Get file path for persisted session
    fn get_session_file_path(&self, session_id: SessionId) -> PathBuf {
        self.persistence_dir.join(format!("{}.json", session_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_session_manager() -> (SessionManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = SessionManager::new(temp_dir.path().to_path_buf());
        manager.initialize().await.unwrap();
        (manager, temp_dir)
    }

    fn create_test_manifest() -> TransferManifest {
        TransferManifest::new("test_peer".to_string())
    }

    #[tokio::test]
    async fn test_create_session() {
        let (manager, _temp_dir) = create_test_session_manager().await;
        let manifest = create_test_manifest();
        
        let session = manager
            .create_session(manifest, "peer1".to_string(), TransportProtocol::Tcp)
            .await
            .unwrap();
        
        assert_eq!(session.state, TransferState::Pending);
        assert_eq!(session.peer_id, "peer1");
        assert_eq!(session.transport, TransportProtocol::Tcp);
    }

    #[tokio::test]
    async fn test_get_session() {
        let (manager, _temp_dir) = create_test_session_manager().await;
        let manifest = create_test_manifest();
        
        let session = manager
            .create_session(manifest, "peer1".to_string(), TransportProtocol::Tcp)
            .await
            .unwrap();
        
        let retrieved = manager.get_session(session.session_id).await.unwrap();
        assert_eq!(retrieved.session_id, session.session_id);
    }

    #[tokio::test]
    async fn test_update_session_state() {
        let (manager, _temp_dir) = create_test_session_manager().await;
        let manifest = create_test_manifest();
        
        let session = manager
            .create_session(manifest, "peer1".to_string(), TransportProtocol::Tcp)
            .await
            .unwrap();
        
        manager
            .update_session_state(session.session_id, TransferState::Negotiating)
            .await
            .unwrap();
        
        let updated = manager.get_session(session.session_id).await.unwrap();
        assert_eq!(updated.state, TransferState::Negotiating);
    }

    #[tokio::test]
    async fn test_invalid_state_transition() {
        let (manager, _temp_dir) = create_test_session_manager().await;
        let manifest = create_test_manifest();
        
        let session = manager
            .create_session(manifest, "peer1".to_string(), TransportProtocol::Tcp)
            .await
            .unwrap();
        
        // Try invalid transition: Pending -> Completed
        let result = manager
            .update_session_state(session.session_id, TransferState::Completed)
            .await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_session_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let persistence_dir = temp_dir.path().to_path_buf();
        
        let session_id = {
            let manager = SessionManager::new(persistence_dir.clone());
            manager.initialize().await.unwrap();
            
            let manifest = create_test_manifest();
            let session = manager
                .create_session(manifest, "peer1".to_string(), TransportProtocol::Tcp)
                .await
                .unwrap();
            
            session.session_id
        };
        
        // Create new manager and verify session was loaded
        let manager = SessionManager::new(persistence_dir);
        manager.initialize().await.unwrap();
        
        let loaded_session = manager.get_session(session_id).await.unwrap();
        assert_eq!(loaded_session.session_id, session_id);
    }

    #[tokio::test]
    async fn test_get_active_sessions() {
        let (manager, _temp_dir) = create_test_session_manager().await;
        let manifest = create_test_manifest();
        
        let session1 = manager
            .create_session(manifest.clone(), "peer1".to_string(), TransportProtocol::Tcp)
            .await
            .unwrap();
        
        let session2 = manager
            .create_session(manifest.clone(), "peer2".to_string(), TransportProtocol::Tcp)
            .await
            .unwrap();
        
        manager
            .update_session_state(session2.session_id, TransferState::Negotiating)
            .await
            .unwrap();
        
        let active = manager.get_active_sessions().await.unwrap();
        assert_eq!(active.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_session() {
        let (manager, _temp_dir) = create_test_session_manager().await;
        let manifest = create_test_manifest();
        
        let session = manager
            .create_session(manifest, "peer1".to_string(), TransportProtocol::Tcp)
            .await
            .unwrap();
        
        manager.remove_session(session.session_id).await.unwrap();
        
        let result = manager.get_session(session.session_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cleanup_old_sessions() {
        let (manager, _temp_dir) = create_test_session_manager().await;
        let manifest = create_test_manifest();
        
        let session = manager
            .create_session(manifest, "peer1".to_string(), TransportProtocol::Tcp)
            .await
            .unwrap();
        
        manager
            .update_session_state(session.session_id, TransferState::Negotiating)
            .await
            .unwrap();
        
        manager
            .update_session_state(session.session_id, TransferState::Transferring)
            .await
            .unwrap();
        
        manager
            .update_session_state(session.session_id, TransferState::Completed)
            .await
            .unwrap();
        
        // Cleanup sessions older than 0 seconds (should remove completed session)
        let removed = manager.cleanup_old_sessions(0).await.unwrap();
        assert_eq!(removed, 1);
    }
}
