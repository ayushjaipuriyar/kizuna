//! Unified Clipboard API
//! 
//! Provides a high-level, platform-abstracted interface for clipboard operations
//! with integrated security, transport, privacy, and history management.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

use crate::clipboard::{
    Clipboard, ClipboardContent, ClipboardResult, ClipboardError,
    PeerId, DeviceId, DeviceSyncStatus, SyncPolicy, ConnectionStatus, HistoryId,
};
use crate::clipboard::monitor::ClipboardMonitor;
use crate::clipboard::sync::{SyncManager, DefaultSyncManager};
use crate::clipboard::privacy::PrivacyPolicyManager;
use crate::clipboard::history::{HistoryManager, HistoryEntry};
use crate::clipboard::security_integration::ClipboardSecurityIntegration;
use crate::clipboard::transport_integration::{ClipboardTransportIntegration, ClipboardMessage};
use crate::clipboard::platform::UnifiedClipboard;
use crate::security::SecuritySystem;
use crate::transport::{KizunaTransport, PeerAddress};

/// Unified clipboard system configuration
#[derive(Debug, Clone)]
pub struct ClipboardSystemConfig {
    /// Sync policy configuration
    pub sync_policy: SyncPolicy,
    /// Enable automatic clipboard monitoring
    pub auto_monitor: bool,
    /// Enable clipboard history
    pub enable_history: bool,
    /// History size limit
    pub history_limit: usize,
    /// Enable privacy filtering
    pub enable_privacy_filter: bool,
    /// Enable notifications
    pub enable_notifications: bool,
}

impl Default for ClipboardSystemConfig {
    fn default() -> Self {
        Self {
            sync_policy: SyncPolicy::default(),
            auto_monitor: true,
            enable_history: true,
            history_limit: 50,
            enable_privacy_filter: true,
            enable_notifications: true,
        }
    }
}

/// Unified clipboard system with integrated security and transport
pub struct ClipboardSystem {
    /// Configuration
    config: Arc<RwLock<ClipboardSystemConfig>>,
    /// Platform-specific clipboard implementation
    platform_clipboard: Arc<UnifiedClipboard>,
    /// Clipboard monitor for change detection
    monitor: Arc<dyn ClipboardMonitor>,
    /// Sync manager for peer synchronization
    sync_manager: Arc<DefaultSyncManager>,
    /// Privacy policy manager
    privacy_manager: Arc<PrivacyPolicyManager>,
    /// History manager
    history_manager: Arc<dyn HistoryManager>,
    /// Security integration
    security_integration: Arc<ClipboardSecurityIntegration>,
    /// Transport integration
    transport_integration: Arc<ClipboardTransportIntegration>,
    /// Peer addresses for connection management
    peer_addresses: Arc<RwLock<HashMap<PeerId, PeerAddress>>>,
    /// Monitoring state
    is_monitoring: Arc<RwLock<bool>>,
}

impl ClipboardSystem {
    /// Create a new clipboard system with provided dependencies
    pub fn new(
        config: ClipboardSystemConfig,
        security_system: Arc<SecuritySystem>,
        transport: Arc<KizunaTransport>,
        monitor: Arc<dyn ClipboardMonitor>,
        history_manager: Arc<dyn HistoryManager>,
    ) -> Self {
        let platform_clipboard = Arc::new(UnifiedClipboard::new());
        let sync_manager = Arc::new(DefaultSyncManager::new());
        let privacy_manager = Arc::new(PrivacyPolicyManager::new());
        let security_integration = Arc::new(ClipboardSecurityIntegration::new(security_system));
        let transport_integration = Arc::new(ClipboardTransportIntegration::new(transport));
        
        Self {
            config: Arc::new(RwLock::new(config)),
            platform_clipboard,
            monitor,
            sync_manager,
            privacy_manager,
            history_manager,
            security_integration,
            transport_integration,
            peer_addresses: Arc::new(RwLock::new(HashMap::new())),
            is_monitoring: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Get current clipboard content
    pub async fn get_content(&self) -> ClipboardResult<Option<ClipboardContent>> {
        self.platform_clipboard.get_content().await
    }
    
    /// Set clipboard content locally
    pub async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()> {
        // Set content on platform clipboard
        self.platform_clipboard.set_content(content.clone()).await?;
        
        // Add to history if enabled
        let config = self.config.read().await;
        if config.enable_history {
            self.history_manager
                .add_to_history(content, crate::clipboard::ContentSource::Local)
                .await?;
        }
        
        Ok(())
    }
    
    /// Start monitoring clipboard changes
    pub async fn start_monitoring(&self) -> ClipboardResult<()> {
        {
            let mut is_monitoring = self.is_monitoring.write().await;
            if *is_monitoring {
                return Ok(());
            }
            *is_monitoring = true;
        }
        
        self.monitor.start_monitoring().await
    }
    
    /// Stop monitoring clipboard changes
    pub async fn stop_monitoring(&self) -> ClipboardResult<()> {
        {
            let mut is_monitoring = self.is_monitoring.write().await;
            if !*is_monitoring {
                return Ok(());
            }
            *is_monitoring = false;
        }
        
        self.monitor.stop_monitoring().await
    }
    
    /// Check if monitoring is active
    pub fn is_monitoring(&self) -> bool {
        self.monitor.is_monitoring()
    }
    
    /// Sync clipboard content to a specific peer
    pub async fn sync_to_peer(&self, peer_id: &PeerId, content: ClipboardContent) -> ClipboardResult<()> {
        // Check if peer is enabled for sync
        let enabled_devices = self.sync_manager.get_enabled_devices()?;
        if !enabled_devices.contains(peer_id) {
            return Err(ClipboardError::sync(
                "sync_to_peer",
                format!("Peer {} is not enabled for clipboard sync", peer_id),
            ));
        }
        
        // Encrypt content
        let encrypted_content = self.security_integration
            .encrypt_content(peer_id, &content)
            .await?;
        
        // Get peer address
        let peer_address = {
            let addresses = self.peer_addresses.read().await;
            addresses
                .get(peer_id)
                .ok_or_else(|| ClipboardError::sync("sync_to_peer", format!("No address for peer {}", peer_id)))?
                .clone()
        };
        
        // Send content via transport
        self.transport_integration
            .send_content(peer_id, &peer_address, encrypted_content)
            .await?;
        
        Ok(())
    }
    
    /// Sync clipboard content to all enabled peers
    pub async fn sync_to_all_peers(&self, content: ClipboardContent) -> ClipboardResult<()> {
        self.sync_manager.sync_content_to_peers(content).await
    }
    
    /// Receive and process clipboard content from a peer
    pub async fn receive_from_peer(&self, peer_id: &PeerId) -> ClipboardResult<()> {
        // Receive message from transport
        let message = self.transport_integration
            .receive_message(peer_id)
            .await?;
        
        if let Some(ClipboardMessage::SyncContent { content: encrypted_content, sequence, .. }) = message {
            // Decrypt content
            let content = self.security_integration
                .decrypt_content(peer_id, &encrypted_content)
                .await?;
            
            // Process received content through sync manager
            self.sync_manager
                .receive_content_from_peer(content.clone(), peer_id.clone())
                .await?;
            
            // Set content on local clipboard
            self.set_content(content).await?;
            
            // Send acknowledgment
            self.transport_integration
                .send_ack(peer_id, sequence, true, None)
                .await?;
        }
        
        Ok(())
    }
    
    /// Enable clipboard sync for a device
    pub async fn enable_sync_for_device(&self, device_id: DeviceId) -> ClipboardResult<()> {
        self.sync_manager.enable_sync_for_device(device_id).await
    }
    
    /// Disable clipboard sync for a device
    pub async fn disable_sync_for_device(&self, device_id: DeviceId) -> ClipboardResult<()> {
        self.sync_manager.disable_sync_for_device(device_id).await
    }
    
    /// Get sync status for all devices
    pub async fn get_sync_status(&self) -> ClipboardResult<Vec<DeviceSyncStatus>> {
        self.sync_manager.get_sync_status().await
    }
    
    /// Add a device to the sync allowlist
    pub async fn add_device(
        &self,
        device_id: DeviceId,
        device_name: String,
        device_type: String,
        peer_address: PeerAddress,
    ) -> ClipboardResult<()> {
        // Add device to sync manager
        self.sync_manager.add_device(device_id.clone(), device_name.clone(), device_type)?;
        
        // Store peer address
        {
            let mut addresses = self.peer_addresses.write().await;
            addresses.insert(device_id.clone(), peer_address);
        }
        
        // Add to trusted peers in security system
        self.security_integration
            .add_trusted_peer(device_id, device_name)
            .await?;
        
        Ok(())
    }
    
    /// Remove a device from the sync allowlist
    pub async fn remove_device(&self, device_id: &DeviceId) -> ClipboardResult<()> {
        // Remove from sync manager
        self.sync_manager.remove_device(device_id)?;
        
        // Remove peer address
        {
            let mut addresses = self.peer_addresses.write().await;
            addresses.remove(device_id);
        }
        
        // Remove from trusted peers
        self.security_integration
            .remove_trusted_peer(device_id)
            .await?;
        
        // Disconnect transport
        self.transport_integration
            .disconnect(device_id)
            .await?;
        
        Ok(())
    }
    
    /// Get clipboard history
    pub async fn get_history(&self, limit: usize) -> ClipboardResult<Vec<HistoryEntry>> {
        self.history_manager.get_history(limit).await
    }
    
    /// Search clipboard history
    pub async fn search_history(&self, query: &str) -> ClipboardResult<Vec<HistoryEntry>> {
        self.history_manager.search_history(query).await
    }
    
    /// Restore content from history
    pub async fn restore_from_history(&self, entry_id: HistoryId) -> ClipboardResult<()> {
        self.history_manager.restore_content(entry_id).await
    }
    
    /// Clear clipboard history
    pub async fn clear_history(&self) -> ClipboardResult<()> {
        self.history_manager.clear_history().await
    }
    
    /// Get current configuration
    pub async fn get_config(&self) -> ClipboardSystemConfig {
        let config = self.config.read().await;
        config.clone()
    }
    
    /// Update configuration
    pub async fn update_config(&self, new_config: ClipboardSystemConfig) -> ClipboardResult<()> {
        let mut config = self.config.write().await;
        *config = new_config;
        Ok(())
    }
    
    /// Get privacy policy manager
    pub fn privacy_manager(&self) -> &PrivacyPolicyManager {
        &self.privacy_manager
    }
    
    /// Get sync manager
    pub fn sync_manager(&self) -> &DefaultSyncManager {
        &self.sync_manager
    }
    
    /// Get security integration
    pub fn security_integration(&self) -> &ClipboardSecurityIntegration {
        &self.security_integration
    }
    
    /// Get transport integration
    pub fn transport_integration(&self) -> &ClipboardTransportIntegration {
        &self.transport_integration
    }
    
    /// Get detailed status information
    pub async fn get_status(&self) -> ClipboardResult<ClipboardSystemStatus> {
        let config = self.config.read().await;
        let sync_status = self.sync_manager.get_sync_status().await?;
        let history_count = self.history_manager.get_history(1).await?.len();
        let connected_peers = self.transport_integration.get_connected_peers().await;
        let trusted_peers = self.security_integration.get_trusted_peers().await?;
        
        Ok(ClipboardSystemStatus {
            is_monitoring: self.is_monitoring(),
            sync_enabled: config.sync_policy.auto_sync_enabled,
            privacy_filter_enabled: config.enable_privacy_filter,
            history_enabled: config.enable_history,
            history_count,
            device_count: sync_status.len(),
            enabled_device_count: sync_status.iter().filter(|s| s.sync_enabled).count(),
            connected_peer_count: connected_peers.len(),
            trusted_peer_count: trusted_peers.len(),
            active_session_count: self.security_integration.active_session_count().await,
        })
    }
    
    /// Shutdown the clipboard system gracefully
    pub async fn shutdown(&self) -> ClipboardResult<()> {
        // Stop monitoring
        self.stop_monitoring().await?;
        
        // Disconnect all peers
        self.transport_integration.disconnect_all().await?;
        
        // Clear sessions
        self.security_integration.clear_all_sessions().await?;
        
        Ok(())
    }
}

#[async_trait]
impl Clipboard for ClipboardSystem {
    async fn get_content(&self) -> ClipboardResult<Option<ClipboardContent>> {
        self.get_content().await
    }
    
    async fn set_content(&self, content: ClipboardContent) -> ClipboardResult<()> {
        self.set_content(content).await
    }
    
    async fn start_monitoring(&self) -> ClipboardResult<()> {
        self.start_monitoring().await
    }
    
    async fn stop_monitoring(&self) -> ClipboardResult<()> {
        self.stop_monitoring().await
    }
    
    fn is_monitoring(&self) -> bool {
        self.is_monitoring()
    }
}

/// Clipboard system status information
#[derive(Debug, Clone)]
pub struct ClipboardSystemStatus {
    /// Whether clipboard monitoring is active
    pub is_monitoring: bool,
    /// Whether automatic sync is enabled
    pub sync_enabled: bool,
    /// Whether privacy filtering is enabled
    pub privacy_filter_enabled: bool,
    /// Whether history is enabled
    pub history_enabled: bool,
    /// Number of entries in history
    pub history_count: usize,
    /// Total number of devices in allowlist
    pub device_count: usize,
    /// Number of devices with sync enabled
    pub enabled_device_count: usize,
    /// Number of currently connected peers
    pub connected_peer_count: usize,
    /// Number of trusted peers
    pub trusted_peer_count: usize,
    /// Number of active encryption sessions
    pub active_session_count: usize,
}

/// Builder for creating ClipboardSystem with fluent API
pub struct ClipboardSystemBuilder {
    config: ClipboardSystemConfig,
    security_system: Option<Arc<SecuritySystem>>,
    transport: Option<Arc<KizunaTransport>>,
    monitor: Option<Arc<dyn ClipboardMonitor>>,
    history_manager: Option<Arc<dyn HistoryManager>>,
}

impl ClipboardSystemBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: ClipboardSystemConfig::default(),
            security_system: None,
            transport: None,
            monitor: None,
            history_manager: None,
        }
    }
    
    /// Set sync policy
    pub fn sync_policy(mut self, policy: SyncPolicy) -> Self {
        self.config.sync_policy = policy;
        self
    }
    
    /// Enable or disable automatic monitoring
    pub fn auto_monitor(mut self, enabled: bool) -> Self {
        self.config.auto_monitor = enabled;
        self
    }
    
    /// Enable or disable history
    pub fn enable_history(mut self, enabled: bool) -> Self {
        self.config.enable_history = enabled;
        self
    }
    
    /// Set history size limit
    pub fn history_limit(mut self, limit: usize) -> Self {
        self.config.history_limit = limit;
        self
    }
    
    /// Enable or disable privacy filtering
    pub fn enable_privacy_filter(mut self, enabled: bool) -> Self {
        self.config.enable_privacy_filter = enabled;
        self
    }
    
    /// Enable or disable notifications
    pub fn enable_notifications(mut self, enabled: bool) -> Self {
        self.config.enable_notifications = enabled;
        self
    }
    
    /// Set security system
    pub fn security_system(mut self, security: Arc<SecuritySystem>) -> Self {
        self.security_system = Some(security);
        self
    }
    
    /// Set transport
    pub fn transport(mut self, transport: Arc<KizunaTransport>) -> Self {
        self.transport = Some(transport);
        self
    }
    
    /// Set clipboard monitor
    pub fn monitor(mut self, monitor: Arc<dyn ClipboardMonitor>) -> Self {
        self.monitor = Some(monitor);
        self
    }
    
    /// Set history manager
    pub fn history_manager(mut self, history: Arc<dyn HistoryManager>) -> Self {
        self.history_manager = Some(history);
        self
    }
    
    /// Build the clipboard system
    pub fn build(self) -> ClipboardResult<ClipboardSystem> {
        let security_system = self.security_system
            .ok_or_else(|| ClipboardError::config("builder", "Security system is required"))?;
        
        let transport = self.transport
            .ok_or_else(|| ClipboardError::config("builder", "Transport is required"))?;
        
        let monitor = self.monitor
            .ok_or_else(|| ClipboardError::config("builder", "Clipboard monitor is required"))?;
        
        let history_manager = self.history_manager
            .ok_or_else(|| ClipboardError::config("builder", "History manager is required"))?;
        
        Ok(ClipboardSystem::new(
            self.config,
            security_system,
            transport,
            monitor,
            history_manager,
        ))
    }
}

impl Default for ClipboardSystemBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::monitor::DefaultClipboardMonitor;
    use crate::clipboard::history::SqliteHistoryManager;
    use std::path::PathBuf;
    
    async fn create_test_system() -> ClipboardSystem {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let transport = Arc::new(KizunaTransport::new().await.unwrap());
        let monitor = Arc::new(DefaultClipboardMonitor::new());
        
        // Create temporary database for testing
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_clipboard_history_{}.db", uuid::Uuid::new_v4()));
        let history_manager = Arc::new(SqliteHistoryManager::new(db_path, 50).unwrap());
        
        ClipboardSystem::new(
            ClipboardSystemConfig::default(),
            security_system,
            transport,
            monitor,
            history_manager,
        )
    }
    
    #[tokio::test]
    async fn test_clipboard_system_creation() {
        let system = create_test_system().await;
        assert!(!system.is_monitoring());
    }
    
    #[tokio::test]
    async fn test_monitoring_lifecycle() {
        let system = create_test_system().await;
        
        assert!(!system.is_monitoring());
        
        system.start_monitoring().await.unwrap();
        assert!(system.is_monitoring());
        
        system.stop_monitoring().await.unwrap();
        assert!(!system.is_monitoring());
    }
    
    #[tokio::test]
    async fn test_system_status() {
        let system = create_test_system().await;
        let status = system.get_status().await.unwrap();
        
        assert!(!status.is_monitoring);
        assert_eq!(status.device_count, 0);
        assert_eq!(status.connected_peer_count, 0);
        assert_eq!(status.trusted_peer_count, 0);
    }
    
    #[tokio::test]
    async fn test_config_update() {
        let system = create_test_system().await;
        
        let mut new_config = ClipboardSystemConfig::default();
        new_config.enable_history = false;
        
        system.update_config(new_config.clone()).await.unwrap();
        
        let current_config = system.get_config().await;
        assert!(!current_config.enable_history);
    }
    
    #[tokio::test]
    async fn test_builder_pattern() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let transport = Arc::new(KizunaTransport::new().await.unwrap());
        let monitor = Arc::new(DefaultClipboardMonitor::new());
        
        let temp_dir = std::env::temp_dir();
        let db_path = temp_dir.join(format!("test_clipboard_history_{}.db", uuid::Uuid::new_v4()));
        let history_manager = Arc::new(SqliteHistoryManager::new(db_path, 50).unwrap());
        
        let system = ClipboardSystemBuilder::new()
            .auto_monitor(false)
            .enable_history(true)
            .history_limit(100)
            .enable_privacy_filter(true)
            .security_system(security_system)
            .transport(transport)
            .monitor(monitor)
            .history_manager(history_manager)
            .build()
            .unwrap();
        
        let config = system.get_config().await;
        assert!(!config.auto_monitor);
        assert!(config.enable_history);
        assert_eq!(config.history_limit, 100);
        assert!(config.enable_privacy_filter);
    }
    
    #[tokio::test]
    async fn test_shutdown() {
        let system = create_test_system().await;
        
        system.start_monitoring().await.unwrap();
        assert!(system.is_monitoring());
        
        system.shutdown().await.unwrap();
        assert!(!system.is_monitoring());
    }
}
