// Adaptive quality scaling and bitrate control
//
// Provides dynamic resolution and framerate adjustment based on network conditions
// and CPU usage, with quality preset system.
//
// Requirements: 4.1, 4.2, 7.1, 7.2

use std::time::{Duration, Instant};

use crate::streaming::{
    EncodingQuality, QualityPreset, Resolution, StreamError, StreamQuality, StreamResult,
};

/// Quality scaler for adaptive bitrate streaming
/// 
/// Dynamically adjusts resolution and framerate based on network conditions
/// and device capabilities.
/// 
/// Requirements: 4.1, 4.2, 7.1, 7.2
pub struct QualityScaler {
    min_quality: QualityPreset,
    max_quality: QualityPreset,
    current_preset: QualityPreset,
    adjustment_history: Vec<QualityAdjustment>,
}

#[derive(Debug, Clone)]
struct QualityAdjustment {
    timestamp: Instant,
    from_preset: QualityPreset,
    to_preset: QualityPreset,
    reason: AdjustmentReason,
}

#[derive(Debug, Clone, Copy)]
enum AdjustmentReason {
    NetworkBandwidth,
    NetworkLatency,
    CpuUsage,
    Manual,
}

impl QualityScaler {
    /// Create a new quality scaler
    pub fn new() -> Self {
        Self {
            min_quality: QualityPreset::Low,
            max_quality: QualityPreset::Ultra,
            current_preset: QualityPreset::Medium,
            adjustment_history: Vec::new(),
        }
    }

    /// Create a quality scaler with custom bounds
    pub fn with_bounds(min: QualityPreset, max: QualityPreset) -> Self {
        Self {
            min_quality: min,
            max_quality: max,
            current_preset: min,
            adjustment_history: Vec::new(),
        }
    }

    /// Adjust quality based on network conditions
    pub async fn adjust_for_network(
        &mut self,
        current_quality: EncodingQuality,
        bandwidth_bps: u32,
        latency_ms: u32,
    ) -> StreamResult<EncodingQuality> {
        // Calculate required bitrate with overhead
        let required_bitrate = (current_quality.bitrate as f32 * 1.2) as u32;
        
        // Determine if we need to scale up or down
        let target_preset = if bandwidth_bps < required_bitrate || latency_ms > 200 {
            // Network struggling, scale down
            self.scale_down_preset(AdjustmentReason::NetworkBandwidth)
        } else if bandwidth_bps > required_bitrate * 2 && latency_ms < 50 {
            // Network has headroom, scale up
            self.scale_up_preset(AdjustmentReason::NetworkBandwidth)
        } else {
            // Network is adequate, maintain current quality
            self.current_preset
        };
        
        self.preset_to_encoding_quality(target_preset, bandwidth_bps)
    }

    /// Adjust quality based on CPU usage
    pub async fn adjust_for_cpu(
        &mut self,
        current_quality: EncodingQuality,
        cpu_usage: f32,
    ) -> StreamResult<EncodingQuality> {
        let target_preset = if cpu_usage > 0.8 {
            // CPU overloaded, scale down aggressively
            self.scale_down_preset(AdjustmentReason::CpuUsage)
        } else if cpu_usage > 0.6 {
            // CPU under pressure, scale down
            self.scale_down_preset(AdjustmentReason::CpuUsage)
        } else if cpu_usage < 0.3 {
            // CPU has headroom, scale up
            self.scale_up_preset(AdjustmentReason::CpuUsage)
        } else {
            // CPU usage acceptable, maintain current quality
            self.current_preset
        };
        
        // Use current bitrate as reference
        self.preset_to_encoding_quality(target_preset, current_quality.bitrate)
    }

    /// Scale down to lower quality preset
    fn scale_down_preset(&mut self, reason: AdjustmentReason) -> QualityPreset {
        let old_preset = self.current_preset;
        
        let new_preset = match self.current_preset {
            QualityPreset::Ultra => QualityPreset::High,
            QualityPreset::High => QualityPreset::Medium,
            QualityPreset::Medium => QualityPreset::Low,
            QualityPreset::Low => QualityPreset::Low,
            QualityPreset::Custom => QualityPreset::Medium,
        };
        
        // Respect minimum quality bound
        let new_preset = if self.preset_level(new_preset) < self.preset_level(self.min_quality) {
            self.min_quality
        } else {
            new_preset
        };
        
        if new_preset != old_preset {
            self.record_adjustment(old_preset, new_preset, reason);
            self.current_preset = new_preset;
        }
        
        new_preset
    }

    /// Scale up to higher quality preset
    fn scale_up_preset(&mut self, reason: AdjustmentReason) -> QualityPreset {
        let old_preset = self.current_preset;
        
        let new_preset = match self.current_preset {
            QualityPreset::Low => QualityPreset::Medium,
            QualityPreset::Medium => QualityPreset::High,
            QualityPreset::High => QualityPreset::Ultra,
            QualityPreset::Ultra => QualityPreset::Ultra,
            QualityPreset::Custom => QualityPreset::Medium,
        };
        
        // Respect maximum quality bound
        let new_preset = if self.preset_level(new_preset) > self.preset_level(self.max_quality) {
            self.max_quality
        } else {
            new_preset
        };
        
        if new_preset != old_preset {
            self.record_adjustment(old_preset, new_preset, reason);
            self.current_preset = new_preset;
        }
        
        new_preset
    }

    /// Get numeric level for preset comparison
    fn preset_level(&self, preset: QualityPreset) -> u8 {
        match preset {
            QualityPreset::Low => 1,
            QualityPreset::Medium => 2,
            QualityPreset::High => 3,
            QualityPreset::Ultra => 4,
            QualityPreset::Custom => 2,
        }
    }

    /// Convert preset to encoding quality
    fn preset_to_encoding_quality(&self, preset: QualityPreset, max_bitrate: u32) -> StreamResult<EncodingQuality> {
        let stream_quality = preset.to_quality();
        
        // Cap bitrate to available bandwidth
        let bitrate = stream_quality.bitrate.min(max_bitrate);
        
        Ok(EncodingQuality {
            bitrate,
            quality_factor: self.preset_to_quality_factor(preset),
            keyframe_interval: self.preset_to_keyframe_interval(preset),
        })
    }

    /// Get quality factor (0-100) for preset
    fn preset_to_quality_factor(&self, preset: QualityPreset) -> u32 {
        match preset {
            QualityPreset::Low => 50,
            QualityPreset::Medium => 65,
            QualityPreset::High => 80,
            QualityPreset::Ultra => 95,
            QualityPreset::Custom => 65,
        }
    }

    /// Get keyframe interval for preset
    fn preset_to_keyframe_interval(&self, preset: QualityPreset) -> u32 {
        match preset {
            QualityPreset::Low => 60,    // Every 4 seconds at 15fps
            QualityPreset::Medium => 90,  // Every 3 seconds at 30fps
            QualityPreset::High => 90,    // Every 3 seconds at 30fps
            QualityPreset::Ultra => 120,  // Every 2 seconds at 60fps
            QualityPreset::Custom => 90,
        }
    }

    /// Record quality adjustment
    fn record_adjustment(&mut self, from: QualityPreset, to: QualityPreset, reason: AdjustmentReason) {
        self.adjustment_history.push(QualityAdjustment {
            timestamp: Instant::now(),
            from_preset: from,
            to_preset: to,
            reason,
        });
        
        // Keep only recent history (last 100 adjustments)
        if self.adjustment_history.len() > 100 {
            self.adjustment_history.remove(0);
        }
    }

    /// Get current quality preset
    pub fn current_preset(&self) -> QualityPreset {
        self.current_preset
    }

    /// Set current quality preset manually
    pub fn set_preset(&mut self, preset: QualityPreset) {
        let old_preset = self.current_preset;
        self.current_preset = preset;
        self.record_adjustment(old_preset, preset, AdjustmentReason::Manual);
    }
}

impl Default for QualityScaler {
    fn default() -> Self {
        Self::new()
    }
}

/// Bitrate controller for adaptive streaming
///
/// Requirements: 4.1, 4.2
pub struct BitrateController {
    target_bitrate: u32,
    min_bitrate: u32,
    max_bitrate: u32,
    current_bitrate: u32,
    bitrate_history: Vec<BitratePoint>,
}

#[derive(Debug, Clone)]
struct BitratePoint {
    timestamp: Instant,
    bitrate: u32,
    packet_loss: f32,
}

impl BitrateController {
    /// Create a new bitrate controller
    pub fn new(target_bitrate: u32) -> Self {
        Self {
            target_bitrate,
            min_bitrate: 100_000,      // 100 kbps minimum
            max_bitrate: 10_000_000,   // 10 Mbps maximum
            current_bitrate: target_bitrate,
            bitrate_history: Vec::new(),
        }
    }

    /// Create a bitrate controller with custom bounds
    pub fn with_bounds(target: u32, min: u32, max: u32) -> Self {
        Self {
            target_bitrate: target,
            min_bitrate: min,
            max_bitrate: max,
            current_bitrate: target,
            bitrate_history: Vec::new(),
        }
    }

    /// Adjust bitrate based on network feedback
    pub fn adjust_bitrate(&mut self, bandwidth_bps: u32, packet_loss: f32, rtt_ms: u32) -> u32 {
        // Record current state
        self.bitrate_history.push(BitratePoint {
            timestamp: Instant::now(),
            bitrate: self.current_bitrate,
            packet_loss,
        });
        
        // Keep only recent history (last 60 seconds)
        let cutoff = Instant::now() - Duration::from_secs(60);
        self.bitrate_history.retain(|p| p.timestamp > cutoff);
        
        // Calculate new bitrate based on conditions
        let new_bitrate = if packet_loss > 0.05 {
            // High packet loss, reduce bitrate aggressively
            (self.current_bitrate as f32 * 0.7) as u32
        } else if packet_loss > 0.02 {
            // Moderate packet loss, reduce bitrate
            (self.current_bitrate as f32 * 0.85) as u32
        } else if rtt_ms > 200 {
            // High latency, reduce bitrate
            (self.current_bitrate as f32 * 0.9) as u32
        } else if bandwidth_bps > self.current_bitrate * 2 && packet_loss < 0.01 {
            // Network has headroom, increase bitrate gradually
            (self.current_bitrate as f32 * 1.1) as u32
        } else {
            // Maintain current bitrate
            self.current_bitrate
        };
        
        // Clamp to bounds
        self.current_bitrate = new_bitrate.clamp(self.min_bitrate, self.max_bitrate);
        self.current_bitrate
    }

    /// Get current bitrate
    pub fn current_bitrate(&self) -> u32 {
        self.current_bitrate
    }

    /// Set target bitrate
    pub fn set_target_bitrate(&mut self, bitrate: u32) {
        self.target_bitrate = bitrate.clamp(self.min_bitrate, self.max_bitrate);
        self.current_bitrate = self.target_bitrate;
    }

    /// Get average packet loss over recent history
    pub fn average_packet_loss(&self) -> f32 {
        if self.bitrate_history.is_empty() {
            return 0.0;
        }
        
        let sum: f32 = self.bitrate_history.iter().map(|p| p.packet_loss).sum();
        sum / self.bitrate_history.len() as f32
    }
}

/// Adaptive quality manager combining quality scaling and bitrate control
///
/// Requirements: 4.1, 4.2, 7.1, 7.2
pub struct AdaptiveQualityManager {
    quality_scaler: QualityScaler,
    bitrate_controller: BitrateController,
    last_adjustment: Instant,
    adjustment_interval: Duration,
}

impl AdaptiveQualityManager {
    /// Create a new adaptive quality manager
    pub fn new(initial_quality: StreamQuality) -> Self {
        Self {
            quality_scaler: QualityScaler::new(),
            bitrate_controller: BitrateController::new(initial_quality.bitrate),
            last_adjustment: Instant::now(),
            adjustment_interval: Duration::from_secs(2),
        }
    }

    /// Update quality based on network and system conditions
    pub async fn update_quality(
        &mut self,
        current_quality: EncodingQuality,
        bandwidth_bps: u32,
        latency_ms: u32,
        packet_loss: f32,
        cpu_usage: f32,
    ) -> StreamResult<EncodingQuality> {
        // Rate limit adjustments
        if self.last_adjustment.elapsed() < self.adjustment_interval {
            return Ok(current_quality);
        }
        
        // Adjust bitrate based on network feedback
        let new_bitrate = self.bitrate_controller.adjust_bitrate(
            bandwidth_bps,
            packet_loss,
            latency_ms,
        );
        
        // Adjust quality based on network conditions
        let mut quality = self.quality_scaler.adjust_for_network(
            current_quality,
            bandwidth_bps,
            latency_ms,
        ).await?;
        
        // Further adjust based on CPU usage
        quality = self.quality_scaler.adjust_for_cpu(quality, cpu_usage).await?;
        
        // Apply bitrate from controller
        quality.bitrate = new_bitrate;
        
        self.last_adjustment = Instant::now();
        Ok(quality)
    }

    /// Get current quality preset
    pub fn current_preset(&self) -> QualityPreset {
        self.quality_scaler.current_preset()
    }

    /// Set quality preset manually
    pub fn set_preset(&mut self, preset: QualityPreset) {
        self.quality_scaler.set_preset(preset);
        let stream_quality = preset.to_quality();
        self.bitrate_controller.set_target_bitrate(stream_quality.bitrate);
    }

    /// Get current bitrate
    pub fn current_bitrate(&self) -> u32 {
        self.bitrate_controller.current_bitrate()
    }

    /// Get average packet loss
    pub fn average_packet_loss(&self) -> f32 {
        self.bitrate_controller.average_packet_loss()
    }

    /// Convert to stream quality
    pub fn to_stream_quality(&self) -> StreamQuality {
        let preset = self.quality_scaler.current_preset();
        let mut quality = preset.to_quality();
        quality.bitrate = self.bitrate_controller.current_bitrate();
        quality
    }
}
