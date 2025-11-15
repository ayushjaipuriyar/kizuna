//! Linux clipboard implementation using X11 and Wayland

use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::env;
use crate::clipboard::{
    ClipboardContent, ClipboardResult, ClipboardError,
    TextContent, ImageContent, ImageFormat, TextFormat, TextEncoding
};
use super::PlatformClipboard;

#[cfg(target_os = "linux")]
use x11::xlib::{
    Display, XOpenDisplay, XCloseDisplay, XDefaultRootWindow, XInternAtom,
    XGetSelectionOwner, XSetSelectionOwner, XConvertSelection, XGetWindowProperty,
    XChangeProperty, XDeleteProperty, XFlush, XSync, XFree,
    XA_STRING, PropModeReplace, AnyPropertyType, Success,
};
#[cfg(target_os = "linux")]
use std::ptr;
#[cfg(target_os = "linux")]
use std::ffi::{CString, CStr};

/// Display backend type
#[derive(Debug, Clone, Copy, PartialEq)]
enum DisplayBackend {
    X11,
    Wayland,
    Unknown,
}

/// Linux clipboard implementation
pub struct LinuxClipboard {
    monitoring: Arc<Mutex<MonitoringState>>,
    backend: DisplayBackend,
}

struct MonitoringState {
    active: bool,
    #[cfg(target_os = "linux")]
    display: Option<*mut Display>,
}

impl LinuxClipboard {
    /// Create new Linux clipboard
    pub fn new() -> Self {
        let backend = Self::detect_backend();
        Self {
            monitoring: Arc::new(Mutex::new(MonitoringState {
                active: false,
                #[cfg(target_os = "linux")]
                display: None,
            })),
            backend,
        }
    }
    
    /// Detect which display backend is in use
    fn detect_backend() -> DisplayBackend {
        if env::var("WAYLAND_DISPLAY").is_ok() {
            DisplayBackend::Wayland
        } else if env::var("DISPLAY").is_ok() {
            DisplayBackend::X11
        } else {
            DisplayBackend::Unknown
        }
    }
    
    #[cfg(target_os = "linux")]
    /// Read text from X11 clipboard
    fn read_x11_text() -> ClipboardResult<Option<String>> {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err(ClipboardError::platform("Failed to open X11 display"));
            }
            
            let clipboard_atom = XInternAtom(display, b"CLIPBOARD\0".as_ptr() as *const i8, 0);
            let utf8_atom = XInternAtom(display, b"UTF8_STRING\0".as_ptr() as *const i8, 0);
            let owner = XGetSelectionOwner(display, clipboard_atom);
            
            if owner == 0 {
                XCloseDisplay(display);
                return Ok(None);
            }
            
            let root = XDefaultRootWindow(display);
            let property = XInternAtom(display, b"KIZUNA_CLIPBOARD\0".as_ptr() as *const i8, 0);
            
            XConvertSelection(display, clipboard_atom, utf8_atom, property, root, 0);
            XFlush(display);
            
            // Wait a bit for the selection to be converted
            std::thread::sleep(std::time::Duration::from_millis(50));
            
            let mut actual_type = 0;
            let mut actual_format = 0;
            let mut nitems = 0;
            let mut bytes_after = 0;
            let mut prop: *mut u8 = ptr::null_mut();
            
            let result = XGetWindowProperty(
                display,
                root,
                property,
                0,
                1024 * 1024, // Max 1MB
                0,
                AnyPropertyType as u64,
                &mut actual_type,
                &mut actual_format,
                &mut nitems,
                &mut bytes_after,
                &mut prop,
            );
            
            let text = if result == Success as i32 && !prop.is_null() && nitems > 0 {
                let slice = std::slice::from_raw_parts(prop, nitems as usize);
                let text = String::from_utf8_lossy(slice).to_string();
                XFree(prop as *mut _);
                Some(text)
            } else {
                None
            };
            
            XDeleteProperty(display, root, property);
            XCloseDisplay(display);
            
            Ok(text)
        }
    }
    
    #[cfg(target_os = "linux")]
    /// Write text to X11 clipboard
    fn write_x11_text(text: &str) -> ClipboardResult<()> {
        unsafe {
            let display = XOpenDisplay(ptr::null());
            if display.is_null() {
                return Err(ClipboardError::platform("Failed to open X11 display"));
            }
            
            let clipboard_atom = XInternAtom(display, b"CLIPBOARD\0".as_ptr() as *const i8, 0);
            let utf8_atom = XInternAtom(display, b"UTF8_STRING\0".as_ptr() as *const i8, 0);
            let root = XDefaultRootWindow(display);
            let property = XInternAtom(display, b"KIZUNA_CLIPBOARD\0".as_ptr() as *const i8, 0);
            
            // Store the text in a property
            XChangeProperty(
                display,
                root,
                property,
                utf8_atom,
                8,
                PropModeReplace,
                text.as_ptr(),
                text.len() as i32,
            );
            
            // Claim ownership of the clipboard
            XSetSelectionOwner(display, clipboard_atom, root, 0);
            
            // Verify ownership
            let owner = XGetSelectionOwner(display, clipboard_atom);
            if owner != root {
                XCloseDisplay(display);
                return Err(ClipboardError::platform("Failed to claim clipboard ownership"));
            }
            
            XFlush(display);
            XCloseDisplay(display);
            
            Ok(())
        }
    }
    
    #[cfg(target_os = "linux")]
    /// Read text from Wayland clipboard (using arboard as fallback)
    fn read_wayland_text() -> ClipboardResult<Option<String>> {
        // Wayland clipboard access is complex and typically requires a compositor-specific protocol
        // For now, we'll use arboard as a fallback which handles Wayland
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                match clipboard.get_text() {
                    Ok(text) => Ok(Some(text)),
                    Err(_) => Ok(None),
                }
            }
            Err(e) => Err(ClipboardError::platform(format!("Failed to access Wayland clipboard: {}", e))),
        }
    }
    
    #[cfg(target_os = "linux")]
    /// Write text to Wayland clipboard (using arboard as fallback)
    fn write_wayland_text(text: &str) -> ClipboardResult<()> {
        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                clipboard.set_text(text)
                    .map_err(|e| ClipboardError::platform(format!("Failed to set Wayland clipboard: {}", e)))
            }
            Err(e) => Err(ClipboardError::platform(format!("Failed to access Wayland clipboard: {}", e))),
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    fn read_x11_text() -> ClipboardResult<Option<String>> {
        Ok(None)
    }
    
    #[cfg(not(target_os = "linux"))]
    fn write_x11_text(_text: &str) -> ClipboardResult<()> {
        Ok(())
    }
    
    #[cfg(not(target_os = "linux"))]
    fn read_wayland_text() -> ClipboardResult<Option<String>> {
        Ok(None)
    }
    
    #[cfg(not(target_os = "linux"))]
    fn write_wayland_text(_text: &str) -> ClipboardResult<()> {
        Ok(())
    }
}

#[async_trait]
impl PlatformClipboard for LinuxClipboard {
    async fn get_content(&self) -> ClipboardResult<Option<ClipboardContent>> {
        let text = match self.backend {
            DisplayBackend::X11 => Self::read_x11_text()?,
            DisplayBackend::Wayland => Self::read_wayland_text()?,
            DisplayBackend::Unknown => {
                return Err(ClipboardError::platform("Unknown display backend"));
            }
        };
        
        if let Some(text) = text {
            let size = text.len();
            return Ok(Some(ClipboardContent::Text(TextContent {
                text,
                encoding: TextEncoding::Utf8,
                format: TextFormat::Plain,
                size,
            })));
        }
        
        Ok(None)
    }
    
    async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()> {
        match content {
            ClipboardContent::Text(text_content) => {
                match self.backend {
                    DisplayBackend::X11 => Self::write_x11_text(&text_content.text)?,
                    DisplayBackend::Wayland => Self::write_wayland_text(&text_content.text)?,
                    DisplayBackend::Unknown => {
                        return Err(ClipboardError::platform("Unknown display backend"));
                    }
                }
                Ok(())
            }
            ClipboardContent::Image(_) => {
                Err(ClipboardError::format("Image clipboard writing not yet implemented on Linux"))
            }
            _ => {
                Err(ClipboardError::format("Unsupported clipboard content type"))
            }
        }
    }
    
    async fn start_monitoring(&self) -> ClipboardResult<()> {
        let mut state = self.monitoring.lock()
            .map_err(|_| ClipboardError::internal("Failed to lock monitoring state"))?;
        
        #[cfg(target_os = "linux")]
        {
            if self.backend == DisplayBackend::X11 {
                unsafe {
                    let display = XOpenDisplay(ptr::null());
                    if !display.is_null() {
                        state.display = Some(display);
                    }
                }
            }
        }
        
        state.active = true;
        Ok(())
    }
    
    async fn stop_monitoring(&self) -> ClipboardResult<()> {
        let mut state = self.monitoring.lock()
            .map_err(|_| ClipboardError::internal("Failed to lock monitoring state"))?;
        
        #[cfg(target_os = "linux")]
        {
            if let Some(display) = state.display {
                unsafe {
                    XCloseDisplay(display);
                }
                state.display = None;
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
        match self.backend {
            DisplayBackend::X11 => "linux-x11",
            DisplayBackend::Wayland => "linux-wayland",
            DisplayBackend::Unknown => "linux-unknown",
        }
    }
}

impl Default for LinuxClipboard {
    fn default() -> Self {
        Self::new()
    }
}

// Safety: The Display pointer is only used within the monitoring state
// and is properly cleaned up when monitoring stops
#[cfg(target_os = "linux")]
unsafe impl Send for MonitoringState {}
#[cfg(target_os = "linux")]
unsafe impl Sync for MonitoringState {}