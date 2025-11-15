// Bandwidth Control Module
//
// Handles bandwidth throttling and rate limiting for file transfers

use crate::file_transfer::{error::Result, types::current_timestamp};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{sleep, Instant};

/// Bandwidth controller for rate limiting file transfers
#[derive(Clone)]
pub struct BandwidthController {
    state: Arc<RwLock<BandwidthState>>,
}

/// Internal state for bandwidth control
struct BandwidthState {
    /// Maximum bytes per second (None for unlimited)
    limit: Option<u64>,
    /// Bytes transferred in current window
    bytes_in_window: u64,
    /// Start of current measurement window
    window_start: Instant,
    /// Window duration for rate calculation (1 second)
    window_duration: Duration,
    /// Total bytes transferred
    total_bytes: u64,
    /// Transfer start time
    start_time: Instant,
}

impl BandwidthController {
    /// Create a new bandwidth controller with no limit
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(BandwidthState {
                limit: None,
                bytes_in_window: 0,
                window_start: Instant::now(),
                window_duration: Duration::from_secs(1),
                total_bytes: 0,
                start_time: Instant::now(),
            })),
        }
    }

    /// Create a bandwidth controller with a specific limit (bytes per second)
    pub fn with_limit(limit: u64) -> Self {
        Self {
            state: Arc::new(RwLock::new(BandwidthState {
                limit: Some(limit),
                bytes_in_window: 0,
                window_start: Instant::now(),
                window_duration: Duration::from_secs(1),
                total_bytes: 0,
                start_time: Instant::now(),
            })),
        }
    }

    /// Set bandwidth limit (bytes per second, None for unlimited)
    pub async fn set_limit(&self, limit: Option<u64>) -> Result<()> {
        let mut state = self.state.write().await;
        state.limit = limit;
        Ok(())
    }

    /// Get current bandwidth limit
    pub async fn get_limit(&self) -> Option<u64> {
        let state = self.state.read().await;
        state.limit
    }

    /// Throttle transfer by waiting if necessary to stay within bandwidth limit
    /// Returns the actual delay applied
    pub async fn throttle(&self, bytes_to_send: usize) -> Result<Duration> {
        let mut state = self.state.write().await;

        // If no limit, no throttling needed
        let limit = match state.limit {
            Some(l) => l,
            None => return Ok(Duration::ZERO),
        };

        // Check if we need to start a new window
        let now = Instant::now();
        let elapsed = now.duration_since(state.window_start);

        if elapsed >= state.window_duration {
            // Start new window
            state.window_start = now;
            state.bytes_in_window = 0;
        }

        // Calculate how many bytes we can send in this window
        let bytes_available = limit.saturating_sub(state.bytes_in_window);

        if bytes_to_send as u64 <= bytes_available {
            // We can send immediately
            state.bytes_in_window += bytes_to_send as u64;
            state.total_bytes += bytes_to_send as u64;
            Ok(Duration::ZERO)
        } else {
            // We need to wait for the next window
            let time_remaining = state.window_duration.saturating_sub(elapsed);
            
            // Release lock before sleeping
            drop(state);
            
            sleep(time_remaining).await;

            // Acquire lock again and update state
            let mut state = self.state.write().await;
            state.window_start = Instant::now();
            state.bytes_in_window = bytes_to_send as u64;
            state.total_bytes += bytes_to_send as u64;

            Ok(time_remaining)
        }
    }

    /// Record bytes transferred (for monitoring without throttling)
    pub async fn record_bytes(&self, bytes: usize) -> Result<()> {
        let mut state = self.state.write().await;
        
        // Check if we need to start a new window
        let now = Instant::now();
        let elapsed = now.duration_since(state.window_start);

        if elapsed >= state.window_duration {
            state.window_start = now;
            state.bytes_in_window = 0;
        }

        state.bytes_in_window += bytes as u64;
        state.total_bytes += bytes as u64;
        
        Ok(())
    }

    /// Get current transfer speed (bytes per second)
    pub async fn current_speed(&self) -> u64 {
        let state = self.state.read().await;
        
        let now = Instant::now();
        let elapsed = now.duration_since(state.window_start);
        
        if elapsed.as_secs() == 0 {
            state.bytes_in_window
        } else {
            state.bytes_in_window / elapsed.as_secs()
        }
    }

    /// Get average transfer speed since start (bytes per second)
    pub async fn average_speed(&self) -> u64 {
        let state = self.state.read().await;
        
        let elapsed = state.start_time.elapsed();
        
        if elapsed.as_secs() == 0 {
            state.total_bytes
        } else {
            state.total_bytes / elapsed.as_secs()
        }
    }

    /// Get total bytes transferred
    pub async fn total_bytes(&self) -> u64 {
        let state = self.state.read().await;
        state.total_bytes
    }

    /// Get bandwidth statistics
    pub async fn get_stats(&self) -> BandwidthStats {
        let state = self.state.read().await;
        
        let elapsed = state.start_time.elapsed();
        let current_speed = if elapsed.as_secs() == 0 {
            state.bytes_in_window
        } else {
            let window_elapsed = Instant::now().duration_since(state.window_start);
            if window_elapsed.as_secs() == 0 {
                state.bytes_in_window
            } else {
                state.bytes_in_window / window_elapsed.as_secs()
            }
        };
        
        let average_speed = if elapsed.as_secs() == 0 {
            state.total_bytes
        } else {
            state.total_bytes / elapsed.as_secs()
        };

        BandwidthStats {
            limit: state.limit,
            current_speed,
            average_speed,
            total_bytes: state.total_bytes,
            elapsed_seconds: elapsed.as_secs(),
        }
    }

    /// Reset bandwidth statistics
    pub async fn reset(&self) -> Result<()> {
        let mut state = self.state.write().await;
        state.bytes_in_window = 0;
        state.total_bytes = 0;
        state.window_start = Instant::now();
        state.start_time = Instant::now();
        Ok(())
    }
}

impl Default for BandwidthController {
    fn default() -> Self {
        Self::new()
    }
}

/// Bandwidth statistics
#[derive(Debug, Clone)]
pub struct BandwidthStats {
    /// Current bandwidth limit (bytes per second)
    pub limit: Option<u64>,
    /// Current transfer speed (bytes per second)
    pub current_speed: u64,
    /// Average transfer speed (bytes per second)
    pub average_speed: u64,
    /// Total bytes transferred
    pub total_bytes: u64,
    /// Elapsed time in seconds
    pub elapsed_seconds: u64,
}

impl BandwidthStats {
    /// Get current speed in human-readable format
    pub fn current_speed_human(&self) -> String {
        format_bytes_per_second(self.current_speed)
    }

    /// Get average speed in human-readable format
    pub fn average_speed_human(&self) -> String {
        format_bytes_per_second(self.average_speed)
    }

    /// Get total bytes in human-readable format
    pub fn total_bytes_human(&self) -> String {
        format_bytes(self.total_bytes)
    }

    /// Get limit in human-readable format
    pub fn limit_human(&self) -> String {
        match self.limit {
            Some(limit) => format_bytes_per_second(limit),
            None => "Unlimited".to_string(),
        }
    }

    /// Calculate percentage of bandwidth limit being used
    pub fn utilization_percentage(&self) -> Option<f64> {
        self.limit.map(|limit| {
            if limit == 0 {
                0.0
            } else {
                (self.current_speed as f64 / limit as f64) * 100.0
            }
        })
    }
}

/// Format bytes as human-readable string
fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Format bytes per second as human-readable string
fn format_bytes_per_second(bytes_per_sec: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_sec))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_no_limit() {
        let controller = BandwidthController::new();
        
        // Should not throttle without limit
        let delay = controller.throttle(1024 * 1024).await.unwrap();
        assert_eq!(delay, Duration::ZERO);
    }

    #[tokio::test]
    async fn test_set_limit() {
        let controller = BandwidthController::new();
        
        controller.set_limit(Some(1024 * 1024)).await.unwrap();
        let limit = controller.get_limit().await;
        
        assert_eq!(limit, Some(1024 * 1024));
    }

    #[tokio::test]
    async fn test_throttle_within_limit() {
        let controller = BandwidthController::with_limit(1024 * 1024); // 1 MB/s
        
        // Send 512KB - should not throttle
        let delay = controller.throttle(512 * 1024).await.unwrap();
        assert_eq!(delay, Duration::ZERO);
        
        // Send another 256KB - still within limit
        let delay = controller.throttle(256 * 1024).await.unwrap();
        assert_eq!(delay, Duration::ZERO);
    }

    #[tokio::test]
    async fn test_throttle_exceeds_limit() {
        let controller = BandwidthController::with_limit(1024 * 1024); // 1 MB/s
        
        // Send 1.5MB - should throttle
        let delay = controller.throttle(1536 * 1024).await.unwrap();
        assert!(delay > Duration::ZERO);
    }

    #[tokio::test]
    async fn test_record_bytes() {
        let controller = BandwidthController::new();
        
        controller.record_bytes(1024).await.unwrap();
        controller.record_bytes(2048).await.unwrap();
        
        let total = controller.total_bytes().await;
        assert_eq!(total, 3072);
    }

    #[tokio::test]
    async fn test_current_speed() {
        let controller = BandwidthController::new();
        
        controller.record_bytes(1024 * 1024).await.unwrap();
        
        let speed = controller.current_speed().await;
        assert!(speed > 0);
    }

    #[tokio::test]
    async fn test_average_speed() {
        let controller = BandwidthController::new();
        
        controller.record_bytes(1024 * 1024).await.unwrap();
        sleep(Duration::from_millis(100)).await;
        controller.record_bytes(1024 * 1024).await.unwrap();
        
        let avg_speed = controller.average_speed().await;
        assert!(avg_speed > 0);
    }

    #[tokio::test]
    async fn test_get_stats() {
        let controller = BandwidthController::with_limit(1024 * 1024);
        
        controller.record_bytes(512 * 1024).await.unwrap();
        
        let stats = controller.get_stats().await;
        assert_eq!(stats.limit, Some(1024 * 1024));
        assert_eq!(stats.total_bytes, 512 * 1024);
        assert!(stats.current_speed > 0);
    }

    #[tokio::test]
    async fn test_reset() {
        let controller = BandwidthController::new();
        
        controller.record_bytes(1024 * 1024).await.unwrap();
        assert_eq!(controller.total_bytes().await, 1024 * 1024);
        
        controller.reset().await.unwrap();
        assert_eq!(controller.total_bytes().await, 0);
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512.00 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1536 * 1024), "1.50 MB");
    }

    #[test]
    fn test_format_bytes_per_second() {
        assert_eq!(format_bytes_per_second(1024), "1.00 KB/s");
        assert_eq!(format_bytes_per_second(1024 * 1024), "1.00 MB/s");
    }

    #[tokio::test]
    async fn test_bandwidth_stats_methods() {
        let controller = BandwidthController::with_limit(1024 * 1024);
        controller.record_bytes(512 * 1024).await.unwrap();
        
        let stats = controller.get_stats().await;
        
        assert!(!stats.current_speed_human().is_empty());
        assert!(!stats.average_speed_human().is_empty());
        assert!(!stats.total_bytes_human().is_empty());
        assert_eq!(stats.limit_human(), "1.00 MB/s");
        
        let utilization = stats.utilization_percentage();
        assert!(utilization.is_some());
    }

    #[tokio::test]
    async fn test_utilization_percentage() {
        let controller = BandwidthController::with_limit(1024 * 1024); // 1 MB/s
        
        // Simulate current speed of 512 KB/s
        controller.record_bytes(512 * 1024).await.unwrap();
        
        let stats = controller.get_stats().await;
        let utilization = stats.utilization_percentage();
        
        assert!(utilization.is_some());
        // Utilization should be around 50% (512KB out of 1MB)
        if let Some(util) = utilization {
            assert!(util >= 0.0 && util <= 100.0);
        }
    }

    #[tokio::test]
    async fn test_unlimited_bandwidth_stats() {
        let controller = BandwidthController::new();
        controller.record_bytes(1024 * 1024).await.unwrap();
        
        let stats = controller.get_stats().await;
        
        assert_eq!(stats.limit, None);
        assert_eq!(stats.limit_human(), "Unlimited");
        assert!(stats.utilization_percentage().is_none());
    }
}
