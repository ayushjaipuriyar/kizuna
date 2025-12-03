// Encoding optimization and performance monitoring
//
// Provides encoding performance monitoring, automatic encoder selection,
// and parameter optimization for different content types.
//
// Requirements: 9.1, 9.3

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use crate::streaming::{
    EncoderCapabilities, EncoderConfig, Resolution, StreamError, StreamResult, VideoCodecType,
};

use super::HardwareAccelerator;

/// Encoder performance metrics
#[derive(Debug, Clone)]
pub struct EncoderMetrics {
    pub frames_encoded: u64,
    pub encoding_time_ms: u64,
    pub average_fps: f32,
    pub cpu_usage: f32,
    pub memory_usage_mb: u64,
    pub dropped_frames: u64,
    pub timestamp: Instant,
}

impl Default for EncoderMetrics {
    fn default() -> Self {
        Self {
            frames_encoded: 0,
            encoding_time_ms: 0,
            average_fps: 0.0,
            cpu_usage: 0.0,
            memory_usage_mb: 0,
            dropped_frames: 0,
            timestamp: Instant::now(),
        }
    }
}

/// Encoder performance monitor
///
/// Tracks encoding performance and resource usage for optimization.
///
/// Requirements: 9.1, 9.3
pub struct EncoderPerformanceMonitor {
    metrics_history: VecDeque<EncoderMetrics>,
    current_metrics: EncoderMetrics,
    start_time: Instant,
    last_frame_time: Instant,
    frame_times: VecDeque<Duration>,
    max_history_size: usize,
}

impl EncoderPerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            metrics_history: VecDeque::new(),
            current_metrics: EncoderMetrics::default(),
            start_time: Instant::now(),
            last_frame_time: Instant::now(),
            frame_times: VecDeque::new(),
            max_history_size: 300, // Keep 10 seconds at 30fps
        }
    }

    /// Record a frame encoding event
    pub fn record_frame_encoded(&mut self, encoding_duration: Duration) {
        self.current_metrics.frames_encoded += 1;
        self.current_metrics.encoding_time_ms += encoding_duration.as_millis() as u64;
        
        // Track frame times for FPS calculation
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame_time);
        self.frame_times.push_back(frame_time);
        self.last_frame_time = now;
        
        // Keep only recent frame times
        if self.frame_times.len() > self.max_history_size {
            self.frame_times.pop_front();
        }
        
        // Update average FPS
        self.update_average_fps();
    }

    /// Record a dropped frame
    pub fn record_frame_dropped(&mut self) {
        self.current_metrics.dropped_frames += 1;
    }

    /// Update CPU usage estimate
    pub fn update_cpu_usage(&mut self, cpu_usage: f32) {
        self.current_metrics.cpu_usage = cpu_usage;
    }

    /// Update memory usage estimate
    pub fn update_memory_usage(&mut self, memory_mb: u64) {
        self.current_metrics.memory_usage_mb = memory_mb;
    }

    /// Calculate average FPS from recent frame times
    fn update_average_fps(&mut self) {
        if self.frame_times.is_empty() {
            self.current_metrics.average_fps = 0.0;
            return;
        }
        
        let total_time: Duration = self.frame_times.iter().sum();
        let avg_frame_time = total_time.as_secs_f32() / self.frame_times.len() as f32;
        
        if avg_frame_time > 0.0 {
            self.current_metrics.average_fps = 1.0 / avg_frame_time;
        }
    }

    /// Get current metrics
    pub fn current_metrics(&self) -> &EncoderMetrics {
        &self.current_metrics
    }

    /// Take a snapshot of current metrics
    pub fn snapshot(&mut self) -> EncoderMetrics {
        let mut snapshot = self.current_metrics.clone();
        snapshot.timestamp = Instant::now();
        
        // Add to history
        self.metrics_history.push_back(snapshot.clone());
        
        // Keep history bounded
        if self.metrics_history.len() > 60 {
            self.metrics_history.pop_front();
        }
        
        snapshot
    }

    /// Get metrics history
    pub fn history(&self) -> &VecDeque<EncoderMetrics> {
        &self.metrics_history
    }

    /// Get average encoding time per frame
    pub fn average_encoding_time_ms(&self) -> f32 {
        if self.current_metrics.frames_encoded == 0 {
            return 0.0;
        }
        
        self.current_metrics.encoding_time_ms as f32 / self.current_metrics.frames_encoded as f32
    }

    /// Get frame drop rate
    pub fn frame_drop_rate(&self) -> f32 {
        let total_frames = self.current_metrics.frames_encoded + self.current_metrics.dropped_frames;
        if total_frames == 0 {
            return 0.0;
        }
        
        self.current_metrics.dropped_frames as f32 / total_frames as f32
    }

    /// Check if performance is acceptable
    pub fn is_performance_acceptable(&self, target_fps: f32) -> bool {
        let fps_ok = self.current_metrics.average_fps >= target_fps * 0.9;
        let drop_rate_ok = self.frame_drop_rate() < 0.05;
        let cpu_ok = self.current_metrics.cpu_usage < 0.9;
        
        fps_ok && drop_rate_ok && cpu_ok
    }

    /// Reset metrics
    pub fn reset(&mut self) {
        self.current_metrics = EncoderMetrics::default();
        self.start_time = Instant::now();
        self.last_frame_time = Instant::now();
        self.frame_times.clear();
    }
}

impl Default for EncoderPerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Encoder selector for automatic hardware detection
///
/// Requirements: 9.1
pub struct EncoderSelector {
    available_accelerators: Vec<HardwareAccelerator>,
    preferred_accelerator: Option<HardwareAccelerator>,
}

impl EncoderSelector {
    /// Create a new encoder selector
    pub fn new() -> StreamResult<Self> {
        let available_accelerators = HardwareAccelerator::detect_available_accelerators()?;
        
        // Prefer hardware acceleration in this order: NVENC > VideoToolbox > QuickSync > VCE > Software
        let preferred_accelerator = available_accelerators.iter()
            .find(|&&acc| matches!(acc, HardwareAccelerator::NVENC))
            .or_else(|| available_accelerators.iter().find(|&&acc| matches!(acc, HardwareAccelerator::VideoToolbox)))
            .or_else(|| available_accelerators.iter().find(|&&acc| matches!(acc, HardwareAccelerator::QuickSync)))
            .or_else(|| available_accelerators.iter().find(|&&acc| matches!(acc, HardwareAccelerator::VCE)))
            .or_else(|| available_accelerators.iter().find(|&&acc| matches!(acc, HardwareAccelerator::Software)))
            .copied();
        
        Ok(Self {
            available_accelerators,
            preferred_accelerator,
        })
    }

    /// Get the best available encoder
    pub fn select_best_encoder(&self) -> StreamResult<HardwareAccelerator> {
        self.preferred_accelerator
            .ok_or_else(|| StreamError::unsupported("No encoder available"))
    }

    /// Select encoder based on configuration
    pub fn select_encoder_for_config(&self, config: &EncoderConfig) -> StreamResult<HardwareAccelerator> {
        // For high resolution or high framerate, prefer hardware acceleration
        let needs_hardware = config.resolution.width >= 1920 || config.framerate >= 60;
        
        if needs_hardware && config.hardware_acceleration {
            // Try to find hardware accelerator
            for acc in &self.available_accelerators {
                if !matches!(acc, HardwareAccelerator::Software) {
                    return Ok(*acc);
                }
            }
        }
        
        // Fall back to best available
        self.select_best_encoder()
    }

    /// Get available accelerators
    pub fn available_accelerators(&self) -> &[HardwareAccelerator] {
        &self.available_accelerators
    }

    /// Check if hardware acceleration is available
    pub fn has_hardware_acceleration(&self) -> bool {
        self.available_accelerators.iter()
            .any(|acc| !matches!(acc, HardwareAccelerator::Software))
    }

    /// Get encoder capabilities
    pub fn get_capabilities(&self) -> EncoderCapabilities {
        EncoderCapabilities {
            supported_codecs: vec![VideoCodecType::H264],
            hardware_acceleration_available: self.has_hardware_acceleration(),
            max_resolution: Resolution { width: 3840, height: 2160 },
            max_framerate: 60,
        }
    }
}

impl Default for EncoderSelector {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            available_accelerators: vec![HardwareAccelerator::Software],
            preferred_accelerator: Some(HardwareAccelerator::Software),
        })
    }
}

/// Content type for encoding optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentType {
    /// Camera feed with natural motion
    Camera,
    /// Screen content with text and UI
    Screen,
    /// Gaming content with fast motion
    Gaming,
    /// General purpose
    General,
}

/// Encoder optimizer for content-specific parameter tuning
///
/// Requirements: 9.3
pub struct EncoderOptimizer {
    content_type: ContentType,
}

impl EncoderOptimizer {
    /// Create a new encoder optimizer
    pub fn new(content_type: ContentType) -> Self {
        Self {
            content_type,
        }
    }

    /// Optimize encoder configuration for content type
    pub fn optimize_config(&self, mut config: EncoderConfig) -> EncoderConfig {
        match self.content_type {
            ContentType::Camera => {
                // Camera content: balance quality and performance
                // Use moderate bitrate, standard keyframe interval
                config.bitrate = self.calculate_bitrate_for_resolution(&config.resolution, 1.0);
            }
            ContentType::Screen => {
                // Screen content: prioritize sharpness for text
                // Use higher bitrate, longer keyframe interval
                config.bitrate = self.calculate_bitrate_for_resolution(&config.resolution, 1.3);
            }
            ContentType::Gaming => {
                // Gaming content: prioritize low latency and smooth motion
                // Use higher bitrate, shorter keyframe interval
                config.bitrate = self.calculate_bitrate_for_resolution(&config.resolution, 1.5);
            }
            ContentType::General => {
                // General content: balanced settings
                config.bitrate = self.calculate_bitrate_for_resolution(&config.resolution, 1.0);
            }
        }
        
        config
    }

    /// Calculate appropriate bitrate for resolution
    fn calculate_bitrate_for_resolution(&self, resolution: &Resolution, multiplier: f32) -> u32 {
        let pixels = resolution.width * resolution.height;
        
        // Base bitrate calculation: ~0.1 bits per pixel per frame at 30fps
        let base_bitrate = (pixels as f32 * 0.1 * 30.0) as u32;
        
        // Apply content-specific multiplier
        (base_bitrate as f32 * multiplier) as u32
    }

    /// Get recommended keyframe interval for content type
    pub fn recommended_keyframe_interval(&self, framerate: u32) -> u32 {
        match self.content_type {
            ContentType::Camera => framerate * 3,      // Every 3 seconds
            ContentType::Screen => framerate * 5,      // Every 5 seconds (less motion)
            ContentType::Gaming => framerate * 2,      // Every 2 seconds (fast motion)
            ContentType::General => framerate * 3,     // Every 3 seconds
        }
    }

    /// Get recommended quality factor for content type
    pub fn recommended_quality_factor(&self) -> u32 {
        match self.content_type {
            ContentType::Camera => 70,
            ContentType::Screen => 85,  // Higher quality for text
            ContentType::Gaming => 75,
            ContentType::General => 70,
        }
    }

    /// Set content type
    pub fn set_content_type(&mut self, content_type: ContentType) {
        self.content_type = content_type;
    }

    /// Get current content type
    pub fn content_type(&self) -> ContentType {
        self.content_type
    }

    /// Detect content type from encoding metrics (heuristic)
    pub fn detect_content_type(&self, metrics: &EncoderMetrics) -> ContentType {
        // Simple heuristic based on encoding characteristics
        // In a real implementation, this would analyze frame content
        
        if metrics.average_fps > 50.0 {
            // High framerate suggests gaming
            ContentType::Gaming
        } else if metrics.cpu_usage < 0.3 {
            // Low CPU usage might indicate screen content (less complex)
            ContentType::Screen
        } else {
            // Default to camera
            ContentType::Camera
        }
    }
}

impl Default for EncoderOptimizer {
    fn default() -> Self {
        Self::new(ContentType::General)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_monitor() {
        let mut monitor = EncoderPerformanceMonitor::new();
        
        // Record some frames
        for _ in 0..10 {
            monitor.record_frame_encoded(Duration::from_millis(30));
        }
        
        assert_eq!(monitor.current_metrics().frames_encoded, 10);
        assert!(monitor.average_encoding_time_ms() > 0.0);
    }

    #[test]
    fn test_encoder_selector() {
        let selector = EncoderSelector::new();
        assert!(selector.is_ok());
        
        if let Ok(selector) = selector {
            let best = selector.select_best_encoder();
            assert!(best.is_ok());
        }
    }

    #[test]
    fn test_encoder_optimizer() {
        let optimizer = EncoderOptimizer::new(ContentType::Screen);
        
        let config = EncoderConfig {
            codec: VideoCodecType::H264,
            resolution: Resolution { width: 1920, height: 1080 },
            framerate: 30,
            bitrate: 1_000_000,
            hardware_acceleration: true,
        };
        
        let optimized = optimizer.optimize_config(config);
        assert!(optimized.bitrate > 0);
    }
}
