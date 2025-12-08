//! Progressive Web App Controller
//! 
//! Manages PWA functionality including service workers, caching, and push notifications.

use crate::browser_support::{BrowserResult, types::AppManifest};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Offline operation for background sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineOperation {
    pub id: Option<u64>,
    pub operation_type: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
    pub status: String,
}

/// Cache entry for offline data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub data: serde_json::Value,
    pub timestamp: u64,
    pub expires_at: u64,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub cache_size: u64,
    pub entry_count: usize,
    pub max_cache_size: u64,
    pub max_cache_age: u64,
}

/// Storage quota information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageQuota {
    pub usage: u64,
    pub quota: u64,
    pub percentage: f64,
    pub available: u64,
}

/// Service worker registration info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceWorkerInfo {
    pub scope: String,
    pub state: String,
    pub script_url: String,
}

/// PWA controller for managing Progressive Web App features
pub struct PWAController {
    manifest: Option<AppManifest>,
    service_worker_registered: bool,
    cached_resources: Arc<RwLock<Vec<String>>>,
    offline_operations: Arc<RwLock<Vec<OfflineOperation>>>,
    settings: Arc<RwLock<HashMap<String, serde_json::Value>>>,
}

impl PWAController {
    /// Create a new PWA controller
    pub fn new() -> Self {
        Self {
            manifest: None,
            service_worker_registered: false,
            cached_resources: Arc::new(RwLock::new(Vec::new())),
            offline_operations: Arc::new(RwLock::new(Vec::new())),
            settings: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Initialize the PWA controller
    pub async fn initialize(&mut self) -> BrowserResult<()> {
        // Create default app manifest
        self.manifest = Some(self.create_default_manifest());
        
        // Initialize default cached resources
        let mut resources = self.cached_resources.write().await;
        *resources = self.get_default_cached_resources();
        
        Ok(())
    }
    
    /// Create default app manifest
    fn create_default_manifest(&self) -> AppManifest {
        AppManifest {
            name: "Kizuna".to_string(),
            short_name: "Kizuna".to_string(),
            description: "Peer-to-peer file sharing and communication".to_string(),
            start_url: "/".to_string(),
            display: crate::browser_support::types::DisplayMode::Standalone,
            theme_color: "#2196F3".to_string(),
            background_color: "#ffffff".to_string(),
            icons: vec![
                crate::browser_support::types::AppIcon {
                    src: "/icons/icon-192.png".to_string(),
                    sizes: "192x192".to_string(),
                    icon_type: "image/png".to_string(),
                    purpose: Some("any".to_string()),
                },
                crate::browser_support::types::AppIcon {
                    src: "/icons/icon-512.png".to_string(),
                    sizes: "512x512".to_string(),
                    icon_type: "image/png".to_string(),
                    purpose: Some("any".to_string()),
                },
            ],
            categories: vec!["productivity".to_string(), "utilities".to_string()],
        }
    }
    
    /// Get default resources to cache
    fn get_default_cached_resources(&self) -> Vec<String> {
        vec![
            "/".to_string(),
            "/index.html".to_string(),
            "/connect.html".to_string(),
            "/kizuna-sdk.js".to_string(),
            "/kizuna-ui.js".to_string(),
            "/kizuna-ui.css".to_string(),
            "/kizuna-file-transfer.js".to_string(),
            "/kizuna-clipboard.js".to_string(),
            "/kizuna-command.js".to_string(),
            "/kizuna-feature-detection.js".to_string(),
            "/kizuna-mobile.js".to_string(),
            "/kizuna-mobile.css".to_string(),
            "/kizuna-responsive.css".to_string(),
            "/kizuna-pwa.js".to_string(),
        ]
    }
    
    /// Get the app manifest
    pub fn get_manifest(&self) -> Option<&AppManifest> {
        self.manifest.as_ref()
    }
    
    /// Update the app manifest
    pub async fn update_manifest(&mut self, manifest: AppManifest) -> BrowserResult<()> {
        self.manifest = Some(manifest);
        Ok(())
    }
    
    /// Register service worker
    pub async fn register_service_worker(&mut self) -> BrowserResult<ServiceWorkerInfo> {
        self.service_worker_registered = true;
        
        Ok(ServiceWorkerInfo {
            scope: "/".to_string(),
            state: "activated".to_string(),
            script_url: "/service-worker.js".to_string(),
        })
    }
    
    /// Check if service worker is registered
    pub fn is_service_worker_registered(&self) -> bool {
        self.service_worker_registered
    }
    
    /// Cache resources
    pub async fn cache_resources(&self, resources: Vec<String>) -> BrowserResult<()> {
        let mut cached = self.cached_resources.write().await;
        
        for resource in resources {
            if !cached.contains(&resource) {
                cached.push(resource);
            }
        }
        
        Ok(())
    }
    
    /// Get cached resources
    pub async fn get_cached_resources(&self) -> Vec<String> {
        self.cached_resources.read().await.clone()
    }
    
    /// Queue operation for background sync
    pub async fn queue_operation(&self, operation: OfflineOperation) -> BrowserResult<u64> {
        let mut operations = self.offline_operations.write().await;
        
        let id = operations.len() as u64;
        let mut op = operation;
        op.id = Some(id);
        
        operations.push(op);
        
        Ok(id)
    }
    
    /// Get queued operations
    pub async fn get_queued_operations(&self) -> Vec<OfflineOperation> {
        self.offline_operations.read().await.clone()
    }
    
    /// Remove queued operation
    pub async fn remove_queued_operation(&self, id: u64) -> BrowserResult<()> {
        let mut operations = self.offline_operations.write().await;
        operations.retain(|op| op.id != Some(id));
        Ok(())
    }
    
    /// Clear all queued operations
    pub async fn clear_queued_operations(&self) -> BrowserResult<()> {
        let mut operations = self.offline_operations.write().await;
        operations.clear();
        Ok(())
    }
    
    /// Save setting
    pub async fn save_setting(&self, key: String, value: serde_json::Value) -> BrowserResult<()> {
        let mut settings = self.settings.write().await;
        settings.insert(key, value);
        Ok(())
    }
    
    /// Get setting
    pub async fn get_setting(&self, key: &str) -> Option<serde_json::Value> {
        let settings = self.settings.read().await;
        settings.get(key).cloned()
    }
    
    /// Get all settings
    pub async fn get_all_settings(&self) -> HashMap<String, serde_json::Value> {
        self.settings.read().await.clone()
    }
    
    /// Send push notification
    pub async fn send_push_notification(&self, notification: crate::browser_support::types::PushNotification) -> BrowserResult<()> {
        // In a real implementation, this would send the notification via a push service
        // For now, we'll just log it
        println!("Push notification: {} - {}", notification.title, notification.body);
        Ok(())
    }
    
    /// Create file transfer notification
    pub fn create_file_transfer_notification(file_name: &str, status: &str) -> crate::browser_support::types::PushNotification {
        let title = if status == "complete" {
            "File Transfer Complete".to_string()
        } else {
            format!("File Transfer {}", status)
        };
        
        crate::browser_support::types::PushNotification {
            title,
            body: file_name.to_string(),
            icon: Some("/icons/file-transfer.png".to_string()),
            badge: Some("/icons/badge-72.png".to_string()),
            tag: Some(format!("file-transfer-{}", chrono::Utc::now().timestamp())),
            data: Some(serde_json::json!({
                "type": "file-transfer",
                "fileName": file_name,
                "status": status,
            })),
            actions: if status == "complete" {
                vec![
                    crate::browser_support::types::NotificationAction {
                        action: "open".to_string(),
                        title: "Open".to_string(),
                        icon: None,
                    },
                    crate::browser_support::types::NotificationAction {
                        action: "dismiss".to_string(),
                        title: "Dismiss".to_string(),
                        icon: None,
                    },
                ]
            } else {
                vec![]
            },
            require_interaction: false,
            vibrate: Some(vec![200, 100, 200]),
        }
    }
    
    /// Create clipboard sync notification
    pub fn create_clipboard_notification(content: &str) -> crate::browser_support::types::PushNotification {
        let preview = if content.len() > 50 {
            format!("{}...", &content[..50])
        } else {
            content.to_string()
        };
        
        crate::browser_support::types::PushNotification {
            title: "Clipboard Synced".to_string(),
            body: preview,
            icon: Some("/icons/clipboard.png".to_string()),
            badge: Some("/icons/badge-72.png".to_string()),
            tag: Some("clipboard-sync".to_string()),
            data: Some(serde_json::json!({
                "type": "clipboard-sync",
                "content": content,
            })),
            actions: vec![],
            require_interaction: false,
            vibrate: Some(vec![200, 100, 200]),
        }
    }
    
    /// Create peer connection notification
    pub fn create_peer_connection_notification(peer_name: &str, status: &str) -> crate::browser_support::types::PushNotification {
        let title = if status == "connected" {
            "Peer Connected".to_string()
        } else {
            "Peer Disconnected".to_string()
        };
        
        crate::browser_support::types::PushNotification {
            title,
            body: peer_name.to_string(),
            icon: Some("/icons/peer.png".to_string()),
            badge: Some("/icons/badge-72.png".to_string()),
            tag: Some(format!("peer-{}", chrono::Utc::now().timestamp())),
            data: Some(serde_json::json!({
                "type": "peer-connection",
                "peerName": peer_name,
                "status": status,
            })),
            actions: vec![],
            require_interaction: false,
            vibrate: Some(vec![200, 100, 200]),
        }
    }
    
    /// Get cache statistics
    pub async fn get_cache_statistics(&self) -> CacheStatistics {
        let resources = self.cached_resources.read().await;
        
        CacheStatistics {
            cache_size: 0, // Would be calculated from actual cache
            entry_count: resources.len(),
            max_cache_size: 50 * 1024 * 1024, // 50 MB
            max_cache_age: 7 * 24 * 60 * 60 * 1000, // 7 days in ms
        }
    }
    
    /// Invalidate cache entry
    pub async fn invalidate_cache(&self, key: &str) -> BrowserResult<bool> {
        // In a real implementation, this would communicate with the service worker
        println!("Cache invalidation requested for: {}", key);
        Ok(true)
    }
    
    /// Clear all caches
    pub async fn clear_all_caches(&self) -> BrowserResult<usize> {
        let mut resources = self.cached_resources.write().await;
        let count = resources.len();
        resources.clear();
        
        println!("Cleared {} cached resources", count);
        Ok(count)
    }
    
    /// Prune cache to fit within size limit
    pub async fn prune_cache(&self) -> BrowserResult<usize> {
        // In a real implementation, this would prune old cache entries
        println!("Cache pruning requested");
        Ok(0)
    }
    
    /// Request persistent storage
    pub async fn request_persistent_storage(&self) -> BrowserResult<bool> {
        // In a real implementation, this would request persistent storage
        println!("Persistent storage requested");
        Ok(true)
    }
    
    /// Shutdown the PWA controller
    pub async fn shutdown(&mut self) -> BrowserResult<()> {
        self.manifest = None;
        self.service_worker_registered = false;
        
        let mut resources = self.cached_resources.write().await;
        resources.clear();
        
        let mut operations = self.offline_operations.write().await;
        operations.clear();
        
        let mut settings = self.settings.write().await;
        settings.clear();
        
        Ok(())
    }
}

impl Default for PWAController {
    fn default() -> Self {
        Self::new()
    }
}