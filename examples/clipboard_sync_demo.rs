//! Clipboard synchronization and peer management demo
//!
//! This example demonstrates:
//! - Device allowlist management
//! - Clipboard content synchronization
//! - Conflict resolution
//! - Retry mechanisms for failed syncs

use kizuna::clipboard::{
    ClipboardContent, TextContent, TextEncoding, TextFormat,
    sync::{DefaultSyncManager, SyncManager, RetryConfig, SyncNotification},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Clipboard Synchronization Demo ===\n");
    
    // Create sync manager
    let sync_manager = DefaultSyncManager::new();
    
    // Set up notification callback
    sync_manager.set_notification_callback(|notification| {
        match notification {
            SyncNotification::SyncStarted { device_id } => {
                println!("📤 Sync started to device: {}", device_id);
            }
            SyncNotification::SyncCompleted { device_id } => {
                println!("✅ Sync completed to device: {}", device_id);
            }
            SyncNotification::SyncFailed { device_id, error } => {
                println!("❌ Sync failed to device {}: {}", device_id, error);
            }
            SyncNotification::ContentBlocked { reason, patterns } => {
                println!("🚫 Content blocked: {} (patterns: {:?})", reason, patterns);
            }
            SyncNotification::ConflictDetected { local_timestamp, remote_timestamp, resolution } => {
                println!("⚠️  Conflict detected - Local: {:?}, Remote: {:?}, Resolution: {:?}",
                    local_timestamp, remote_timestamp, resolution);
            }
            SyncNotification::RetryScheduled { device_id, attempt, delay_ms } => {
                println!("🔄 Retry scheduled for device {} (attempt {}, delay {}ms)",
                    device_id, attempt, delay_ms);
            }
            _ => {}
        }
    })?;
    
    // Configure retry settings
    let retry_config = RetryConfig {
        max_attempts: 3,
        initial_delay_ms: 1000,
        max_delay_ms: 10000,
        backoff_multiplier: 2.0,
    };
    sync_manager.set_retry_config(retry_config)?;
    
    println!("1. Device Allowlist Management\n");
    
    // Add devices to allowlist
    sync_manager.add_device(
        "device-1".to_string(),
        "Laptop".to_string(),
        "Desktop".to_string(),
    )?;
    println!("✓ Added device: Laptop (device-1)");
    
    sync_manager.add_device(
        "device-2".to_string(),
        "Phone".to_string(),
        "Mobile".to_string(),
    )?;
    println!("✓ Added device: Phone (device-2)");
    
    sync_manager.add_device(
        "device-3".to_string(),
        "Tablet".to_string(),
        "Tablet".to_string(),
    )?;
    println!("✓ Added device: Tablet (device-3)\n");
    
    // List all devices
    let devices = sync_manager.get_all_devices()?;
    println!("Total devices in allowlist: {}", devices.len());
    for device in &devices {
        println!("  - {} ({}): {}", device.device_name, device.device_id, device.device_type);
    }
    println!();
    
    // Enable sync for specific devices
    println!("2. Enabling Sync for Devices\n");
    sync_manager.enable_sync_for_device("device-1".to_string()).await?;
    println!("✓ Enabled sync for device-1 (Laptop)");
    
    sync_manager.enable_sync_for_device("device-2".to_string()).await?;
    println!("✓ Enabled sync for device-2 (Phone)\n");
    
    // Get enabled devices
    let enabled_devices = sync_manager.get_enabled_devices()?;
    println!("Enabled devices: {:?}\n", enabled_devices);
    
    // Get sync status
    println!("3. Sync Status\n");
    let status = sync_manager.get_sync_status().await?;
    for device_status in &status {
        println!("Device: {} ({})", device_status.device_name, device_status.device_id);
        println!("  Sync enabled: {}", device_status.sync_enabled);
        println!("  Connection: {:?}", device_status.connection_status);
        println!("  Sync count: {}", device_status.sync_count);
        println!("  Last sync: {:?}", device_status.last_sync);
        println!();
    }
    
    // Sync clipboard content
    println!("4. Syncing Clipboard Content\n");
    
    let text_content = ClipboardContent::Text(TextContent {
        text: "Hello from clipboard sync!".to_string(),
        encoding: TextEncoding::Utf8,
        format: TextFormat::Plain,
        size: 27,
    });
    
    println!("Syncing text content to enabled devices...");
    sync_manager.sync_content_to_peers(text_content.clone()).await?;
    println!();
    
    // Get updated statistics
    println!("5. Device Statistics\n");
    for device_id in &enabled_devices {
        if let Some(stats) = sync_manager.get_device_statistics(device_id)? {
            println!("Device: {}", device_id);
            println!("  Total syncs: {}", stats.total_syncs);
            println!("  Successful: {}", stats.successful_syncs);
            println!("  Failed: {}", stats.failed_syncs);
            println!("  Bytes sent: {}", stats.bytes_sent);
            println!("  Bytes received: {}", stats.bytes_received);
            if let Some(duration) = stats.last_sync_duration_ms {
                println!("  Last sync duration: {}ms", duration);
            }
            if let Some(avg) = stats.average_sync_duration_ms {
                println!("  Average sync duration: {}ms", avg);
            }
            println!();
        }
    }
    
    // Simulate receiving content from peer
    println!("6. Receiving Content from Peer\n");
    
    let received_content = ClipboardContent::Text(TextContent {
        text: "Content from remote device".to_string(),
        encoding: TextEncoding::Utf8,
        format: TextFormat::Plain,
        size: 26,
    });
    
    println!("Receiving content from device-1...");
    sync_manager.receive_content_from_peer(received_content, "device-1".to_string()).await?;
    println!("✓ Content received and applied\n");
    
    // Test privacy filtering
    println!("7. Privacy Filtering\n");
    
    let sensitive_content = ClipboardContent::Text(TextContent {
        text: "My password is: secret123".to_string(),
        encoding: TextEncoding::Utf8,
        format: TextFormat::Plain,
        size: 25,
    });
    
    println!("Attempting to sync sensitive content...");
    match sync_manager.sync_content_to_peers(sensitive_content).await {
        Ok(_) => println!("Content synced (unexpected)"),
        Err(e) => println!("✓ Content blocked as expected: {}\n", e),
    }
    
    // Disable sync for a device
    println!("8. Disabling Sync\n");
    sync_manager.disable_sync_for_device("device-2".to_string()).await?;
    println!("✓ Disabled sync for device-2 (Phone)\n");
    
    let enabled_devices = sync_manager.get_enabled_devices()?;
    println!("Enabled devices after disabling: {:?}\n", enabled_devices);
    
    // Check pending retries
    println!("9. Retry Management\n");
    let retry_count = sync_manager.pending_retry_count()?;
    println!("Pending retries: {}", retry_count);
    
    if retry_count > 0 {
        println!("Processing pending retries...");
        sync_manager.process_pending_retries().await?;
        println!("✓ Retries processed\n");
    }
    
    // Get last known content
    println!("10. Last Known Content\n");
    if let Some(last_content) = sync_manager.get_last_content()? {
        println!("Last content timestamp: {:?}", last_content.timestamp);
        println!("Source device: {}", last_content.source_device);
        println!("Sequence number: {}", last_content.sequence_number);
    } else {
        println!("No last content recorded");
    }
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}
