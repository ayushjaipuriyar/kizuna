//! Clipboard content processing demonstration
//!
//! This example demonstrates the clipboard content processing capabilities including:
//! - Text content processing with UTF-8 support and format conversion
//! - Image content processing with compression
//! - Cross-platform format conversion

use kizuna::clipboard::{
    ClipboardContent, TextContent, ImageContent, TextFormat, ImageFormat,
};
use kizuna::clipboard::content::{
    DefaultContentProcessor, ContentProcessor, TextProcessor, ImageProcessor, FormatConverter,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Clipboard Content Processing Demo ===\n");
    
    // Create content processor
    let processor = DefaultContentProcessor::new();
    
    // Demo 1: Text content processing
    println!("1. Text Content Processing");
    println!("--------------------------");
    
    let text_processor = TextProcessor::new();
    
    // Process plain text
    let plain_text = "Hello, World!\nThis is a test of clipboard text processing.";
    let text_content = text_processor.process_text(plain_text, TextFormat::Plain)?;
    println!("Plain text processed: {} bytes", text_content.size);
    
    // Convert to HTML
    let html_content = text_processor.preserve_format(&text_content, TextFormat::Html)?;
    println!("Converted to HTML: {} bytes", html_content.size);
    println!("HTML preview: {}...\n", &html_content.text[..100.min(html_content.text.len())]);
    
    // Process HTML and strip to plain text
    let html_input = "<html><body><h1>Title</h1><p>This is <b>bold</b> text.</p></body></html>";
    let html_text = text_processor.process_text(html_input, TextFormat::Html)?;
    let plain_from_html = text_processor.to_plain_text(&html_text)?;
    println!("HTML stripped to plain: {}", plain_from_html.text);
    
    // Validate text content
    let validation = text_processor.validate_text(&text_content);
    println!("Text validation: valid={}, warnings={}\n", validation.is_valid, validation.warnings.len());
    
    // Demo 2: Image content processing
    println!("2. Image Content Processing");
    println!("---------------------------");
    
    let image_processor = ImageProcessor::new();
    
    // Create a simple test image (1x1 red pixel PNG)
    let test_png = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
        0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
        0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
        0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D,
        0xB4, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E,
        0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    
    let image_content = image_processor.process_image(&test_png, ImageFormat::Png)?;
    println!("Image processed: {}x{} pixels, {} bytes", 
        image_content.width, image_content.height, image_content.data.len());
    
    // Validate image
    let img_validation = image_processor.validate_image(&image_content);
    println!("Image validation: valid={}, warnings={}", 
        img_validation.is_valid, img_validation.warnings.len());
    
    // Convert image format
    let jpeg_image = image_processor.convert_format(&image_content, ImageFormat::Jpeg)?;
    println!("Converted to JPEG: {} bytes (compressed={})\n", 
        jpeg_image.data.len(), jpeg_image.compressed);
    
    // Demo 3: Cross-platform format conversion
    println!("3. Cross-Platform Format Conversion");
    println!("-----------------------------------");
    
    let format_converter = FormatConverter::new();
    
    // Convert text to Windows format
    let text_clip = ClipboardContent::Text(text_content.clone());
    let windows_formats = format_converter.to_platform_format(&text_clip, "windows")?;
    println!("Windows formats generated: {}", windows_formats.len());
    for (format, data) in &windows_formats {
        println!("  - {:?}: {} bytes", format, data.len());
    }
    
    // Convert text to macOS format
    let macos_formats = format_converter.to_platform_format(&text_clip, "macos")?;
    println!("\nmacOS formats generated: {}", macos_formats.len());
    for (format, data) in &macos_formats {
        println!("  - {:?}: {} bytes", format, data.len());
    }
    
    // Convert text to Linux format
    let linux_formats = format_converter.to_platform_format(&text_clip, "linux")?;
    println!("\nLinux formats generated: {}", linux_formats.len());
    for (format, data) in &linux_formats {
        println!("  - {:?}: {} bytes", format, data.len());
    }
    
    // Demo 4: Content validation and integrity
    println!("\n4. Content Validation and Integrity");
    println!("-----------------------------------");
    
    let clipboard_content = ClipboardContent::Text(text_content);
    let validation = processor.validate_content(&clipboard_content).await?;
    println!("Content validation:");
    println!("  Valid: {}", validation.is_valid);
    println!("  Size: {} bytes", validation.size_bytes);
    println!("  Errors: {}", validation.errors.len());
    println!("  Warnings: {}", validation.warnings.len());
    
    let integrity_ok = processor.validate_integrity(&clipboard_content)?;
    println!("  Integrity check: {}", if integrity_ok { "PASS" } else { "FAIL" });
    
    // Demo 5: Process outgoing content
    println!("\n5. Process Outgoing Content");
    println!("---------------------------");
    
    let processed = processor.process_outgoing_content(clipboard_content).await?;
    println!("Processed content:");
    println!("  Original size: {} bytes", processed.original_size);
    println!("  Processed size: {} bytes", processed.processed_size);
    println!("  Compressed: {}", processed.compressed);
    println!("  Reduction: {:.1}%", 
        if processed.original_size > 0 {
            100.0 * (1.0 - processed.processed_size as f64 / processed.original_size as f64)
        } else {
            0.0
        }
    );
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}
