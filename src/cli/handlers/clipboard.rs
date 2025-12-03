// Clipboard management command handler
//
// Implements "kizuna clipboard share" command with toggle functionality,
// clipboard status display, and per-device control.
//
// Requirements: 4.1, 4.2, 4.3, 4.4, 4.5

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::types::{ConnectionStatus, PeerInfo};
use crate::clipboard::api::{ClipboardSystem, ClipboardSystemStatus};
use crate::clipboard::{ClipboardContent, ContentSource, TextContent};
use crate::clipboard::history::HistoryEntry;
use std::sync::Arc;
use uuid::Uuid;

/// Clipboard command arguments
#[derive(Debug, Clone)]
pub struct ClipboardArgs {
    pub action: ClipboardAction,
    pub device_id: Option<String>,
}

/// Clipboard action types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardAction {
    /// Toggle clipboard sharing on/off
    Share,
    /// Get current clipboard status
    Status,
    /// Enable sync for a specific device
    EnableDevice(String),
    /// Disable sync for a specific device
    DisableDevice(String),
    /// View clipboard history
    History { limit: usize },
    /// Search clipboard history
    Search { query: String },
    /// Restore content from history
    Restore { entry_id: Uuid },
    /// Clear clipboard history
    ClearHistory,
    /// Get current clipboard content
    Get,
    /// Set clipboard content
    Set { content: String },
}

/// Clipboard command result
#[derive(Debug, Clone)]
pub struct ClipboardResult {
    pub success: bool,
    pub message: String,
    pub status: Option<ClipboardSystemStatus>,
    pub history: Option<Vec<HistoryEntry>>,
    pub content: Option<ClipboardContent>,
}

/// Clipboard command handler implementation
pub struct ClipboardHandler {
    clipboard_system: Arc<ClipboardSystem>,
}

impl ClipboardHandler {
    /// Create a new clipboard handler
    pub fn new(clipboard_system: Arc<ClipboardSystem>) -> Self {
        Self { clipboard_system }
    }

    /// Handle clipboard command
    pub async fn handle_clipboard(&self, args: ClipboardArgs) -> CLIResult<ClipboardResult> {
        match args.action {
            ClipboardAction::Share => self.toggle_sharing().await,
            ClipboardAction::Status => self.get_status().await,
            ClipboardAction::EnableDevice(device_id) => self.enable_device(device_id).await,
            ClipboardAction::DisableDevice(device_id) => self.disable_device(device_id).await,
            ClipboardAction::History { limit } => self.get_history(limit).await,
            ClipboardAction::Search { query } => self.search_history(query).await,
            ClipboardAction::Restore { entry_id } => self.restore_from_history(entry_id).await,
            ClipboardAction::ClearHistory => self.clear_history().await,
            ClipboardAction::Get => self.get_content().await,
            ClipboardAction::Set { content } => self.set_content(content).await,
        }
    }

    /// Toggle clipboard sharing
    async fn toggle_sharing(&self) -> CLIResult<ClipboardResult> {
        let is_monitoring = self.clipboard_system.is_monitoring();

        if is_monitoring {
            self.clipboard_system
                .stop_monitoring()
                .await
                .map_err(|e| CLIError::clipboard(format!("Failed to stop monitoring: {}", e)))?;

            Ok(ClipboardResult {
                success: true,
                message: "Clipboard sharing disabled".to_string(),
                status: Some(self.clipboard_system.get_status().await.map_err(|e| {
                    CLIError::clipboard(format!("Failed to get status: {}", e))
                })?),
                history: None,
                content: None,
            })
        } else {
            self.clipboard_system
                .start_monitoring()
                .await
                .map_err(|e| CLIError::clipboard(format!("Failed to start monitoring: {}", e)))?;

            Ok(ClipboardResult {
                success: true,
                message: "Clipboard sharing enabled".to_string(),
                status: Some(self.clipboard_system.get_status().await.map_err(|e| {
                    CLIError::clipboard(format!("Failed to get status: {}", e))
                })?),
                history: None,
                content: None,
            })
        }
    }

    /// Get clipboard status
    async fn get_status(&self) -> CLIResult<ClipboardResult> {
        let status = self
            .clipboard_system
            .get_status()
            .await
            .map_err(|e| CLIError::clipboard(format!("Failed to get status: {}", e)))?;

        let message = format!(
            "Clipboard Status:\n\
             - Monitoring: {}\n\
             - Sync Enabled: {}\n\
             - Privacy Filter: {}\n\
             - History: {} ({})\n\
             - Devices: {} ({} enabled)\n\
             - Connected Peers: {}\n\
             - Trusted Peers: {}\n\
             - Active Sessions: {}",
            if status.is_monitoring { "ON" } else { "OFF" },
            if status.sync_enabled { "ON" } else { "OFF" },
            if status.privacy_filter_enabled {
                "ON"
            } else {
                "OFF"
            },
            if status.history_enabled { "ON" } else { "OFF" },
            status.history_count,
            status.device_count,
            status.enabled_device_count,
            status.connected_peer_count,
            status.trusted_peer_count,
            status.active_session_count,
        );

        Ok(ClipboardResult {
            success: true,
            message,
            status: Some(status),
            history: None,
            content: None,
        })
    }

    /// Enable clipboard sync for a device
    async fn enable_device(&self, device_id: String) -> CLIResult<ClipboardResult> {
        self.clipboard_system
            .enable_sync_for_device(device_id.clone())
            .await
            .map_err(|e| CLIError::clipboard(format!("Failed to enable device: {}", e)))?;

        Ok(ClipboardResult {
            success: true,
            message: format!("Clipboard sync enabled for device: {}", device_id),
            status: None,
            history: None,
            content: None,
        })
    }

    /// Disable clipboard sync for a device
    async fn disable_device(&self, device_id: String) -> CLIResult<ClipboardResult> {
        self.clipboard_system
            .disable_sync_for_device(device_id.clone())
            .await
            .map_err(|e| CLIError::clipboard(format!("Failed to disable device: {}", e)))?;

        Ok(ClipboardResult {
            success: true,
            message: format!("Clipboard sync disabled for device: {}", device_id),
            status: None,
            history: None,
            content: None,
        })
    }

    /// Get clipboard history
    async fn get_history(&self, limit: usize) -> CLIResult<ClipboardResult> {
        let history = self
            .clipboard_system
            .get_history(limit)
            .await
            .map_err(|e| CLIError::clipboard(format!("Failed to get history: {}", e)))?;

        Ok(ClipboardResult {
            success: true,
            message: format!("Retrieved {} history entries", history.len()),
            status: None,
            history: Some(history),
            content: None,
        })
    }

    /// Search clipboard history
    async fn search_history(&self, query: String) -> CLIResult<ClipboardResult> {
        let history = self
            .clipboard_system
            .search_history(&query)
            .await
            .map_err(|e| CLIError::clipboard(format!("Failed to search history: {}", e)))?;

        Ok(ClipboardResult {
            success: true,
            message: format!("Found {} matching entries", history.len()),
            status: None,
            history: Some(history),
            content: None,
        })
    }

    /// Restore content from history
    async fn restore_from_history(&self, entry_id: Uuid) -> CLIResult<ClipboardResult> {
        self.clipboard_system
            .restore_from_history(entry_id)
            .await
            .map_err(|e| CLIError::clipboard(format!("Failed to restore from history: {}", e)))?;

        Ok(ClipboardResult {
            success: true,
            message: format!("Restored content from history entry: {}", entry_id),
            status: None,
            history: None,
            content: None,
        })
    }

    /// Clear clipboard history
    async fn clear_history(&self) -> CLIResult<ClipboardResult> {
        self.clipboard_system
            .clear_history()
            .await
            .map_err(|e| CLIError::clipboard(format!("Failed to clear history: {}", e)))?;

        Ok(ClipboardResult {
            success: true,
            message: "Clipboard history cleared".to_string(),
            status: None,
            history: None,
            content: None,
        })
    }

    /// Get current clipboard content
    async fn get_content(&self) -> CLIResult<ClipboardResult> {
        let content = self
            .clipboard_system
            .get_content()
            .await
            .map_err(|e| CLIError::clipboard(format!("Failed to get content: {}", e)))?;

        let message = if let Some(ref c) = content {
            format!("Clipboard content type: {:?}", c.content_type())
        } else {
            "Clipboard is empty".to_string()
        };

        Ok(ClipboardResult {
            success: true,
            message,
            status: None,
            history: None,
            content,
        })
    }

    /// Set clipboard content
    async fn set_content(&self, content: String) -> CLIResult<ClipboardResult> {
        let clipboard_content = ClipboardContent::Text(TextContent::new(content.clone()));

        self.clipboard_system
            .set_content(clipboard_content.clone())
            .await
            .map_err(|e| CLIError::clipboard(format!("Failed to set content: {}", e)))?;

        Ok(ClipboardResult {
            success: true,
            message: "Clipboard content set".to_string(),
            status: None,
            history: None,
            content: Some(clipboard_content),
        })
    }

    /// Get sync status for all devices
    pub async fn get_device_sync_status(&self) -> CLIResult<Vec<PeerInfo>> {
        let sync_status = self
            .clipboard_system
            .get_sync_status()
            .await
            .map_err(|e| CLIError::clipboard(format!("Failed to get sync status: {}", e)))?;

        let peers = sync_status
            .into_iter()
            .map(|status| PeerInfo {
                id: Uuid::new_v4(),
                name: status.device_name.clone(),
                device_type: status.device_name,
                connection_status: match status.connection_status {
                    crate::clipboard::ConnectionStatus::Connected => ConnectionStatus::Connected,
                    crate::clipboard::ConnectionStatus::Disconnected => {
                        ConnectionStatus::Disconnected
                    }
                    crate::clipboard::ConnectionStatus::Connecting => ConnectionStatus::Connecting,
                    crate::clipboard::ConnectionStatus::Error(_) => ConnectionStatus::Error,
                },
                capabilities: vec!["clipboard".to_string()],
                trust_status: if status.sync_enabled {
                    crate::cli::types::TrustStatus::Trusted
                } else {
                    crate::cli::types::TrustStatus::Untrusted
                },
                last_seen: status.last_sync.map(|st| chrono::DateTime::from(st)),
            })
            .collect();

        Ok(peers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::api::ClipboardSystemBuilder;
    use crate::clipboard::history::SqliteHistoryManager;
    use crate::clipboard::monitor::DefaultClipboardMonitor;
    use crate::security::SecuritySystem;
    use crate::transport::KizunaTransport;
    use tempfile::TempDir;

    async fn create_test_handler() -> (ClipboardHandler, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let transport = Arc::new(KizunaTransport::new().await.unwrap());
        let monitor = Arc::new(DefaultClipboardMonitor::new());

        let db_path = temp_dir
            .path()
            .join(format!("test_clipboard_{}.db", Uuid::new_v4()));
        let history_manager = Arc::new(SqliteHistoryManager::new(db_path, 50).unwrap());

        let clipboard_system = Arc::new(
            ClipboardSystemBuilder::new()
                .security_system(security_system)
                .transport(transport)
                .monitor(monitor)
                .history_manager(history_manager)
                .build()
                .unwrap(),
        );

        let handler = ClipboardHandler::new(clipboard_system);
        (handler, temp_dir)
    }

    #[tokio::test]
    async fn test_clipboard_handler_creation() {
        let (handler, _temp_dir) = create_test_handler().await;
        assert!(!handler.clipboard_system.is_monitoring());
    }

    #[tokio::test]
    async fn test_get_status() {
        let (handler, _temp_dir) = create_test_handler().await;
        let args = ClipboardArgs {
            action: ClipboardAction::Status,
            device_id: None,
        };

        let result = handler.handle_clipboard(args).await.unwrap();
        assert!(result.success);
        assert!(result.status.is_some());
    }

    #[tokio::test]
    async fn test_toggle_sharing() {
        let (handler, _temp_dir) = create_test_handler().await;

        // Enable sharing
        let args = ClipboardArgs {
            action: ClipboardAction::Share,
            device_id: None,
        };
        let result = handler.handle_clipboard(args).await.unwrap();
        assert!(result.success);
        assert!(handler.clipboard_system.is_monitoring());

        // Disable sharing
        let args = ClipboardArgs {
            action: ClipboardAction::Share,
            device_id: None,
        };
        let result = handler.handle_clipboard(args).await.unwrap();
        assert!(result.success);
        assert!(!handler.clipboard_system.is_monitoring());
    }

    #[tokio::test]
    async fn test_set_and_get_content() {
        let (handler, _temp_dir) = create_test_handler().await;

        // Set content
        let set_args = ClipboardArgs {
            action: ClipboardAction::Set {
                content: "test content".to_string(),
            },
            device_id: None,
        };
        let result = handler.handle_clipboard(set_args).await.unwrap();
        assert!(result.success);

        // Get content
        let get_args = ClipboardArgs {
            action: ClipboardAction::Get,
            device_id: None,
        };
        let result = handler.handle_clipboard(get_args).await.unwrap();
        assert!(result.success);
        assert!(result.content.is_some());
    }

    #[tokio::test]
    async fn test_get_history() {
        let (handler, _temp_dir) = create_test_handler().await;
        let args = ClipboardArgs {
            action: ClipboardAction::History { limit: 10 },
            device_id: None,
        };

        let result = handler.handle_clipboard(args).await.unwrap();
        assert!(result.success);
        assert!(result.history.is_some());
    }

    #[tokio::test]
    async fn test_clear_history() {
        let (handler, _temp_dir) = create_test_handler().await;
        let args = ClipboardArgs {
            action: ClipboardAction::ClearHistory,
            device_id: None,
        };

        let result = handler.handle_clipboard(args).await.unwrap();
        assert!(result.success);
    }
}
