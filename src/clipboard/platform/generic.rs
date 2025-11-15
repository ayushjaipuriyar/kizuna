//! Generic clipboard implementation using arboard as fallback

use async_trait::async_trait;
use arboard::Clipboard as ArboardClipboard;
use std::sync::{Arc, Mutex};
use crate::clipboard::{
    ClipboardContent, ClipboardResult, ClipboardError,
    TextContent, ImageContent, ImageFormat, TextFormat, TextEncoding
};
use super::PlatformClipboard;

/// Generic clipboard implementation using arboard
pub struct GenericClipboard {
    clipboard: Arc<Mutex<ArboardClipboard>>,
    monitoring: Arc<Mutex<bool>>,
}

impl GenericClipboard {
    /// Create new generic clipboard
    pub fn new() -> Self {
        let clipboard = ArboardClipboard::new()
            .map_err(|e| ClipboardError::platform(format!("Failed to create clipboard: {}", e)))
            .unwrap_or_else(|_| {
                // Fallback to a mock clipboard if arboard fails
                panic!("Failed to initialize clipboard")
            });
            
        Self {
            clipboard: Arc::new(Mutex::new(clipboard)),
            monitoring: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait]
impl PlatformClipboard for GenericClipboard {
    async fn get_content(&self) -> ClipboardResult<Option<ClipboardContent>> {
        let mut clipboard = self.clipboard.lock()
            .map_err(|_| ClipboardError::internal("Failed to lock clipboard"))?;
            
        // Try to get text content first
        if let Ok(text) = clipboard.get_text() {
            let content = TextContent {
                text: text.clone(),
                encoding: TextEncoding::Utf8,
                format: TextFormat::Plain,
                size: text.len(),
            };
            return Ok(Some(ClipboardContent::Text(content)));
        }
        
        // Try to get image content
        if let Ok(image_data) = clipboard.get_image() {
            let content = ImageContent {
                data: image_data.bytes.into_owned(),
                format: ImageFormat::Png, // arboard typically provides PNG
                width: image_data.width as u32,
                height: image_data.height as u32,
                compressed: false,
            };
            return Ok(Some(ClipboardContent::Image(content)));
        }
        
        Ok(None)
    }
    
    async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()> {
        let mut clipboard = self.clipboard.lock()
            .map_err(|_| ClipboardError::internal("Failed to lock clipboard"))?;
            
        match content {
            ClipboardContent::Text(text_content) => {
                clipboard.set_text(&text_content.text)?;
            }
            ClipboardContent::Image(image_content) => {
                let image_data = arboard::ImageData {
                    width: image_content.width as usize,
                    height: image_content.height as usize,
                    bytes: std::borrow::Cow::Borrowed(&image_content.data),
                };
                clipboard.set_image(image_data)?;
            }
            ClipboardContent::Files(_) => {
                return Err(ClipboardError::format("File clipboard not supported in generic implementation"));
            }
            ClipboardContent::Custom { .. } => {
                return Err(ClipboardError::format("Custom clipboard formats not supported in generic implementation"));
            }
        }
        
        Ok(())
    }
    
    async fn start_monitoring(&self) -> ClipboardResult<()> {
        let mut monitoring = self.monitoring.lock()
            .map_err(|_| ClipboardError::internal("Failed to lock monitoring state"))?;
        *monitoring = true;
        
        // Note: Generic implementation doesn't support real-time monitoring
        // This would need to be implemented with polling in a real scenario
        Ok(())
    }
    
    async fn stop_monitoring(&self) -> ClipboardResult<()> {
        let mut monitoring = self.monitoring.lock()
            .map_err(|_| ClipboardError::internal("Failed to lock monitoring state"))?;
        *monitoring = false;
        Ok(())
    }
    
    fn is_monitoring(&self) -> bool {
        self.monitoring.lock()
            .map(|m| *m)
            .unwrap_or(false)
    }
    
    fn platform_name(&self) -> &'static str {
        "generic"
    }
}

impl Default for GenericClipboard {
    fn default() -> Self {
        Self::new()
    }
}