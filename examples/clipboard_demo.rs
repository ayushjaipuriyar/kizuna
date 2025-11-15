//! Clipboard system demonstration
//! 
//! This example demonstrates the platform abstraction layer for clipboard access.

use kizuna::clipboard::{
    ClipboardContent, TextContent, TextEncoding, TextFormat,
    monitor::{ClipboardMonitor, UnifiedClipboardMonitor},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Kizuna Clipboard System Demo ===\n");
    
    // Create unified clipboard monitor
    let monitor = UnifiedClipboardMonitor::new();
    println!("Platform: {}", monitor.platform_name());
    
    // Test 1: Get current clipboard content
    println!("\n--- Test 1: Get Current Clipboard Content ---");
    match monitor.get_current_content().await {
        Ok(Some(content)) => {
            println!("Current clipboard content:");
            match content {
                ClipboardContent::Text(text) => {
                    println!("  Type: Text");
                    println!("  Size: {} bytes", text.size);
                    println!("  Preview: {}", 
                        if text.text.len() > 50 {
                            format!("{}...", &text.text[..50])
                        } else {
                            text.text.clone()
                        }
                    );
                }
                ClipboardContent::Image(img) => {
                    println!("  Type: Image");
                    println!("  Format: {:?}", img.format);
                    println!("  Size: {}x{}", img.width, img.height);
                    println!("  Data size: {} bytes", img.data.len());
                }
                _ => println!("  Type: Other"),
            }
        }
        Ok(None) => println!("Clipboard is empty"),
        Err(e) => println!("Error reading clipboard: {}", e),
    }
    
    // Test 2: Set clipboard content
    println!("\n--- Test 2: Set Clipboard Content ---");
    let test_text = TextContent {
        text: "Hello from Kizuna clipboard system!".to_string(),
        encoding: TextEncoding::Utf8,
        format: TextFormat::Plain,
        size: 36,
    };
    
    match monitor.set_content(ClipboardContent::Text(test_text)).await {
        Ok(_) => println!("✓ Successfully set clipboard content"),
        Err(e) => println!("✗ Failed to set clipboard: {}", e),
    }
    
    // Verify the content was set
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    match monitor.get_current_content().await {
        Ok(Some(ClipboardContent::Text(text))) => {
            println!("✓ Verified clipboard content: {}", text.text);
        }
        Ok(_) => println!("✗ Clipboard content type mismatch"),
        Err(e) => println!("✗ Failed to verify: {}", e),
    }
    
    // Test 3: Start monitoring
    println!("\n--- Test 3: Clipboard Monitoring ---");
    println!("Starting clipboard monitor...");
    
    match monitor.start_monitoring().await {
        Ok(_) => {
            println!("✓ Monitor started successfully");
            println!("Monitoring status: {}", monitor.is_monitoring());
            
            // Subscribe to changes
            let mut receiver = monitor.subscribe_to_changes();
            
            println!("\nMonitoring clipboard for 5 seconds...");
            println!("Try copying some text to see change detection!");
            
            let timeout = tokio::time::sleep(tokio::time::Duration::from_secs(5));
            tokio::pin!(timeout);
            
            loop {
                tokio::select! {
                    _ = &mut timeout => {
                        println!("\nMonitoring period ended");
                        break;
                    }
                    event = receiver.recv() => {
                        match event {
                            Ok(evt) => {
                                println!("\n📋 Clipboard change detected!");
                                println!("  Event ID: {}", evt.event_id);
                                println!("  Event type: {:?}", evt.event_type);
                                println!("  Source: {:?}", evt.source);
                                if let Some(content) = evt.content {
                                    match content {
                                        ClipboardContent::Text(text) => {
                                            println!("  Content: {}", 
                                                if text.text.len() > 50 {
                                                    format!("{}...", &text.text[..50])
                                                } else {
                                                    text.text
                                                }
                                            );
                                        }
                                        _ => println!("  Content: Non-text"),
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Error receiving event: {}", e);
                                break;
                            }
                        }
                    }
                }
            }
            
            // Stop monitoring
            match monitor.stop_monitoring().await {
                Ok(_) => println!("\n✓ Monitor stopped successfully"),
                Err(e) => println!("\n✗ Failed to stop monitor: {}", e),
            }
        }
        Err(e) => println!("✗ Failed to start monitor: {}", e),
    }
    
    println!("\n=== Demo Complete ===");
    Ok(())
}
