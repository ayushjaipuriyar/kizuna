//! Platform-specific clipboard implementations

use async_trait::async_trait;
use crate::clipboard::{ClipboardContent, ClipboardResult, Clipboard};

#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

pub mod generic;

/// Platform-specific clipboard trait
#[async_trait]
pub trait PlatformClipboard: Send + Sync {
    /// Get current clipboard content
    async fn get_content(&self) -> ClipboardResult<Option<ClipboardContent>>;
    
    /// Set clipboard content
    async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()>;
    
    /// Start monitoring clipboard changes
    async fn start_monitoring(&self) -> ClipboardResult<()>;
    
    /// Stop monitoring clipboard changes
    async fn stop_monitoring(&self) -> ClipboardResult<()>;
    
    /// Check if monitoring is active
    fn is_monitoring(&self) -> bool;
    
    /// Get platform name
    fn platform_name(&self) -> &'static str;
}

/// Create platform-specific clipboard implementation
pub fn create_platform_clipboard() -> Box<dyn PlatformClipboard> {
    #[cfg(windows)]
    {
        Box::new(windows::WindowsClipboard::new())
    }
    
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOSClipboard::new())
    }
    
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxClipboard::new())
    }
    
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        Box::new(generic::GenericClipboard::new())
    }
}

/// Unified clipboard implementation that wraps platform-specific implementations
pub struct UnifiedClipboard {
    platform_clipboard: Box<dyn PlatformClipboard>,
}

impl UnifiedClipboard {
    /// Create new unified clipboard
    pub fn new() -> Self {
        Self {
            platform_clipboard: create_platform_clipboard(),
        }
    }
    
    /// Get platform name
    pub fn platform_name(&self) -> &'static str {
        self.platform_clipboard.platform_name()
    }
}

#[async_trait]
impl Clipboard for UnifiedClipboard {
    async fn get_content(&self) -> ClipboardResult<Option<ClipboardContent>> {
        self.platform_clipboard.get_content().await
    }
    
    async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()> {
        self.platform_clipboard.set_content(content).await
    }
    
    async fn start_monitoring(&self) -> ClipboardResult<()> {
        self.platform_clipboard.start_monitoring().await
    }
    
    async fn stop_monitoring(&self) -> ClipboardResult<()> {
        self.platform_clipboard.stop_monitoring().await
    }
    
    fn is_monitoring(&self) -> bool {
        self.platform_clipboard.is_monitoring()
    }
}

impl Default for UnifiedClipboard {
    fn default() -> Self {
        Self::new()
    }
}