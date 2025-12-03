// Multi-viewer broadcasting and management demonstration
//
// This example demonstrates the viewer management system including:
// - Viewer registration and authentication
// - Connection management and status monitoring
// - Multi-viewer broadcasting with quality adaptation
// - Permission management and access control

use kizuna::streaming::{
    ViewerManager, ViewerPermissions, QualityPreset, StreamQuality,
    Resolution, VideoStream, StreamSource, ScreenRegion,
};
use kizuna::streaming::viewer::{
    ViewerManagerImpl, ViewerManagementControls, ViewerConnectionResult,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Multi-Viewer Broadcasting Demo ===\n");

    // Create viewer manager
    let manager = ViewerManagerImpl::new();
    let registry = manager.registry();
    let controls = ViewerManagementControls::new(registry.clone());

    println!("1. Adding viewers with different permissions...\n");

    // Add viewer 1 with high quality permission
    let viewer1_perms = ViewerPermissions {
        can_view: true,
        can_record: true,
        can_control_quality: true,
        max_quality: QualityPreset::High,
    };
    
    match controls.handle_viewer_connection(
        "peer-001".to_string(),
        viewer1_perms,
        false,
    ).await? {
        ViewerConnectionResult::Connected(id) => {
            println!("✓ Viewer 1 connected: {}", id);
        }
        _ => println!("✗ Viewer 1 connection failed"),
    }

    // Add viewer 2 with medium quality permission
    let viewer2_perms = ViewerPermissions {
        can_view: true,
        can_record: false,
        can_control_quality: false,
        max_quality: QualityPreset::Medium,
    };
    
    match controls.handle_viewer_connection(
        "peer-002".to_string(),
        viewer2_perms,
        false,
    ).await? {
        ViewerConnectionResult::Connected(id) => {
            println!("✓ Viewer 2 connected: {}", id);
        }
        _ => println!("✗ Viewer 2 connection failed"),
    }

    // Add viewer 3 requiring approval
    let viewer3_perms = ViewerPermissions {
        can_view: true,
        can_record: false,
        can_control_quality: false,
        max_quality: QualityPreset::Low,
    };
    
    match controls.handle_viewer_connection(
        "peer-003".to_string(),
        viewer3_perms.clone(),
        true,
    ).await? {
        ViewerConnectionResult::PendingApproval => {
            println!("⏳ Viewer 3 pending approval");
        }
        _ => println!("✗ Viewer 3 connection failed"),
    }

    println!("\n2. Checking pending approvals...\n");
    let pending = controls.get_pending_approvals().await?;
    println!("Pending approvals: {}", pending.len());
    for approval in &pending {
        println!("  - Peer: {}", approval.peer_id);
    }

    // Approve viewer 3
    if !pending.is_empty() {
        let viewer3_id = controls.approve_pending_viewer("peer-003".to_string()).await?;
        println!("\n✓ Viewer 3 approved: {}", viewer3_id);
    }

    println!("\n3. Getting viewer status...\n");
    let statuses = manager.get_viewer_status().await?;
    println!("Connected viewers: {}", statuses.len());
    for status in &statuses {
        println!("  Viewer: {}", status.viewer_id);
        println!("    Peer: {}", status.peer_id);
        println!("    Device: {}", status.device_name);
        println!("    Quality: {:?}", status.connection_quality);
        println!("    Max Quality: {:?}", status.permissions.max_quality);
        println!("    Can Record: {}", status.permissions.can_record);
        println!();
    }

    println!("4. Broadcasting to viewers...\n");

    // Create a test video stream
    let stream = VideoStream {
        id: Uuid::new_v4(),
        source: StreamSource::Screen(ScreenRegion {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }),
        quality: StreamQuality {
            resolution: Resolution {
                width: 1920,
                height: 1080,
            },
            framerate: 30,
            bitrate: 3_000_000,
            quality_preset: QualityPreset::High,
            hardware_acceleration: true,
        },
    };

    // Broadcast to all viewers
    manager.broadcast_to_viewers(stream).await?;
    println!("✓ Broadcast sent to all viewers");

    println!("\n5. Monitoring connection quality...\n");
    
    // Simulate connection quality updates
    for status in &statuses {
        let quality = controls.monitor_connection_quality(
            status.viewer_id,
            100, // 100ms latency
            0.02, // 2% packet loss
        ).await?;
        println!("  Viewer {}: {:?}", status.viewer_id, quality);
    }

    println!("\n6. Getting detailed status reports...\n");
    let reports = controls.get_all_status_reports().await?;
    for report in &reports {
        println!("  Viewer: {}", report.status.viewer_id);
        println!("    Duration: {:?}", report.connection_duration);
        println!("    Avg Bitrate: {} bps", report.average_bitrate);
        println!("    Healthy: {}", report.is_healthy);
        println!();
    }

    println!("7. Managing viewer permissions...\n");
    
    if let Some(status) = statuses.first() {
        let viewer_id = status.viewer_id;
        
        // Grant recording permission
        controls.grant_recording_permission(viewer_id).await?;
        println!("✓ Granted recording permission to viewer {}", viewer_id);
        
        // Grant quality control
        controls.grant_quality_control(viewer_id).await?;
        println!("✓ Granted quality control to viewer {}", viewer_id);
    }

    println!("\n8. Checking viewer limits...\n");
    println!("Current viewer count: {}", controls.get_viewer_count().await);
    println!("Viewer limit reached: {}", controls.is_viewer_limit_reached().await);

    println!("\n9. Disconnecting viewers...\n");
    
    // Disconnect all viewers
    for status in &statuses {
        controls.handle_viewer_disconnection(status.viewer_id).await?;
        println!("✓ Disconnected viewer {}", status.viewer_id);
    }

    println!("\nFinal viewer count: {}", controls.get_viewer_count().await);

    println!("\n=== Demo Complete ===");

    Ok(())
}
