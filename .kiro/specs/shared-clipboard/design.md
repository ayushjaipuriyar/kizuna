# Shared Clipboard System Design

## Overview

The Shared Clipboard system provides seamless cross-device clipboard synchronization with privacy controls and platform abstraction. The design emphasizes real-time synchronization, user privacy, and cross-platform compatibility while maintaining system performance and security.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                   Shared Clipboard System                  │
├─────────────────────────────────────────────────────────────┤
│  Clipboard Monitor │  Sync Manager    │  Privacy Filter    │
│  - Change Detection │  - Peer Sync     │  - Content Analysis│
│  - Platform APIs    │  - Device Control│  - Sensitive Data  │
│  - Event Generation │  - Conflict Res. │  - User Policies   │
├─────────────────────────────────────────────────────────────┤
│  Content Processor │  History Manager │  Notification Mgr  │
│  - Format Conversion│  - Local Storage │  - Event Dispatch  │
│  - Compression      │  - Search/Browse │  - User Alerts     │
│  - Validation       │  - Recovery      │  - Status Updates  │
├─────────────────────────────────────────────────────────────┤
│              Platform Abstraction Layer                    │
│              - Windows Clipboard API                       │
│              - macOS Pasteboard API                        │
│              - Linux X11/Wayland APIs                      │
├─────────────────────────────────────────────────────────────┤
│                   Sync Protocol                            │
│                   - Content Serialization                  │
│                   - Peer Communication                     │
│                   - Conflict Resolution                    │
└─────────────────────────────────────────────────────────────┘
```

## Components and Interfaces

### Clipboard Monitor

**Purpose**: Detects and monitors system clipboard changes across platforms

**Key Components**:
- `PlatformClipboard`: Platform-specific clipboard API implementations
- `ChangeDetector`: Monitors clipboard for content modifications
- `EventDispatcher`: Generates clipboard change events
- `LoopPrevention`: Prevents infinite sync loops

**Interface**:
```rust
trait ClipboardMonitor {
    async fn start_monitoring() -> Result<()>;
    async fn stop_monitoring() -> Result<()>;
    async fn get_current_content() -> Result<ClipboardContent>;
    async fn set_content(content: ClipboardContent) -> Result<()>;
    fn subscribe_to_changes() -> Receiver<ClipboardEvent>;
}
```

### Sync Manager

**Purpose**: Manages clipboard synchronization between peers

**Key Components**:
- `PeerSyncController`: Coordinates sync operations with connected peers
- `DeviceAllowlist`: Manages per-device sync permissions
- `ConflictResolver`: Handles simultaneous clipboard changes
- `SyncScheduler`: Manages sync timing and batching

**Interface**:
```rust
trait SyncManager {
    async fn enable_sync_for_device(device_id: DeviceId) -> Result<()>;
    async fn disable_sync_for_device(device_id: DeviceId) -> Result<()>;
    async fn sync_content_to_peers(content: ClipboardContent) -> Result<()>;
    async fn receive_content_from_peer(content: ClipboardContent, peer_id: PeerId) -> Result<()>;
    async fn get_sync_status() -> Result<Vec<DeviceSyncStatus>>;
}
```

### Content Processor

**Purpose**: Handles clipboard content format conversion and validation

**Key Components**:
- `FormatConverter`: Converts between platform-specific clipboard formats
- `ImageProcessor`: Handles image compression and format conversion
- `TextProcessor`: Manages text encoding and formatting preservation
- `ContentValidator`: Validates clipboard content integrity and size limits

**Interface**:
```rust
trait ContentProcessor {
    async fn process_outgoing_content(content: ClipboardContent) -> Result<ProcessedContent>;
    async fn process_incoming_content(content: ProcessedContent) -> Result<ClipboardContent>;
    async fn compress_image(image: ImageData) -> Result<CompressedImage>;
    async fn validate_content(content: &ClipboardContent) -> Result<ValidationResult>;
}
```

### Privacy Filter

**Purpose**: Analyzes and filters sensitive clipboard content

**Key Components**:
- `SensitiveContentDetector`: Identifies potentially sensitive information
- `PolicyEngine`: Applies user-defined privacy policies
- `ContentAnalyzer`: Analyzes content patterns and characteristics
- `UserPromptManager`: Handles user confirmation for sensitive content

**Interface**:
```rust
trait PrivacyFilter {
    async fn analyze_content(content: &ClipboardContent) -> Result<PrivacyAnalysis>;
    async fn should_sync_content(content: &ClipboardContent) -> Result<SyncDecision>;
    async fn add_privacy_rule(rule: PrivacyRule) -> Result<()>;
    async fn prompt_user_for_sensitive_content(content: &ClipboardContent) -> Result<UserDecision>;
}
```

### History Manager

**Purpose**: Manages local clipboard history and recovery

**Key Components**:
- `HistoryStorage`: Persistent storage for clipboard history
- `HistoryBrowser`: Interface for browsing and searching history
- `ContentRecovery`: Restores previous clipboard content
- `HistoryMaintenance`: Manages history size limits and cleanup

**Interface**:
```rust
trait HistoryManager {
    async fn add_to_history(content: ClipboardContent, source: ContentSource) -> Result<()>;
    async fn get_history(limit: usize) -> Result<Vec<HistoryEntry>>;
    async fn search_history(query: &str) -> Result<Vec<HistoryEntry>>;
    async fn restore_content(entry_id: HistoryId) -> Result<()>;
    async fn clear_history() -> Result<()>;
}
```

## Data Models

### Clipboard Content
```rust
enum ClipboardContent {
    Text(TextContent),
    Image(ImageContent),
    Files(FileList),
    Custom(CustomContent),
}

struct TextContent {
    text: String,
    encoding: TextEncoding,
    format: TextFormat, // Plain, RTF, HTML, etc.
    size: usize,
}

struct ImageContent {
    data: Vec<u8>,
    format: ImageFormat, // PNG, JPEG, BMP, etc.
    width: u32,
    height: u32,
    compressed: bool,
}
```

### Clipboard Event
```rust
struct ClipboardEvent {
    event_id: EventId,
    event_type: ClipboardEventType,
    content: Option<ClipboardContent>,
    source: ContentSource,
    timestamp: Timestamp,
}

enum ClipboardEventType {
    ContentChanged,
    ContentReceived,
    SyncStarted,
    SyncCompleted,
    SyncFailed,
}

enum ContentSource {
    Local,
    Remote(PeerId),
    History(HistoryId),
}
```

### History Entry
```rust
struct HistoryEntry {
    entry_id: HistoryId,
    content: ClipboardContent,
    source: ContentSource,
    created_at: Timestamp,
    access_count: u32,
    last_accessed: Timestamp,
    tags: Vec<String>,
}
```

### Privacy Analysis
```rust
struct PrivacyAnalysis {
    sensitivity_score: f32, // 0.0 to 1.0
    detected_patterns: Vec<SensitivePattern>,
    recommendation: SyncRecommendation,
    user_prompt_required: bool,
}

enum SensitivePattern {
    Password,
    CreditCard,
    SocialSecurity,
    Email,
    PhoneNumber,
    ApiKey,
    Custom(String),
}

enum SyncRecommendation {
    Allow,
    Block,
    Prompt,
}
```

### Device Sync Status
```rust
struct DeviceSyncStatus {
    device_id: DeviceId,
    device_name: String,
    sync_enabled: bool,
    last_sync: Option<Timestamp>,
    sync_count: u64,
    connection_status: ConnectionStatus,
}
```

### Sync Policy
```rust
struct SyncPolicy {
    auto_sync_enabled: bool,
    max_content_size: usize,
    image_compression_threshold: usize,
    privacy_filter_enabled: bool,
    notification_enabled: bool,
    history_retention_days: u32,
    allowed_content_types: Vec<ContentType>,
}
```

## Error Handling

### Clipboard Error Types
- `PlatformError`: Platform-specific clipboard API failures
- `PermissionError`: Clipboard access permission denied
- `ContentError`: Invalid or corrupted clipboard content
- `SyncError`: Network or peer communication failures
- `PrivacyError`: Privacy policy violations or sensitive content detection

### Error Recovery Strategies
- **Platform API Failures**: Retry with exponential backoff, fallback to polling
- **Permission Errors**: Prompt user for permissions, provide manual sync options
- **Content Corruption**: Skip corrupted content, log for debugging
- **Sync Failures**: Queue for retry, notify user of sync issues
- **Privacy Violations**: Block sync, log security event, notify user

## Testing Strategy

### Unit Tests
- Platform-specific clipboard API implementations
- Content format conversion and validation
- Privacy filter pattern detection
- History management operations
- Sync policy enforcement

### Integration Tests
- Cross-platform clipboard synchronization
- Multi-device sync scenarios
- Privacy filter integration with sync operations
- History persistence and recovery
- Error handling and recovery workflows

### Platform Tests
- Windows clipboard API integration
- macOS Pasteboard API integration
- Linux X11 and Wayland clipboard support
- Platform-specific content format handling
- Permission handling across platforms

### Performance Tests
- Clipboard monitoring overhead and responsiveness
- Large content synchronization performance
- Image compression effectiveness and speed
- History search and retrieval performance
- Memory usage during extended operation

## Platform-Specific Implementations

### Windows Implementation
- Use Windows Clipboard API with RegisterClipboardFormat
- Monitor clipboard changes using SetClipboardViewer or AddClipboardFormatListener
- Handle Windows-specific formats (CF_TEXT, CF_UNICODETEXT, CF_BITMAP, CF_DIB)
- Integrate with Windows notification system

### macOS Implementation
- Use NSPasteboard API for clipboard operations
- Monitor changes using NSPasteboard changeCount polling
- Handle macOS-specific pasteboard types (NSStringPboardType, NSPICTPboardType)
- Integrate with macOS notification center

### Linux Implementation
- Support both X11 and Wayland clipboard protocols
- Use X11 selection mechanism (PRIMARY, CLIPBOARD, SECONDARY)
- Handle Wayland wl_data_device_manager protocol
- Support common MIME types for cross-application compatibility
- Integrate with desktop notification systems (libnotify)

## Security Considerations

### Privacy Protection
- Content analysis performed locally only
- Sensitive pattern detection using local rules
- User consent required for potentially sensitive content
- No clipboard content stored on remote servers

### Data Security
- All clipboard content encrypted during transmission
- Temporary content cleared from memory after use
- History stored with local encryption
- Secure deletion of sensitive clipboard history

### Access Control
- Per-device sync permissions
- User-controlled allowlists and blocklists
- Audit logging of sync operations
- Rate limiting to prevent abuse