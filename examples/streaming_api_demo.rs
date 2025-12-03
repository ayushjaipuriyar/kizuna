// Unified Streaming API Demo
//
// This example demonstrates the high-level streaming API that abstracts
// codec and network complexity behind simple interfaces.

use kizuna::streaming::{
    Streaming, StreamingApi, StreamConfig, ScreenConfig, StreamQuality,
    QualityPreset, ScreenRegion, StreamEvent, StreamEventHandler,
    ViewerPermissions, RecordingConfig, VideoFormat,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

/// Example event handler that logs all stream events
struct LoggingEventHandler {
    events: Arc<RwLock<Vec<String>>>,
}

impl LoggingEventHandler {
    fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    async fn print_events(&self) {
        let events = self.events.read().await;
        println!("\n=== Stream Events ===");
        for event in events.iter() {
            println!("  {}", event);
        }
        println!("=====================\n");
    }
}

#[async_trait]
impl StreamEventHandler for LoggingEventHandler {
    async fn on_event(&self, event: StreamEvent) {
        let message = match event {
            StreamEvent::SessionStarted { session_id, stream_type } => {
                format!("Session started: {:?} (type: {:?})", session_id, stream_type)
            }
            StreamEvent::SessionStopped { session_id, reason } => {
                format!("Session stopped: {:?} (reason: {:?})", session_id, reason)
            }
            StreamEvent::StateChanged { session_id, old_state, new_state } => {
                format!("State changed: {:?} ({:?} -> {:?})", session_id, old_state, new_state)
            }
            StreamEvent::QualityChanged { session_id, old_quality, new_quality, reason } => {
                format!(
                    "Quality changed: {:?} ({:?} -> {:?}, reason: {:?})",
                    session_id, old_quality.quality_preset, new_quality.quality_preset, reason
                )
            }
            StreamEvent::ViewerConnected { session_id, viewer_id, peer_id } => {
                format!("Viewer connected: {:?} (viewer: {:?}, peer: {})", session_id, viewer_id, peer_id)
            }
            StreamEvent::ViewerDisconnected { session_id, viewer_id, reason } => {
                format!("Viewer disconnected: {:?} (viewer: {:?}, reason: {})", session_id, viewer_id, reason)
            }
            StreamEvent::ViewerRequestReceived { session_id, peer_id, device_name } => {
                format!("Viewer request: {:?} (peer: {}, device: {})", session_id, peer_id, device_name)
            }
            StreamEvent::StatsUpdated { session_id, stats } => {
                format!(
                    "Stats updated: {:?} (bitrate: {} kbps, latency: {} ms)",
                    session_id, stats.current_bitrate / 1000, stats.latency_ms
                )
            }
            StreamEvent::Error { session_id, error, recoverable } => {
                format!("Error: {:?} (error: {}, recoverable: {})", session_id, error, recoverable)
            }
            StreamEvent::RecordingStarted { session_id, recording_session } => {
                format!("Recording started: {:?} (recording: {:?})", session_id, recording_session)
            }
            StreamEvent::RecordingStopped { session_id, recording_session, file_path } => {
                format!("Recording stopped: {:?} (recording: {:?}, file: {})", session_id, recording_session, file_path)
            }
            StreamEvent::NetworkConditionChanged { session_id, bandwidth_kbps, latency_ms, packet_loss } => {
                format!(
                    "Network changed: {:?} (bandwidth: {} kbps, latency: {} ms, loss: {:.2}%)",
                    session_id, bandwidth_kbps, latency_ms, packet_loss * 100.0
                )
            }
        };
        
        println!("[EVENT] {}", message);
        self.events.write().await.push(message);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Unified Streaming API Demo ===\n");
    
    // Create the streaming API
    let api = StreamingApi::new();
    
    // Create and register event handler
    let event_handler = Arc::new(LoggingEventHandler::new());
    api.register_event_handler(event_handler.clone()).await?;
    
    println!("1. Starting camera stream with Medium quality...");
    let camera_config = StreamConfig {
        quality: QualityPreset::Medium.to_quality(),
        enable_audio: true,
        enable_recording: false,
        max_viewers: 5,
    };
    
    let camera_session = api.start_camera_stream(camera_config).await?;
    println!("   Camera session started: {}", camera_session.session_id);
    println!("   Quality: {:?}", camera_session.quality.quality_preset);
    println!("   State: {:?}", camera_session.state);
    
    // Wait for events to process
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\n2. Starting screen stream...");
    let screen_config = ScreenConfig {
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
    
    let screen_session = api.start_screen_stream(screen_config).await?;
    println!("   Screen session started: {}", screen_session.session_id);
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\n3. Adding viewers to camera stream...");
    let viewer1 = api.add_viewer(
        camera_session.session_id,
        "peer-123".to_string(),
        ViewerPermissions {
            can_view: true,
            can_record: false,
            can_control_quality: false,
            max_quality: QualityPreset::Medium,
        },
    ).await?;
    println!("   Viewer 1 added: {}", viewer1);
    
    let viewer2 = api.add_viewer(
        camera_session.session_id,
        "peer-456".to_string(),
        ViewerPermissions {
            can_view: true,
            can_record: true,
            can_control_quality: true,
            max_quality: QualityPreset::High,
        },
    ).await?;
    println!("   Viewer 2 added: {}", viewer2);
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\n4. Adjusting camera stream quality to High...");
    let high_quality = QualityPreset::High.to_quality();
    api.adjust_quality(camera_session.session_id, high_quality).await?;
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\n5. Getting stream statistics...");
    let stats = api.get_stream_stats(camera_session.session_id).await?;
    println!("   Bytes sent: {}", stats.bytes_sent);
    println!("   Frames encoded: {}", stats.frames_encoded);
    println!("   Current bitrate: {} kbps", stats.current_bitrate / 1000);
    
    println!("\n6. Pausing camera stream...");
    api.pause_stream(camera_session.session_id).await?;
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\n7. Resuming camera stream...");
    api.resume_stream(camera_session.session_id).await?;
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\n8. Starting recording...");
    let recording_config = RecordingConfig {
        output_path: std::path::PathBuf::from("/tmp/stream_recording.mp4"),
        format: VideoFormat::MP4,
        quality: QualityPreset::High.to_quality(),
        max_file_size: Some(1024 * 1024 * 100), // 100 MB
        max_duration: Some(std::time::Duration::from_secs(300)), // 5 minutes
    };
    
    let recording_session = api.start_recording(
        camera_session.session_id,
        recording_config,
    ).await?;
    println!("   Recording started: {}", recording_session.session_id);
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\n9. Listing all active streams...");
    let active_streams = api.get_active_streams().await?;
    println!("   Active streams: {}", active_streams.len());
    for stream in active_streams {
        println!("     - {} ({:?}, {:?})", stream.session_id, stream.stream_type, stream.state);
    }
    
    println!("\n10. Removing viewer 1...");
    api.remove_viewer(camera_session.session_id, viewer1).await?;
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\n11. Stopping recording...");
    let recording_file = api.stop_recording(recording_session.session_id).await?;
    println!("   Recording saved: {:?}", recording_file.path);
    
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
    println!("\n12. Stopping streams...");
    api.stop_stream(camera_session.session_id).await?;
    println!("   Camera stream stopped");
    
    api.stop_stream(screen_session.session_id).await?;
    println!("   Screen stream stopped");
    
    // Wait for final events to process
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    // Print all events
    event_handler.print_events().await;
    
    println!("=== Demo Complete ===");
    
    Ok(())
}
