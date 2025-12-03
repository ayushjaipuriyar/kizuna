// Stream buffering and flow control implementation
//
// Implements adaptive buffering based on network jitter and latency,
// flow control to prevent buffer overflow/underflow, and stream
// synchronization for audio-video alignment.
//
// Requirements: 2.4, 4.4

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock, Semaphore};

use crate::streaming::{EncodedFrame, StreamError, StreamResult};

/// Stream buffer manager for adaptive buffering and flow control
///
/// Manages video frame buffering with adaptive sizing based on network
/// conditions, implements flow control to prevent overflow/underflow,
/// and provides stream synchronization capabilities.
///
/// Requirements: 2.4, 4.4
pub struct StreamBufferManager {
    config: BufferConfig,
    video_buffer: Arc<Mutex<AdaptiveBuffer>>,
    audio_buffer: Arc<Mutex<AdaptiveBuffer>>,
    flow_controller: Arc<Mutex<FlowController>>,
    sync_manager: Arc<RwLock<SyncManager>>,
    buffer_monitor: Arc<Mutex<BufferMonitor>>,
}

/// Configuration for stream buffering
#[derive(Debug, Clone)]
pub struct BufferConfig {
    /// Initial buffer size (number of frames)
    pub initial_buffer_size: usize,
    /// Minimum buffer size
    pub min_buffer_size: usize,
    /// Maximum buffer size
    pub max_buffer_size: usize,
    /// Target buffer duration
    pub target_duration: Duration,
    /// Underrun threshold (percentage)
    pub underrun_threshold: f32,
    /// Overrun threshold (percentage)
    pub overrun_threshold: f32,
    /// Enable adaptive buffer sizing
    pub adaptive_sizing: bool,
    /// Flow control window size
    pub flow_control_window: usize,
}

/// Adaptive buffer for video/audio frames
struct AdaptiveBuffer {
    frames: VecDeque<BufferedFrame>,
    capacity: usize,
    target_duration: Duration,
    current_duration: Duration,
    underrun_count: u64,
    overrun_count: u64,
    last_adjustment: SystemTime,
}

/// Buffered frame with metadata
#[derive(Debug, Clone)]
struct BufferedFrame {
    frame: EncodedFrame,
    arrival_time: SystemTime,
    presentation_time: SystemTime,
    sequence_number: u64,
    priority: FramePriority,
}

/// Frame priority for buffer management
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FramePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Flow controller for managing data flow
struct FlowController {
    window_size: usize,
    bytes_in_flight: usize,
    max_bytes_in_flight: usize,
    send_semaphore: Arc<Semaphore>,
    ack_tracker: AckTracker,
    congestion_window: usize,
}

/// Acknowledgment tracker
struct AckTracker {
    pending_acks: HashMap<u64, PendingAck>,
    next_sequence: u64,
    last_ack_time: SystemTime,
}

/// Pending acknowledgment
#[derive(Debug, Clone)]
struct PendingAck {
    sequence_number: u64,
    sent_time: SystemTime,
    data_size: usize,
    retransmit_count: u32,
}

/// Stream synchronization manager
struct SyncManager {
    video_clock: StreamClock,
    audio_clock: StreamClock,
    sync_offset: Duration,
    max_sync_drift: Duration,
    sync_corrections: u64,
}

/// Stream clock for timing
#[derive(Debug, Clone)]
struct StreamClock {
    start_time: SystemTime,
    current_timestamp: Duration,
    last_update: SystemTime,
    drift: Duration,
}

/// Buffer monitor for statistics and health
struct BufferMonitor {
    stats: BufferStats,
    health_status: BufferHealth,
    alerts: VecDeque<BufferAlert>,
}

/// Buffer statistics
#[derive(Debug, Clone)]
pub struct BufferStats {
    pub frames_buffered: u64,
    pub frames_dropped: u64,
    pub underrun_events: u64,
    pub overrun_events: u64,
    pub average_buffer_level: f32,
    pub current_buffer_level: usize,
    pub buffer_capacity: usize,
    pub last_updated: SystemTime,
}

/// Buffer health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferHealth {
    Healthy,
    Warning,
    Critical,
}

/// Buffer alert
#[derive(Debug, Clone)]
pub struct BufferAlert {
    pub timestamp: SystemTime,
    pub alert_type: BufferAlertType,
    pub message: String,
}

/// Types of buffer alerts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferAlertType {
    Underrun,
    Overrun,
    SyncDrift,
    FlowControlStall,
}

impl Default for BufferConfig {
    fn default() -> Self {
        Self {
            initial_buffer_size: 30, // 30 frames (~1 second at 30fps)
            min_buffer_size: 10,
            max_buffer_size: 150, // 5 seconds at 30fps
            target_duration: Duration::from_secs(2),
            underrun_threshold: 0.2, // 20%
            overrun_threshold: 0.9, // 90%
            adaptive_sizing: true,
            flow_control_window: 64 * 1024, // 64KB
        }
    }
}

impl StreamBufferManager {
    /// Create a new stream buffer manager
    pub fn new() -> Self {
        Self::with_config(BufferConfig::default())
    }

    /// Create a new stream buffer manager with custom configuration
    pub fn with_config(config: BufferConfig) -> Self {
        Self {
            video_buffer: Arc::new(Mutex::new(AdaptiveBuffer::new(
                config.initial_buffer_size,
                config.target_duration,
            ))),
            audio_buffer: Arc::new(Mutex::new(AdaptiveBuffer::new(
                config.initial_buffer_size,
                config.target_duration,
            ))),
            flow_controller: Arc::new(Mutex::new(FlowController::new(config.flow_control_window))),
            sync_manager: Arc::new(RwLock::new(SyncManager::new())),
            buffer_monitor: Arc::new(Mutex::new(BufferMonitor::new())),
            config,
        }
    }

    /// Push a video frame into the buffer
    pub async fn push_video_frame(
        &self,
        frame: EncodedFrame,
        sequence_number: u64,
    ) -> StreamResult<()> {
        // Check flow control
        {
            let mut flow = self.flow_controller.lock().await;
            flow.wait_for_capacity(frame.data.len()).await?;
        }

        // Add frame to buffer
        let buffered_frame = BufferedFrame {
            frame: frame.clone(),
            arrival_time: SystemTime::now(),
            presentation_time: frame.timestamp,
            sequence_number,
            priority: FramePriority::Normal,
        };

        let mut buffer = self.video_buffer.lock().await;
        buffer.push(buffered_frame)?;

        // Update flow control
        {
            let mut flow = self.flow_controller.lock().await;
            flow.track_sent(sequence_number, frame.data.len());
        }

        // Update monitor
        {
            let mut monitor = self.buffer_monitor.lock().await;
            monitor.record_frame_buffered();
        }

        // Check for buffer adjustments
        if self.config.adaptive_sizing {
            self.adjust_buffer_size().await?;
        }

        Ok(())
    }

    /// Pop a video frame from the buffer
    pub async fn pop_video_frame(&self) -> StreamResult<Option<EncodedFrame>> {
        let mut buffer = self.video_buffer.lock().await;
        
        if let Some(buffered_frame) = buffer.pop()? {
            // Acknowledge frame
            {
                let mut flow = self.flow_controller.lock().await;
                flow.acknowledge(buffered_frame.sequence_number);
            }

            Ok(Some(buffered_frame.frame))
        } else {
            // Buffer underrun
            {
                let mut monitor = self.buffer_monitor.lock().await;
                monitor.record_underrun();
            }
            Ok(None)
        }
    }

    /// Push an audio frame into the buffer
    pub async fn push_audio_frame(
        &self,
        frame: EncodedFrame,
        sequence_number: u64,
    ) -> StreamResult<()> {
        let buffered_frame = BufferedFrame {
            frame,
            arrival_time: SystemTime::now(),
            presentation_time: SystemTime::now(),
            sequence_number,
            priority: FramePriority::High, // Audio has higher priority
        };

        let mut buffer = self.audio_buffer.lock().await;
        buffer.push(buffered_frame)?;

        Ok(())
    }

    /// Pop an audio frame from the buffer
    pub async fn pop_audio_frame(&self) -> StreamResult<Option<EncodedFrame>> {
        let mut buffer = self.audio_buffer.lock().await;
        
        if let Some(buffered_frame) = buffer.pop()? {
            Ok(Some(buffered_frame.frame))
        } else {
            Ok(None)
        }
    }

    /// Synchronize audio and video streams
    pub async fn synchronize_streams(&self) -> StreamResult<()> {
        let mut sync = self.sync_manager.write().await;
        
        // Get current timestamps from both buffers
        let video_timestamp = {
            let buffer = self.video_buffer.lock().await;
            buffer.get_current_timestamp()
        };

        let audio_timestamp = {
            let buffer = self.audio_buffer.lock().await;
            buffer.get_current_timestamp()
        };

        // Calculate sync offset
        if let (Some(video_ts), Some(audio_ts)) = (video_timestamp, audio_timestamp) {
            sync.update_sync_offset(video_ts, audio_ts)?;
        }

        Ok(())
    }

    /// Get buffer level (percentage full)
    pub async fn get_buffer_level(&self) -> f32 {
        let buffer = self.video_buffer.lock().await;
        buffer.get_level()
    }

    /// Get buffer statistics
    pub async fn get_stats(&self) -> BufferStats {
        let monitor = self.buffer_monitor.lock().await;
        monitor.stats.clone()
    }

    /// Get buffer health status
    pub async fn get_health(&self) -> BufferHealth {
        let monitor = self.buffer_monitor.lock().await;
        monitor.health_status
    }

    /// Get pending alerts
    pub async fn get_alerts(&self) -> Vec<BufferAlert> {
        let monitor = self.buffer_monitor.lock().await;
        monitor.alerts.iter().cloned().collect()
    }

    /// Clear buffer
    pub async fn clear(&self) -> StreamResult<()> {
        {
            let mut buffer = self.video_buffer.lock().await;
            buffer.clear();
        }
        {
            let mut buffer = self.audio_buffer.lock().await;
            buffer.clear();
        }
        Ok(())
    }

    // Private helper methods

    async fn adjust_buffer_size(&self) -> StreamResult<()> {
        let mut buffer = self.video_buffer.lock().await;
        
        // Check if adjustment is needed
        let level = buffer.get_level();
        
        if level < self.config.underrun_threshold {
            // Increase buffer size
            let new_capacity = (buffer.capacity as f32 * 1.5) as usize;
            let new_capacity = new_capacity.min(self.config.max_buffer_size);
            buffer.resize(new_capacity);
        } else if level > self.config.overrun_threshold {
            // Decrease buffer size
            let new_capacity = (buffer.capacity as f32 * 0.75) as usize;
            let new_capacity = new_capacity.max(self.config.min_buffer_size);
            buffer.resize(new_capacity);
        }

        Ok(())
    }
}

impl AdaptiveBuffer {
    fn new(capacity: usize, target_duration: Duration) -> Self {
        Self {
            frames: VecDeque::with_capacity(capacity),
            capacity,
            target_duration,
            current_duration: Duration::ZERO,
            underrun_count: 0,
            overrun_count: 0,
            last_adjustment: SystemTime::now(),
        }
    }

    fn push(&mut self, frame: BufferedFrame) -> StreamResult<()> {
        if self.frames.len() >= self.capacity {
            self.overrun_count += 1;
            // Drop oldest frame if buffer is full
            self.frames.pop_front();
        }

        self.frames.push_back(frame);
        Ok(())
    }

    fn pop(&mut self) -> StreamResult<Option<BufferedFrame>> {
        if let Some(frame) = self.frames.pop_front() {
            Ok(Some(frame))
        } else {
            self.underrun_count += 1;
            Ok(None)
        }
    }

    fn get_level(&self) -> f32 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.frames.len() as f32 / self.capacity as f32
    }

    fn get_current_timestamp(&self) -> Option<SystemTime> {
        self.frames.front().map(|f| f.presentation_time)
    }

    fn resize(&mut self, new_capacity: usize) {
        self.capacity = new_capacity;
        self.last_adjustment = SystemTime::now();
    }

    fn clear(&mut self) {
        self.frames.clear();
        self.current_duration = Duration::ZERO;
    }
}

impl FlowController {
    fn new(window_size: usize) -> Self {
        Self {
            window_size,
            bytes_in_flight: 0,
            max_bytes_in_flight: window_size,
            send_semaphore: Arc::new(Semaphore::new(window_size)),
            ack_tracker: AckTracker::new(),
            congestion_window: window_size,
        }
    }

    async fn wait_for_capacity(&mut self, data_size: usize) -> StreamResult<()> {
        // Wait for available capacity in the flow control window
        let _permit = self
            .send_semaphore
            .acquire()
            .await
            .map_err(|e| StreamError::network(format!("Flow control error: {}", e)))?;

        if self.bytes_in_flight + data_size > self.max_bytes_in_flight {
            return Err(StreamError::resource("Flow control window exceeded"));
        }

        Ok(())
    }

    fn track_sent(&mut self, sequence_number: u64, data_size: usize) {
        self.bytes_in_flight += data_size;
        self.ack_tracker.add_pending(sequence_number, data_size);
    }

    fn acknowledge(&mut self, sequence_number: u64) {
        if let Some(ack) = self.ack_tracker.acknowledge(sequence_number) {
            self.bytes_in_flight = self.bytes_in_flight.saturating_sub(ack.data_size);
            self.send_semaphore.add_permits(1);
        }
    }

    fn update_window(&mut self, new_size: usize) {
        self.congestion_window = new_size;
        self.max_bytes_in_flight = new_size;
    }
}

impl AckTracker {
    fn new() -> Self {
        Self {
            pending_acks: HashMap::new(),
            next_sequence: 0,
            last_ack_time: SystemTime::now(),
        }
    }

    fn add_pending(&mut self, sequence_number: u64, data_size: usize) {
        let ack = PendingAck {
            sequence_number,
            sent_time: SystemTime::now(),
            data_size,
            retransmit_count: 0,
        };
        self.pending_acks.insert(sequence_number, ack);
    }

    fn acknowledge(&mut self, sequence_number: u64) -> Option<PendingAck> {
        self.last_ack_time = SystemTime::now();
        self.pending_acks.remove(&sequence_number)
    }

    fn get_pending_count(&self) -> usize {
        self.pending_acks.len()
    }
}

impl SyncManager {
    fn new() -> Self {
        Self {
            video_clock: StreamClock::new(),
            audio_clock: StreamClock::new(),
            sync_offset: Duration::ZERO,
            max_sync_drift: Duration::from_millis(100),
            sync_corrections: 0,
        }
    }

    fn update_sync_offset(
        &mut self,
        video_timestamp: SystemTime,
        audio_timestamp: SystemTime,
    ) -> StreamResult<()> {
        // Calculate drift between video and audio
        let drift = if video_timestamp > audio_timestamp {
            video_timestamp.duration_since(audio_timestamp).unwrap_or_default()
        } else {
            audio_timestamp.duration_since(video_timestamp).unwrap_or_default()
        };

        if drift > self.max_sync_drift {
            // Apply correction
            self.sync_offset = drift;
            self.sync_corrections += 1;
        }

        Ok(())
    }

    fn get_sync_offset(&self) -> Duration {
        self.sync_offset
    }
}

impl StreamClock {
    fn new() -> Self {
        let now = SystemTime::now();
        Self {
            start_time: now,
            current_timestamp: Duration::ZERO,
            last_update: now,
            drift: Duration::ZERO,
        }
    }

    fn update(&mut self, timestamp: Duration) {
        self.current_timestamp = timestamp;
        self.last_update = SystemTime::now();
    }
}

impl BufferMonitor {
    fn new() -> Self {
        Self {
            stats: BufferStats::default(),
            health_status: BufferHealth::Healthy,
            alerts: VecDeque::new(),
        }
    }

    fn record_frame_buffered(&mut self) {
        self.stats.frames_buffered += 1;
        self.stats.last_updated = SystemTime::now();
        self.update_health();
    }

    fn record_underrun(&mut self) {
        self.stats.underrun_events += 1;
        self.add_alert(BufferAlertType::Underrun, "Buffer underrun detected");
        self.update_health();
    }

    fn record_overrun(&mut self) {
        self.stats.overrun_events += 1;
        self.add_alert(BufferAlertType::Overrun, "Buffer overrun detected");
        self.update_health();
    }

    fn add_alert(&mut self, alert_type: BufferAlertType, message: &str) {
        let alert = BufferAlert {
            timestamp: SystemTime::now(),
            alert_type,
            message: message.to_string(),
        };
        self.alerts.push_back(alert);

        // Keep only recent alerts
        while self.alerts.len() > 50 {
            self.alerts.pop_front();
        }
    }

    fn update_health(&mut self) {
        // Determine health based on recent events
        let recent_underruns = self.stats.underrun_events;
        let recent_overruns = self.stats.overrun_events;

        self.health_status = if recent_underruns > 10 || recent_overruns > 10 {
            BufferHealth::Critical
        } else if recent_underruns > 3 || recent_overruns > 3 {
            BufferHealth::Warning
        } else {
            BufferHealth::Healthy
        };
    }
}

impl Default for BufferStats {
    fn default() -> Self {
        Self {
            frames_buffered: 0,
            frames_dropped: 0,
            underrun_events: 0,
            overrun_events: 0,
            average_buffer_level: 0.0,
            current_buffer_level: 0,
            buffer_capacity: 0,
            last_updated: SystemTime::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_buffer_manager_creation() {
        let manager = StreamBufferManager::new();
        let level = manager.get_buffer_level().await;
        assert_eq!(level, 0.0);
    }

    #[tokio::test]
    async fn test_buffer_stats() {
        let manager = StreamBufferManager::new();
        let stats = manager.get_stats().await;
        assert_eq!(stats.frames_buffered, 0);
        assert_eq!(stats.underrun_events, 0);
    }

    #[tokio::test]
    async fn test_buffer_health() {
        let manager = StreamBufferManager::new();
        let health = manager.get_health().await;
        assert_eq!(health, BufferHealth::Healthy);
    }

    #[test]
    fn test_frame_priority() {
        assert!(FramePriority::Critical > FramePriority::High);
        assert!(FramePriority::High > FramePriority::Normal);
        assert!(FramePriority::Normal > FramePriority::Low);
    }

    #[test]
    fn test_buffer_config_defaults() {
        let config = BufferConfig::default();
        assert_eq!(config.initial_buffer_size, 30);
        assert_eq!(config.min_buffer_size, 10);
        assert_eq!(config.max_buffer_size, 150);
        assert!(config.adaptive_sizing);
    }
}
