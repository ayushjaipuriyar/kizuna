# File Transfer System Design

## Overview

The File Transfer system provides reliable, efficient, and secure file sharing capabilities between Kizuna peers. The design emphasizes performance through intelligent transport selection, resumability for unreliable networks, and user control through queue management and bandwidth throttling.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    File Transfer System                     │
├─────────────────────────────────────────────────────────────┤
│  Transfer Manager  │  Queue Manager   │  Progress Tracker  │
│  - Session Control │  - Priority Queue │  - Real-time Stats │
│  - Resume Logic    │  - Scheduling     │  - ETA Calculation │
│  - Error Recovery  │  - Persistence    │  - Speed Monitoring│
├─────────────────────────────────────────────────────────────┤
│  Manifest Builder  │  Chunk Engine    │  Compression Engine│
│  - File Scanning   │  - Chunking       │  - LZ4 Compression │
│  - Checksum Calc   │  - Verification   │  - Auto Detection  │
│  - Structure Map   │  - Streaming      │  - Size Optimization│
├─────────────────────────────────────────────────────────────┤
│              Transport Negotiation                          │
│              - Protocol Selection                           │
│              - Capability Exchange                          │
│              - Performance Optimization                     │
├─────────────────────────────────────────────────────────────┤
│                   Storage Manager                           │
│                   - Temp File Handling                      │
│                   - Disk Space Checking                     │
│                   - Path Resolution                         │
└─────────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### Transfer Manager

**Purpose**: Orchestrates file transfer operations and session management

**Key Components**:
- `TransferSession`: Manages individual transfer operations
- `ResumeManager`: Handles interrupted transfer recovery
- `BandwidthController`: Implements throttling and rate limiting
- `ParallelStreamManager`: Coordinates multiple simultaneous streams

**Interface**:
```rust
trait TransferManager {
    async fn start_transfer(manifest: TransferManifest, peer_id: PeerId) -> Result<TransferSession>;
    async fn resume_transfer(resume_token: ResumeToken) -> Result<TransferSession>;
    async fn cancel_transfer(session_id: SessionId) -> Result<()>;
    async fn set_bandwidth_limit(limit: Option<u64>) -> Result<()>;
    async fn get_active_transfers() -> Result<Vec<TransferSession>>;
}
```

### Queue Manager

**Purpose**: Manages transfer queue, scheduling, and prioritization

**Key Components**:
- `TransferQueue`: Priority queue for pending transfers
- `QueueScheduler`: Determines transfer execution order
- `QueuePersistence`: Saves queue state across restarts
- `ResourceAllocator`: Manages connection slots and bandwidth

**Interface**:
```rust
trait QueueManager {
    async fn enqueue_transfer(request: TransferRequest, priority: Priority) -> Result<QueueId>;
    async fn reorder_queue(queue_id: QueueId, new_position: usize) -> Result<()>;
    async fn pause_queue_item(queue_id: QueueId) -> Result<()>;
    async fn cancel_queue_item(queue_id: QueueId) -> Result<()>;
    async fn get_queue_status() -> Result<Vec<QueueItem>>;
}
```

### Manifest Builder

**Purpose**: Creates transfer manifests with file metadata and structure

**Key Components**:
- `FileScanner`: Recursively scans files and directories
- `ChecksumCalculator`: Computes SHA-256 hashes for integrity
- `StructureMapper`: Preserves directory hierarchy and permissions
- `MetadataExtractor`: Captures file attributes and timestamps

**Interface**:
```rust
trait ManifestBuilder {
    async fn build_file_manifest(path: PathBuf) -> Result<TransferManifest>;
    async fn build_multi_file_manifest(paths: Vec<PathBuf>) -> Result<TransferManifest>;
    async fn build_folder_manifest(path: PathBuf, recursive: bool) -> Result<TransferManifest>;
    async fn verify_manifest(manifest: &TransferManifest) -> Result<bool>;
}
```

### Chunk Engine

**Purpose**: Handles file chunking, streaming, and verification

**Key Components**:
- `ChunkProcessor`: Splits files into 64KB chunks
- `StreamingEngine`: Manages chunk transmission and reception
- `ChunkVerifier`: Validates individual chunk integrity
- `ReassemblyEngine`: Reconstructs files from received chunks

**Interface**:
```rust
trait ChunkEngine {
    async fn create_chunks(file_path: PathBuf) -> Result<Vec<Chunk>>;
    async fn stream_chunk(chunk: Chunk, stream: &mut Stream) -> Result<()>;
    async fn receive_chunk(stream: &mut Stream) -> Result<Chunk>;
    async fn verify_chunk(chunk: &Chunk) -> Result<bool>;
    async fn reassemble_file(chunks: Vec<Chunk>, output_path: PathBuf) -> Result<()>;
}
```

### Transport Negotiation

**Purpose**: Selects optimal transport protocol for file transfers

**Key Components**:
- `CapabilityExchange`: Discovers peer transport capabilities
- `ProtocolSelector`: Chooses best transport based on criteria
- `PerformanceProfiler`: Measures transport performance characteristics
- `FallbackManager`: Handles transport failures and switching

**Interface**:
```rust
trait TransportNegotiator {
    async fn negotiate_transport(peer_id: PeerId, file_size: u64) -> Result<TransportProtocol>;
    async fn get_peer_capabilities(peer_id: PeerId) -> Result<TransportCapabilities>;
    async fn benchmark_transport(protocol: TransportProtocol, peer_id: PeerId) -> Result<PerformanceMetrics>;
    async fn fallback_transport(current: TransportProtocol) -> Result<Option<TransportProtocol>>;
}
```

## Data Models

### Transfer Manifest
```rust
struct TransferManifest {
    transfer_id: TransferId,
    sender_id: PeerId,
    created_at: Timestamp,
    total_size: u64,
    file_count: usize,
    files: Vec<FileEntry>,
    directories: Vec<DirectoryEntry>,
    checksum: [u8; 32], // SHA-256 of entire manifest
}

struct FileEntry {
    path: PathBuf,
    size: u64,
    checksum: [u8; 32],
    permissions: FilePermissions,
    modified_at: Timestamp,
    chunk_count: usize,
}

struct DirectoryEntry {
    path: PathBuf,
    permissions: FilePermissions,
    created_at: Timestamp,
}
```

### Transfer Session
```rust
struct TransferSession {
    session_id: SessionId,
    manifest: TransferManifest,
    peer_id: PeerId,
    transport: TransportProtocol,
    state: TransferState,
    progress: TransferProgress,
    bandwidth_limit: Option<u64>,
    parallel_streams: usize,
    resume_token: Option<ResumeToken>,
}

enum TransferState {
    Pending,
    Negotiating,
    Transferring,
    Paused,
    Completed,
    Failed(TransferError),
    Cancelled,
}
```

### Transfer Progress
```rust
struct TransferProgress {
    bytes_transferred: u64,
    total_bytes: u64,
    files_completed: usize,
    total_files: usize,
    current_speed: u64, // bytes per second
    average_speed: u64,
    eta_seconds: Option<u64>,
    last_update: Timestamp,
}
```

### Resume Token
```rust
struct ResumeToken {
    transfer_id: TransferId,
    session_id: SessionId,
    last_completed_file: Option<PathBuf>,
    last_completed_chunk: Option<ChunkId>,
    bytes_completed: u64,
    created_at: Timestamp,
    expires_at: Timestamp,
}
```

### Queue Item
```rust
struct QueueItem {
    queue_id: QueueId,
    transfer_request: TransferRequest,
    priority: Priority,
    estimated_start: Option<Timestamp>,
    state: QueueState,
    created_at: Timestamp,
}

enum Priority {
    Low,
    Normal,
    High,
    Urgent,
}

enum QueueState {
    Pending,
    Scheduled,
    Paused,
    Cancelled,
}
```

## Error Handling

### Transfer Error Types
- `ManifestError`: File scanning, checksum calculation failures
- `TransportError`: Network connectivity and protocol failures
- `StorageError`: Disk space, permission, and I/O failures
- `IntegrityError`: Checksum mismatches and corruption detection
- `ResumeError`: Resume token validation and state recovery failures

### Error Recovery Strategies
- **Network Failures**: Automatic retry with exponential backoff
- **Corruption Detection**: Re-transfer affected chunks
- **Storage Failures**: Prompt user for alternative location
- **Transport Failures**: Automatic fallback to alternative protocol
- **Resume Failures**: Restart transfer from beginning with user confirmation

## Testing Strategy

### Unit Tests
- Manifest building and validation
- Chunk creation, verification, and reassembly
- Queue operations and persistence
- Progress calculation and ETA estimation
- Compression effectiveness and performance

### Integration Tests
- End-to-end file transfer scenarios
- Resume functionality with simulated interruptions
- Multi-file and folder transfer workflows
- Transport negotiation with different peer capabilities
- Bandwidth throttling and parallel stream coordination

### Performance Tests
- Large file transfer throughput
- Many small files transfer efficiency
- Memory usage during transfers
- Compression performance impact
- Parallel stream scaling benefits

### Reliability Tests
- Network interruption and recovery
- Disk space exhaustion handling
- Concurrent transfer stress testing
- Long-running transfer stability
- Error recovery effectiveness

## Performance Optimizations

### Chunking Strategy
- 64KB chunks for optimal memory usage and resumability
- Adaptive chunk size based on network conditions
- Parallel chunk processing for large files
- Chunk prefetching to reduce latency

### Compression Optimization
- LZ4 for fast compression with good ratios
- Automatic compression detection based on file type
- Streaming compression to reduce memory usage
- Compression bypass for already compressed files

### Transport Selection
- QUIC preferred for large files and unreliable networks
- TCP for stable connections and maximum compatibility
- WebRTC for browser peers and NAT traversal
- Automatic fallback based on performance metrics

### Memory Management
- Streaming I/O to handle files larger than available memory
- Bounded chunk queues to prevent memory exhaustion
- Efficient buffer reuse and pooling
- Garbage collection optimization for long transfers