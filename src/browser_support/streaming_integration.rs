//! Streaming Integration for Browser Support
//!
//! Integrates browser video viewing with the existing streaming system,
//! enabling browser clients to view camera and screen streams from native peers.

#![cfg(feature = "streaming")]

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use crate::browser_support::{BrowserResult, BrowserSupportError, BrowserSession, WebRTCConnection};
use crate::streaming::{
    Streaming, StreamSession, StreamConfig, StreamQuality, SessionId,
    ViewerId, ViewerPermissions, PeerId,
};

/// Browser streaming integration
pub struct BrowserStreamingIntegration {
    /// Core streaming system
    streaming_system: Arc<dyn Streaming>,
    /// Active browser viewers by session
    browser_viewers: Arc<RwLock<HashMap<ViewerId, BrowserViewer>>>,
    /// WebRTC connections for streaming
    webrtc_connections: Arc<RwLock<HashMap<String, Arc<WebRTCConnection>>>>,
}

/// Browser viewer information
#[derive(Debug, Clone)]
pub struct BrowserViewer {
    /// Viewer ID
    pub viewer_id: ViewerId,
    /// Browser session ID
    pub browser_session_id: String,
    /// Stream session ID
    pub stream_session_id: SessionId,
    /// Peer ID providing the stream
    pub peer_id: PeerId,
    /// Current stream quality
    pub quality: StreamQuality,
    /// Viewer permissions
    pub permissions: ViewerPermissions,
    /// Connection quality metrics
    pub connection_quality: ConnectionQuality,
}

/// Connection quality metrics for adaptive streaming
#[derive(Debug, Clone)]
pub struct ConnectionQuality {
    /// Bandwidth estimate in bits per second
    pub bandwidth_bps: u64,
    /// Round-trip time in milliseconds
    pub rtt_ms: u32,
    /// Packet loss percentage (0-100)
    pub packet_loss_percent: f32,
    /// Jitter in milliseconds
    pub jitter_ms: u32,
}

impl Default for ConnectionQuality {
    fn default() -> Self {
        Self {
            bandwidth_bps: 1_000_000, // 1 Mbps default
            rtt_ms: 50,
            packet_loss_percent: 0.0,
            jitter_ms: 10,
        }
    }
}

impl BrowserStreamingIntegration {
    /// Create a new browser streaming integration
    pub fn new(
        streaming_system: Arc<dyn Streaming>,
        webrtc_connections: Arc<RwLock<HashMap<String, Arc<WebRTCConnection>>>>,
    ) -> Self {
        Self {
            streaming_system,
            browser_viewers: Arc::new(RwLock::new(HashMap::new())),
            webrtc_connections,
        }
    }

    /// Add browser as viewer to a stream
    pub async fn add_browser_viewer(
        &self,
        browser_session: &BrowserSession,
        stream_session_id: SessionId,
        peer_id: PeerId,
        permissions: ViewerPermissions,
    ) -> BrowserResult<ViewerId> {
        // Add viewer through streaming system
        let viewer_id = self.streaming_system
            .add_viewer(stream_session_id, peer_id.clone(), permissions.clone())
            .await
            .map_err(|e| BrowserSupportError::integration("streaming", format!("Failed to add viewer: {}", e)))?;

        // Create browser viewer record
        let browser_viewer = BrowserViewer {
            viewer_id,
            browser_session_id: browser_session.session_id.clone(),
            stream_session_id,
            peer_id,
            quality: StreamQuality::Auto,
            permissions,
            connection_quality: ConnectionQuality::default(),
        };

        // Store browser viewer
        {
            let mut viewers = self.browser_viewers.write().await;
            viewers.insert(viewer_id, browser_viewer);
        }

        Ok(viewer_id)
    }

    /// Remove browser viewer from stream
    pub async fn remove_browser_viewer(&self, viewer_id: ViewerId) -> BrowserResult<()> {
        // Get viewer info
        let viewer = {
            let viewers = self.browser_viewers.read().await;
            viewers.get(&viewer_id).cloned()
        };

        if let Some(viewer) = viewer {
            // Remove from streaming system
            self.streaming_system
                .remove_viewer(viewer.stream_session_id, viewer_id)
                .await
                .map_err(|e| BrowserSupportError::integration("streaming", format!("Failed to remove viewer: {}", e)))?;

            // Remove from browser viewers
            let mut viewers = self.browser_viewers.write().await;
            viewers.remove(&viewer_id);

            Ok(())
        } else {
            Err(BrowserSupportError::not_found("Viewer not found"))
        }
    }

    /// Adjust stream quality for browser viewer
    pub async fn adjust_browser_quality(
        &self,
        viewer_id: ViewerId,
        quality: StreamQuality,
    ) -> BrowserResult<()> {
        // Get viewer info
        let viewer = {
            let viewers = self.browser_viewers.read().await;
            viewers.get(&viewer_id).cloned()
        };

        if let Some(viewer) = viewer {
            // Adjust quality through streaming system
            self.streaming_system
                .adjust_quality(viewer.stream_session_id, quality.clone())
                .await
                .map_err(|e| BrowserSupportError::integration("streaming", format!("Failed to adjust quality: {}", e)))?;

            // Update browser viewer record
            let mut viewers = self.browser_viewers.write().await;
            if let Some(v) = viewers.get_mut(&viewer_id) {
                v.quality = quality;
            }

            Ok(())
        } else {
            Err(BrowserSupportError::not_found("Viewer not found"))
        }
    }

    /// Update connection quality metrics for adaptive streaming
    pub async fn update_connection_quality(
        &self,
        viewer_id: ViewerId,
        quality: ConnectionQuality,
    ) -> BrowserResult<()> {
        let mut viewers = self.browser_viewers.write().await;
        
        if let Some(viewer) = viewers.get_mut(&viewer_id) {
            viewer.connection_quality = quality.clone();

            // Automatically adjust stream quality based on connection
            let recommended_quality = self.recommend_quality(&quality);
            if recommended_quality != viewer.quality {
                drop(viewers); // Release lock before async call
                self.adjust_browser_quality(viewer_id, recommended_quality).await?;
            }

            Ok(())
        } else {
            Err(BrowserSupportError::not_found("Viewer not found"))
        }
    }

    /// Recommend stream quality based on connection metrics
    fn recommend_quality(&self, quality: &ConnectionQuality) -> StreamQuality {
        // Simple quality recommendation based on bandwidth and packet loss
        if quality.packet_loss_percent > 5.0 || quality.bandwidth_bps < 500_000 {
            StreamQuality::Low
        } else if quality.packet_loss_percent > 2.0 || quality.bandwidth_bps < 1_500_000 {
            StreamQuality::Medium
        } else if quality.bandwidth_bps < 3_000_000 {
            StreamQuality::High
        } else {
            StreamQuality::Ultra
        }
    }

    /// Get browser viewer information
    pub async fn get_browser_viewer(&self, viewer_id: ViewerId) -> Option<BrowserViewer> {
        let viewers = self.browser_viewers.read().await;
        viewers.get(&viewer_id).cloned()
    }

    /// Get all browser viewers for a session
    pub async fn get_session_viewers(&self, browser_session_id: &str) -> Vec<BrowserViewer> {
        let viewers = self.browser_viewers.read().await;
        viewers.values()
            .filter(|v| v.browser_session_id == browser_session_id)
            .cloned()
            .collect()
    }

    /// Get all active browser viewers
    pub async fn get_all_browser_viewers(&self) -> Vec<BrowserViewer> {
        let viewers = self.browser_viewers.read().await;
        viewers.values().cloned().collect()
    }

    /// Handle WebRTC video track for browser viewer
    pub async fn handle_video_track(
        &self,
        viewer_id: ViewerId,
        track_data: Vec<u8>,
    ) -> BrowserResult<()> {
        // Verify viewer exists
        let viewers = self.browser_viewers.read().await;
        if !viewers.contains_key(&viewer_id) {
            return Err(BrowserSupportError::not_found("Viewer not found"));
        }

        // In a real implementation, this would:
        // 1. Decode video frame
        // 2. Apply any necessary transformations
        // 3. Send through WebRTC data channel or media track
        // 4. Update statistics

        Ok(())
    }

    /// Request keyframe for browser viewer (for error recovery)
    pub async fn request_keyframe(&self, viewer_id: ViewerId) -> BrowserResult<()> {
        let viewer = {
            let viewers = self.browser_viewers.read().await;
            viewers.get(&viewer_id).cloned()
        };

        if let Some(viewer) = viewer {
            // Request keyframe through streaming system
            self.streaming_system
                .request_keyframe(viewer.stream_session_id)
                .await
                .map_err(|e| BrowserSupportError::integration("streaming", format!("Failed to request keyframe: {}", e)))?;

            Ok(())
        } else {
            Err(BrowserSupportError::not_found("Viewer not found"))
        }
    }

    /// Get stream statistics for browser viewer
    pub async fn get_viewer_stats(&self, viewer_id: ViewerId) -> BrowserResult<ViewerStats> {
        let viewer = {
            let viewers = self.browser_viewers.read().await;
            viewers.get(&viewer_id).cloned()
        };

        if let Some(viewer) = viewer {
            // Get stats from streaming system
            let stream_stats = self.streaming_system
                .get_stream_stats(viewer.stream_session_id)
                .await
                .map_err(|e| BrowserSupportError::integration("streaming", format!("Failed to get stats: {}", e)))?;

            Ok(ViewerStats {
                viewer_id,
                stream_session_id: viewer.stream_session_id,
                current_quality: viewer.quality,
                connection_quality: viewer.connection_quality,
                frames_received: stream_stats.frames_sent, // Approximate
                frames_dropped: 0, // Would be tracked separately
                bitrate_kbps: stream_stats.bitrate_kbps,
            })
        } else {
            Err(BrowserSupportError::not_found("Viewer not found"))
        }
    }

    /// Clean up viewers for disconnected browser sessions
    pub async fn cleanup_disconnected_viewers(&self, active_sessions: &[String]) -> usize {
        let mut viewers = self.browser_viewers.write().await;
        let initial_count = viewers.len();

        // Collect viewer IDs to remove
        let to_remove: Vec<ViewerId> = viewers
            .iter()
            .filter(|(_, v)| !active_sessions.contains(&v.browser_session_id))
            .map(|(id, _)| *id)
            .collect();

        // Remove viewers
        for viewer_id in to_remove {
            if let Some(viewer) = viewers.remove(&viewer_id) {
                // Attempt to remove from streaming system (ignore errors)
                let _ = self.streaming_system
                    .remove_viewer(viewer.stream_session_id, viewer_id)
                    .await;
            }
        }

        initial_count - viewers.len()
    }
}

/// Viewer statistics for browser clients
#[derive(Debug, Clone)]
pub struct ViewerStats {
    pub viewer_id: ViewerId,
    pub stream_session_id: SessionId,
    pub current_quality: StreamQuality,
    pub connection_quality: ConnectionQuality,
    pub frames_received: u64,
    pub frames_dropped: u64,
    pub bitrate_kbps: u32,
}

/// Trait for browser streaming operations
#[async_trait]
pub trait BrowserStreaming: Send + Sync {
    /// Add browser as viewer
    async fn add_viewer(
        &self,
        browser_session: &BrowserSession,
        stream_session_id: SessionId,
        peer_id: PeerId,
        permissions: ViewerPermissions,
    ) -> BrowserResult<ViewerId>;

    /// Remove browser viewer
    async fn remove_viewer(&self, viewer_id: ViewerId) -> BrowserResult<()>;

    /// Adjust stream quality
    async fn adjust_quality(&self, viewer_id: ViewerId, quality: StreamQuality) -> BrowserResult<()>;

    /// Update connection quality
    async fn update_connection_quality(
        &self,
        viewer_id: ViewerId,
        quality: ConnectionQuality,
    ) -> BrowserResult<()>;

    /// Get viewer statistics
    async fn get_stats(&self, viewer_id: ViewerId) -> BrowserResult<ViewerStats>;
}

#[async_trait]
impl BrowserStreaming for BrowserStreamingIntegration {
    async fn add_viewer(
        &self,
        browser_session: &BrowserSession,
        stream_session_id: SessionId,
        peer_id: PeerId,
        permissions: ViewerPermissions,
    ) -> BrowserResult<ViewerId> {
        self.add_browser_viewer(browser_session, stream_session_id, peer_id, permissions).await
    }

    async fn remove_viewer(&self, viewer_id: ViewerId) -> BrowserResult<()> {
        self.remove_browser_viewer(viewer_id).await
    }

    async fn adjust_quality(&self, viewer_id: ViewerId, quality: StreamQuality) -> BrowserResult<()> {
        self.adjust_browser_quality(viewer_id, quality).await
    }

    async fn update_connection_quality(
        &self,
        viewer_id: ViewerId,
        quality: ConnectionQuality,
    ) -> BrowserResult<()> {
        self.update_connection_quality(viewer_id, quality).await
    }

    async fn get_stats(&self, viewer_id: ViewerId) -> BrowserResult<ViewerStats> {
        self.get_viewer_stats(viewer_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_quality_default() {
        let quality = ConnectionQuality::default();
        assert_eq!(quality.bandwidth_bps, 1_000_000);
        assert_eq!(quality.rtt_ms, 50);
        assert_eq!(quality.packet_loss_percent, 0.0);
        assert_eq!(quality.jitter_ms, 10);
    }

    #[test]
    fn test_connection_quality_creation() {
        let quality = ConnectionQuality {
            bandwidth_bps: 5_000_000,
            rtt_ms: 30,
            packet_loss_percent: 1.5,
            jitter_ms: 5,
        };

        assert_eq!(quality.bandwidth_bps, 5_000_000);
        assert_eq!(quality.rtt_ms, 30);
        assert_eq!(quality.packet_loss_percent, 1.5);
        assert_eq!(quality.jitter_ms, 5);
    }
}
