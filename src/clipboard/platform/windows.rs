//! Windows clipboard implementation using Windows API

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::ptr;
use std::mem;
use crate::clipboard::{
    ClipboardContent, ClipboardResult, ClipboardError,
    TextContent, ImageContent, ImageFormat, TextFormat, TextEncoding
};
use super::PlatformClipboard;

#[cfg(windows)]
use winapi::um::winuser::{
    OpenClipboard, CloseClipboard, EmptyClipboard, SetClipboardData,
    GetClipboardData, IsClipboardFormatAvailable, AddClipboardFormatListener,
    RemoveClipboardFormatListener, GetDesktopWindow,
    CF_TEXT, CF_UNICODETEXT, CF_BITMAP, CF_DIB,
};
#[cfg(windows)]
use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalUnlock, GlobalSize, GMEM_MOVEABLE};
#[cfg(windows)]
use winapi::shared::windef::HWND;

/// Windows clipboard implementation
pub struct WindowsClipboard {
    monitoring: Arc<Mutex<MonitoringState>>,
}

struct MonitoringState {
    active: bool,
    #[cfg(windows)]
    hwnd: Option<HWND>,
}

impl WindowsClipboard {
    /// Create new Windows clipboard
    pub fn new() -> Self {
        Self {
            monitoring: Arc::new(Mutex::new(MonitoringState {
                active: false,
                #[cfg(windows)]
                hwnd: None,
            })),
        }
    }
    
    #[cfg(windows)]
    /// Read text from Windows clipboard
    fn read_text_internal() -> ClipboardResult<Option<String>> {
        unsafe {
            if OpenClipboard(ptr::null_mut()) == 0 {
                return Err(ClipboardError::platform("Failed to open clipboard"));
            }
            
            let result = if IsClipboardFormatAvailable(CF_UNICODETEXT) != 0 {
                let handle = GetClipboardData(CF_UNICODETEXT);
                if handle.is_null() {
                    None
                } else {
                    let ptr = GlobalLock(handle) as *const u16;
                    if ptr.is_null() {
                        None
                    } else {
                        let len = (0..).take_while(|&i| *ptr.offset(i) != 0).count();
                        let slice = std::slice::from_raw_parts(ptr, len);
                        let text = String::from_utf16_lossy(slice);
                        GlobalUnlock(handle);
                        Some(text)
                    }
                }
            } else if IsClipboardFormatAvailable(CF_TEXT) != 0 {
                let handle = GetClipboardData(CF_TEXT);
                if handle.is_null() {
                    None
                } else {
                    let ptr = GlobalLock(handle) as *const u8;
                    if ptr.is_null() {
                        None
                    } else {
                        let len = (0..).take_while(|&i| *ptr.offset(i) != 0).count();
                        let slice = std::slice::from_raw_parts(ptr, len);
                        let text = String::from_utf8_lossy(slice).to_string();
                        GlobalUnlock(handle);
                        Some(text)
                    }
                }
            } else {
                None
            };
            
            CloseClipboard();
            Ok(result)
        }
    }
    
    #[cfg(windows)]
    /// Write text to Windows clipboard
    fn write_text_internal(text: &str) -> ClipboardResult<()> {
        unsafe {
            if OpenClipboard(ptr::null_mut()) == 0 {
                return Err(ClipboardError::platform("Failed to open clipboard"));
            }
            
            EmptyClipboard();
            
            // Convert to UTF-16
            let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let size = wide.len() * mem::size_of::<u16>();
            
            let handle = GlobalAlloc(GMEM_MOVEABLE, size);
            if handle.is_null() {
                CloseClipboard();
                return Err(ClipboardError::platform("Failed to allocate memory"));
            }
            
            let ptr = GlobalLock(handle) as *mut u16;
            if ptr.is_null() {
                CloseClipboard();
                return Err(ClipboardError::platform("Failed to lock memory"));
            }
            
            ptr::copy_nonoverlapping(wide.as_ptr(), ptr, wide.len());
            GlobalUnlock(handle);
            
            if SetClipboardData(CF_UNICODETEXT, handle).is_null() {
                CloseClipboard();
                return Err(ClipboardError::platform("Failed to set clipboard data"));
            }
            
            CloseClipboard();
            Ok(())
        }
    }
    
    #[cfg(windows)]
    /// Check if bitmap is available
    fn has_bitmap() -> bool {
        unsafe {
            IsClipboardFormatAvailable(CF_BITMAP) != 0 || IsClipboardFormatAvailable(CF_DIB) != 0
        }
    }
    
    #[cfg(not(windows))]
    fn read_text_internal() -> ClipboardResult<Option<String>> {
        Ok(None)
    }
    
    #[cfg(not(windows))]
    fn write_text_internal(_text: &str) -> ClipboardResult<()> {
        Ok(())
    }
    
    #[cfg(not(windows))]
    fn has_bitmap() -> bool {
        false
    }
}

#[async_trait]
impl PlatformClipboard for WindowsClipboard {
    async fn get_content(&self) -> ClipboardResult<Option<ClipboardContent>> {
        // Try to read text first
        if let Some(text) = Self::read_text_internal()? {
            let size = text.len();
            return Ok(Some(ClipboardContent::Text(TextContent {
                text,
                encoding: TextEncoding::Utf8,
                format: TextFormat::Plain,
                size,
            })));
        }
        
        // Check for bitmap (basic detection, actual reading would require more complex code)
        if Self::has_bitmap() {
            // For now, we just detect that an image is present
            // Full bitmap reading would require additional implementation
            return Ok(None);
        }
        
        Ok(None)
    }
    
    async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()> {
        match content {
            ClipboardContent::Text(text_content) => {
                Self::write_text_internal(&text_content.text)?;
                Ok(())
            }
            ClipboardContent::Image(_) => {
                // Image writing would require additional implementation
                Err(ClipboardError::format("Image clipboard writing not yet implemented on Windows"))
            }
            _ => {
                Err(ClipboardError::format("Unsupported clipboard content type"))
            }
        }
    }
    
    async fn start_monitoring(&self) -> ClipboardResult<()> {
        let mut state = self.monitoring.lock()
            .map_err(|_| ClipboardError::internal("Failed to lock monitoring state"))?;
        
        #[cfg(windows)]
        {
            unsafe {
                let hwnd = GetDesktopWindow();
                if AddClipboardFormatListener(hwnd) == 0 {
                    return Err(ClipboardError::platform("Failed to add clipboard format listener"));
                }
                state.hwnd = Some(hwnd);
            }
        }
        
        state.active = true;
        Ok(())
    }
    
    async fn stop_monitoring(&self) -> ClipboardResult<()> {
        let mut state = self.monitoring.lock()
            .map_err(|_| ClipboardError::internal("Failed to lock monitoring state"))?;
        
        #[cfg(windows)]
        {
            if let Some(hwnd) = state.hwnd {
                unsafe {
                    RemoveClipboardFormatListener(hwnd);
                }
                state.hwnd = None;
            }
        }
        
        state.active = false;
        Ok(())
    }
    
    fn is_monitoring(&self) -> bool {
        self.monitoring.lock()
            .map(|m| m.active)
            .unwrap_or(false)
    }
    
    fn platform_name(&self) -> &'static str {
        "windows"
    }
}

impl Default for WindowsClipboard {
    fn default() -> Self {
        Self::new()
    }
}