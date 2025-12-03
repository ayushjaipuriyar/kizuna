# Unified Streaming API Guide

## Overview

The Unified Streaming API provides a high-level, event-driven interface for all video streaming operations in Kizuna. It abstracts away the complexity of video codecs, network protocols, and platform-specific implementations, offering simple methods for:

- Camera streaming
- Screen sharing
- Multi-viewer broadcasting
- Stream recording
- Quality management
- Event-driven notifications

**Requirements Addressed**: 10.1, 10.2, 10.3

## Core Concepts

### Streaming Trait

The `Streaming` trait is the main interface for all streaming operations. It provides methods for:

1. **Session Management**: Start, stop, pause, and resume streams
2. **Quality Control**: Adjust stream quality and enable automatic adaptation
3. **Device Management**: List and configure camera devices
4. **Viewer Management**: Add, remove, and manage stream viewers
5. **Recording**: Record streams locally with configurable settings
6. **Event Handling**: Register handlers for stream events

### StreamingApi Implementation

`StreamingApi` is the concrete implementation of the `Streaming` trait. It:

- Manages active stream sessions
- Coordinates between capture, encoding, and network components
- Distributes events to registered handlers
- Handles state transitions and error recovery

### Event-Driven Architecture

The API uses an event-driven model where all significant state changes, quality adjustments, and viewer actions trigger events. This allows applications to:

- React to stream status changes
- Monitor quality adaptations
- Track viewer connections
- Handle errors gracefully
- Update UI in real-time

## Quick Start

### Basic Camera Streaming

```rust
use kizuna::streaming::{Streaming, StreamingApi, StreamConfig, QualityPreset};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the API
    let api = StreamingApi::new();
    
    // Configure the stream
    let config = StreamConfig {
        quality: QualityPreset::Medium.to_quality(),
        enable_audio: true,
        enable_recording: false,
        max_viewers: 10,
    };
    
    // Start streaming
    let session = api.start_camera_stream(config).await?;
    println!("Streaming started: {}", session.session_id);
    
    // Stop when done
    api.stop_stream(session.session_id).await?;
    
    Ok(())
}
```

### Screen Sharing

```rust
use kizuna::streaming::{Streaming, StreamingApi, ScreenConfig, ScreenRegion, QualityPreset};

let api = StreamingApi::new();

let config = ScreenConfig {
    region: ScreenRegion {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    },
    capture_cursor: true,
    capture_audio: false,
    monitor_index: Some(0),
    quality: QualityPreset::High.to_quality(),
};

let session = api.start_screen_stream(config).await?;
```

## Event Handling

### Implementing an Event Handler

```rust
use kizuna::streaming::{StreamEvent, StreamEventHandler};
use async_trait::async_trait;

struct MyEventHandler;

#[async_trait]
impl StreamEventHandler for MyEventHandler {
    async fn on_event(&self, event: StreamEvent) {
        match event {
            StreamEvent::SessionStarted { session_id, stream_type } => {
                println!("Stream started: {:?}", session_id);
            }
            StreamEvent::QualityChanged { session_id, new_quality, reason, .. } => {
                println!("Quality changed to {:?} (reason: {:?})", 
                    new_quality.quality_preset, reason);
            }
            StreamEvent::ViewerConnected { viewer_id, peer_id, .. } => {
                println!("Viewer {} connected from {}", viewer_id, peer_id);
            }
            StreamEvent::Error { error, recoverable, .. } => {
                eprintln!("Stream error: {} (recoverable: {})", error, recoverable);
            }
            _ => {}
        }
    }
}

// Register the handler
let handler = Arc::new(MyEventHandler);
api.register_event_handler(handler).await?;
```

### Available Events

- **SessionStarted**: Stream session successfully started
- **SessionStopped**: Stream session stopped
- **StateChanged**: Stream state transition (Starting → Active → Paused → Stopping)
- **QualityChanged**: Stream quality adjusted (manual or automatic)
- **ViewerConnected**: New viewer joined the stream
- **ViewerDisconnected**: Viewer left the stream
- **ViewerRequestReceived**: Viewer requested access (requires approval)
- **StatsUpdated**: Stream statistics updated
- **Error**: Error occurred (with recoverability flag)
- **RecordingStarted**: Recording began
- **RecordingStopped**: Recording completed
- **NetworkConditionChanged**: Network conditions changed

## Quality Management

### Quality Presets

The API provides four quality presets:

```rust
use kizuna::streaming::QualityPreset;

// Low: 480p, 15fps, 500kbps
let low = QualityPreset::Low.to_quality();

// Medium: 720p, 30fps, 1.5Mbps
let medium = QualityPreset::Medium.to_quality();

// High: 1080p, 30fps, 3Mbps
let high = QualityPreset::High.to_quality();

// Ultra: 1080p, 60fps, 6Mbps
let ultra = QualityPreset::Ultra.to_quality();
```

### Custom Quality Settings

```rust
use kizuna::streaming::{StreamQuality, Resolution, QualityPreset};

let custom_quality = StreamQuality {
    resolution: Resolution { width: 1280, height: 720 },
    framerate: 25,
    bitrate: 2_000_000, // 2 Mbps
    quality_preset: QualityPreset::Custom,
    hardware_acceleration: true,
};

api.adjust_quality(session_id, custom_quality).await?;
```

### Automatic Quality Adaptation

```rust
// Enable automatic quality adjustment based on network conditions
api.set_auto_quality(session_id, true).await?;

// The API will automatically adjust quality and emit QualityChanged events
```

## Multi-Viewer Broadcasting

### Adding Viewers

```rust
use kizuna::streaming::ViewerPermissions;

let permissions = ViewerPermissions {
    can_view: true,
    can_record: false,
    can_control_quality: false,
    max_quality: QualityPreset::Medium,
};

let viewer_id = api.add_viewer(
    session_id,
    "peer-device-id".to_string(),
    permissions,
).await?;
```

### Managing Viewers

```rust
// Get all viewers for a stream
let viewers = api.get_viewers(session_id).await?;
for viewer in viewers {
    println!("Viewer: {} ({})", viewer.viewer_id, viewer.device_name);
    println!("  Quality: {:?}", viewer.current_quality.quality_preset);
    println!("  Connection: {:?}", viewer.connection_quality);
}

// Remove a viewer
api.remove_viewer(session_id, viewer_id).await?;
```

### Viewer Approval Workflow

```rust
// When a ViewerRequestReceived event is received:
impl StreamEventHandler for MyHandler {
    async fn on_event(&self, event: StreamEvent) {
        if let StreamEvent::ViewerRequestReceived { session_id, peer_id, device_name } = event {
            println!("Viewer request from {} ({})", device_name, peer_id);
            
            // Approve the viewer
            let viewer_id = api.approve_viewer(session_id, peer_id.clone()).await?;
            
            // Or reject
            // api.reject_viewer(session_id, peer_id).await?;
        }
    }
}
```

## Recording

### Start Recording

```rust
use kizuna::streaming::{RecordingConfig, VideoFormat};
use std::path::PathBuf;

let config = RecordingConfig {
    output_path: PathBuf::from("/recordings/stream.mp4"),
    format: VideoFormat::MP4,
    quality: QualityPreset::High.to_quality(),
    max_file_size: Some(1024 * 1024 * 500), // 500 MB
    max_duration: Some(std::time::Duration::from_secs(3600)), // 1 hour
};

let recording = api.start_recording(session_id, config).await?;
```

### Control Recording

```rust
// Pause recording
api.pause_recording(recording.session_id).await?;

// Resume recording
api.resume_recording(recording.session_id).await?;

// Get recording status
let status = api.get_recording_status(recording.session_id).await?;
println!("Recording: {} bytes, {} frames", status.file_size, status.frames_recorded);

// Stop and finalize
let file = api.stop_recording(recording.session_id).await?;
println!("Recording saved: {:?} ({} bytes)", file.path, file.file_size);
```

## Stream Statistics

### Monitoring Stream Performance

```rust
let stats = api.get_stream_stats(session_id).await?;

println!("Bitrate: {} kbps", stats.current_bitrate / 1000);
println!("Latency: {} ms", stats.latency_ms);
println!("Jitter: {} ms", stats.jitter_ms);
println!("Packet loss: {:.2}%", stats.packet_loss_rate * 100.0);
println!("Frames encoded: {}", stats.frames_encoded);
println!("Frames dropped: {}", stats.frames_dropped);
```

## Session Management

### Listing Active Streams

```rust
let active = api.get_active_streams().await?;
for stream in active {
    println!("Session: {}", stream.session_id);
    println!("  Type: {:?}", stream.stream_type);
    println!("  State: {:?}", stream.state);
    println!("  Viewers: {}", stream.viewers.len());
    println!("  Quality: {:?}", stream.quality.quality_preset);
}
```

### Pausing and Resuming

```rust
// Pause a stream (stops transmission but keeps session alive)
api.pause_stream(session_id).await?;

// Resume a paused stream
api.resume_stream(session_id).await?;

// Stop completely (ends session)
api.stop_stream(session_id).await?;
```

## Device Management

### Listing Cameras

```rust
let cameras = api.list_cameras().await?;
for camera in cameras {
    println!("Camera: {} ({})", camera.name, camera.id);
    if let Some(desc) = camera.description {
        println!("  Description: {}", desc);
    }
}
```

### Getting Camera Capabilities

```rust
let capabilities = api.get_camera_capabilities(camera_device).await?;

println!("Supported resolutions:");
for res in capabilities.supported_resolutions {
    println!("  {}x{}", res.width, res.height);
}

println!("Supported framerates: {:?}", capabilities.supported_framerates);
println!("Auto exposure: {}", capabilities.has_auto_exposure);
println!("Auto focus: {}", capabilities.has_auto_focus);
```

## Error Handling

### Handling Errors

```rust
match api.start_camera_stream(config).await {
    Ok(session) => {
        println!("Stream started: {}", session.session_id);
    }
    Err(e) => {
        match e {
            StreamError::Permission(msg) => {
                eprintln!("Permission denied: {}", msg);
                // Request permissions from user
            }
            StreamError::DeviceNotFound(device) => {
                eprintln!("Camera not found: {}", device);
                // Show device selection UI
            }
            StreamError::Network(msg) => {
                eprintln!("Network error: {}", msg);
                // Retry or show error to user
            }
            _ => {
                eprintln!("Stream error: {}", e);
            }
        }
    }
}
```

### Error Events

```rust
impl StreamEventHandler for MyHandler {
    async fn on_event(&self, event: StreamEvent) {
        if let StreamEvent::Error { session_id, error, recoverable } = event {
            if recoverable {
                println!("Recoverable error in {:?}: {}", session_id, error);
                // Attempt automatic recovery
            } else {
                eprintln!("Fatal error in {:?}: {}", session_id, error);
                // Stop stream and notify user
            }
        }
    }
}
```

## Best Practices

### 1. Always Register Event Handlers Early

```rust
let api = StreamingApi::new();
let handler = Arc::new(MyEventHandler);
api.register_event_handler(handler).await?;

// Now start streams
let session = api.start_camera_stream(config).await?;
```

### 2. Handle All Error Cases

```rust
// Don't just unwrap
let session = api.start_camera_stream(config).await?;

// Do handle errors appropriately
match api.start_camera_stream(config).await {
    Ok(session) => { /* handle success */ }
    Err(e) => { /* handle error */ }
}
```

### 3. Clean Up Resources

```rust
// Always stop streams when done
api.stop_stream(session_id).await?;

// Stop recordings before stopping streams
api.stop_recording(recording_id).await?;
api.stop_stream(session_id).await?;
```

### 4. Monitor Stream Statistics

```rust
// Periodically check stream health
tokio::spawn(async move {
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        if let Ok(stats) = api.get_stream_stats(session_id).await {
            if stats.packet_loss_rate > 0.05 {
                // High packet loss, consider reducing quality
                api.adjust_quality(session_id, lower_quality).await?;
            }
        }
    }
});
```

### 5. Use Quality Presets for Simplicity

```rust
// Prefer presets over custom settings
let config = StreamConfig {
    quality: QualityPreset::Medium.to_quality(),
    // ...
};

// Only use custom settings when necessary
```

## Complete Example

See `examples/streaming_api_demo.rs` for a comprehensive example demonstrating all API features.

## API Reference

### Streaming Trait Methods

| Method | Description |
|--------|-------------|
| `start_camera_stream` | Start streaming from camera |
| `start_screen_stream` | Start screen sharing |
| `stop_stream` | Stop a stream session |
| `pause_stream` | Pause a stream |
| `resume_stream` | Resume a paused stream |
| `get_stream` | Get stream session info |
| `get_active_streams` | List all active streams |
| `adjust_quality` | Change stream quality |
| `get_stream_stats` | Get stream statistics |
| `set_auto_quality` | Enable/disable auto quality |
| `list_cameras` | List available cameras |
| `get_camera_capabilities` | Get camera capabilities |
| `add_viewer` | Add a viewer to stream |
| `remove_viewer` | Remove a viewer |
| `get_viewers` | List all viewers |
| `approve_viewer` | Approve viewer request |
| `reject_viewer` | Reject viewer request |
| `start_recording` | Start recording stream |
| `stop_recording` | Stop and finalize recording |
| `pause_recording` | Pause recording |
| `resume_recording` | Resume recording |
| `get_recording_status` | Get recording status |
| `register_event_handler` | Register event handler |
| `unregister_event_handler` | Unregister event handler |

## Integration with Other Systems

The Unified Streaming API automatically integrates with:

- **Security System**: All streams are encrypted and authenticated
- **Transport Layer**: Optimal transport protocol selection
- **Discovery**: Automatic peer discovery for viewers
- **Quality Manager**: Automatic quality adaptation (when enabled)

No additional configuration is required for these integrations.
