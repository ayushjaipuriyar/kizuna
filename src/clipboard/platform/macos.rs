//! macOS clipboard implementation using NSPasteboard

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use crate::clipboard::{
    ClipboardContent, ClipboardResult, ClipboardError,
    TextContent, ImageContent, ImageFormat, TextFormat, TextEncoding
};
use super::PlatformClipboard;

#[cfg(target_os = "macos")]
use cocoa::base::{id, nil};
#[cfg(target_os = "macos")]
use cocoa::foundation::{NSString, NSData, NSArray};
#[cfg(target_os = "macos")]
use cocoa::appkit::{NSPasteboard, NSPasteboardTypeString, NSPasteboardTypePNG, NSPasteboardTypeTIFF};
#[cfg(target_os = "macos")]
use objc::runtime::{Object, Class};
#[cfg(target_os = "macos")]
use objc::{msg_send, sel, sel_impl};

/// macOS clipboard implementation
pub struct MacOSClipboard {
    monitoring: Arc<Mutex<MonitoringState>>,
}

struct MonitoringState {
    active: bool,
    last_change_count: i64,
}

impl MacOSClipboard {
    /// Create new macOS clipboard
    pub fn new() -> Self {
        Self {
            monitoring: Arc::new(Mutex::new(MonitoringState {
                active: false,
                last_change_count: 0,
            })),
        }
    }
    
    #[cfg(target_os = "macos")]
    /// Get the general pasteboard
    fn get_pasteboard() -> id {
        unsafe {
            NSPasteboard::generalPasteboard(nil)
        }
    }
    
    #[cfg(target_os = "macos")]
    /// Get current change count
    fn get_change_count() -> i64 {
        unsafe {
            let pasteboard = Self::get_pasteboard();
            msg_send![pasteboard, changeCount]
        }
    }
    
    #[cfg(target_os = "macos")]
    /// Read string from pasteboard
    fn read_string_internal() -> ClipboardResult<Option<String>> {
        unsafe {
            let pasteboard = Self::get_pasteboard();
            let string_type = NSPasteboardTypeString;
            
            let string: id = msg_send![pasteboard, stringForType: string_type];
            if string == nil {
                return Ok(None);
            }
            
            let utf8_ptr: *const u8 = msg_send![string, UTF8String];
            if utf8_ptr.is_null() {
                return Ok(None);
            }
            
            let c_str = std::ffi::CStr::from_ptr(utf8_ptr as *const i8);
            let text = c_str.to_string_lossy().to_string();
            Ok(Some(text))
        }
    }
    
    #[cfg(target_os = "macos")]
    /// Write string to pasteboard
    fn write_string_internal(text: &str) -> ClipboardResult<()> {
        unsafe {
            let pasteboard = Self::get_pasteboard();
            
            // Clear the pasteboard
            let _: () = msg_send![pasteboard, clearContents];
            
            // Create NSString
            let ns_string = NSString::alloc(nil);
            let ns_string = NSString::init_str(ns_string, text);
            
            // Create array with string type
            let string_type = NSPasteboardTypeString;
            let types = NSArray::arrayWithObject(nil, string_type);
            
            // Declare types
            let _: () = msg_send![pasteboard, declareTypes:types owner:nil];
            
            // Set string
            let success: bool = msg_send![pasteboard, setString:ns_string forType:string_type];
            
            if !success {
                return Err(ClipboardError::platform("Failed to set pasteboard string"));
            }
            
            Ok(())
        }
    }
    
    #[cfg(target_os = "macos")]
    /// Check if image is available
    fn has_image() -> bool {
        unsafe {
            let pasteboard = Self::get_pasteboard();
            let png_type = NSPasteboardTypePNG;
            let tiff_type = NSPasteboardTypeTIFF;
            
            let has_png: bool = msg_send![pasteboard, availableTypeFromArray:NSArray::arrayWithObject(nil, png_type)];
            let has_tiff: bool = msg_send![pasteboard, availableTypeFromArray:NSArray::arrayWithObject(nil, tiff_type)];
            
            has_png || has_tiff
        }
    }
    
    #[cfg(not(target_os = "macos"))]
    fn get_change_count() -> i64 {
        0
    }
    
    #[cfg(not(target_os = "macos"))]
    fn read_string_internal() -> ClipboardResult<Option<String>> {
        Ok(None)
    }
    
    #[cfg(not(target_os = "macos"))]
    fn write_string_internal(_text: &str) -> ClipboardResult<()> {
        Ok(())
    }
    
    #[cfg(not(target_os = "macos"))]
    fn has_image() -> bool {
        false
    }
}

#[async_trait]
impl PlatformClipboard for MacOSClipboard {
    async fn get_content(&self) -> ClipboardResult<Option<ClipboardContent>> {
        // Try to read string first
        if let Some(text) = Self::read_string_internal()? {
            let size = text.len();
            return Ok(Some(ClipboardContent::Text(TextContent {
                text,
                encoding: TextEncoding::Utf8,
                format: TextFormat::Plain,
                size,
            })));
        }
        
        // Check for image
        if Self::has_image() {
            // For now, we just detect that an image is present
            // Full image reading would require additional implementation
            return Ok(None);
        }
        
        Ok(None)
    }
    
    async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()> {
        match content {
            ClipboardContent::Text(text_content) => {
                Self::write_string_internal(&text_content.text)?;
                Ok(())
            }
            ClipboardContent::Image(_) => {
                // Image writing would require additional implementation
                Err(ClipboardError::format("Image clipboard writing not yet implemented on macOS"))
            }
            _ => {
                Err(ClipboardError::format("Unsupported clipboard content type"))
            }
        }
    }
    
    async fn start_monitoring(&self) -> ClipboardResult<()> {
        let mut state = self.monitoring.lock()
            .map_err(|_| ClipboardError::internal("Failed to lock monitoring state"))?;
        
        state.last_change_count = Self::get_change_count();
        state.active = true;
        Ok(())
    }
    
    async fn stop_monitoring(&self) -> ClipboardResult<()> {
        let mut state = self.monitoring.lock()
            .map_err(|_| ClipboardError::internal("Failed to lock monitoring state"))?;
        
        state.active = false;
        Ok(())
    }
    
    fn is_monitoring(&self) -> bool {
        self.monitoring.lock()
            .map(|m| m.active)
            .unwrap_or(false)
    }
    
    fn platform_name(&self) -> &'static str {
        "macos"
    }
}

impl Default for MacOSClipboard {
    fn default() -> Self {
        Self::new()
    }
}