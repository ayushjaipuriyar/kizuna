// Parallel Stream Management Module
//
// Handles multiple parallel streams for file transfers between peer pairs

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::*,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum number of parallel streams allowed between peer pairs
pub const MAX_PARALLEL_STREAMS: usize = 4;

/// Parallel stream manager coordinates multiple streams for file transfers
#[derive(Clone)]
pub struct ParallelStreamManager {
    state: Arc<RwLock<StreamManagerState>>,
}

/// Internal state for stream management
struct StreamManagerState {
    /// Active streams per peer
    peer_streams: HashMap<PeerId, Vec<StreamInfo>>,
    /// Maximum parallel streams per peer
    max_streams_per_peer: usize,
    /// File distribution strategy
    distribution_strategy: DistributionStrategy,
}

/// Information about an active stream
#[derive(Debug, Clone)]
struct StreamInfo {
    stream_id: StreamId,
    peer_id: PeerId,
    state: StreamState,
    bytes_transferred: u64,
    files_assigned: Vec<FileEntry>,
    current_file: Option<FileEntry>,
}

/// Stream identifier
pub type StreamId = uuid::Uuid;

/// Stream state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StreamState {
    Idle,
    Active,
    Paused,
    Closed,
}

/// Distribution strategy for files across streams
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DistributionStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Distribute based on file size (balance load)
    LoadBalanced,
    /// Assign largest files first
    LargestFirst,
}

impl ParallelStreamManager {
    /// Create a new parallel stream manager with default settings
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(StreamManagerState {
                peer_streams: HashMap::new(),
                max_streams_per_peer: MAX_PARALLEL_STREAMS,
                distribution_strategy: DistributionStrategy::LoadBalanced,
            })),
        }
    }

    /// Create a parallel stream manager with custom max streams
    pub fn with_max_streams(max_streams: usize) -> Self {
        let max_streams = max_streams.min(MAX_PARALLEL_STREAMS);
        Self {
            state: Arc::new(RwLock::new(StreamManagerState {
                peer_streams: HashMap::new(),
                max_streams_per_peer: max_streams,
                distribution_strategy: DistributionStrategy::LoadBalanced,
            })),
        }
    }

    /// Set distribution strategy
    pub async fn set_distribution_strategy(&self, strategy: DistributionStrategy) -> Result<()> {
        let mut state = self.state.write().await;
        state.distribution_strategy = strategy;
        Ok(())
    }

    /// Get current distribution strategy
    pub async fn get_distribution_strategy(&self) -> DistributionStrategy {
        let state = self.state.read().await;
        state.distribution_strategy
    }

    /// Register a new stream for a peer
    pub async fn register_stream(&self, peer_id: PeerId) -> Result<StreamId> {
        let mut state = self.state.write().await;

        // Check if we've reached max streams for this peer
        let current_streams = state
            .peer_streams
            .get(&peer_id)
            .map(|v| v.len())
            .unwrap_or(0);

        if current_streams >= state.max_streams_per_peer {
            return Err(FileTransferError::InternalError(format!(
                "Maximum parallel streams ({}) reached for peer {}",
                state.max_streams_per_peer, peer_id
            )));
        }

        // Create new stream
        let stream_id = uuid::Uuid::new_v4();
        let stream_info = StreamInfo {
            stream_id,
            peer_id: peer_id.clone(),
            state: StreamState::Idle,
            bytes_transferred: 0,
            files_assigned: Vec::new(),
            current_file: None,
        };

        // Add to peer streams
        state
            .peer_streams
            .entry(peer_id)
            .or_insert_with(Vec::new)
            .push(stream_info);

        Ok(stream_id)
    }

    /// Unregister a stream
    pub async fn unregister_stream(&self, stream_id: StreamId) -> Result<()> {
        let mut state = self.state.write().await;

        // Find and remove the stream
        for streams in state.peer_streams.values_mut() {
            if let Some(pos) = streams.iter().position(|s| s.stream_id == stream_id) {
                streams.remove(pos);
                return Ok(());
            }
        }

        Err(FileTransferError::InternalError(format!(
            "Stream not found: {}",
            stream_id
        )))
    }

    /// Distribute files across available streams for a peer
    pub async fn distribute_files(
        &self,
        peer_id: &PeerId,
        files: Vec<FileEntry>,
    ) -> Result<HashMap<StreamId, Vec<FileEntry>>> {
        let mut state = self.state.write().await;

        // Check if peer has streams
        if !state.peer_streams.contains_key(peer_id) {
            return Err(FileTransferError::InternalError(format!("No streams found for peer {}", peer_id)));
        }

        let streams = state.peer_streams.get(peer_id).unwrap();
        if streams.is_empty() {
            return Err(FileTransferError::InternalError(format!(
                "No active streams for peer {}",
                peer_id
            )));
        }

        // Get distribution strategy and clone streams data for distribution
        let distribution_strategy = state.distribution_strategy;
        let streams_clone = streams.clone();
        
        // Distribute files based on strategy (using cloned data)
        let distribution = match distribution_strategy {
            DistributionStrategy::RoundRobin => {
                self.distribute_round_robin(&streams_clone, files)
            }
            DistributionStrategy::LoadBalanced => {
                self.distribute_load_balanced(&streams_clone, files)
            }
            DistributionStrategy::LargestFirst => {
                self.distribute_largest_first(&streams_clone, files)
            }
        };

        // Update stream assignments in the actual state
        if let Some(streams_mut) = state.peer_streams.get_mut(peer_id) {
            for (stream_id, assigned_files) in &distribution {
                if let Some(stream) = streams_mut.iter_mut().find(|s| s.stream_id == *stream_id) {
                    stream.files_assigned.extend(assigned_files.clone());
                }
            }
        }

        Ok(distribution)
    }

    /// Round-robin distribution
    fn distribute_round_robin(
        &self,
        streams: &[StreamInfo],
        files: Vec<FileEntry>,
    ) -> HashMap<StreamId, Vec<FileEntry>> {
        let mut distribution: HashMap<StreamId, Vec<FileEntry>> = HashMap::new();
        
        for stream in streams {
            distribution.insert(stream.stream_id, Vec::new());
        }

        for (i, file) in files.into_iter().enumerate() {
            let stream_index = i % streams.len();
            let stream_id = streams[stream_index].stream_id;
            distribution.get_mut(&stream_id).unwrap().push(file);
        }

        distribution
    }

    /// Load-balanced distribution (balance by total file size)
    fn distribute_load_balanced(
        &self,
        streams: &[StreamInfo],
        mut files: Vec<FileEntry>,
    ) -> HashMap<StreamId, Vec<FileEntry>> {
        // Sort files by size (largest first)
        files.sort_by(|a, b| b.size.cmp(&a.size));

        // Track load per stream
        let mut stream_loads: Vec<(StreamId, u64)> = streams
            .iter()
            .map(|s| (s.stream_id, s.bytes_transferred))
            .collect();

        let mut distribution: HashMap<StreamId, Vec<FileEntry>> = HashMap::new();
        for stream in streams {
            distribution.insert(stream.stream_id, Vec::new());
        }

        // Assign each file to the stream with the least load
        for file in files {
            // Find stream with minimum load
            stream_loads.sort_by_key(|(_, load)| *load);
            let (stream_id, load) = &mut stream_loads[0];
            
            // Assign file to this stream
            distribution.get_mut(stream_id).unwrap().push(file.clone());
            
            // Update load
            *load += file.size;
        }

        distribution
    }

    /// Largest-first distribution
    fn distribute_largest_first(
        &self,
        streams: &[StreamInfo],
        mut files: Vec<FileEntry>,
    ) -> HashMap<StreamId, Vec<FileEntry>> {
        // Sort files by size (largest first)
        files.sort_by(|a, b| b.size.cmp(&a.size));

        let mut distribution: HashMap<StreamId, Vec<FileEntry>> = HashMap::new();
        for stream in streams {
            distribution.insert(stream.stream_id, Vec::new());
        }

        // Assign files round-robin (but sorted by size)
        for (i, file) in files.into_iter().enumerate() {
            let stream_index = i % streams.len();
            let stream_id = streams[stream_index].stream_id;
            distribution.get_mut(&stream_id).unwrap().push(file);
        }

        distribution
    }

    /// Update stream state
    pub async fn update_stream_state(
        &self,
        stream_id: StreamId,
        new_state: StreamState,
    ) -> Result<()> {
        let mut state = self.state.write().await;

        for streams in state.peer_streams.values_mut() {
            if let Some(stream) = streams.iter_mut().find(|s| s.stream_id == stream_id) {
                stream.state = new_state;
                return Ok(());
            }
        }

        Err(FileTransferError::InternalError(format!(
            "Stream not found: {}",
            stream_id
        )))
    }

    /// Record bytes transferred on a stream
    pub async fn record_bytes(&self, stream_id: StreamId, bytes: u64) -> Result<()> {
        let mut state = self.state.write().await;

        for streams in state.peer_streams.values_mut() {
            if let Some(stream) = streams.iter_mut().find(|s| s.stream_id == stream_id) {
                stream.bytes_transferred += bytes;
                return Ok(());
            }
        }

        Err(FileTransferError::InternalError(format!(
            "Stream not found: {}",
            stream_id
        )))
    }

    /// Set current file being transferred on a stream
    pub async fn set_current_file(
        &self,
        stream_id: StreamId,
        file: Option<FileEntry>,
    ) -> Result<()> {
        let mut state = self.state.write().await;

        for streams in state.peer_streams.values_mut() {
            if let Some(stream) = streams.iter_mut().find(|s| s.stream_id == stream_id) {
                stream.current_file = file;
                return Ok(());
            }
        }

        Err(FileTransferError::InternalError(format!(
            "Stream not found: {}",
            stream_id
        )))
    }

    /// Get stream statistics for a peer
    pub async fn get_peer_stream_stats(&self, peer_id: &PeerId) -> Result<Vec<StreamStats>> {
        let state = self.state.read().await;

        let streams = state
            .peer_streams
            .get(peer_id)
            .ok_or_else(|| {
                FileTransferError::InternalError(format!("No streams found for peer {}", peer_id))
            })?;

        Ok(streams
            .iter()
            .map(|s| StreamStats {
                stream_id: s.stream_id,
                state: s.state,
                bytes_transferred: s.bytes_transferred,
                files_assigned: s.files_assigned.len(),
                current_file: s.current_file.as_ref().map(|f| f.path.clone()),
            })
            .collect())
    }

    /// Get total number of active streams
    pub async fn active_stream_count(&self) -> usize {
        let state = self.state.read().await;
        state
            .peer_streams
            .values()
            .map(|streams| streams.iter().filter(|s| s.state == StreamState::Active).count())
            .sum()
    }

    /// Get total number of streams for a peer
    pub async fn peer_stream_count(&self, peer_id: &PeerId) -> usize {
        let state = self.state.read().await;
        state
            .peer_streams
            .get(peer_id)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Close all streams for a peer
    pub async fn close_peer_streams(&self, peer_id: &PeerId) -> Result<()> {
        let mut state = self.state.write().await;
        
        if let Some(streams) = state.peer_streams.get_mut(peer_id) {
            for stream in streams.iter_mut() {
                stream.state = StreamState::Closed;
            }
        }
        
        state.peer_streams.remove(peer_id);
        Ok(())
    }

    /// Get combined statistics for all streams
    pub async fn get_combined_stats(&self) -> CombinedStreamStats {
        let state = self.state.read().await;

        let mut total_streams = 0;
        let mut active_streams = 0;
        let mut total_bytes = 0;
        let peer_count = state.peer_streams.len();

        for streams in state.peer_streams.values() {
            total_streams += streams.len();
            active_streams += streams.iter().filter(|s| s.state == StreamState::Active).count();
            total_bytes += streams.iter().map(|s| s.bytes_transferred).sum::<u64>();
        }

        CombinedStreamStats {
            total_streams,
            active_streams,
            total_bytes_transferred: total_bytes,
            peer_count,
            max_streams_per_peer: state.max_streams_per_peer,
        }
    }
}

impl Default for ParallelStreamManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for a single stream
#[derive(Debug, Clone)]
pub struct StreamStats {
    pub stream_id: StreamId,
    pub state: StreamState,
    pub bytes_transferred: u64,
    pub files_assigned: usize,
    pub current_file: Option<std::path::PathBuf>,
}

/// Combined statistics for all streams
#[derive(Debug, Clone)]
pub struct CombinedStreamStats {
    pub total_streams: usize,
    pub active_streams: usize,
    pub total_bytes_transferred: u64,
    pub peer_count: usize,
    pub max_streams_per_peer: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_file(name: &str, size: u64) -> FileEntry {
        FileEntry {
            path: PathBuf::from(name),
            size,
            checksum: [0u8; 32],
            permissions: FilePermissions::default(),
            modified_at: current_timestamp(),
            chunk_count: (size / 65536) as usize + 1,
        }
    }

    #[tokio::test]
    async fn test_register_stream() {
        let manager = ParallelStreamManager::new();
        let peer_id = "peer1".to_string();

        let stream_id = manager.register_stream(peer_id.clone()).await.unwrap();
        assert_eq!(manager.peer_stream_count(&peer_id).await, 1);
    }

    #[tokio::test]
    async fn test_max_streams_limit() {
        let manager = ParallelStreamManager::with_max_streams(2);
        let peer_id = "peer1".to_string();

        // Register 2 streams (should succeed)
        manager.register_stream(peer_id.clone()).await.unwrap();
        manager.register_stream(peer_id.clone()).await.unwrap();

        // Try to register 3rd stream (should fail)
        let result = manager.register_stream(peer_id.clone()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unregister_stream() {
        let manager = ParallelStreamManager::new();
        let peer_id = "peer1".to_string();

        let stream_id = manager.register_stream(peer_id.clone()).await.unwrap();
        assert_eq!(manager.peer_stream_count(&peer_id).await, 1);

        manager.unregister_stream(stream_id).await.unwrap();
        assert_eq!(manager.peer_stream_count(&peer_id).await, 0);
    }

    #[tokio::test]
    async fn test_distribute_files_round_robin() {
        let manager = ParallelStreamManager::new();
        manager
            .set_distribution_strategy(DistributionStrategy::RoundRobin)
            .await
            .unwrap();

        let peer_id = "peer1".to_string();

        // Register 2 streams
        manager.register_stream(peer_id.clone()).await.unwrap();
        manager.register_stream(peer_id.clone()).await.unwrap();

        // Create test files
        let files = vec![
            create_test_file("file1.txt", 1000),
            create_test_file("file2.txt", 2000),
            create_test_file("file3.txt", 3000),
            create_test_file("file4.txt", 4000),
        ];

        let distribution = manager.distribute_files(&peer_id, files).await.unwrap();

        // Should have 2 streams with files distributed
        assert_eq!(distribution.len(), 2);
        
        // Each stream should have 2 files
        for files in distribution.values() {
            assert_eq!(files.len(), 2);
        }
    }

    #[tokio::test]
    async fn test_distribute_files_load_balanced() {
        let manager = ParallelStreamManager::new();
        manager
            .set_distribution_strategy(DistributionStrategy::LoadBalanced)
            .await
            .unwrap();

        let peer_id = "peer1".to_string();

        // Register 2 streams
        manager.register_stream(peer_id.clone()).await.unwrap();
        manager.register_stream(peer_id.clone()).await.unwrap();

        // Create test files with varying sizes
        let files = vec![
            create_test_file("large.txt", 10000),
            create_test_file("medium.txt", 5000),
            create_test_file("small1.txt", 1000),
            create_test_file("small2.txt", 1000),
        ];

        let distribution = manager.distribute_files(&peer_id, files).await.unwrap();

        // Should distribute to balance load
        assert_eq!(distribution.len(), 2);
        
        // Calculate total size per stream
        let mut stream_sizes: Vec<u64> = distribution
            .values()
            .map(|files| files.iter().map(|f| f.size).sum())
            .collect();
        
        stream_sizes.sort();
        
        // Loads should be relatively balanced
        let diff = stream_sizes[1] - stream_sizes[0];
        assert!(diff <= 5000); // Reasonable balance
    }

    #[tokio::test]
    async fn test_update_stream_state() {
        let manager = ParallelStreamManager::new();
        let peer_id = "peer1".to_string();

        let stream_id = manager.register_stream(peer_id.clone()).await.unwrap();

        manager
            .update_stream_state(stream_id, StreamState::Active)
            .await
            .unwrap();

        let stats = manager.get_peer_stream_stats(&peer_id).await.unwrap();
        assert_eq!(stats[0].state, StreamState::Active);
    }

    #[tokio::test]
    async fn test_record_bytes() {
        let manager = ParallelStreamManager::new();
        let peer_id = "peer1".to_string();

        let stream_id = manager.register_stream(peer_id.clone()).await.unwrap();

        manager.record_bytes(stream_id, 1024).await.unwrap();
        manager.record_bytes(stream_id, 2048).await.unwrap();

        let stats = manager.get_peer_stream_stats(&peer_id).await.unwrap();
        assert_eq!(stats[0].bytes_transferred, 3072);
    }

    #[tokio::test]
    async fn test_close_peer_streams() {
        let manager = ParallelStreamManager::new();
        let peer_id = "peer1".to_string();

        manager.register_stream(peer_id.clone()).await.unwrap();
        manager.register_stream(peer_id.clone()).await.unwrap();

        assert_eq!(manager.peer_stream_count(&peer_id).await, 2);

        manager.close_peer_streams(&peer_id).await.unwrap();

        assert_eq!(manager.peer_stream_count(&peer_id).await, 0);
    }

    #[tokio::test]
    async fn test_combined_stats() {
        let manager = ParallelStreamManager::new();

        let peer1 = "peer1".to_string();
        let peer2 = "peer2".to_string();

        let stream1 = manager.register_stream(peer1.clone()).await.unwrap();
        let stream2 = manager.register_stream(peer2.clone()).await.unwrap();

        manager.update_stream_state(stream1, StreamState::Active).await.unwrap();
        manager.record_bytes(stream1, 1000).await.unwrap();
        manager.record_bytes(stream2, 2000).await.unwrap();

        let stats = manager.get_combined_stats().await;

        assert_eq!(stats.total_streams, 2);
        assert_eq!(stats.active_streams, 1);
        assert_eq!(stats.total_bytes_transferred, 3000);
        assert_eq!(stats.peer_count, 2);
    }

    #[tokio::test]
    async fn test_active_stream_count() {
        let manager = ParallelStreamManager::new();
        let peer_id = "peer1".to_string();

        let stream1 = manager.register_stream(peer_id.clone()).await.unwrap();
        let stream2 = manager.register_stream(peer_id.clone()).await.unwrap();

        manager.update_stream_state(stream1, StreamState::Active).await.unwrap();

        assert_eq!(manager.active_stream_count().await, 1);

        manager.update_stream_state(stream2, StreamState::Active).await.unwrap();

        assert_eq!(manager.active_stream_count().await, 2);
    }
}
