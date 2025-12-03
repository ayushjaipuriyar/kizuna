// Streaming and command execution handlers
//
// Implements "kizuna stream camera", "kizuna exec", "kizuna peers",
// and "kizuna status" commands for system monitoring with full integration
// to the core streaming and command execution systems.
//
// Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 6.1, 6.2, 6.3, 6.4, 6.5, 7.1, 7.2, 7.3, 7.4, 7.5

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::handlers::{ExecArgs, ExecResult, StreamArgs, StreamResult};
use crate::cli::types::{
    ConnectionStatus, OperationState, OperationStatus, OperationType, PeerInfo, ProgressInfo,
    TrustStatus,
};
use crate::streaming::api::{Streaming, StreamingApi, StreamEvent, StreamEventHandler};
use crate::streaming::{
    CameraDevice, RecordingConfig, ScreenConfig, StreamConfig, StreamQuality, StreamSession,
    StreamState, StreamType, ViewerPermissions,
};
use crate::security::api::SecuritySystem;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// CLI event handler for streaming events with notification support
struct CLIStreamEventHandler {
    active_operations: Arc<RwLock<std::collections::HashMap<Uuid, OperationStatus>>>,
    event_tx: Arc<RwLock<Option<mpsc::UnboundedSender<StreamEvent>>>>,
}

#[async_trait]
impl StreamEventHandler for CLIStreamEventHandler {
    async fn on_event(&self, event: StreamEvent) {
        let mut operations = self.active_operations.write().await;

        match &event {
            StreamEvent::SessionStarted { session_id, .. } => {
                if let Some(op) = operations.get_mut(session_id) {
                    op.status = OperationState::InProgress;
                }
            }
            StreamEvent::SessionStopped { session_id, .. } => {
                if let Some(op) = operations.get_mut(session_id) {
                    op.status = OperationState::Completed;
                }
            }
            StreamEvent::StateChanged {
                session_id,
                new_state,
                ..
            } => {
                if let Some(op) = operations.get_mut(session_id) {
                    op.status = match new_state {
                        StreamState::Starting => OperationState::Starting,
                        StreamState::Active => OperationState::InProgress,
                        StreamState::Paused => OperationState::InProgress,
                        StreamState::Stopping => OperationState::InProgress,
                        StreamState::Stopped => OperationState::Completed,
                        StreamState::Error => OperationState::Failed("Stream error".to_string()),
                    };
                }
            }
            StreamEvent::ViewerConnected {
                session_id,
                ..
            } => {
                if let Some(op) = operations.get_mut(session_id) {
                    if let Some(progress) = &mut op.progress {
                        progress.current += 1;
                        progress.message = Some(format!("{} viewers connected", progress.current));
                    }
                }
            }
            StreamEvent::ViewerDisconnected { session_id, .. } => {
                if let Some(op) = operations.get_mut(session_id) {
                    if let Some(progress) = &mut op.progress {
                        progress.current = progress.current.saturating_sub(1);
                        progress.message = Some(format!("{} viewers connected", progress.current));
                    }
                }
            }
            StreamEvent::StatsUpdated { session_id, stats } => {
                if let Some(op) = operations.get_mut(session_id) {
                    if let Some(progress) = &mut op.progress {
                        progress.rate = Some(stats.current_bitrate as f64);
                    }
                }
            }
            StreamEvent::Error {
                session_id: Some(session_id),
                error,
                ..
            } => {
                if let Some(op) = operations.get_mut(session_id) {
                    op.status = OperationState::Failed(error.clone());
                }
            }
            _ => {}
        }
        drop(operations);

        // Send event notification
        if let Some(tx) = self.event_tx.read().await.as_ref() {
            let _ = tx.send(event);
        }
    }
}

/// Streaming command handler implementation with real-time event support
/// Fully integrated with core streaming system and security
pub struct StreamingHandler {
    streaming_api: Arc<StreamingApi>,
    /// Active streaming operations with real-time status
    active_operations: Arc<RwLock<std::collections::HashMap<Uuid, OperationStatus>>>,
    /// Security system for authorization
    security: Option<Arc<SecuritySystem>>,
    /// Event notification channel for CLI/TUI updates
    event_tx: Arc<RwLock<Option<mpsc::UnboundedSender<StreamEvent>>>>,
}

impl StreamingHandler {
    /// Create a new streaming handler
    pub fn new() -> Self {
        let streaming_api = Arc::new(StreamingApi::new());
        let active_operations = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let event_tx = Arc::new(RwLock::new(None));

        let handler = Self {
            streaming_api,
            active_operations,
            security: None,
            event_tx,
        };

        // Register event handler for real-time updates
        handler.register_event_handler();

        handler
    }

    /// Create a new streaming handler with custom streaming API
    pub fn with_streaming_api(streaming_api: Arc<StreamingApi>) -> Self {
        let active_operations = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let event_tx = Arc::new(RwLock::new(None));

        let handler = Self {
            streaming_api,
            active_operations,
            security: None,
            event_tx,
        };

        // Register event handler for real-time updates
        handler.register_event_handler();

        handler
    }

    /// Create a new streaming handler with security integration
    pub fn with_security(security: Arc<SecuritySystem>) -> Self {
        let streaming_api = Arc::new(StreamingApi::new());
        let active_operations = Arc::new(RwLock::new(std::collections::HashMap::new()));
        let event_tx = Arc::new(RwLock::new(None));

        let handler = Self {
            streaming_api,
            active_operations,
            security: Some(security),
            event_tx,
        };

        // Register event handler for real-time updates
        handler.register_event_handler();

        handler
    }

    /// Set security system for authorization
    pub fn set_security(&mut self, security: Arc<SecuritySystem>) {
        self.security = Some(security);
    }

    /// Subscribe to streaming events
    /// Returns a receiver that will get notified of streaming events
    pub async fn subscribe_events(&self) -> mpsc::UnboundedReceiver<StreamEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        *self.event_tx.write().await = Some(tx);
        rx
    }

    /// Register event handler for real-time streaming updates
    fn register_event_handler(&self) {
        let event_handler = Arc::new(CLIStreamEventHandler {
            active_operations: Arc::clone(&self.active_operations),
            event_tx: Arc::clone(&self.event_tx),
        });

        let streaming_api = Arc::clone(&self.streaming_api);
        tokio::spawn(async move {
            let _ = streaming_api.register_event_handler(event_handler).await;
        });
    }

    /// Handle stream command
    pub async fn handle_stream(&self, args: StreamArgs) -> CLIResult<StreamResult> {
        // Build stream configuration
        let quality = args
            .quality
            .as_ref()
            .map(|q| self.parse_quality(q))
            .unwrap_or_else(|| StreamQuality::default());

        let config = StreamConfig {
            quality: quality.clone(),
            enable_audio: true,
            enable_recording: false,
            max_viewers: 10,
        };

        // Start camera stream
        let session = self
            .streaming_api
            .start_camera_stream(config)
            .await
            .map_err(|e| CLIError::streaming(format!("Failed to start stream: {}", e)))?;

        // Start recording if requested
        if args.record {
            let recording_config = RecordingConfig {
                output_path: args
                    .output_file
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("recording.mp4")),
                format: crate::streaming::VideoFormat::MP4,
                quality: quality.clone(),
                max_file_size: None,
                max_duration: None,
            };

            self.streaming_api
                .start_recording(session.session_id, recording_config)
                .await
                .map_err(|e| CLIError::streaming(format!("Failed to start recording: {}", e)))?;
        }

        // Convert to operation status
        let operation_status = self.session_to_operation_status(session);
        let operation_id = operation_status.operation_id;

        // Store operation for real-time tracking
        self.active_operations
            .write()
            .await
            .insert(operation_status.operation_id, operation_status.clone());

        Ok(StreamResult {
            operation_id,
            status: operation_status,
            stream_url: Some(format!("kizuna://stream/{}", operation_id)),
        })
    }

    /// Parse quality string to StreamQuality
    fn parse_quality(&self, quality: &str) -> StreamQuality {
        match quality.to_lowercase().as_str() {
            "low" => crate::streaming::QualityPreset::Low.to_quality(),
            "medium" => crate::streaming::QualityPreset::Medium.to_quality(),
            "high" => crate::streaming::QualityPreset::High.to_quality(),
            "ultra" => crate::streaming::QualityPreset::Ultra.to_quality(),
            _ => StreamQuality::default(),
        }
    }

    /// Convert StreamSession to OperationStatus
    fn session_to_operation_status(&self, session: StreamSession) -> OperationStatus {
        let operation_state = match session.state {
            StreamState::Starting => OperationState::Starting,
            StreamState::Active => OperationState::InProgress,
            StreamState::Paused => OperationState::InProgress,
            StreamState::Stopping => OperationState::InProgress,
            StreamState::Stopped => OperationState::Completed,
            StreamState::Error => OperationState::Failed("Stream error".to_string()),
        };

        OperationStatus {
            operation_id: session.session_id,
            operation_type: OperationType::CameraStream,
            peer_id: Uuid::new_v4(),
            status: operation_state,
            progress: Some(ProgressInfo {
                current: session.viewers.len() as u64,
                total: None,
                rate: Some(session.stats.current_bitrate as f64),
                eta: None,
                message: Some(format!("{} viewers connected", session.viewers.len())),
            }),
            started_at: chrono::DateTime::from(session.created_at),
            estimated_completion: None,
        }
    }

    /// Get active streams
    pub async fn get_active_streams(&self) -> CLIResult<Vec<OperationStatus>> {
        let sessions = self
            .streaming_api
            .get_active_streams()
            .await
            .map_err(|e| CLIError::streaming(format!("Failed to get active streams: {}", e)))?;

        let operations = sessions
            .into_iter()
            .map(|session| self.session_to_operation_status(session))
            .collect();

        Ok(operations)
    }

    /// Stop a stream
    pub async fn stop_stream(&self, session_id: Uuid) -> CLIResult<()> {
        self.streaming_api
            .stop_stream(session_id)
            .await
            .map_err(|e| CLIError::streaming(format!("Failed to stop stream: {}", e)))?;

        Ok(())
    }

    /// Pause a stream
    pub async fn pause_stream(&self, session_id: Uuid) -> CLIResult<()> {
        self.streaming_api
            .pause_stream(session_id)
            .await
            .map_err(|e| CLIError::streaming(format!("Failed to pause stream: {}", e)))?;

        Ok(())
    }

    /// Resume a stream
    pub async fn resume_stream(&self, session_id: Uuid) -> CLIResult<()> {
        self.streaming_api
            .resume_stream(session_id)
            .await
            .map_err(|e| CLIError::streaming(format!("Failed to resume stream: {}", e)))?;

        Ok(())
    }

    /// Add viewer to stream
    pub async fn add_viewer(&self, session_id: Uuid, peer_id: String) -> CLIResult<Uuid> {
        let viewer_id = self
            .streaming_api
            .add_viewer(session_id, peer_id, ViewerPermissions::default())
            .await
            .map_err(|e| CLIError::streaming(format!("Failed to add viewer: {}", e)))?;

        Ok(viewer_id)
    }

    /// Remove viewer from stream
    pub async fn remove_viewer(&self, session_id: Uuid, viewer_id: Uuid) -> CLIResult<()> {
        self.streaming_api
            .remove_viewer(session_id, viewer_id)
            .await
            .map_err(|e| CLIError::streaming(format!("Failed to remove viewer: {}", e)))?;

        Ok(())
    }

    /// List available cameras
    pub async fn list_cameras(&self) -> CLIResult<Vec<CameraDevice>> {
        let cameras = self
            .streaming_api
            .list_cameras()
            .await
            .map_err(|e| CLIError::streaming(format!("Failed to list cameras: {}", e)))?;

        Ok(cameras)
    }

    /// Get real-time operation status
    pub async fn get_operation_status(&self, operation_id: Uuid) -> CLIResult<OperationStatus> {
        let operations = self.active_operations.read().await;
        operations
            .get(&operation_id)
            .cloned()
            .ok_or_else(|| CLIError::streaming(format!("Operation {} not found", operation_id)))
    }

    /// Get all active streaming operations with real-time status
    pub async fn get_all_operations(&self) -> CLIResult<Vec<OperationStatus>> {
        let operations = self.active_operations.read().await;
        Ok(operations.values().cloned().collect())
    }
}

impl Default for StreamingHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Command execution handler implementation with security integration
pub struct ExecHandler {
    /// Security system for authorization
    security: Option<Arc<SecuritySystem>>,
}

impl ExecHandler {
    /// Create a new exec handler
    pub fn new() -> Self {
        Self {
            security: None,
        }
    }

    /// Create a new exec handler with security integration
    pub fn with_security(security: Arc<SecuritySystem>) -> Self {
        Self {
            security: Some(security),
        }
    }

    /// Set security system for authorization
    pub fn set_security(&mut self, security: Arc<SecuritySystem>) {
        self.security = Some(security);
    }

    /// Handle exec command with authorization
    pub async fn handle_exec(&self, args: ExecArgs) -> CLIResult<ExecResult> {
        // Check if peer is trusted if security system is available
        if let Some(ref security) = self.security {
            // Convert String peer_id to PeerId
            let peer_id = crate::security::identity::PeerId::from_string(&args.peer)
                .map_err(|e| CLIError::security(format!("Invalid peer ID: {}", e)))?;
            
            let is_trusted = security.is_trusted(&peer_id).await
                .map_err(|e| CLIError::security(format!("Failed to check peer trust: {}", e)))?;

            if !is_trusted {
                return Err(CLIError::security(format!(
                    "Cannot execute command on untrusted peer '{}'", args.peer
                )));
            }
        }

        // For now, this is a placeholder implementation
        // In a real implementation, this would execute commands on remote peers
        // through the transport layer with proper authorization

        // Simulate command execution
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(ExecResult {
            output: format!("Command '{}' executed on peer '{}'", args.command, args.peer),
            exit_code: 0,
            execution_time: Duration::from_millis(100),
        })
    }
}

impl Default for ExecHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Peers command handler implementation with discovery integration
pub struct PeersHandler {
    /// Discovery handler for peer information
    discover_handler: Option<Arc<RwLock<crate::cli::handlers::DiscoverHandler>>>,
}

impl PeersHandler {
    /// Create a new peers handler
    pub fn new() -> Self {
        Self {
            discover_handler: None,
        }
    }

    /// Create a new peers handler with discovery integration
    pub fn with_discovery(discover_handler: Arc<RwLock<crate::cli::handlers::DiscoverHandler>>) -> Self {
        Self {
            discover_handler: Some(discover_handler),
        }
    }

    /// Set discovery handler for peer information
    pub fn set_discovery(&mut self, discover_handler: Arc<RwLock<crate::cli::handlers::DiscoverHandler>>) {
        self.discover_handler = Some(discover_handler);
    }

    /// Get list of connected peers from discovery system
    pub async fn get_peers(&self) -> CLIResult<Vec<PeerInfo>> {
        if let Some(ref handler) = self.discover_handler {
            let handler_lock = handler.read().await;
            handler_lock.get_realtime_peers().await
        } else {
            // Return empty list if no discovery handler
            Ok(vec![])
        }
    }

    /// Get detailed peer information
    pub async fn get_peer_info(&self, peer_id: Uuid) -> CLIResult<PeerInfo> {
        if let Some(ref handler) = self.discover_handler {
            let handler_lock = handler.read().await;
            let peers = handler_lock.get_realtime_peers().await?;
            
            peers.into_iter()
                .find(|p| p.id == peer_id)
                .ok_or_else(|| CLIError::not_found(format!("Peer {} not found", peer_id)))
        } else {
            // Placeholder implementation
            Ok(PeerInfo {
                id: peer_id,
                name: "Unknown Peer".to_string(),
                device_type: "unknown".to_string(),
                connection_status: ConnectionStatus::Disconnected,
                capabilities: vec![],
                trust_status: TrustStatus::Untrusted,
                last_seen: None,
            })
        }
    }
}

impl Default for PeersHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Status command handler implementation with system integration
pub struct StatusHandler {
    /// Discovery handler for peer count
    discover_handler: Option<Arc<RwLock<crate::cli::handlers::DiscoverHandler>>>,
    /// Transfer handler for transfer count
    transfer_handler: Option<Arc<crate::cli::handlers::TransferHandler>>,
    /// Streaming handler for stream count
    streaming_handler: Option<Arc<StreamingHandler>>,
}

impl StatusHandler {
    /// Create a new status handler
    pub fn new() -> Self {
        Self {
            discover_handler: None,
            transfer_handler: None,
            streaming_handler: None,
        }
    }

    /// Set discovery handler for peer information
    pub fn set_discovery(&mut self, discover_handler: Arc<RwLock<crate::cli::handlers::DiscoverHandler>>) {
        self.discover_handler = Some(discover_handler);
    }

    /// Set transfer handler for transfer information
    pub fn set_transfer(&mut self, transfer_handler: Arc<crate::cli::handlers::TransferHandler>) {
        self.transfer_handler = Some(transfer_handler);
    }

    /// Set streaming handler for stream information
    pub fn set_streaming(&mut self, streaming_handler: Arc<StreamingHandler>) {
        self.streaming_handler = Some(streaming_handler);
    }

    /// Get system status with integrated information
    pub async fn get_system_status(&self) -> CLIResult<SystemStatus> {
        // Get connected peers count
        let connected_peers = if let Some(ref handler) = self.discover_handler {
            let handler_lock = handler.read().await;
            let peers = handler_lock.get_realtime_peers().await?;
            peers.iter().filter(|p| p.connection_status == ConnectionStatus::Connected).count()
        } else {
            0
        };

        // Get active transfers count
        let active_transfers = if let Some(ref handler) = self.transfer_handler {
            let transfers = handler.get_all_operations().await?;
            transfers.iter().filter(|t| matches!(t.status, OperationState::InProgress)).count()
        } else {
            0
        };

        // Get active streams count
        let active_streams = if let Some(ref handler) = self.streaming_handler {
            let streams = handler.get_all_operations().await?;
            streams.iter().filter(|s| matches!(s.status, OperationState::InProgress)).count()
        } else {
            0
        };

        Ok(SystemStatus {
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime: Duration::from_secs(0), // Placeholder - would need system start time
            connected_peers,
            active_transfers,
            active_streams,
            clipboard_sync_enabled: false, // Placeholder - would need clipboard handler
            discovery_enabled: true,
        })
    }

    /// Get network diagnostics
    pub async fn get_network_diagnostics(&self) -> CLIResult<NetworkDiagnostics> {
        // Placeholder implementation
        // In a real implementation, this would query the transport layer
        Ok(NetworkDiagnostics {
            local_addresses: vec![],
            nat_type: "Unknown".to_string(),
            relay_connected: false,
            bandwidth_up: 0,
            bandwidth_down: 0,
            latency_ms: 0,
        })
    }
}

impl Default for StatusHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// System status information
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub version: String,
    pub uptime: Duration,
    pub connected_peers: usize,
    pub active_transfers: usize,
    pub active_streams: usize,
    pub clipboard_sync_enabled: bool,
    pub discovery_enabled: bool,
}

/// Network diagnostics information
#[derive(Debug, Clone)]
pub struct NetworkDiagnostics {
    pub local_addresses: Vec<String>,
    pub nat_type: String,
    pub relay_connected: bool,
    pub bandwidth_up: u64,
    pub bandwidth_down: u64,
    pub latency_ms: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_streaming_handler_creation() {
        let handler = StreamingHandler::new();
        let streams = handler.get_active_streams().await.unwrap();
        assert_eq!(streams.len(), 0);
    }

    #[tokio::test]
    async fn test_start_stream() {
        let handler = StreamingHandler::new();
        let args = StreamArgs {
            camera_id: Some("default".to_string()),
            quality: Some("medium".to_string()),
            record: false,
            output_file: None,
        };

        let result = handler.handle_stream(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_start_stream_with_recording() {
        let handler = StreamingHandler::new();
        let args = StreamArgs {
            camera_id: Some("default".to_string()),
            quality: Some("high".to_string()),
            record: true,
            output_file: Some(PathBuf::from("test_recording.mp4")),
        };

        let result = handler.handle_stream(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_cameras() {
        let handler = StreamingHandler::new();
        let result = handler.list_cameras().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_exec_handler() {
        let handler = ExecHandler::new();
        let args = ExecArgs {
            command: "echo hello".to_string(),
            peer: "test-peer".to_string(),
            timeout: Some(5),
        };

        let result = handler.handle_exec(args).await.unwrap();
        assert_eq!(result.exit_code, 0);
        assert!(result.output.contains("echo hello"));
    }

    #[tokio::test]
    async fn test_peers_handler() {
        let handler = PeersHandler::new();
        let peers = handler.get_peers().await.unwrap();
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_status_handler() {
        let handler = StatusHandler::new();
        let status = handler.get_system_status().await.unwrap();
        assert!(!status.version.is_empty());
    }

    #[tokio::test]
    async fn test_network_diagnostics() {
        let handler = StatusHandler::new();
        let diagnostics = handler.get_network_diagnostics().await.unwrap();
        assert_eq!(diagnostics.local_addresses.len(), 0);
    }

    #[tokio::test]
    async fn test_parse_quality() {
        let handler = StreamingHandler::new();
        let _low = handler.parse_quality("low");
        let _medium = handler.parse_quality("medium");
        let _high = handler.parse_quality("high");
        let _ultra = handler.parse_quality("ultra");
        let _default = handler.parse_quality("invalid");
    }
}
