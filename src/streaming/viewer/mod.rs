// Multi-viewer broadcasting and management module
//
// Manages multiple viewers, viewer permissions, and efficient broadcasting
// to multiple peers simultaneously.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::streaming::{
    ConnectionQuality, PeerId, StreamError, StreamQuality, StreamResult, ViewerId,
    ViewerPermissions, ViewerStatus, VideoStream,
};

/// Maximum number of concurrent viewers supported
const MAX_VIEWERS: usize = 10;

/// Viewer manager implementation
/// 
/// Manages multiple viewers for broadcasting scenarios, including viewer
/// authentication, connection management, and resource allocation.
/// 
/// Requirements: 6.1, 6.3, 8.3
pub struct ViewerManagerImpl {
    registry: Arc<ViewerRegistry>,
    broadcast_controller: Arc<BroadcastController>,
}

impl ViewerManagerImpl {
    /// Create a new viewer manager
    pub fn new() -> Self {
        Self {
            registry: Arc::new(ViewerRegistry::new()),
            broadcast_controller: Arc::new(BroadcastController::new()),
        }
    }

    /// Get the viewer registry
    pub fn registry(&self) -> Arc<ViewerRegistry> {
        Arc::clone(&self.registry)
    }

    /// Get the broadcast controller
    pub fn broadcast_controller(&self) -> Arc<BroadcastController> {
        Arc::clone(&self.broadcast_controller)
    }
}

#[async_trait]
impl crate::streaming::ViewerManager for ViewerManagerImpl {
    async fn add_viewer(
        &self,
        peer_id: PeerId,
        permissions: ViewerPermissions,
    ) -> StreamResult<ViewerId> {
        self.registry.add_viewer(peer_id, permissions).await
    }

    async fn remove_viewer(&self, viewer_id: ViewerId) -> StreamResult<()> {
        self.registry.remove_viewer(viewer_id).await
    }

    async fn broadcast_to_viewers(&self, stream: VideoStream) -> StreamResult<()> {
        self.broadcast_controller.broadcast_to_viewers(stream, &self.registry).await
    }

    async fn get_viewer_status(&self) -> StreamResult<Vec<ViewerStatus>> {
        self.registry.get_all_viewer_status().await
    }

    async fn approve_viewer_request(&self, peer_id: PeerId) -> StreamResult<ViewerId> {
        self.registry.approve_viewer_request(peer_id).await
    }
}

impl Default for ViewerManagerImpl {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal viewer information
/// 
/// Requirements: 6.1, 6.4
#[derive(Debug, Clone)]
struct ViewerInfo {
    viewer_id: ViewerId,
    peer_id: PeerId,
    device_name: String,
    permissions: ViewerPermissions,
    connection_quality: ConnectionQuality,
    connected_at: SystemTime,
    bytes_sent: u64,
    current_quality: StreamQuality,
    state: ViewerState,
}

impl ViewerInfo {
    /// Create new viewer info
    fn new(peer_id: PeerId, permissions: ViewerPermissions) -> Self {
        Self {
            viewer_id: Uuid::new_v4(),
            peer_id: peer_id.clone(),
            device_name: format!("Device-{}", &peer_id[..8]),
            permissions,
            connection_quality: ConnectionQuality::Good,
            connected_at: SystemTime::now(),
            bytes_sent: 0,
            current_quality: StreamQuality::default(),
            state: ViewerState::Connected,
        }
    }

    /// Convert to ViewerStatus
    fn to_status(&self) -> ViewerStatus {
        ViewerStatus {
            viewer_id: self.viewer_id,
            peer_id: self.peer_id.clone(),
            device_name: self.device_name.clone(),
            connection_quality: self.connection_quality,
            permissions: self.permissions.clone(),
            connected_at: self.connected_at,
            bytes_sent: self.bytes_sent,
            current_quality: self.current_quality.clone(),
        }
    }

    /// Update connection quality based on network metrics
    fn update_connection_quality(&mut self, latency_ms: u32, packet_loss_rate: f32) {
        self.connection_quality = if packet_loss_rate > 0.1 || latency_ms > 500 {
            ConnectionQuality::Poor
        } else if packet_loss_rate > 0.05 || latency_ms > 300 {
            ConnectionQuality::Fair
        } else if packet_loss_rate > 0.01 || latency_ms > 150 {
            ConnectionQuality::Good
        } else {
            ConnectionQuality::Excellent
        };
    }

    /// Update bytes sent
    fn add_bytes_sent(&mut self, bytes: u64) {
        self.bytes_sent = self.bytes_sent.saturating_add(bytes);
    }

    /// Update current quality
    fn set_quality(&mut self, quality: StreamQuality) {
        self.current_quality = quality;
    }
}

/// Viewer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViewerState {
    PendingApproval,
    Connected,
    Disconnected,
}

/// Viewer registry for tracking connected viewers
/// 
/// Manages viewer registration, authentication, and connection tracking.
/// 
/// Requirements: 6.1, 6.4, 8.3, 8.4
pub struct ViewerRegistry {
    viewers: Arc<RwLock<HashMap<ViewerId, ViewerInfo>>>,
    pending_requests: Arc<RwLock<HashMap<PeerId, ViewerPermissions>>>,
}

impl ViewerRegistry {
    /// Create a new viewer registry
    pub fn new() -> Self {
        Self {
            viewers: Arc::new(RwLock::new(HashMap::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a viewer with authentication and permissions
    /// 
    /// Requirements: 6.1, 6.4, 8.3, 8.4
    pub async fn add_viewer(
        &self,
        peer_id: PeerId,
        permissions: ViewerPermissions,
    ) -> StreamResult<ViewerId> {
        let mut viewers = self.viewers.write().await;

        // Check if viewer limit reached
        if viewers.len() >= MAX_VIEWERS {
            return Err(StreamError::viewer(format!(
                "Maximum viewer limit ({}) reached",
                MAX_VIEWERS
            )));
        }

        // Check if peer is already connected
        if viewers.values().any(|v| v.peer_id == peer_id) {
            return Err(StreamError::viewer(format!(
                "Peer {} is already connected as a viewer",
                peer_id
            )));
        }

        // Validate permissions
        if !permissions.can_view {
            return Err(StreamError::permission(
                "Viewer must have view permission enabled",
            ));
        }

        // Create viewer info
        let viewer_info = ViewerInfo::new(peer_id.clone(), permissions);
        let viewer_id = viewer_info.viewer_id;

        // Add to registry
        viewers.insert(viewer_id, viewer_info);

        // Remove from pending requests if present
        let mut pending = self.pending_requests.write().await;
        pending.remove(&peer_id);

        Ok(viewer_id)
    }

    /// Remove a viewer from the registry
    /// 
    /// Requirements: 6.3, 6.4
    pub async fn remove_viewer(&self, viewer_id: ViewerId) -> StreamResult<()> {
        let mut viewers = self.viewers.write().await;

        if viewers.remove(&viewer_id).is_none() {
            return Err(StreamError::viewer(format!(
                "Viewer {} not found",
                viewer_id
            )));
        }

        Ok(())
    }

    /// Get viewer information
    pub async fn get_viewer(&self, viewer_id: ViewerId) -> StreamResult<ViewerInfo> {
        let viewers = self.viewers.read().await;
        viewers
            .get(&viewer_id)
            .cloned()
            .ok_or_else(|| StreamError::viewer(format!("Viewer {} not found", viewer_id)))
    }

    /// Get all viewer status information
    /// 
    /// Requirements: 6.3, 8.5
    pub async fn get_all_viewer_status(&self) -> StreamResult<Vec<ViewerStatus>> {
        let viewers = self.viewers.read().await;
        Ok(viewers.values().map(|v| v.to_status()).collect())
    }

    /// Get count of connected viewers
    pub async fn viewer_count(&self) -> usize {
        let viewers = self.viewers.read().await;
        viewers.len()
    }

    /// Check if a viewer exists
    pub async fn has_viewer(&self, viewer_id: ViewerId) -> bool {
        let viewers = self.viewers.read().await;
        viewers.contains_key(&viewer_id)
    }

    /// Update viewer connection quality
    /// 
    /// Requirements: 6.3, 8.5
    pub async fn update_connection_quality(
        &self,
        viewer_id: ViewerId,
        latency_ms: u32,
        packet_loss_rate: f32,
    ) -> StreamResult<()> {
        let mut viewers = self.viewers.write().await;
        let viewer = viewers
            .get_mut(&viewer_id)
            .ok_or_else(|| StreamError::viewer(format!("Viewer {} not found", viewer_id)))?;

        viewer.update_connection_quality(latency_ms, packet_loss_rate);
        Ok(())
    }

    /// Update bytes sent to viewer
    pub async fn add_bytes_sent(&self, viewer_id: ViewerId, bytes: u64) -> StreamResult<()> {
        let mut viewers = self.viewers.write().await;
        let viewer = viewers
            .get_mut(&viewer_id)
            .ok_or_else(|| StreamError::viewer(format!("Viewer {} not found", viewer_id)))?;

        viewer.add_bytes_sent(bytes);
        Ok(())
    }

    /// Update viewer quality
    pub async fn set_viewer_quality(
        &self,
        viewer_id: ViewerId,
        quality: StreamQuality,
    ) -> StreamResult<()> {
        let mut viewers = self.viewers.write().await;
        let viewer = viewers
            .get_mut(&viewer_id)
            .ok_or_else(|| StreamError::viewer(format!("Viewer {} not found", viewer_id)))?;

        viewer.set_quality(quality);
        Ok(())
    }

    /// Request viewer approval (add to pending requests)
    /// 
    /// Requirements: 6.4, 8.3, 8.4
    pub async fn request_viewer_access(
        &self,
        peer_id: PeerId,
        permissions: ViewerPermissions,
    ) -> StreamResult<()> {
        let mut pending = self.pending_requests.write().await;

        // Check if already pending
        if pending.contains_key(&peer_id) {
            return Err(StreamError::viewer(format!(
                "Viewer request from {} is already pending",
                peer_id
            )));
        }

        // Check if already connected
        let viewers = self.viewers.read().await;
        if viewers.values().any(|v| v.peer_id == peer_id) {
            return Err(StreamError::viewer(format!(
                "Peer {} is already connected",
                peer_id
            )));
        }

        pending.insert(peer_id, permissions);
        Ok(())
    }

    /// Approve a viewer request
    /// 
    /// Requirements: 6.4, 8.3, 8.4
    pub async fn approve_viewer_request(&self, peer_id: PeerId) -> StreamResult<ViewerId> {
        let pending = self.pending_requests.read().await;
        let permissions = pending
            .get(&peer_id)
            .cloned()
            .ok_or_else(|| StreamError::viewer(format!("No pending request from {}", peer_id)))?;

        drop(pending);

        // Add viewer with the requested permissions
        self.add_viewer(peer_id, permissions).await
    }

    /// Reject a viewer request
    /// 
    /// Requirements: 6.4, 8.3, 8.4
    pub async fn reject_viewer_request(&self, peer_id: PeerId) -> StreamResult<()> {
        let mut pending = self.pending_requests.write().await;

        if pending.remove(&peer_id).is_none() {
            return Err(StreamError::viewer(format!(
                "No pending request from {}",
                peer_id
            )));
        }

        Ok(())
    }

    /// Get all pending viewer requests
    pub async fn get_pending_requests(&self) -> StreamResult<Vec<(PeerId, ViewerPermissions)>> {
        let pending = self.pending_requests.read().await;
        Ok(pending
            .iter()
            .map(|(peer_id, perms)| (peer_id.clone(), perms.clone()))
            .collect())
    }

    /// Get all viewer IDs
    pub async fn get_viewer_ids(&self) -> Vec<ViewerId> {
        let viewers = self.viewers.read().await;
        viewers.keys().copied().collect()
    }

    /// Check if viewer has permission
    pub async fn check_permission(
        &self,
        viewer_id: ViewerId,
        check: impl Fn(&ViewerPermissions) -> bool,
    ) -> StreamResult<bool> {
        let viewers = self.viewers.read().await;
        let viewer = viewers
            .get(&viewer_id)
            .ok_or_else(|| StreamError::viewer(format!("Viewer {} not found", viewer_id)))?;

        Ok(check(&viewer.permissions))
    }
}

impl Default for ViewerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Broadcast controller for multi-viewer streaming
/// 
/// Optimizes encoding and bandwidth allocation across multiple viewers,
/// supporting simultaneous streaming to up to 10 viewers with viewer-specific
/// quality adaptation.
/// 
/// Requirements: 6.1, 6.2, 6.5
pub struct BroadcastController {
    active_broadcasts: Arc<RwLock<HashMap<Uuid, BroadcastSession>>>,
}

impl BroadcastController {
    pub fn new() -> Self {
        Self {
            active_broadcasts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Broadcast stream to all viewers
    /// 
    /// Efficiently streams to multiple viewers with optimized encoding
    /// and bandwidth allocation.
    /// 
    /// Requirements: 6.1, 6.2, 6.5
    pub async fn broadcast_to_viewers(
        &self,
        stream: VideoStream,
        registry: &ViewerRegistry,
    ) -> StreamResult<()> {
        let viewer_ids = registry.get_viewer_ids().await;

        if viewer_ids.is_empty() {
            return Err(StreamError::viewer("No viewers connected"));
        }

        // Check viewer limit
        if viewer_ids.len() > MAX_VIEWERS {
            return Err(StreamError::viewer(format!(
                "Too many viewers: {} (max: {})",
                viewer_ids.len(),
                MAX_VIEWERS
            )));
        }

        // Create or get broadcast session
        let session_id = stream.id;
        let mut broadcasts = self.active_broadcasts.write().await;
        
        let session = broadcasts
            .entry(session_id)
            .or_insert_with(|| BroadcastSession::new(session_id, stream.quality.clone()));

        // Update viewer list
        session.update_viewers(viewer_ids.clone());

        // Calculate optimal encoding settings for all viewers
        let optimal_quality = self.calculate_optimal_quality(registry, &viewer_ids).await?;

        // Update session quality
        session.set_quality(optimal_quality.clone());

        // Simulate broadcasting to each viewer
        // In a real implementation, this would use the network streamer
        for viewer_id in viewer_ids {
            let viewer_quality = self.get_viewer_specific_quality(registry, viewer_id, &optimal_quality).await?;
            
            // Update viewer quality in registry
            registry.set_viewer_quality(viewer_id, viewer_quality.clone()).await?;

            // Track bytes sent (simulated)
            let frame_size = self.estimate_frame_size(&viewer_quality);
            registry.add_bytes_sent(viewer_id, frame_size).await?;

            session.increment_frames_sent();
        }

        Ok(())
    }

    /// Calculate optimal quality for all viewers
    /// 
    /// Determines the best quality settings that balance bandwidth usage
    /// across all connected viewers.
    /// 
    /// Requirements: 6.2, 6.5
    async fn calculate_optimal_quality(
        &self,
        registry: &ViewerRegistry,
        viewer_ids: &[ViewerId],
    ) -> StreamResult<StreamQuality> {
        let mut min_quality = StreamQuality::default();
        let mut total_bandwidth = 0u32;

        // Get all viewer statuses
        for viewer_id in viewer_ids {
            let viewer = registry.get_viewer(*viewer_id).await?;

            // Adjust quality based on connection quality
            let quality_factor = match viewer.connection_quality {
                ConnectionQuality::Excellent => 1.0,
                ConnectionQuality::Good => 0.8,
                ConnectionQuality::Fair => 0.6,
                ConnectionQuality::Poor => 0.4,
                ConnectionQuality::Disconnected => 0.0,
            };

            // Calculate viewer's maximum quality
            let viewer_max_quality = viewer.permissions.max_quality.to_quality();
            let adjusted_bitrate = (viewer_max_quality.bitrate as f32 * quality_factor) as u32;

            total_bandwidth += adjusted_bitrate;

            // Use the lowest quality among all viewers as baseline
            if adjusted_bitrate < min_quality.bitrate {
                min_quality = viewer_max_quality;
                min_quality.bitrate = adjusted_bitrate;
            }
        }

        // Calculate average bandwidth per viewer
        let avg_bandwidth = if !viewer_ids.is_empty() {
            total_bandwidth / viewer_ids.len() as u32
        } else {
            min_quality.bitrate
        };

        // Set optimal quality based on average bandwidth
        min_quality.bitrate = avg_bandwidth;

        Ok(min_quality)
    }

    /// Get viewer-specific quality adaptation
    /// 
    /// Adapts quality for individual viewers based on their connection
    /// quality and permissions.
    /// 
    /// Requirements: 6.2, 6.5
    async fn get_viewer_specific_quality(
        &self,
        registry: &ViewerRegistry,
        viewer_id: ViewerId,
        base_quality: &StreamQuality,
    ) -> StreamResult<StreamQuality> {
        let viewer = registry.get_viewer(viewer_id).await?;

        // Start with base quality
        let mut quality = base_quality.clone();

        // Apply viewer permission limits
        let max_quality = viewer.permissions.max_quality.to_quality();
        if quality.bitrate > max_quality.bitrate {
            quality.bitrate = max_quality.bitrate;
        }
        if quality.resolution.width > max_quality.resolution.width {
            quality.resolution = max_quality.resolution;
        }
        if quality.framerate > max_quality.framerate {
            quality.framerate = max_quality.framerate;
        }

        // Adjust based on connection quality
        let quality_multiplier = match viewer.connection_quality {
            ConnectionQuality::Excellent => 1.0,
            ConnectionQuality::Good => 0.85,
            ConnectionQuality::Fair => 0.65,
            ConnectionQuality::Poor => 0.45,
            ConnectionQuality::Disconnected => 0.0,
        };

        quality.bitrate = (quality.bitrate as f32 * quality_multiplier) as u32;
        
        // Reduce resolution for poor connections
        if viewer.connection_quality == ConnectionQuality::Poor {
            quality.resolution.width = (quality.resolution.width as f32 * 0.75) as u32;
            quality.resolution.height = (quality.resolution.height as f32 * 0.75) as u32;
        }

        Ok(quality)
    }

    /// Estimate frame size based on quality settings
    fn estimate_frame_size(&self, quality: &StreamQuality) -> u64 {
        // Rough estimate: bitrate / framerate
        (quality.bitrate / quality.framerate / 8) as u64
    }

    /// Optimize encoding for multiple viewers
    /// 
    /// Adjusts encoding parameters to efficiently serve multiple viewers
    /// with different quality requirements.
    /// 
    /// Requirements: 6.2, 6.5
    pub async fn optimize_encoding(&self) -> StreamResult<()> {
        let broadcasts = self.active_broadcasts.read().await;

        for session in broadcasts.values() {
            // Calculate encoding efficiency
            let viewer_count = session.viewer_count();
            
            if viewer_count == 0 {
                continue;
            }

            // Optimize GOP size based on viewer count
            // More viewers = larger GOP for better compression
            let optimal_gop = (viewer_count * 15).min(120);

            // Optimize thread count based on viewer count
            let optimal_threads = (viewer_count / 2).max(1).min(8);

            println!(
                "Optimized encoding for session {}: {} viewers, GOP={}, threads={}",
                session.session_id, viewer_count, optimal_gop, optimal_threads
            );
        }

        Ok(())
    }

    /// Get broadcast statistics
    pub async fn get_broadcast_stats(&self, session_id: Uuid) -> StreamResult<BroadcastStats> {
        let broadcasts = self.active_broadcasts.read().await;
        let session = broadcasts
            .get(&session_id)
            .ok_or_else(|| StreamError::session_not_found(session_id))?;

        Ok(BroadcastStats {
            session_id,
            viewer_count: session.viewer_count(),
            total_frames_sent: session.total_frames_sent,
            current_quality: session.current_quality.clone(),
            started_at: session.started_at,
        })
    }

    /// Stop a broadcast session
    pub async fn stop_broadcast(&self, session_id: Uuid) -> StreamResult<()> {
        let mut broadcasts = self.active_broadcasts.write().await;
        
        if broadcasts.remove(&session_id).is_none() {
            return Err(StreamError::session_not_found(session_id));
        }

        Ok(())
    }

    /// Get all active broadcast sessions
    pub async fn get_active_sessions(&self) -> Vec<Uuid> {
        let broadcasts = self.active_broadcasts.read().await;
        broadcasts.keys().copied().collect()
    }
}

impl Default for BroadcastController {
    fn default() -> Self {
        Self::new()
    }
}

/// Broadcast session tracking
#[derive(Debug, Clone)]
struct BroadcastSession {
    session_id: Uuid,
    viewers: Vec<ViewerId>,
    current_quality: StreamQuality,
    total_frames_sent: u64,
    started_at: SystemTime,
}

impl BroadcastSession {
    fn new(session_id: Uuid, quality: StreamQuality) -> Self {
        Self {
            session_id,
            viewers: Vec::new(),
            current_quality: quality,
            total_frames_sent: 0,
            started_at: SystemTime::now(),
        }
    }

    fn update_viewers(&mut self, viewers: Vec<ViewerId>) {
        self.viewers = viewers;
    }

    fn viewer_count(&self) -> usize {
        self.viewers.len()
    }

    fn set_quality(&mut self, quality: StreamQuality) {
        self.current_quality = quality;
    }

    fn increment_frames_sent(&mut self) {
        self.total_frames_sent += 1;
    }
}

/// Broadcast statistics
#[derive(Debug, Clone)]
pub struct BroadcastStats {
    pub session_id: Uuid,
    pub viewer_count: usize,
    pub total_frames_sent: u64,
    pub current_quality: StreamQuality,
    pub started_at: SystemTime,
}

/// Viewer management controls
/// 
/// Provides comprehensive viewer management including connection handling,
/// permission management, and status monitoring.
/// 
/// Requirements: 6.3, 6.4, 8.5
pub struct ViewerManagementControls {
    registry: Arc<ViewerRegistry>,
}

impl ViewerManagementControls {
    /// Create new viewer management controls
    pub fn new(registry: Arc<ViewerRegistry>) -> Self {
        Self { registry }
    }

    /// Handle viewer connection
    /// 
    /// Processes a new viewer connection request with authentication
    /// and permission validation.
    /// 
    /// Requirements: 6.3, 8.3, 8.4
    pub async fn handle_viewer_connection(
        &self,
        peer_id: PeerId,
        permissions: ViewerPermissions,
        require_approval: bool,
    ) -> StreamResult<ViewerConnectionResult> {
        // Check if viewer limit reached
        let current_count = self.registry.viewer_count().await;
        if current_count >= MAX_VIEWERS {
            return Ok(ViewerConnectionResult::Rejected(format!(
                "Maximum viewer limit ({}) reached",
                MAX_VIEWERS
            )));
        }

        // Validate permissions
        if !permissions.can_view {
            return Ok(ViewerConnectionResult::Rejected(
                "View permission is required".to_string(),
            ));
        }

        // If approval required, add to pending requests
        if require_approval {
            self.registry
                .request_viewer_access(peer_id.clone(), permissions)
                .await?;
            return Ok(ViewerConnectionResult::PendingApproval);
        }

        // Otherwise, add viewer directly
        let viewer_id = self.registry.add_viewer(peer_id, permissions).await?;
        Ok(ViewerConnectionResult::Connected(viewer_id))
    }

    /// Handle viewer disconnection
    /// 
    /// Processes viewer disconnection and cleanup.
    /// 
    /// Requirements: 6.3
    pub async fn handle_viewer_disconnection(&self, viewer_id: ViewerId) -> StreamResult<()> {
        self.registry.remove_viewer(viewer_id).await?;
        println!("Viewer {} disconnected", viewer_id);
        Ok(())
    }

    /// Update viewer permissions
    /// 
    /// Modifies permissions for an existing viewer.
    /// 
    /// Requirements: 6.3, 6.4, 8.5
    pub async fn update_viewer_permissions(
        &self,
        viewer_id: ViewerId,
        permissions: ViewerPermissions,
    ) -> StreamResult<()> {
        // Update permissions directly in the registry
        let mut viewers = self.registry.viewers.write().await;
        let viewer = viewers
            .get_mut(&viewer_id)
            .ok_or_else(|| StreamError::viewer(format!("Viewer {} not found", viewer_id)))?;

        viewer.permissions = permissions;
        Ok(())
    }

    /// Grant recording permission to viewer
    /// 
    /// Requirements: 6.4, 8.5
    pub async fn grant_recording_permission(&self, viewer_id: ViewerId) -> StreamResult<()> {
        let viewer = self.registry.get_viewer(viewer_id).await?;
        let mut permissions = viewer.permissions;
        permissions.can_record = true;

        self.update_viewer_permissions(viewer_id, permissions).await
    }

    /// Revoke recording permission from viewer
    /// 
    /// Requirements: 6.4, 8.5
    pub async fn revoke_recording_permission(&self, viewer_id: ViewerId) -> StreamResult<()> {
        let viewer = self.registry.get_viewer(viewer_id).await?;
        let mut permissions = viewer.permissions;
        permissions.can_record = false;

        self.update_viewer_permissions(viewer_id, permissions).await
    }

    /// Grant quality control permission to viewer
    /// 
    /// Requirements: 6.4, 8.5
    pub async fn grant_quality_control(&self, viewer_id: ViewerId) -> StreamResult<()> {
        let viewer = self.registry.get_viewer(viewer_id).await?;
        let mut permissions = viewer.permissions;
        permissions.can_control_quality = true;

        self.update_viewer_permissions(viewer_id, permissions).await
    }

    /// Revoke quality control permission from viewer
    /// 
    /// Requirements: 6.4, 8.5
    pub async fn revoke_quality_control(&self, viewer_id: ViewerId) -> StreamResult<()> {
        let viewer = self.registry.get_viewer(viewer_id).await?;
        let mut permissions = viewer.permissions;
        permissions.can_control_quality = false;

        self.update_viewer_permissions(viewer_id, permissions).await
    }

    /// Get viewer status report
    /// 
    /// Provides detailed status information for a specific viewer.
    /// 
    /// Requirements: 6.3, 8.5
    pub async fn get_viewer_status_report(&self, viewer_id: ViewerId) -> StreamResult<ViewerStatusReport> {
        let viewer = self.registry.get_viewer(viewer_id).await?;
        let status = viewer.to_status();

        // Calculate connection duration
        let duration = SystemTime::now()
            .duration_since(viewer.connected_at)
            .unwrap_or_default();

        // Calculate average bitrate
        let avg_bitrate = if duration.as_secs() > 0 {
            (viewer.bytes_sent * 8) / duration.as_secs()
        } else {
            0
        };

        Ok(ViewerStatusReport {
            status,
            connection_duration: duration,
            average_bitrate: avg_bitrate,
            is_healthy: viewer.connection_quality != ConnectionQuality::Poor
                && viewer.connection_quality != ConnectionQuality::Disconnected,
        })
    }

    /// Get all viewer status reports
    /// 
    /// Requirements: 6.3, 8.5
    pub async fn get_all_status_reports(&self) -> StreamResult<Vec<ViewerStatusReport>> {
        let viewer_ids = self.registry.get_viewer_ids().await;
        let mut reports = Vec::new();

        for viewer_id in viewer_ids {
            if let Ok(report) = self.get_viewer_status_report(viewer_id).await {
                reports.push(report);
            }
        }

        Ok(reports)
    }

    /// Monitor viewer connection quality
    /// 
    /// Updates connection quality metrics for a viewer.
    /// 
    /// Requirements: 6.3, 8.5
    pub async fn monitor_connection_quality(
        &self,
        viewer_id: ViewerId,
        latency_ms: u32,
        packet_loss_rate: f32,
    ) -> StreamResult<ConnectionQuality> {
        self.registry
            .update_connection_quality(viewer_id, latency_ms, packet_loss_rate)
            .await?;

        let viewer = self.registry.get_viewer(viewer_id).await?;
        Ok(viewer.connection_quality)
    }

    /// Kick viewer (force disconnect)
    /// 
    /// Immediately disconnects a viewer.
    /// 
    /// Requirements: 6.3, 8.5
    pub async fn kick_viewer(&self, viewer_id: ViewerId, reason: String) -> StreamResult<()> {
        println!("Kicking viewer {}: {}", viewer_id, reason);
        self.registry.remove_viewer(viewer_id).await
    }

    /// Get viewers with poor connection
    /// 
    /// Returns list of viewers experiencing connection issues.
    /// 
    /// Requirements: 6.3, 8.5
    pub async fn get_poor_connection_viewers(&self) -> StreamResult<Vec<ViewerId>> {
        let statuses = self.registry.get_all_viewer_status().await?;
        
        Ok(statuses
            .iter()
            .filter(|s| {
                s.connection_quality == ConnectionQuality::Poor
                    || s.connection_quality == ConnectionQuality::Disconnected
            })
            .map(|s| s.viewer_id)
            .collect())
    }

    /// Get viewer count
    pub async fn get_viewer_count(&self) -> usize {
        self.registry.viewer_count().await
    }

    /// Check if viewer limit reached
    pub async fn is_viewer_limit_reached(&self) -> bool {
        self.registry.viewer_count().await >= MAX_VIEWERS
    }

    /// Get pending approval requests
    pub async fn get_pending_approvals(&self) -> StreamResult<Vec<PendingApproval>> {
        let pending = self.registry.get_pending_requests().await?;
        
        Ok(pending
            .into_iter()
            .map(|(peer_id, permissions)| PendingApproval {
                peer_id,
                permissions,
                requested_at: SystemTime::now(), // In real impl, track this
            })
            .collect())
    }

    /// Approve pending viewer request
    /// 
    /// Requirements: 6.4, 8.3, 8.4
    pub async fn approve_pending_viewer(&self, peer_id: PeerId) -> StreamResult<ViewerId> {
        self.registry.approve_viewer_request(peer_id).await
    }

    /// Reject pending viewer request
    /// 
    /// Requirements: 6.4, 8.3, 8.4
    pub async fn reject_pending_viewer(&self, peer_id: PeerId, reason: String) -> StreamResult<()> {
        println!("Rejecting viewer request from {}: {}", peer_id, reason);
        self.registry.reject_viewer_request(peer_id).await
    }
}

/// Result of viewer connection attempt
#[derive(Debug, Clone)]
pub enum ViewerConnectionResult {
    /// Viewer connected successfully
    Connected(ViewerId),
    /// Connection pending approval
    PendingApproval,
    /// Connection rejected with reason
    Rejected(String),
}

/// Detailed viewer status report
/// 
/// Requirements: 6.3, 8.5
#[derive(Debug, Clone)]
pub struct ViewerStatusReport {
    pub status: ViewerStatus,
    pub connection_duration: std::time::Duration,
    pub average_bitrate: u64,
    pub is_healthy: bool,
}

/// Pending approval request
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub peer_id: PeerId,
    pub permissions: ViewerPermissions,
    pub requested_at: SystemTime,
}
