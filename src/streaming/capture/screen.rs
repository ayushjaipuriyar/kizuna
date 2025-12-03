// Screen capture optimization and utilities
//
// Provides efficient screen region capture, change detection, cursor handling,
// and resolution change adaptation.

use crate::streaming::{CaptureConfig, ScreenRegion, StreamError, StreamResult, Resolution};
use std::time::{Duration, SystemTime};

/// Screen capture optimizer for efficient frame capture
/// 
/// Requirements: 3.2, 3.4, 3.5
pub struct ScreenCaptureOptimizer {
    last_capture_time: Option<SystemTime>,
    frame_interval: Duration,
    change_detection_enabled: bool,
    last_frame_hash: Option<u64>,
}

impl ScreenCaptureOptimizer {
    /// Create a new screen capture optimizer
    pub fn new(framerate: u32) -> Self {
        let frame_interval = Duration::from_secs_f64(1.0 / framerate as f64);
        
        Self {
            last_capture_time: None,
            frame_interval,
            change_detection_enabled: true,
            last_frame_hash: None,
        }
    }

    /// Check if it's time to capture a new frame
    /// Requirements: 3.4
    pub fn should_capture_frame(&mut self) -> bool {
        let now = SystemTime::now();
        
        match self.last_capture_time {
            None => {
                self.last_capture_time = Some(now);
                true
            }
            Some(last_time) => {
                if let Ok(elapsed) = now.duration_since(last_time) {
                    if elapsed >= self.frame_interval {
                        self.last_capture_time = Some(now);
                        true
                    } else {
                        false
                    }
                } else {
                    // Clock went backwards, capture anyway
                    self.last_capture_time = Some(now);
                    true
                }
            }
        }
    }

    /// Detect if frame has changed from previous capture
    /// Requirements: 3.4
    pub fn has_frame_changed(&mut self, frame_data: &[u8]) -> bool {
        if !self.change_detection_enabled {
            return true;
        }

        let current_hash = self.compute_frame_hash(frame_data);
        
        match self.last_frame_hash {
            None => {
                self.last_frame_hash = Some(current_hash);
                true
            }
            Some(last_hash) => {
                let changed = current_hash != last_hash;
                if changed {
                    self.last_frame_hash = Some(current_hash);
                }
                changed
            }
        }
    }

    /// Compute a simple hash of frame data for change detection
    /// Requirements: 3.4
    fn compute_frame_hash(&self, data: &[u8]) -> u64 {
        // Simple hash using FNV-1a algorithm
        // In production, this could use a more sophisticated algorithm
        // or sample specific regions of the frame
        let mut hash: u64 = 0xcbf29ce484222325;
        
        // Sample every 1000th byte for performance
        for (i, &byte) in data.iter().enumerate() {
            if i % 1000 == 0 {
                hash ^= byte as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        
        hash
    }

    /// Enable or disable change detection
    pub fn set_change_detection(&mut self, enabled: bool) {
        self.change_detection_enabled = enabled;
    }

    /// Update framerate and recalculate frame interval
    pub fn set_framerate(&mut self, framerate: u32) {
        self.frame_interval = Duration::from_secs_f64(1.0 / framerate as f64);
    }
}

/// Region selector for screen capture
/// 
/// Requirements: 3.2, 3.4
pub struct RegionSelector {
    full_screen: ScreenRegion,
}

impl RegionSelector {
    /// Create a new region selector for a screen
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        Self {
            full_screen: ScreenRegion {
                x: 0,
                y: 0,
                width: screen_width,
                height: screen_height,
            },
        }
    }

    /// Validate and clamp a region to screen bounds
    /// Requirements: 3.2
    pub fn validate_region(&self, region: ScreenRegion) -> StreamResult<ScreenRegion> {
        // Check if region is completely outside screen bounds
        if region.x >= self.full_screen.width || region.y >= self.full_screen.height {
            return Err(StreamError::configuration("Region is outside screen bounds"));
        }

        // Clamp region to screen bounds
        let x = region.x.min(self.full_screen.width - 1);
        let y = region.y.min(self.full_screen.height - 1);
        
        let max_width = self.full_screen.width - x;
        let max_height = self.full_screen.height - y;
        
        let width = region.width.min(max_width).max(1);
        let height = region.height.min(max_height).max(1);

        Ok(ScreenRegion { x, y, width, height })
    }

    /// Create a centered region with specified dimensions
    /// Requirements: 3.2
    pub fn create_centered_region(&self, width: u32, height: u32) -> StreamResult<ScreenRegion> {
        if width > self.full_screen.width || height > self.full_screen.height {
            return Err(StreamError::configuration("Region dimensions exceed screen size"));
        }

        let x = (self.full_screen.width - width) / 2;
        let y = (self.full_screen.height - height) / 2;

        Ok(ScreenRegion { x, y, width, height })
    }

    /// Get the full screen region
    pub fn full_screen_region(&self) -> ScreenRegion {
        self.full_screen
    }
}

/// Cursor capture handler
/// 
/// Requirements: 3.4
pub struct CursorCapture {
    enabled: bool,
    cursor_position: Option<(u32, u32)>,
}

impl CursorCapture {
    /// Create a new cursor capture handler
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            cursor_position: None,
        }
    }

    /// Check if cursor capture is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Enable or disable cursor capture
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Update cursor position
    pub fn update_cursor_position(&mut self, x: u32, y: u32) {
        self.cursor_position = Some((x, y));
    }

    /// Get current cursor position
    pub fn cursor_position(&self) -> Option<(u32, u32)> {
        self.cursor_position
    }

    /// Check if cursor is within a region
    pub fn is_cursor_in_region(&self, region: &ScreenRegion) -> bool {
        if let Some((x, y)) = self.cursor_position {
            x >= region.x && x < region.x + region.width
                && y >= region.y && y < region.y + region.height
        } else {
            false
        }
    }
}

/// Resolution change detector
/// 
/// Requirements: 3.4, 3.5
pub struct ResolutionChangeDetector {
    current_resolution: Resolution,
    last_check_time: SystemTime,
    check_interval: Duration,
}

impl ResolutionChangeDetector {
    /// Create a new resolution change detector
    pub fn new(initial_resolution: Resolution) -> Self {
        Self {
            current_resolution: initial_resolution,
            last_check_time: SystemTime::now(),
            check_interval: Duration::from_secs(1), // Check every second
        }
    }

    /// Check if resolution has changed
    /// Requirements: 3.4
    pub fn check_resolution_change(&mut self, new_resolution: Resolution) -> bool {
        let now = SystemTime::now();
        
        // Only check at specified intervals to avoid excessive checks
        if let Ok(elapsed) = now.duration_since(self.last_check_time) {
            if elapsed < self.check_interval {
                return false;
            }
        }
        
        self.last_check_time = now;
        
        let changed = self.current_resolution.width != new_resolution.width
            || self.current_resolution.height != new_resolution.height;
        
        if changed {
            self.current_resolution = new_resolution;
        }
        
        changed
    }

    /// Get current resolution
    pub fn current_resolution(&self) -> Resolution {
        self.current_resolution
    }

    /// Update resolution
    pub fn update_resolution(&mut self, resolution: Resolution) {
        self.current_resolution = resolution;
    }

    /// Set check interval
    pub fn set_check_interval(&mut self, interval: Duration) {
        self.check_interval = interval;
    }
}

/// Screen capture configuration optimizer
/// 
/// Requirements: 3.4, 3.5
pub struct CaptureConfigOptimizer;

impl CaptureConfigOptimizer {
    /// Optimize capture configuration based on region size
    /// Requirements: 3.4
    pub fn optimize_for_region(
        config: CaptureConfig,
        region: &ScreenRegion,
    ) -> CaptureConfig {
        let mut optimized = config;
        
        // Adjust resolution to match region if needed
        if optimized.resolution.width != region.width
            || optimized.resolution.height != region.height
        {
            optimized.resolution = Resolution {
                width: region.width,
                height: region.height,
            };
        }
        
        // Reduce framerate for large regions to save bandwidth
        let pixel_count = region.width * region.height;
        if pixel_count > 1920 * 1080 && optimized.framerate > 30 {
            optimized.framerate = 30;
        }
        
        optimized
    }

    /// Optimize configuration for performance
    /// Requirements: 3.5
    pub fn optimize_for_performance(config: CaptureConfig) -> CaptureConfig {
        let mut optimized = config;
        
        // Reduce buffer count for lower latency
        optimized.buffer_count = optimized.buffer_count.min(2);
        
        // Use more efficient pixel format if available
        optimized.pixel_format = crate::streaming::PixelFormat::NV12;
        
        optimized
    }

    /// Optimize configuration for quality
    /// Requirements: 3.4
    pub fn optimize_for_quality(config: CaptureConfig) -> CaptureConfig {
        let mut optimized = config;
        
        // Increase buffer count for smoother capture
        optimized.buffer_count = optimized.buffer_count.max(4);
        
        // Use higher quality pixel format
        optimized.pixel_format = crate::streaming::PixelFormat::RGB24;
        
        optimized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_capture_optimizer_frame_timing() {
        let mut optimizer = ScreenCaptureOptimizer::new(30);
        
        // First frame should always be captured
        assert!(optimizer.should_capture_frame());
        
        // Immediate second call should not capture
        assert!(!optimizer.should_capture_frame());
    }

    #[test]
    fn test_region_selector_validation() {
        let selector = RegionSelector::new(1920, 1080);
        
        // Valid region
        let region = ScreenRegion { x: 100, y: 100, width: 800, height: 600 };
        let validated = selector.validate_region(region).unwrap();
        assert_eq!(validated.width, 800);
        assert_eq!(validated.height, 600);
        
        // Region extending beyond screen should be clamped
        let region = ScreenRegion { x: 1800, y: 1000, width: 500, height: 500 };
        let validated = selector.validate_region(region).unwrap();
        assert!(validated.width <= 120); // Clamped to screen bounds
        assert!(validated.height <= 80);
    }

    #[test]
    fn test_cursor_capture() {
        let mut cursor = CursorCapture::new(true);
        assert!(cursor.is_enabled());
        
        cursor.update_cursor_position(100, 200);
        assert_eq!(cursor.cursor_position(), Some((100, 200)));
        
        let region = ScreenRegion { x: 50, y: 150, width: 200, height: 100 };
        assert!(cursor.is_cursor_in_region(&region));
    }

    #[test]
    fn test_resolution_change_detector() {
        let mut detector = ResolutionChangeDetector::new(Resolution { width: 1920, height: 1080 });
        
        // Same resolution should not trigger change
        assert!(!detector.check_resolution_change(Resolution { width: 1920, height: 1080 }));
        
        // Different resolution should trigger change
        assert!(detector.check_resolution_change(Resolution { width: 2560, height: 1440 }));
    }
}
