//! Clipboard Monitor Demo
//! 
//! Demonstrates clipboard change detection with intelligent filtering and error handling.

use kizuna::clipboard::monitor::{ClipboardMonitor, UnifiedClipboardMonitor, ChangeFilterConfig, ErrorHandlingConfig};
use kizuna::clipboard::{ClipboardContent, TextContent};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Clipboard Monitor Demo ===\n");
    
    // Create a clipboard monitor with custom configuration
    let filter_config = ChangeFilterConfig {
        min_change_interval: Duration::from_millis(100),
        programmatic_ignore_window: Duration::from_secs(1),
        max_changes_per_window: 10,
        rate_limit_window: Duration::from_secs(5),
    };
    
    let error_config = ErrorHandlingConfig {
        max_retries: 3,
        initial_backoff: Duration::from_millis(100),
        max_backoff: Duration::from_secs(5),
        backoff_multiplier: 2.0,
        continue_on_permission_error: false,
    };
    
    let monitor = UnifiedClipboardMonitor::with_config(filter_config);
    monitor.set_error_config(error_config).await;
    
    println!("Platform: {}", monitor.platform_name());
    println!("Starting clipboard monitoring...\n");
    
    // Subscribe to clipboard changes
    let mut receiver = monitor.subscribe_to_changes();
    
    // Start monitoring
    monitor.start_monitoring().await?;
    
    // Spawn a task to listen for clipboard changes
    let listener = tokio::spawn(async move {
        let mut count = 0;
        while let Ok(event) = receiver.recv().await {
            count += 1;
            println!("📋 Clipboard Change #{}", count);
            println!("   Event ID: {}", event.event_id);
            println!("   Type: {:?}", event.event_type);
            println!("   Source: {:?}", event.source);
            
            if let Some(content) = &event.content {
                match content {
                    ClipboardContent::Text(text) => {
                        let preview = if text.text.len() > 50 {
                            format!("{}...", &text.text[..50])
                        } else {
                            text.text.clone()
                        };
                        println!("   Content: \"{}\"", preview);
                        println!("   Size: {} bytes", text.size);
                    }
                    ClipboardContent::Image(img) => {
                        println!("   Content: Image ({}x{})", img.width, img.height);
                        println!("   Format: {:?}", img.format);
                        println!("   Size: {} bytes", img.data.len());
                    }
                    ClipboardContent::Files(files) => {
                        println!("   Content: {} file(s)", files.len());
                    }
                    ClipboardContent::Custom { mime_type, data } => {
                        println!("   Content: Custom ({})", mime_type);
                        println!("   Size: {} bytes", data.len());
                    }
                }
            }
            println!();
            
            // Stop after 5 changes for demo purposes
            if count >= 5 {
                break;
            }
        }
    });
    
    // Demonstrate programmatic clipboard changes (should be filtered out)
    println!("Setting clipboard content programmatically...");
    let test_content = ClipboardContent::Text(TextContent::new(
        "This is a programmatic change - should not trigger event".to_string()
    ));
    monitor.set_content(test_content).await?;
    
    sleep(Duration::from_secs(2)).await;
    
    println!("\n💡 Try copying some text or images to your clipboard!");
    println!("   The monitor will detect changes and display them here.");
    println!("   Press Ctrl+C to stop.\n");
    
    // Wait for the listener to finish or timeout after 60 seconds
    tokio::select! {
        _ = listener => {
            println!("\n✅ Demo completed - detected 5 clipboard changes");
        }
        _ = sleep(Duration::from_secs(60)) => {
            println!("\n⏱️  Demo timeout - stopping monitor");
        }
    }
    
    // Stop monitoring
    monitor.stop_monitoring().await?;
    println!("Monitoring stopped.");
    
    Ok(())
}
