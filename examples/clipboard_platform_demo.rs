//! Clipboard platform abstraction demo
//!
//! This example demonstrates the platform-specific clipboard implementations
//! and the unified clipboard monitor interface.

use kizuna::clipboard::{
    ClipboardContent, TextContent, ClipboardMonitor,
};
use kizuna::clipboard::monitor::UnifiedClipboardMonitor;
use kizuna::clipboard::platform::UnifiedClipboard;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Clipboard Platform Abstraction Demo ===\n");
    
    // Create unified clipboard
    let clipboard = UnifiedClipboard::new();
    println!("Platform detected: {}", clipboard.platform_name());
    
    // Test basic clipboard operations
    println!("\n--- Testing Basic Clipboard Operations ---");
    
    // Set some text content
    let test_text = TextContent::new("Hello from Kizuna clipboard!".to_string());
    let content = ClipboardContent::Text(test_text);
    
    println!("Setting clipboard content...");
    clipboard.set_content(content.clone()).await?;
    println!("✓ Content set successfully");
    
    // Read it back
    println!("Reading clipboard content...");
    if let Some(read_content) = clipboard.get_content().await? {
        match read_content {
            ClipboardContent::Text(text) => {
                println!("✓ Read text: {}", text.text);
            }
            _ => println!("✗ Unexpected content type"),
        }
    } else {
        println!("✗ No content found");
    }
    
    // Test clipboard monitoring
    println!("\n--- Testing Clipboard Monitoring ---");
    
    let monitor = UnifiedClipboardMonitor::new();
    println!("Monitor platform: {}", monitor.platform_name());
    
    // Subscribe to changes
    let mut receiver = monitor.subscribe_to_changes();
    
    // Start monitoring
    println!("Starting clipboard monitoring...");
    monitor.start_monitoring().await?;
    println!("✓ Monitoring started (polling every 500ms)");
    
    // Set content through monitor
    println!("\nSetting content through monitor...");
    let test_text2 = TextContent::new("Testing change detection!".to_string());
    monitor.set_content(ClipboardContent::Text(test_text2)).await?;
    println!("✓ Content set");
    
    // Wait a bit for monitoring
    println!("\nMonitoring for clipboard changes for 3 seconds...");
    println!("(Try copying something to your clipboard!)");
    
    let timeout = tokio::time::timeout(Duration::from_secs(3), async {
        while let Ok(event) = receiver.recv().await {
            println!("\n📋 Clipboard event detected!");
            println!("   Event ID: {}", event.event_id);
            println!("   Event type: {:?}", event.event_type);
            println!("   Source: {:?}", event.source);
            
            if let Some(content) = event.content {
                match content {
                    ClipboardContent::Text(text) => {
                        let preview = if text.text.len() > 50 {
                            format!("{}...", &text.text[..50])
                        } else {
                            text.text.clone()
                        };
                        println!("   Content: {}", preview);
                    }
                    ClipboardContent::Image(img) => {
                        println!("   Content: Image ({}x{}, {} bytes)",
                            img.width, img.height, img.data.len());
                    }
                    _ => println!("   Content: Other type"),
                }
            }
        }
    }).await;
    
    match timeout {
        Ok(_) => println!("\n✓ Monitoring completed"),
        Err(_) => println!("\n✓ Monitoring timeout (no changes detected)"),
    }
    
    // Stop monitoring
    println!("\nStopping monitoring...");
    monitor.stop_monitoring().await?;
    println!("✓ Monitoring stopped");
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}
