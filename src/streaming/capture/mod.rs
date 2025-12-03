// Video and audio capture module
//
// Provides platform-agnostic interfaces for camera, screen, and audio capture
// with platform-specific implementations.

pub mod platform;
pub mod screen;

use async_trait::async_trait;

use crate::streaming::{
    CameraDevice, CaptureCapabilities, CaptureConfig, CaptureStream, ScreenRegion, StreamError,
    StreamResult,
};

/// Platform-agnostic capture engine implementation
/// 
/// Delegates to platform-specific implementations for camera and screen capture.
/// Automatically detects the platform and uses the appropriate backend.
/// 
/// Requirements: 1.1, 1.2, 3.1
pub struct CaptureEngineImpl {
    backend: Box<dyn platform::PlatformCaptureBackend>,
}

impl CaptureEngineImpl {
    /// Create a new capture engine with automatic platform detection
    /// Requirements: 1.1, 1.2
    pub fn new() -> StreamResult<Self> {
        let backend = Self::create_platform_backend()?;
        Ok(Self { backend })
    }

    /// Create the appropriate platform-specific backend
    /// Requirements: 1.1, 1.2
    fn create_platform_backend() -> StreamResult<Box<dyn platform::PlatformCaptureBackend>> {
        #[cfg(target_os = "windows")]
        {
            let backend = platform::WindowsCaptureBackend::new()?;
            Ok(Box::new(backend))
        }

        #[cfg(target_os = "macos")]
        {
            let backend = platform::MacOSCaptureBackend::new()?;
            Ok(Box::new(backend))
        }

        #[cfg(target_os = "linux")]
        {
            let backend = platform::LinuxCaptureBackend::new()?;
            Ok(Box::new(backend))
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Err(StreamError::unsupported("Platform not supported for camera capture"))
        }
    }
}

#[async_trait]
impl crate::streaming::CaptureEngine for CaptureEngineImpl {
    /// List available camera devices
    /// Requirements: 1.1
    async fn list_cameras(&self) -> StreamResult<Vec<CameraDevice>> {
        self.backend.list_cameras().await
    }

    /// Start camera capture with configurable settings
    /// Requirements: 1.1, 1.2
    async fn start_camera_capture(
        &self,
        device: CameraDevice,
        config: CaptureConfig,
    ) -> StreamResult<CaptureStream> {
        self.backend.start_camera_capture(device, config).await
    }

    /// Start screen capture
    /// Requirements: 3.1
    async fn start_screen_capture(
        &self,
        region: ScreenRegion,
        config: CaptureConfig,
    ) -> StreamResult<CaptureStream> {
        self.backend.start_screen_capture(region, config).await
    }

    /// Stop active capture stream
    /// Requirements: 1.1
    async fn stop_capture(&self, stream: CaptureStream) -> StreamResult<()> {
        self.backend.stop_capture(stream).await
    }

    /// Get camera device capabilities
    /// Requirements: 1.1, 1.2
    async fn get_capture_capabilities(
        &self,
        device: CameraDevice,
    ) -> StreamResult<CaptureCapabilities> {
        self.backend.get_capture_capabilities(device).await
    }
}

impl Default for CaptureEngineImpl {
    fn default() -> Self {
        Self::new().expect("Failed to create capture engine")
    }
}
