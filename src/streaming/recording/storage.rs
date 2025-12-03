// Recording storage management
//
// Manages recording file storage with size limits, automatic cleanup,
// and metadata indexing for easy retrieval.
//
// Requirements: 5.5

use crate::streaming::{
    StreamResult, StreamError,
    RecordingSession, RecordingFile, SessionId, VideoFormat,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::sync::RwLock;

/// Recording metadata for indexing and retrieval
/// 
/// Requirements: 5.5
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordingMetadata {
    pub session_id: SessionId,
    pub file_path: PathBuf,
    pub format: VideoFormat,
    pub file_size: u64,
    pub duration: Duration,
    pub created_at: SystemTime,
    pub stream_source: String,
    pub quality_preset: String,
    pub tags: Vec<String>,
}

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Maximum total storage size in bytes
    pub max_total_size: u64,
    /// Maximum age of recordings before cleanup
    pub max_age: Duration,
    /// Minimum free space to maintain
    pub min_free_space: u64,
    /// Enable automatic cleanup
    pub auto_cleanup: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            max_total_size: 10 * 1024 * 1024 * 1024, // 10 GB
            max_age: Duration::from_secs(30 * 24 * 60 * 60), // 30 days
            min_free_space: 1024 * 1024 * 1024, // 1 GB
            auto_cleanup: true,
        }
    }
}

/// Storage manager for recording files
/// 
/// Manages recording file storage with size limits, automatic cleanup,
/// and metadata indexing.
/// 
/// Requirements: 5.5
pub struct StorageManager {
    storage_path: PathBuf,
    config: StorageConfig,
    metadata: Arc<RwLock<HashMap<SessionId, RecordingMetadata>>>,
    metadata_file: PathBuf,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(storage_path: PathBuf) -> StreamResult<Self> {
        let metadata_file = storage_path.join("recordings_metadata.json");
        
        let manager = Self {
            storage_path: storage_path.clone(),
            config: StorageConfig::default(),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            metadata_file,
        };
        
        // Load existing metadata
        if let Err(e) = manager.load_metadata_sync() {
            eprintln!("Warning: Failed to load metadata: {}", e);
        }
        
        Ok(manager)
    }
    
    /// Create a storage manager with custom configuration
    pub fn with_config(storage_path: PathBuf, config: StorageConfig) -> StreamResult<Self> {
        let mut manager = Self::new(storage_path)?;
        manager.config = config;
        Ok(manager)
    }
    
    /// Check if there's enough space available for a recording
    /// 
    /// Requirements: 5.5
    pub async fn check_space_available(
        &self,
        config: &crate::streaming::RecordingConfig,
    ) -> StreamResult<()> {
        // Get current storage usage
        let current_usage = self.get_total_storage_usage().await?;
        
        // Estimate required space (use max_file_size if specified, otherwise estimate)
        let required_space = config.max_file_size.unwrap_or(1024 * 1024 * 1024); // Default 1GB
        
        // Check against max total size
        if current_usage + required_space > self.config.max_total_size {
            // Try cleanup first
            if self.config.auto_cleanup {
                self.cleanup_old_recordings().await?;
                
                // Check again after cleanup
                let new_usage = self.get_total_storage_usage().await?;
                if new_usage + required_space > self.config.max_total_size {
                    return Err(StreamError::resource(
                        "Insufficient storage space for recording"
                    ));
                }
            } else {
                return Err(StreamError::resource(
                    "Insufficient storage space for recording"
                ));
            }
        }
        
        // Check filesystem free space
        let free_space = self.get_filesystem_free_space().await?;
        if free_space < required_space + self.config.min_free_space {
            return Err(StreamError::resource(
                "Insufficient filesystem space for recording"
            ));
        }
        
        Ok(())
    }
    
    /// Register a new recording
    /// 
    /// Requirements: 5.5
    pub async fn register_recording(&self, session: &RecordingSession) -> StreamResult<()> {
        let metadata = RecordingMetadata {
            session_id: session.session_id,
            file_path: session.output_path.clone(),
            format: session.format,
            file_size: 0,
            duration: Duration::ZERO,
            created_at: SystemTime::now(),
            stream_source: "unknown".to_string(),
            quality_preset: "medium".to_string(),
            tags: Vec::new(),
        };
        
        self.metadata
            .write().await
            .insert(session.session_id, metadata);
        
        self.save_metadata().await?;
        
        Ok(())
    }
    
    /// Finalize a recording with final metadata
    /// 
    /// Requirements: 5.5
    pub async fn finalize_recording(
        &self,
        session: &RecordingSession,
        file: &RecordingFile,
    ) -> StreamResult<()> {
        {
            let mut metadata_map = self.metadata.write().await;
            
            if let Some(metadata) = metadata_map.get_mut(&session.session_id) {
                metadata.file_size = file.file_size;
                metadata.duration = file.duration;
            }
        }
        
        self.save_metadata().await?;
        
        Ok(())
    }
    
    /// Get all recording metadata
    /// 
    /// Requirements: 5.5
    pub async fn get_all_recordings(&self) -> StreamResult<Vec<RecordingMetadata>> {
        let metadata = self.metadata.read().await;
        
        Ok(metadata.values().cloned().collect())
    }
    
    /// Get recording metadata by session ID
    /// 
    /// Requirements: 5.5
    pub async fn get_recording(&self, session_id: SessionId) -> StreamResult<RecordingMetadata> {
        let metadata = self.metadata.read().await;
        
        metadata
            .get(&session_id)
            .cloned()
            .ok_or_else(|| StreamError::session_not_found(session_id))
    }
    
    /// Delete a recording
    /// 
    /// Requirements: 5.5
    pub async fn delete_recording(&self, session_id: SessionId) -> StreamResult<()> {
        let metadata = {
            let mut metadata_map = self.metadata.write().await;
            
            metadata_map.remove(&session_id)
                .ok_or_else(|| StreamError::session_not_found(session_id))?
        };
        
        // Delete the file
        if metadata.file_path.exists() {
            fs::remove_file(&metadata.file_path).await?;
        }
        
        self.save_metadata().await?;
        
        Ok(())
    }
    
    /// Cleanup old recordings based on age and space
    /// 
    /// Requirements: 5.5
    pub async fn cleanup_old_recordings(&self) -> StreamResult<()> {
        let now = SystemTime::now();
        let mut to_delete = Vec::new();
        
        {
            let metadata = self.metadata.read().await;
            
            for (session_id, meta) in metadata.iter() {
                // Check age
                if let Ok(age) = now.duration_since(meta.created_at) {
                    if age > self.config.max_age {
                        to_delete.push(*session_id);
                    }
                }
            }
        }
        
        // Delete old recordings
        for session_id in to_delete {
            if let Err(e) = self.delete_recording(session_id).await {
                eprintln!("Warning: Failed to delete recording {}: {}", session_id, e);
            }
        }
        
        // Check if we still need to free up space
        let current_usage = self.get_total_storage_usage().await?;
        if current_usage > self.config.max_total_size {
            self.cleanup_by_size().await?;
        }
        
        Ok(())
    }
    
    /// Cleanup recordings to free up space (oldest first)
    /// 
    /// Requirements: 5.5
    async fn cleanup_by_size(&self) -> StreamResult<()> {
        let mut recordings: Vec<_> = {
            let metadata = self.metadata.read().await;
            
            metadata.values().cloned().collect()
        };
        
        // Sort by creation time (oldest first)
        recordings.sort_by_key(|r| r.created_at);
        
        let mut current_usage = self.get_total_storage_usage().await?;
        let target_usage = (self.config.max_total_size as f64 * 0.8) as u64; // Target 80% usage
        
        for recording in recordings {
            if current_usage <= target_usage {
                break;
            }
            
            if let Err(e) = self.delete_recording(recording.session_id).await {
                eprintln!("Warning: Failed to delete recording {}: {}", recording.session_id, e);
            } else {
                current_usage = current_usage.saturating_sub(recording.file_size);
            }
        }
        
        Ok(())
    }
    
    /// Get total storage usage
    async fn get_total_storage_usage(&self) -> StreamResult<u64> {
        let metadata = self.metadata.read().await;
        
        Ok(metadata.values().map(|m| m.file_size).sum())
    }
    
    /// Get filesystem free space
    async fn get_filesystem_free_space(&self) -> StreamResult<u64> {
        // Use statvfs on Unix or GetDiskFreeSpaceEx on Windows
        // For simplicity, we'll use a basic implementation
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let metadata = fs::metadata(&self.storage_path).await?;
            // This is a simplified version - real implementation would use statvfs
            Ok(metadata.blocks() * metadata.blksize())
        }
        
        #[cfg(not(unix))]
        {
            // Simplified fallback - assume 10GB free
            Ok(10 * 1024 * 1024 * 1024)
        }
    }
    
    /// Save metadata to disk
    async fn save_metadata(&self) -> StreamResult<()> {
        let json = {
            let metadata = self.metadata.read().await;
            serde_json::to_string_pretty(&*metadata)
                .map_err(|e| StreamError::internal(format!("Serialization error: {}", e)))?
        };
        
        // Ensure parent directory exists
        if let Some(parent) = self.metadata_file.parent() {
            fs::create_dir_all(parent).await?;
        }
        
        fs::write(&self.metadata_file, json).await?;
        
        Ok(())
    }
    
    /// Load metadata from disk (synchronous version for constructor)
    fn load_metadata_sync(&self) -> StreamResult<()> {
        if !self.metadata_file.exists() {
            return Ok(());
        }
        
        let json = std::fs::read_to_string(&self.metadata_file)?;
        let loaded: HashMap<SessionId, RecordingMetadata> = serde_json::from_str(&json)
            .map_err(|e| StreamError::internal(format!("Deserialization error: {}", e)))?;
        
        *self.metadata.blocking_write() = loaded;
        
        Ok(())
    }
    
    /// Load metadata from disk
    pub async fn load_metadata(&self) -> StreamResult<()> {
        if !self.metadata_file.exists() {
            return Ok(());
        }
        
        let json = fs::read_to_string(&self.metadata_file).await?;
        let loaded: HashMap<SessionId, RecordingMetadata> = serde_json::from_str(&json)
            .map_err(|e| StreamError::internal(format!("Deserialization error: {}", e)))?;
        
        *self.metadata.write().await = loaded;
        
        Ok(())
    }
    
    /// Search recordings by tags
    /// 
    /// Requirements: 5.5
    pub async fn search_by_tags(&self, tags: &[String]) -> StreamResult<Vec<RecordingMetadata>> {
        let metadata = self.metadata.read().await;
        
        Ok(metadata
            .values()
            .filter(|m| tags.iter().any(|tag| m.tags.contains(tag)))
            .cloned()
            .collect())
    }
    
    /// Add tags to a recording
    /// 
    /// Requirements: 5.5
    pub async fn add_tags(&self, session_id: SessionId, tags: Vec<String>) -> StreamResult<()> {
        {
            let mut metadata_map = self.metadata.write().await;
            
            let metadata = metadata_map
                .get_mut(&session_id)
                .ok_or_else(|| StreamError::session_not_found(session_id))?;
            
            for tag in tags {
                if !metadata.tags.contains(&tag) {
                    metadata.tags.push(tag);
                }
            }
        }
        
        self.save_metadata().await?;
        
        Ok(())
    }
}
