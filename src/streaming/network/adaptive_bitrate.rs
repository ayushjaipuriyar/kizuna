// Adaptive bitrate streaming implementation
//
// Monitors network conditions and automatically adjusts stream quality
// to maintain smooth playback with congestion control and packet loss recovery.
//
// Requirements: 4.1, 4.2, 4.4, 4.5

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{Mutex, RwLock};

use crate::streaming::{QualityPreset, StreamError, StreamQuality, StreamResult};

/// Adaptive bitrate controller for video streaming
///
/// Monitors network conditions including bandwidth, latency, and packet loss,
/// then automatically adjusts stream quality to maintain optimal playback.
///
/// Requirements: 4.1, 4.2, 4.4, 4.5
pub struct AdaptiveBitrateController {
    config: AdaptiveBitrateConfig,
    network_monitor: Arc<Mutex<NetworkMonitor>>,
    quality_selector: Arc<RwLock<QualitySelector>>,
    congestion_controller: Arc<Mutex<CongestionController>>,
    packet_loss_recovery: Arc<Mutex<PacketLossRecovery>>,
}

/// Configuration for adaptive bitrate control
#[derive(Debug, Clone)]
pub struct AdaptiveBitrateConfig {
    /// Minimum bitrate (bps)
    pub min_bitrate: u32,
    /// Maximum bitrate (bps)
    pub max_bitrate: u32,
    /// Target buffer duration
    pub target_buffer_duration: Duration,
    /// Bandwidth estimation window
    pub bandwidth_window: Duration,
    /// Quality adjustment interval
    pub adjustment_interval: Duration,
    /// Packet loss threshold for quality reduction (0.0-1.0)
    pub packet_loss_threshold: f32,
    /// RTT threshold for quality reduction (ms)
    pub rtt_threshold_ms: u32,
    /// Enable aggressive quality upgrades
    pub aggressive_upgrades: bool,
}

/// Network condition monitor
struct NetworkMonitor {
    bandwidth_samples: VecDeque<BandwidthSample>,
    latency_samples: VecDeque<LatencySample>,
    packet_loss_samples: VecDeque<PacketLossSample>,
    current_conditions: NetworkConditions,
    last_update: SystemTime,
}

/// Bandwidth sample
#[derive(Debug, Clone)]
struct BandwidthSample {
    timestamp: SystemTime,
    bytes_sent: u64,
    duration: Duration,
    estimated_bandwidth: u32, // bps
}

/// Latency sample
#[derive(Debug, Clone)]
struct LatencySample {
    timestamp: SystemTime,
    rtt: Duration,
    jitter: Duration,
}

/// Packet loss sample
#[derive(Debug, Clone)]
struct PacketLossSample {
    timestamp: SystemTime,
    packets_sent: u64,
    packets_lost: u64,
    loss_rate: f32,
}

/// Current network conditions
#[derive(Debug, Clone)]
pub struct NetworkConditions {
    pub estimated_bandwidth: u32, // bps
    pub average_rtt: Duration,
    pub jitter: Duration,
    pub packet_loss_rate: f32,
    pub congestion_level: CongestionLevel,
    pub last_updated: SystemTime,
}

/// Congestion level indicator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionLevel {
    None,
    Low,
    Medium,
    High,
    Severe,
}

/// Quality selector for choosing appropriate stream quality
struct QualitySelector {
    current_quality: StreamQuality,
    target_quality: StreamQuality,
    quality_history: VecDeque<QualityChange>,
    last_change: SystemTime,
}

/// Quality change record
#[derive(Debug, Clone)]
struct QualityChange {
    timestamp: SystemTime,
    from_quality: StreamQuality,
    to_quality: StreamQuality,
    reason: QualityChangeReason,
}

/// Reason for quality change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityChangeReason {
    BandwidthIncrease,
    BandwidthDecrease,
    HighLatency,
    PacketLoss,
    Congestion,
    BufferUnderrun,
    BufferOverflow,
    Manual,
}

/// Congestion controller for managing network congestion
struct CongestionController {
    state: CongestionState,
    cwnd: u32, // Congestion window (bytes)
    ssthresh: u32, // Slow start threshold
    rtt_min: Duration,
    rtt_max: Duration,
    in_recovery: bool,
}

/// Congestion control state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CongestionState {
    SlowStart,
    CongestionAvoidance,
    FastRecovery,
}

/// Packet loss recovery mechanism
struct PacketLossRecovery {
    lost_packets: VecDeque<LostPacket>,
    recovery_attempts: HashMap<u64, u32>,
    max_recovery_attempts: u32,
}

use std::collections::HashMap;

/// Lost packet information
#[derive(Debug, Clone)]
struct LostPacket {
    sequence_number: u64,
    timestamp: SystemTime,
    data: Vec<u8>,
}

impl Default for AdaptiveBitrateConfig {
    fn default() -> Self {
        Self {
            min_bitrate: 500_000, // 500 kbps
            max_bitrate: 6_000_000, // 6 Mbps
            target_buffer_duration: Duration::from_secs(2),
            bandwidth_window: Duration::from_secs(5),
            adjustment_interval: Duration::from_secs(1),
            packet_loss_threshold: 0.05, // 5%
            rtt_threshold_ms: 200,
            aggressive_upgrades: false,
        }
    }
}

impl AdaptiveBitrateController {
    /// Create a new adaptive bitrate controller
    pub fn new() -> Self {
        Self::with_config(AdaptiveBitrateConfig::default())
    }

    /// Create a new adaptive bitrate controller with custom configuration
    pub fn with_config(config: AdaptiveBitrateConfig) -> Self {
        Self {
            config: config.clone(),
            network_monitor: Arc::new(Mutex::new(NetworkMonitor::new())),
            quality_selector: Arc::new(RwLock::new(QualitySelector::new())),
            congestion_controller: Arc::new(Mutex::new(CongestionController::new())),
            packet_loss_recovery: Arc::new(Mutex::new(PacketLossRecovery::new())),
        }
    }

    /// Update network statistics and get recommended bitrate adjustment
    pub async fn update_network_stats(
        &self,
        bandwidth_bps: u32,
        latency_ms: u32,
        packet_loss: f32,
    ) -> StreamResult<Option<u32>> {
        // Update network monitor
        let conditions = {
            let mut monitor = self.network_monitor.lock().await;
            monitor.add_bandwidth_sample(bandwidth_bps);
            monitor.add_latency_sample(Duration::from_millis(latency_ms as u64));
            monitor.add_packet_loss_sample(packet_loss);
            monitor.get_current_conditions()
        };

        // Update congestion controller
        {
            let mut congestion = self.congestion_controller.lock().await;
            congestion.update(
                Duration::from_millis(latency_ms as u64),
                packet_loss > 0.0,
            );
        }

        // Handle packet loss recovery if needed
        if packet_loss > self.config.packet_loss_threshold {
            let mut recovery = self.packet_loss_recovery.lock().await;
            recovery.trigger_recovery();
        }

        // Determine if quality adjustment is needed
        let recommended_bitrate = self.calculate_recommended_bitrate(&conditions).await?;

        Ok(Some(recommended_bitrate))
    }

    /// Get current network conditions
    pub async fn get_network_conditions(&self) -> NetworkConditions {
        let monitor = self.network_monitor.lock().await;
        monitor.get_current_conditions()
    }

    /// Adjust stream quality based on network conditions
    pub async fn adjust_quality(&self, conditions: &NetworkConditions) -> StreamResult<Option<StreamQuality>> {
        let mut selector = self.quality_selector.write().await;
        
        // Determine if quality change is needed
        let should_change = self.should_change_quality(conditions, &selector.current_quality).await?;
        
        if !should_change {
            return Ok(None);
        }

        // Calculate target quality
        let target_quality = self.calculate_target_quality(conditions)?;
        
        // Check if we can change quality (rate limiting)
        if !selector.can_change_quality(self.config.adjustment_interval) {
            return Ok(None);
        }

        // Determine reason for change
        let reason = self.determine_change_reason(conditions, &selector.current_quality, &target_quality);

        // Apply quality change
        selector.change_quality(target_quality.clone(), reason);

        Ok(Some(target_quality))
    }

    /// Estimate available bandwidth
    pub async fn estimate_bandwidth(&self) -> u32 {
        let monitor = self.network_monitor.lock().await;
        monitor.estimate_bandwidth()
    }

    /// Get congestion level
    pub async fn get_congestion_level(&self) -> CongestionLevel {
        let monitor = self.network_monitor.lock().await;
        monitor.current_conditions.congestion_level
    }

    /// Handle packet loss event
    pub async fn handle_packet_loss(&self, sequence_number: u64, data: Vec<u8>) -> StreamResult<()> {
        let mut recovery = self.packet_loss_recovery.lock().await;
        recovery.add_lost_packet(sequence_number, data);
        Ok(())
    }

    /// Attempt to recover lost packets
    pub async fn recover_lost_packets(&self) -> StreamResult<Vec<LostPacket>> {
        let mut recovery = self.packet_loss_recovery.lock().await;
        Ok(recovery.get_recoverable_packets())
    }

    // Private helper methods

    async fn calculate_recommended_bitrate(&self, conditions: &NetworkConditions) -> StreamResult<u32> {
        // Start with estimated bandwidth
        let mut recommended = conditions.estimated_bandwidth;

        // Apply safety margin based on congestion level
        let safety_margin = match conditions.congestion_level {
            CongestionLevel::None => 0.9,
            CongestionLevel::Low => 0.8,
            CongestionLevel::Medium => 0.7,
            CongestionLevel::High => 0.6,
            CongestionLevel::Severe => 0.5,
        };

        recommended = (recommended as f32 * safety_margin) as u32;

        // Apply packet loss penalty
        if conditions.packet_loss_rate > 0.0 {
            let loss_penalty = 1.0 - (conditions.packet_loss_rate * 2.0).min(0.5);
            recommended = (recommended as f32 * loss_penalty) as u32;
        }

        // Apply latency penalty
        if conditions.average_rtt.as_millis() > self.config.rtt_threshold_ms as u128 {
            let latency_penalty = 0.8;
            recommended = (recommended as f32 * latency_penalty) as u32;
        }

        // Clamp to configured limits
        recommended = recommended.max(self.config.min_bitrate).min(self.config.max_bitrate);

        Ok(recommended)
    }

    async fn should_change_quality(
        &self,
        conditions: &NetworkConditions,
        current_quality: &StreamQuality,
    ) -> StreamResult<bool> {
        // Check if bandwidth has changed significantly
        let bandwidth_ratio = conditions.estimated_bandwidth as f32 / current_quality.bitrate as f32;
        
        if bandwidth_ratio < 0.7 {
            // Bandwidth dropped significantly, should downgrade
            return Ok(true);
        }

        if bandwidth_ratio > 1.5 && self.config.aggressive_upgrades {
            // Bandwidth increased significantly, can upgrade
            return Ok(true);
        }

        // Check for high packet loss
        if conditions.packet_loss_rate > self.config.packet_loss_threshold {
            return Ok(true);
        }

        // Check for high latency
        if conditions.average_rtt.as_millis() > self.config.rtt_threshold_ms as u128 {
            return Ok(true);
        }

        Ok(false)
    }

    fn calculate_target_quality(&self, conditions: &NetworkConditions) -> StreamResult<StreamQuality> {
        // Select quality preset based on available bandwidth
        let preset = if conditions.estimated_bandwidth >= 5_000_000 {
            QualityPreset::Ultra
        } else if conditions.estimated_bandwidth >= 2_500_000 {
            QualityPreset::High
        } else if conditions.estimated_bandwidth >= 1_000_000 {
            QualityPreset::Medium
        } else {
            QualityPreset::Low
        };

        let mut quality = preset.to_quality();

        // Adjust based on packet loss
        if conditions.packet_loss_rate > 0.1 {
            // High packet loss, reduce framerate
            quality.framerate = (quality.framerate as f32 * 0.75) as u32;
        }

        // Adjust based on latency
        if conditions.average_rtt.as_millis() > 300 {
            // High latency, reduce resolution
            quality.resolution.width = (quality.resolution.width as f32 * 0.75) as u32;
            quality.resolution.height = (quality.resolution.height as f32 * 0.75) as u32;
        }

        Ok(quality)
    }

    fn determine_change_reason(
        &self,
        conditions: &NetworkConditions,
        current: &StreamQuality,
        target: &StreamQuality,
    ) -> QualityChangeReason {
        if target.bitrate > current.bitrate {
            QualityChangeReason::BandwidthIncrease
        } else if conditions.packet_loss_rate > self.config.packet_loss_threshold {
            QualityChangeReason::PacketLoss
        } else if conditions.average_rtt.as_millis() > self.config.rtt_threshold_ms as u128 {
            QualityChangeReason::HighLatency
        } else if conditions.congestion_level != CongestionLevel::None {
            QualityChangeReason::Congestion
        } else {
            QualityChangeReason::BandwidthDecrease
        }
    }
}

impl NetworkMonitor {
    fn new() -> Self {
        Self {
            bandwidth_samples: VecDeque::new(),
            latency_samples: VecDeque::new(),
            packet_loss_samples: VecDeque::new(),
            current_conditions: NetworkConditions::default(),
            last_update: SystemTime::now(),
        }
    }

    fn add_bandwidth_sample(&mut self, bandwidth_bps: u32) {
        let sample = BandwidthSample {
            timestamp: SystemTime::now(),
            bytes_sent: 0,
            duration: Duration::from_secs(1),
            estimated_bandwidth: bandwidth_bps,
        };

        self.bandwidth_samples.push_back(sample);
        
        // Keep only recent samples (last 10 seconds)
        while self.bandwidth_samples.len() > 10 {
            self.bandwidth_samples.pop_front();
        }

        self.update_conditions();
    }

    fn add_latency_sample(&mut self, rtt: Duration) {
        let jitter = if let Some(last) = self.latency_samples.back() {
            if rtt > last.rtt {
                rtt - last.rtt
            } else {
                last.rtt - rtt
            }
        } else {
            Duration::ZERO
        };

        let sample = LatencySample {
            timestamp: SystemTime::now(),
            rtt,
            jitter,
        };

        self.latency_samples.push_back(sample);
        
        while self.latency_samples.len() > 10 {
            self.latency_samples.pop_front();
        }

        self.update_conditions();
    }

    fn add_packet_loss_sample(&mut self, loss_rate: f32) {
        let sample = PacketLossSample {
            timestamp: SystemTime::now(),
            packets_sent: 100,
            packets_lost: (100.0 * loss_rate) as u64,
            loss_rate,
        };

        self.packet_loss_samples.push_back(sample);
        
        while self.packet_loss_samples.len() > 10 {
            self.packet_loss_samples.pop_front();
        }

        self.update_conditions();
    }

    fn update_conditions(&mut self) {
        self.current_conditions.estimated_bandwidth = self.estimate_bandwidth();
        self.current_conditions.average_rtt = self.calculate_average_rtt();
        self.current_conditions.jitter = self.calculate_jitter();
        self.current_conditions.packet_loss_rate = self.calculate_packet_loss_rate();
        self.current_conditions.congestion_level = self.determine_congestion_level();
        self.current_conditions.last_updated = SystemTime::now();
    }

    fn estimate_bandwidth(&self) -> u32 {
        if self.bandwidth_samples.is_empty() {
            return 1_000_000; // Default 1 Mbps
        }

        let sum: u32 = self.bandwidth_samples.iter().map(|s| s.estimated_bandwidth).sum();
        sum / self.bandwidth_samples.len() as u32
    }

    fn calculate_average_rtt(&self) -> Duration {
        if self.latency_samples.is_empty() {
            return Duration::from_millis(50);
        }

        let sum: Duration = self.latency_samples.iter().map(|s| s.rtt).sum();
        sum / self.latency_samples.len() as u32
    }

    fn calculate_jitter(&self) -> Duration {
        if self.latency_samples.is_empty() {
            return Duration::ZERO;
        }

        let sum: Duration = self.latency_samples.iter().map(|s| s.jitter).sum();
        sum / self.latency_samples.len() as u32
    }

    fn calculate_packet_loss_rate(&self) -> f32 {
        if self.packet_loss_samples.is_empty() {
            return 0.0;
        }

        let sum: f32 = self.packet_loss_samples.iter().map(|s| s.loss_rate).sum();
        sum / self.packet_loss_samples.len() as f32
    }

    fn determine_congestion_level(&self) -> CongestionLevel {
        let loss_rate = self.calculate_packet_loss_rate();
        let rtt = self.calculate_average_rtt();

        if loss_rate > 0.15 || rtt.as_millis() > 500 {
            CongestionLevel::Severe
        } else if loss_rate > 0.10 || rtt.as_millis() > 300 {
            CongestionLevel::High
        } else if loss_rate > 0.05 || rtt.as_millis() > 200 {
            CongestionLevel::Medium
        } else if loss_rate > 0.02 || rtt.as_millis() > 100 {
            CongestionLevel::Low
        } else {
            CongestionLevel::None
        }
    }

    fn get_current_conditions(&self) -> NetworkConditions {
        self.current_conditions.clone()
    }
}

impl Default for NetworkConditions {
    fn default() -> Self {
        Self {
            estimated_bandwidth: 1_000_000,
            average_rtt: Duration::from_millis(50),
            jitter: Duration::ZERO,
            packet_loss_rate: 0.0,
            congestion_level: CongestionLevel::None,
            last_updated: SystemTime::now(),
        }
    }
}

impl QualitySelector {
    fn new() -> Self {
        Self {
            current_quality: StreamQuality::default(),
            target_quality: StreamQuality::default(),
            quality_history: VecDeque::new(),
            last_change: SystemTime::now(),
        }
    }

    fn can_change_quality(&self, min_interval: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.last_change)
            .unwrap_or_default()
            >= min_interval
    }

    fn change_quality(&mut self, new_quality: StreamQuality, reason: QualityChangeReason) {
        let change = QualityChange {
            timestamp: SystemTime::now(),
            from_quality: self.current_quality.clone(),
            to_quality: new_quality.clone(),
            reason,
        };

        self.quality_history.push_back(change);
        
        while self.quality_history.len() > 20 {
            self.quality_history.pop_front();
        }

        self.current_quality = new_quality;
        self.last_change = SystemTime::now();
    }
}

impl CongestionController {
    fn new() -> Self {
        Self {
            state: CongestionState::SlowStart,
            cwnd: 10_000, // Initial congestion window
            ssthresh: 65535,
            rtt_min: Duration::from_millis(10),
            rtt_max: Duration::from_millis(1000),
            in_recovery: false,
        }
    }

    fn update(&mut self, rtt: Duration, packet_lost: bool) {
        // Update RTT bounds
        if rtt < self.rtt_min {
            self.rtt_min = rtt;
        }
        if rtt > self.rtt_max {
            self.rtt_max = rtt;
        }

        if packet_lost {
            self.handle_packet_loss();
        } else {
            self.handle_ack();
        }
    }

    fn handle_packet_loss(&mut self) {
        if !self.in_recovery {
            self.ssthresh = self.cwnd / 2;
            self.cwnd = self.ssthresh;
            self.state = CongestionState::FastRecovery;
            self.in_recovery = true;
        }
    }

    fn handle_ack(&mut self) {
        self.in_recovery = false;

        match self.state {
            CongestionState::SlowStart => {
                self.cwnd += 1000; // Increase by 1 MSS
                if self.cwnd >= self.ssthresh {
                    self.state = CongestionState::CongestionAvoidance;
                }
            }
            CongestionState::CongestionAvoidance => {
                self.cwnd += 1000 * 1000 / self.cwnd; // Additive increase
            }
            CongestionState::FastRecovery => {
                self.state = CongestionState::CongestionAvoidance;
            }
        }
    }
}

impl PacketLossRecovery {
    fn new() -> Self {
        Self {
            lost_packets: VecDeque::new(),
            recovery_attempts: HashMap::new(),
            max_recovery_attempts: 3,
        }
    }

    fn add_lost_packet(&mut self, sequence_number: u64, data: Vec<u8>) {
        let packet = LostPacket {
            sequence_number,
            timestamp: SystemTime::now(),
            data,
        };

        self.lost_packets.push_back(packet);
        self.recovery_attempts.insert(sequence_number, 0);

        // Limit queue size
        while self.lost_packets.len() > 100 {
            if let Some(old_packet) = self.lost_packets.pop_front() {
                self.recovery_attempts.remove(&old_packet.sequence_number);
            }
        }
    }

    fn trigger_recovery(&mut self) {
        // Mark recovery as triggered
    }

    fn get_recoverable_packets(&mut self) -> Vec<LostPacket> {
        let mut recoverable = Vec::new();

        for packet in &self.lost_packets {
            let attempts = self.recovery_attempts.get(&packet.sequence_number).copied().unwrap_or(0);
            
            if attempts < self.max_recovery_attempts {
                recoverable.push(packet.clone());
                self.recovery_attempts.insert(packet.sequence_number, attempts + 1);
            }
        }

        recoverable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_adaptive_bitrate_controller_creation() {
        let controller = AdaptiveBitrateController::new();
        let conditions = controller.get_network_conditions().await;
        assert_eq!(conditions.congestion_level, CongestionLevel::None);
    }

    #[tokio::test]
    async fn test_network_stats_update() {
        let controller = AdaptiveBitrateController::new();
        let result = controller.update_network_stats(2_000_000, 50, 0.01).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_bandwidth_estimation() {
        let controller = AdaptiveBitrateController::new();
        controller.update_network_stats(2_000_000, 50, 0.0).await.unwrap();
        let bandwidth = controller.estimate_bandwidth().await;
        assert!(bandwidth > 0);
    }

    #[test]
    fn test_congestion_levels() {
        assert_eq!(CongestionLevel::None, CongestionLevel::None);
        assert_ne!(CongestionLevel::Low, CongestionLevel::High);
    }

    #[test]
    fn test_quality_change_reasons() {
        assert_eq!(QualityChangeReason::BandwidthIncrease, QualityChangeReason::BandwidthIncrease);
        assert_ne!(QualityChangeReason::PacketLoss, QualityChangeReason::Congestion);
    }
}
