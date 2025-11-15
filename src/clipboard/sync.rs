//! Clipboard synchronization and peer management

use async_trait::async_trait;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::time::SystemTime;
use crate::clipboard::{
    ClipboardContent, ClipboardResult, ClipboardError, DeviceId, PeerId, DeviceSyncStatus, ConnectionStatus
};
use crate::clipboard::privacy::{PrivacyPolicyManager, SyncDecision, SensitivePattern};

/// Clipboard sync manager trait
#[async_trait]
pub trait SyncManager: Send + Sync {
    /// Enable clipboard sync for a specific device
    async fn enable_sync_for_device(&self, device_id: DeviceId) -> ClipboardResult<()>;
    
    /// Disable clipboard sync for a specific device
    async fn disable_sync_for_device(&self, device_id: DeviceId) -> ClipboardResult<()>;
    
    /// Sync clipboard content to all enabled peers
    async fn sync_content_to_peers(&self, content: ClipboardContent) -> ClipboardResult<()>;
    
    /// Receive clipboard content from a peer
    async fn receive_content_from_peer(&self, content: ClipboardContent, peer_id: PeerId) -> ClipboardResult<()>;
    
    /// Get sync status for all devices
    async fn get_sync_status(&self) -> ClipboardResult<Vec<DeviceSyncStatus>>;
}

/// Privacy violation event for logging
#[derive(Debug, Clone)]
pub struct PrivacyViolation {
    pub timestamp: SystemTime,
    pub content_type: String,
    pub reason: String,
    pub detected_patterns: Vec<SensitivePattern>,
    pub action_taken: PrivacyAction,
}

/// Action taken for privacy violation
#[derive(Debug, Clone, PartialEq)]
pub enum PrivacyAction {
    Blocked,
    UserPrompted,
    Allowed,
}

/// Privacy violation logger
pub struct PrivacyViolationLogger {
    violations: Arc<RwLock<Vec<PrivacyViolation>>>,
    max_violations: usize,
}

impl PrivacyViolationLogger {
    /// Create new violation logger
    pub fn new() -> Self {
        Self {
            violations: Arc::new(RwLock::new(Vec::new())),
            max_violations: 1000,
        }
    }
    
    /// Create with custom max violations
    pub fn with_max_violations(max: usize) -> Self {
        Self {
            violations: Arc::new(RwLock::new(Vec::new())),
            max_violations: max,
        }
    }
    
    /// Log a privacy violation
    pub fn log_violation(&self, violation: PrivacyViolation) -> ClipboardResult<()> {
        let mut violations = self.violations.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on violations"))?;
        
        violations.push(violation);
        
        // Trim old violations if we exceed max
        let len = violations.len();
        if len > self.max_violations {
            violations.drain(0..len - self.max_violations);
        }
        
        Ok(())
    }
    
    /// Get all violations
    pub fn get_violations(&self) -> ClipboardResult<Vec<PrivacyViolation>> {
        let violations = self.violations.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on violations"))?;
        
        Ok(violations.clone())
    }
    
    /// Get violations since a specific time
    pub fn get_violations_since(&self, since: SystemTime) -> ClipboardResult<Vec<PrivacyViolation>> {
        let violations = self.violations.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on violations"))?;
        
        Ok(violations.iter()
            .filter(|v| v.timestamp >= since)
            .cloned()
            .collect())
    }
    
    /// Clear all violations
    pub fn clear_violations(&self) -> ClipboardResult<()> {
        let mut violations = self.violations.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on violations"))?;
        
        violations.clear();
        Ok(())
    }
    
    /// Get violation count
    pub fn violation_count(&self) -> ClipboardResult<usize> {
        let violations = self.violations.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on violations"))?;
        
        Ok(violations.len())
    }
}

impl Default for PrivacyViolationLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Sync notification callback type
pub type SyncNotificationCallback = Arc<dyn Fn(SyncNotification) + Send + Sync>;

/// Sync notification types
#[derive(Debug, Clone)]
pub enum SyncNotification {
    /// Content was blocked due to privacy concerns
    ContentBlocked {
        reason: String,
        patterns: Vec<SensitivePattern>,
    },
    /// User was prompted for sensitive content
    UserPrompted {
        content_type: String,
    },
    /// Sync operation started
    SyncStarted {
        device_id: DeviceId,
    },
    /// Sync operation completed
    SyncCompleted {
        device_id: DeviceId,
    },
    /// Sync operation failed
    SyncFailed {
        device_id: DeviceId,
        error: String,
    },
    /// Sync conflict detected
    ConflictDetected {
        local_timestamp: SystemTime,
        remote_timestamp: SystemTime,
        resolution: ConflictResolution,
    },
    /// Sync retry scheduled
    RetryScheduled {
        device_id: DeviceId,
        attempt: u32,
        delay_ms: u64,
    },
}

/// Conflict resolution strategy
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictResolution {
    /// Use local content (local is newer)
    UseLocal,
    /// Use remote content (remote is newer)
    UseRemote,
    /// Merge content (if possible)
    Merge,
    /// Prompt user for decision
    PromptUser,
}

/// Clipboard content with metadata for conflict resolution
#[derive(Debug, Clone)]
pub struct TimestampedContent {
    pub content: ClipboardContent,
    pub timestamp: SystemTime,
    pub source_device: DeviceId,
    pub sequence_number: u64,
}

/// Retry configuration for failed sync operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Pending retry operation
#[derive(Debug, Clone)]
struct PendingRetry {
    device_id: DeviceId,
    content: ClipboardContent,
    attempt: u32,
    next_retry_at: SystemTime,
}

/// Device information for allowlist management
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub device_id: DeviceId,
    pub device_name: String,
    pub device_type: String,
    pub added_at: SystemTime,
    pub last_seen: SystemTime,
}

/// Sync statistics for a device
#[derive(Debug, Clone, Default)]
pub struct SyncStatistics {
    pub total_syncs: u64,
    pub successful_syncs: u64,
    pub failed_syncs: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub last_sync_duration_ms: Option<u64>,
    pub average_sync_duration_ms: Option<u64>,
}

/// Default sync manager implementation with privacy integration
pub struct DefaultSyncManager {
    /// Privacy policy manager
    privacy_manager: Arc<PrivacyPolicyManager>,
    /// Privacy violation logger
    violation_logger: Arc<PrivacyViolationLogger>,
    /// Device allowlist (device_id -> enabled)
    device_allowlist: Arc<RwLock<HashMap<DeviceId, bool>>>,
    /// Device information
    device_info: Arc<RwLock<HashMap<DeviceId, DeviceInfo>>>,
    /// Device sync status
    device_status: Arc<RwLock<HashMap<DeviceId, DeviceSyncStatus>>>,
    /// Device sync statistics
    device_statistics: Arc<RwLock<HashMap<DeviceId, SyncStatistics>>>,
    /// Notification callback
    notification_callback: Arc<RwLock<Option<SyncNotificationCallback>>>,
    /// Last known content with timestamp for conflict resolution
    last_content: Arc<RwLock<Option<TimestampedContent>>>,
    /// Retry configuration
    retry_config: Arc<RwLock<RetryConfig>>,
    /// Pending retry operations
    pending_retries: Arc<RwLock<Vec<PendingRetry>>>,
}

impl DefaultSyncManager {
    /// Create new sync manager with privacy integration
    pub fn new() -> Self {
        Self {
            privacy_manager: Arc::new(PrivacyPolicyManager::new()),
            violation_logger: Arc::new(PrivacyViolationLogger::new()),
            device_allowlist: Arc::new(RwLock::new(HashMap::new())),
            device_info: Arc::new(RwLock::new(HashMap::new())),
            device_status: Arc::new(RwLock::new(HashMap::new())),
            device_statistics: Arc::new(RwLock::new(HashMap::new())),
            notification_callback: Arc::new(RwLock::new(None)),
            last_content: Arc::new(RwLock::new(None)),
            retry_config: Arc::new(RwLock::new(RetryConfig::default())),
            pending_retries: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Create with custom privacy manager
    pub fn with_privacy_manager(privacy_manager: PrivacyPolicyManager) -> Self {
        Self {
            privacy_manager: Arc::new(privacy_manager),
            violation_logger: Arc::new(PrivacyViolationLogger::new()),
            device_allowlist: Arc::new(RwLock::new(HashMap::new())),
            device_info: Arc::new(RwLock::new(HashMap::new())),
            device_status: Arc::new(RwLock::new(HashMap::new())),
            device_statistics: Arc::new(RwLock::new(HashMap::new())),
            notification_callback: Arc::new(RwLock::new(None)),
            last_content: Arc::new(RwLock::new(None)),
            retry_config: Arc::new(RwLock::new(RetryConfig::default())),
            pending_retries: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Get reference to privacy manager
    pub fn privacy_manager(&self) -> &PrivacyPolicyManager {
        &self.privacy_manager
    }
    
    /// Get reference to violation logger
    pub fn violation_logger(&self) -> &PrivacyViolationLogger {
        &self.violation_logger
    }
    
    /// Set notification callback
    pub fn set_notification_callback<F>(&self, callback: F) -> ClipboardResult<()>
    where
        F: Fn(SyncNotification) + Send + Sync + 'static,
    {
        let mut cb = self.notification_callback.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on callback"))?;
        
        *cb = Some(Arc::new(callback));
        Ok(())
    }
    
    /// Send notification
    fn notify(&self, notification: SyncNotification) {
        if let Ok(cb) = self.notification_callback.read() {
            if let Some(callback) = cb.as_ref() {
                callback(notification);
            }
        }
    }
    
    /// Check if device is in allowlist and enabled
    fn is_device_enabled(&self, device_id: &DeviceId) -> ClipboardResult<bool> {
        let allowlist = self.device_allowlist.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on allowlist"))?;
        
        Ok(allowlist.get(device_id).copied().unwrap_or(false))
    }
    
    /// Perform privacy analysis before sync
    async fn analyze_content_for_sync(&self, content: &ClipboardContent) -> ClipboardResult<SyncDecision> {
        self.privacy_manager.should_sync_content(content).await
    }
    
    /// Log privacy violation
    fn log_privacy_violation(
        &self,
        content: &ClipboardContent,
        reason: String,
        patterns: Vec<SensitivePattern>,
        action: PrivacyAction,
    ) -> ClipboardResult<()> {
        let content_type = match content {
            ClipboardContent::Text(_) => "text",
            ClipboardContent::Image(_) => "image",
            ClipboardContent::Files(_) => "files",
            ClipboardContent::Custom { mime_type, .. } => mime_type.as_str(),
        }.to_string();
        
        let violation = PrivacyViolation {
            timestamp: SystemTime::now(),
            content_type,
            reason,
            detected_patterns: patterns,
            action_taken: action,
        };
        
        self.violation_logger.log_violation(violation)
    }
    
    /// Add device to allowlist with information
    pub fn add_device(&self, device_id: DeviceId, device_name: String, device_type: String) -> ClipboardResult<()> {
        let now = SystemTime::now();
        
        // Add device info
        let mut info_map = self.device_info.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device info"))?;
        
        info_map.insert(device_id.clone(), DeviceInfo {
            device_id: device_id.clone(),
            device_name: device_name.clone(),
            device_type,
            added_at: now,
            last_seen: now,
        });
        
        // Add to allowlist (disabled by default)
        let mut allowlist = self.device_allowlist.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on allowlist"))?;
        
        allowlist.insert(device_id.clone(), false);
        
        // Initialize device status
        let mut status_map = self.device_status.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device status"))?;
        
        status_map.insert(device_id.clone(), DeviceSyncStatus {
            device_id: device_id.clone(),
            device_name,
            sync_enabled: false,
            last_sync: None,
            sync_count: 0,
            connection_status: ConnectionStatus::Disconnected,
        });
        
        // Initialize statistics
        let mut stats_map = self.device_statistics.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device statistics"))?;
        
        stats_map.insert(device_id, SyncStatistics::default());
        
        Ok(())
    }
    
    /// Remove device from allowlist
    pub fn remove_device(&self, device_id: &DeviceId) -> ClipboardResult<()> {
        let mut allowlist = self.device_allowlist.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on allowlist"))?;
        
        allowlist.remove(device_id);
        
        let mut info_map = self.device_info.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device info"))?;
        
        info_map.remove(device_id);
        
        let mut status_map = self.device_status.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device status"))?;
        
        status_map.remove(device_id);
        
        let mut stats_map = self.device_statistics.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device statistics"))?;
        
        stats_map.remove(device_id);
        
        Ok(())
    }
    
    /// Get device information
    pub fn get_device_info(&self, device_id: &DeviceId) -> ClipboardResult<Option<DeviceInfo>> {
        let info_map = self.device_info.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on device info"))?;
        
        Ok(info_map.get(device_id).cloned())
    }
    
    /// Get all devices in allowlist
    pub fn get_all_devices(&self) -> ClipboardResult<Vec<DeviceInfo>> {
        let info_map = self.device_info.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on device info"))?;
        
        Ok(info_map.values().cloned().collect())
    }
    
    /// Get enabled devices
    pub fn get_enabled_devices(&self) -> ClipboardResult<Vec<DeviceId>> {
        let allowlist = self.device_allowlist.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on allowlist"))?;
        
        Ok(allowlist.iter()
            .filter(|(_, enabled)| **enabled)
            .map(|(id, _)| id.clone())
            .collect())
    }
    
    /// Get device statistics
    pub fn get_device_statistics(&self, device_id: &DeviceId) -> ClipboardResult<Option<SyncStatistics>> {
        let stats_map = self.device_statistics.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on device statistics"))?;
        
        Ok(stats_map.get(device_id).cloned())
    }
    
    /// Update device last seen timestamp
    pub fn update_device_last_seen(&self, device_id: &DeviceId) -> ClipboardResult<()> {
        let mut info_map = self.device_info.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device info"))?;
        
        if let Some(info) = info_map.get_mut(device_id) {
            info.last_seen = SystemTime::now();
        }
        
        Ok(())
    }
    
    /// Update device connection status
    pub fn update_device_connection_status(&self, device_id: &DeviceId, status: ConnectionStatus) -> ClipboardResult<()> {
        let mut status_map = self.device_status.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device status"))?;
        
        if let Some(device_status) = status_map.get_mut(device_id) {
            device_status.connection_status = status;
        }
        
        Ok(())
    }
    
    /// Record sync operation for statistics
    fn record_sync_operation(
        &self,
        device_id: &DeviceId,
        success: bool,
        bytes_sent: u64,
        duration_ms: u64,
    ) -> ClipboardResult<()> {
        let mut stats_map = self.device_statistics.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device statistics"))?;
        
        if let Some(stats) = stats_map.get_mut(device_id) {
            stats.total_syncs += 1;
            if success {
                stats.successful_syncs += 1;
            } else {
                stats.failed_syncs += 1;
            }
            stats.bytes_sent += bytes_sent;
            stats.last_sync_duration_ms = Some(duration_ms);
            
            // Calculate rolling average
            if let Some(avg) = stats.average_sync_duration_ms {
                stats.average_sync_duration_ms = Some((avg + duration_ms) / 2);
            } else {
                stats.average_sync_duration_ms = Some(duration_ms);
            }
        }
        
        Ok(())
    }
    
    /// Record received content for statistics
    fn record_received_content(&self, device_id: &DeviceId, bytes_received: u64) -> ClipboardResult<()> {
        let mut stats_map = self.device_statistics.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device statistics"))?;
        
        if let Some(stats) = stats_map.get_mut(device_id) {
            stats.bytes_received += bytes_received;
        }
        
        Ok(())
    }
    
    /// Serialize clipboard content for transmission
    fn serialize_content(&self, content: &ClipboardContent) -> ClipboardResult<Vec<u8>> {
        serde_json::to_vec(content)
            .map_err(|e| ClipboardError::serialization("clipboard_content", e))
    }
    
    /// Deserialize clipboard content from transmission
    fn deserialize_content(&self, data: &[u8]) -> ClipboardResult<ClipboardContent> {
        serde_json::from_slice(data)
            .map_err(|e| ClipboardError::serialization("clipboard_content", e))
    }
    
    /// Transmit content to a specific device
    async fn transmit_content_to_device(&self, device_id: &DeviceId, content: &[u8]) -> ClipboardResult<()> {
        // TODO: Integrate with transport layer for actual transmission
        // For now, this is a placeholder that simulates successful transmission
        
        // Validate device is connected
        let connection_status = {
            let status_map = self.device_status.read()
                .map_err(|_| ClipboardError::internal("Failed to acquire read lock on device status"))?;
            
            status_map.get(device_id).map(|s| s.connection_status.clone())
        };
        
        if let Some(status) = connection_status {
            match status {
                ConnectionStatus::Connected | ConnectionStatus::Connecting => {
                    // Simulate transmission delay (would be actual network call)
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    Ok(())
                }
                ConnectionStatus::Disconnected => {
                    Err(ClipboardError::sync(
                        "transmit_content",
                        format!("Device {} is disconnected", device_id)
                    ))
                }
                ConnectionStatus::Error(e) => {
                    Err(ClipboardError::sync(
                        "transmit_content",
                        format!("Device {} has connection error: {}", device_id, e)
                    ))
                }
            }
        } else {
            Err(ClipboardError::sync(
                "transmit_content",
                format!("Device {} not found", device_id)
            ))
        }
    }
    
    /// Apply received content to local clipboard
    async fn apply_content_to_clipboard(&self, content: &ClipboardContent) -> ClipboardResult<()> {
        // TODO: Integrate with clipboard monitor to update local clipboard
        // This would call the platform-specific clipboard implementation
        // For now, this is a placeholder
        Ok(())
    }
    
    /// Resolve conflict between local and remote content
    fn resolve_conflict(
        &self,
        local: &TimestampedContent,
        remote: &TimestampedContent,
    ) -> ClipboardResult<ConflictResolution> {
        // Timestamp-based conflict resolution
        let resolution = if remote.timestamp > local.timestamp {
            // Remote is newer, use remote content
            ConflictResolution::UseRemote
        } else if local.timestamp > remote.timestamp {
            // Local is newer, use local content
            ConflictResolution::UseLocal
        } else {
            // Same timestamp, use sequence number as tiebreaker
            if remote.sequence_number > local.sequence_number {
                ConflictResolution::UseRemote
            } else {
                ConflictResolution::UseLocal
            }
        };
        
        // Notify about conflict
        self.notify(SyncNotification::ConflictDetected {
            local_timestamp: local.timestamp,
            remote_timestamp: remote.timestamp,
            resolution: resolution.clone(),
        });
        
        Ok(resolution)
    }
    
    /// Update retry configuration
    pub fn set_retry_config(&self, config: RetryConfig) -> ClipboardResult<()> {
        let mut retry_config = self.retry_config.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on retry config"))?;
        
        *retry_config = config;
        Ok(())
    }
    
    /// Get retry configuration
    pub fn get_retry_config(&self) -> ClipboardResult<RetryConfig> {
        let retry_config = self.retry_config.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on retry config"))?;
        
        Ok(retry_config.clone())
    }
    
    /// Schedule retry for failed sync operation
    fn schedule_retry(
        &self,
        device_id: DeviceId,
        content: ClipboardContent,
        attempt: u32,
    ) -> ClipboardResult<()> {
        let config = self.get_retry_config()?;
        
        if attempt >= config.max_attempts {
            return Err(ClipboardError::sync(
                "schedule_retry",
                format!("Max retry attempts ({}) reached for device {}", config.max_attempts, device_id)
            ));
        }
        
        // Calculate exponential backoff delay
        let delay_ms = (config.initial_delay_ms as f64 
            * config.backoff_multiplier.powi(attempt as i32))
            .min(config.max_delay_ms as f64) as u64;
        
        let next_retry_at = SystemTime::now() + std::time::Duration::from_millis(delay_ms);
        
        let retry = PendingRetry {
            device_id: device_id.clone(),
            content,
            attempt: attempt + 1,
            next_retry_at,
        };
        
        let mut pending_retries = self.pending_retries.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on pending retries"))?;
        
        pending_retries.push(retry);
        
        // Notify about scheduled retry
        self.notify(SyncNotification::RetryScheduled {
            device_id,
            attempt: attempt + 1,
            delay_ms,
        });
        
        Ok(())
    }
    
    /// Process pending retries
    pub async fn process_pending_retries(&self) -> ClipboardResult<()> {
        let now = SystemTime::now();
        let mut retries_to_process = Vec::new();
        
        // Get retries that are ready to process
        {
            let mut pending_retries = self.pending_retries.write()
                .map_err(|_| ClipboardError::internal("Failed to acquire write lock on pending retries"))?;
            
            // Split retries into ready and not ready
            let (ready, not_ready): (Vec<_>, Vec<_>) = pending_retries
                .drain(..)
                .partition(|retry| retry.next_retry_at <= now);
            
            retries_to_process = ready;
            *pending_retries = not_ready;
        }
        
        // Process ready retries
        for retry in retries_to_process {
            let serialized_content = self.serialize_content(&retry.content)?;
            
            match self.transmit_content_to_device(&retry.device_id, &serialized_content).await {
                Ok(_) => {
                    // Retry succeeded
                    let mut status_map = self.device_status.write()
                        .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device status"))?;
                    
                    if let Some(status) = status_map.get_mut(&retry.device_id) {
                        status.last_sync = Some(SystemTime::now());
                        status.connection_status = ConnectionStatus::Connected;
                    }
                    
                    self.notify(SyncNotification::SyncCompleted {
                        device_id: retry.device_id.clone(),
                    });
                }
                Err(_) => {
                    // Retry failed, schedule another retry if attempts remain
                    let _ = self.schedule_retry(retry.device_id, retry.content, retry.attempt);
                }
            }
        }
        
        Ok(())
    }
    
    /// Get count of pending retries
    pub fn pending_retry_count(&self) -> ClipboardResult<usize> {
        let pending_retries = self.pending_retries.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on pending retries"))?;
        
        Ok(pending_retries.len())
    }
    
    /// Clear all pending retries
    pub fn clear_pending_retries(&self) -> ClipboardResult<()> {
        let mut pending_retries = self.pending_retries.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on pending retries"))?;
        
        pending_retries.clear();
        Ok(())
    }
    
    /// Update last known content for conflict resolution
    fn update_last_content(
        &self,
        content: ClipboardContent,
        source_device: DeviceId,
        sequence_number: u64,
    ) -> ClipboardResult<()> {
        let mut last_content = self.last_content.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on last content"))?;
        
        *last_content = Some(TimestampedContent {
            content,
            timestamp: SystemTime::now(),
            source_device,
            sequence_number,
        });
        
        Ok(())
    }
    
    /// Get last known content
    pub fn get_last_content(&self) -> ClipboardResult<Option<TimestampedContent>> {
        let last_content = self.last_content.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on last content"))?;
        
        Ok(last_content.clone())
    }
}

#[async_trait]
impl SyncManager for DefaultSyncManager {
    async fn enable_sync_for_device(&self, device_id: DeviceId) -> ClipboardResult<()> {
        // Check if device exists in allowlist
        {
            let info_map = self.device_info.read()
                .map_err(|_| ClipboardError::internal("Failed to acquire read lock on device info"))?;
            
            if !info_map.contains_key(&device_id) {
                return Err(ClipboardError::config(
                    "device_allowlist",
                    format!("Device {} not found in allowlist", device_id)
                ));
            }
        }
        
        let mut allowlist = self.device_allowlist.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on allowlist"))?;
        
        allowlist.insert(device_id.clone(), true);
        
        // Update device status
        let mut status_map = self.device_status.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device status"))?;
        
        if let Some(status) = status_map.get_mut(&device_id) {
            status.sync_enabled = true;
        }
        
        Ok(())
    }
    
    async fn disable_sync_for_device(&self, device_id: DeviceId) -> ClipboardResult<()> {
        let mut allowlist = self.device_allowlist.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on allowlist"))?;
        
        allowlist.insert(device_id.clone(), false);
        
        // Update device status
        let mut status_map = self.device_status.write()
            .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device status"))?;
        
        if let Some(status) = status_map.get_mut(&device_id) {
            status.sync_enabled = false;
        }
        
        Ok(())
    }
    
    async fn sync_content_to_peers(&self, content: ClipboardContent) -> ClipboardResult<()> {
        // Perform privacy analysis before sync
        let decision = self.analyze_content_for_sync(&content).await?;
        
        match decision {
            SyncDecision::Allow => {
                // Get enabled devices
                let enabled_devices = self.get_enabled_devices()?;
                
                if enabled_devices.is_empty() {
                    return Ok(());
                }
                
                let enabled_device_count = enabled_devices.len();
                
                // Calculate content size for statistics
                let content_size = content.size() as u64;
                
                // Serialize content for transmission
                let serialized_content = self.serialize_content(&content)?;
                
                // Sync to each enabled device
                let mut sync_errors = Vec::new();
                
                for device_id in enabled_devices {
                    let sync_start = SystemTime::now();
                    
                    self.notify(SyncNotification::SyncStarted {
                        device_id: device_id.clone(),
                    });
                    
                    // Attempt to sync content
                    match self.transmit_content_to_device(&device_id, &serialized_content).await {
                        Ok(_) => {
                            // Calculate sync duration
                            let duration_ms = sync_start.elapsed()
                                .unwrap_or_default()
                                .as_millis() as u64;
                            
                            // Update device status
                            let mut status_map = self.device_status.write()
                                .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device status"))?;
                            
                            if let Some(status) = status_map.get_mut(&device_id) {
                                status.last_sync = Some(SystemTime::now());
                                status.sync_count += 1;
                                status.connection_status = ConnectionStatus::Connected;
                            }
                            
                            // Record statistics
                            self.record_sync_operation(&device_id, true, content_size, duration_ms)?;
                            
                            // Update last seen
                            self.update_device_last_seen(&device_id)?;
                            
                            self.notify(SyncNotification::SyncCompleted {
                                device_id: device_id.clone(),
                            });
                        }
                        Err(e) => {
                            // Calculate sync duration even for failures
                            let duration_ms = sync_start.elapsed()
                                .unwrap_or_default()
                                .as_millis() as u64;
                            
                            // Update connection status
                            self.update_device_connection_status(
                                &device_id,
                                ConnectionStatus::Error(e.to_string())
                            )?;
                            
                            // Record failed sync
                            self.record_sync_operation(&device_id, false, 0, duration_ms)?;
                            
                            self.notify(SyncNotification::SyncFailed {
                                device_id: device_id.clone(),
                                error: e.to_string(),
                            });
                            
                            // Schedule retry for failed sync
                            if let Err(retry_err) = self.schedule_retry(device_id.clone(), content.clone(), 0) {
                                // If retry scheduling fails, just log it
                                sync_errors.push((device_id.clone(), retry_err));
                            }
                            
                            sync_errors.push((device_id, e));
                        }
                    }
                }
                
                // If all syncs failed, return error
                if !sync_errors.is_empty() && sync_errors.len() == enabled_device_count {
                    return Err(ClipboardError::sync(
                        "sync_to_peers",
                        format!("Failed to sync to all {} devices", sync_errors.len())
                    ));
                }
                
                Ok(())
            }
            SyncDecision::Block { reason, patterns } => {
                // Log privacy violation
                self.log_privacy_violation(&content, reason.clone(), patterns.clone(), PrivacyAction::Blocked)?;
                
                // Send notification
                self.notify(SyncNotification::ContentBlocked {
                    reason: reason.clone(),
                    patterns: patterns.clone(),
                });
                
                Err(ClipboardError::privacy(reason))
            }
        }
    }
    
    async fn receive_content_from_peer(&self, content: ClipboardContent, peer_id: PeerId) -> ClipboardResult<()> {
        // Check if peer is in allowlist and enabled
        if !self.is_device_enabled(&peer_id)? {
            return Err(ClipboardError::sync(
                "receive_content",
                format!("Peer {} is not enabled for clipboard sync", peer_id)
            ));
        }
        
        // Perform privacy analysis on received content
        let decision = self.analyze_content_for_sync(&content).await?;
        
        match decision {
            SyncDecision::Allow => {
                // Create timestamped content for conflict resolution
                let remote_content = TimestampedContent {
                    content: content.clone(),
                    timestamp: SystemTime::now(),
                    source_device: peer_id.clone(),
                    sequence_number: 0, // TODO: Get actual sequence number from transmission
                };
                
                // Check for conflicts with local content
                let should_apply = if let Some(local_content) = self.get_last_content()? {
                    let resolution = self.resolve_conflict(&local_content, &remote_content)?;
                    
                    match resolution {
                        ConflictResolution::UseRemote => true,
                        ConflictResolution::UseLocal => false,
                        ConflictResolution::Merge => {
                            // TODO: Implement merge logic for compatible content types
                            true
                        }
                        ConflictResolution::PromptUser => {
                            // TODO: Implement user prompt for conflict resolution
                            true
                        }
                    }
                } else {
                    // No local content, always apply remote
                    true
                };
                
                if should_apply {
                    // Calculate content size for statistics
                    let content_size = content.size() as u64;
                    
                    // Apply content to local clipboard
                    self.apply_content_to_clipboard(&content).await?;
                    
                    // Update last known content
                    self.update_last_content(content, peer_id.clone(), 0)?;
                    
                    // Update device status
                    let mut status_map = self.device_status.write()
                        .map_err(|_| ClipboardError::internal("Failed to acquire write lock on device status"))?;
                    
                    if let Some(status) = status_map.get_mut(&peer_id) {
                        status.last_sync = Some(SystemTime::now());
                        status.connection_status = ConnectionStatus::Connected;
                    }
                    
                    // Record received content statistics
                    self.record_received_content(&peer_id, content_size)?;
                    
                    // Update last seen
                    self.update_device_last_seen(&peer_id)?;
                }
                
                Ok(())
            }
            SyncDecision::Block { reason, patterns } => {
                // Log privacy violation
                self.log_privacy_violation(&content, reason.clone(), patterns.clone(), PrivacyAction::Blocked)?;
                
                // Send notification
                self.notify(SyncNotification::ContentBlocked {
                    reason: reason.clone(),
                    patterns: patterns.clone(),
                });
                
                Err(ClipboardError::privacy(format!(
                    "Blocked content from peer {}: {}",
                    peer_id, reason
                )))
            }
        }
    }
    
    async fn get_sync_status(&self) -> ClipboardResult<Vec<DeviceSyncStatus>> {
        let status_map = self.device_status.read()
            .map_err(|_| ClipboardError::internal("Failed to acquire read lock on device status"))?;
        
        Ok(status_map.values().cloned().collect())
    }
}

impl Default for DefaultSyncManager {
    fn default() -> Self {
        Self::new()
    }
}