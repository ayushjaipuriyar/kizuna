// Notification System for Cross-Device Messaging
//
// This module provides platform-specific notification delivery with support for
// Windows, macOS, and Linux desktop notification systems.

use crate::command_execution::error::{CommandError, CommandResult};
use crate::command_execution::types::*;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod linux;

pub mod formatter;
pub mod delivery;

#[cfg(test)]
mod integration_test;

pub use formatter::{
    NotificationFormatter, FormattedNotification, NotificationStyle,
    NotificationColor, NotificationUrgency, NotificationBuilder,
};
pub use delivery::{
    NotificationQueue, DeliveryTracker, DeliveryService,
    DeliveryInfo, DeliveryAnalytics,
};

/// Platform-specific notification backend trait
pub trait NotificationBackend: Send + Sync {
    /// Display a notification on the platform
    fn show_notification(&self, notification: &Notification) -> CommandResult<()>;
    
    /// Check if notifications are supported on this platform
    fn is_supported(&self) -> bool;
    
    /// Get platform-specific capabilities
    fn get_capabilities(&self) -> NotificationCapabilities;
}

/// Notification system capabilities
#[derive(Debug, Clone)]
pub struct NotificationCapabilities {
    pub supports_actions: bool,
    pub supports_duration: bool,
    pub supports_priority: bool,
    pub supports_icons: bool,
    pub max_title_length: Option<usize>,
    pub max_message_length: Option<usize>,
}

/// Notification manager for cross-device messaging
pub struct NotificationManager {
    backend: Box<dyn NotificationBackend>,
    formatter: NotificationFormatter,
    delivery_service: DeliveryService,
    notification_history: Arc<Mutex<Vec<NotificationRecord>>>,
    pending_notifications: Arc<Mutex<HashMap<NotificationId, Notification>>>,
    delivery_status: Arc<Mutex<HashMap<NotificationId, DeliveryStatus>>>,
}

/// Notification history record
#[derive(Debug, Clone)]
pub struct NotificationRecord {
    pub notification: Notification,
    pub delivered_at: Option<Timestamp>,
    pub status: DeliveryStatus,
    pub error_message: Option<String>,
}

/// Filter options for notification history queries
#[derive(Debug, Clone, Default)]
pub struct NotificationHistoryFilter {
    pub status: Option<DeliveryStatus>,
    pub sender: Option<PeerId>,
    pub notification_type: Option<NotificationType>,
    pub after: Option<Timestamp>,
    pub before: Option<Timestamp>,
}

/// Helper function to match delivery status
fn matches_status(actual: &DeliveryStatus, expected: &DeliveryStatus) -> bool {
    match (actual, expected) {
        (DeliveryStatus::Pending, DeliveryStatus::Pending) => true,
        (DeliveryStatus::Delivered, DeliveryStatus::Delivered) => true,
        (DeliveryStatus::Failed(_), DeliveryStatus::Failed(_)) => true,
        (DeliveryStatus::Cancelled, DeliveryStatus::Cancelled) => true,
        _ => false,
    }
}

impl NotificationManager {
    /// Create a new notification manager with platform-specific backend
    pub fn new() -> CommandResult<Self> {
        Self::with_retry_config(3, std::time::Duration::from_secs(5))
    }
    
    /// Create a new notification manager with custom retry configuration
    pub fn with_retry_config(max_retries: usize, retry_delay: std::time::Duration) -> CommandResult<Self> {
        let backend = Self::create_platform_backend()?;
        let capabilities = backend.get_capabilities();
        let formatter = NotificationFormatter::new(
            capabilities.max_title_length,
            capabilities.max_message_length,
        );
        let delivery_service = DeliveryService::new(max_retries, retry_delay);
        
        Ok(Self {
            backend,
            formatter,
            delivery_service,
            notification_history: Arc::new(Mutex::new(Vec::new())),
            pending_notifications: Arc::new(Mutex::new(HashMap::new())),
            delivery_status: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    /// Create the appropriate platform-specific backend
    fn create_platform_backend() -> CommandResult<Box<dyn NotificationBackend>> {
        #[cfg(target_os = "windows")]
        {
            Ok(Box::new(windows::WindowsNotificationBackend::new()?))
        }
        
        #[cfg(target_os = "macos")]
        {
            Ok(Box::new(macos::MacOSNotificationBackend::new()?))
        }
        
        #[cfg(target_os = "linux")]
        {
            Ok(Box::new(linux::LinuxNotificationBackend::new()?))
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Err(CommandError::NotificationError(
                "Notifications not supported on this platform".to_string()
            ))
        }
    }
    
    /// Send a notification to the local device with retry support
    pub async fn send_notification(
        &self,
        notification: Notification,
        target: PeerId,
    ) -> CommandResult<NotificationId> {
        let notification_id = notification.notification_id;
        
        // Validate notification
        self.formatter.validate(&notification)?;
        
        // Add to pending queue
        {
            let mut pending = self.pending_notifications.lock().unwrap();
            pending.insert(notification_id, notification.clone());
        }
        
        // Update status to pending
        {
            let mut status = self.delivery_status.lock().unwrap();
            status.insert(notification_id, DeliveryStatus::Pending);
        }
        
        // Queue notification for delivery with retry support
        self.delivery_service.queue_notification(notification.clone(), target.clone());
        
        // Attempt immediate delivery
        let backend = &self.backend;
        match backend.show_notification(&notification) {
            Ok(()) => {
                // Mark as delivered in tracker
                self.delivery_service.mark_delivered(notification_id);
                
                let delivered_at = Utc::now();
                {
                    let mut status = self.delivery_status.lock().unwrap();
                    status.insert(notification_id, DeliveryStatus::Delivered);
                }
                
                // Add to history
                {
                    let mut history = self.notification_history.lock().unwrap();
                    history.push(NotificationRecord {
                        notification: notification.clone(),
                        delivered_at: Some(delivered_at),
                        status: DeliveryStatus::Delivered,
                        error_message: None,
                    });
                }
                
                // Remove from pending
                {
                    let mut pending = self.pending_notifications.lock().unwrap();
                    pending.remove(&notification_id);
                }
                
                Ok(notification_id)
            }
            Err(e) => {
                // Mark as failed in tracker
                let error_msg = e.to_string();
                self.delivery_service.mark_failed(notification_id, error_msg.clone());
                
                {
                    let mut status = self.delivery_status.lock().unwrap();
                    status.insert(notification_id, DeliveryStatus::Failed(error_msg.clone()));
                }
                
                // Add to history with error
                {
                    let mut history = self.notification_history.lock().unwrap();
                    history.push(NotificationRecord {
                        notification: notification.clone(),
                        delivered_at: None,
                        status: DeliveryStatus::Failed(error_msg.clone()),
                        error_message: Some(error_msg.clone()),
                    });
                }
                
                // Keep in pending for retry via queue processing
                
                Err(e)
            }
        }
    }
    
    /// Process the notification delivery queue with retry logic
    /// This should be called periodically to retry failed deliveries
    pub async fn process_delivery_queue(&self) -> CommandResult<usize> {
        let backend = self.backend.as_ref();
        let notification_history = Arc::clone(&self.notification_history);
        let delivery_status = Arc::clone(&self.delivery_status);
        let pending_notifications = Arc::clone(&self.pending_notifications);
        
        self.delivery_service.process_queue(|notification, _target| {
            let notification_id = notification.notification_id;
            
            // Attempt delivery
            match backend.show_notification(&notification) {
                Ok(()) => {
                    // Mark as delivered
                    let delivered_at = Utc::now();
                    {
                        let mut status = delivery_status.lock().unwrap();
                        status.insert(notification_id, DeliveryStatus::Delivered);
                    }
                    
                    // Add to history
                    {
                        let mut history = notification_history.lock().unwrap();
                        history.push(NotificationRecord {
                            notification: notification.clone(),
                            delivered_at: Some(delivered_at),
                            status: DeliveryStatus::Delivered,
                            error_message: None,
                        });
                    }
                    
                    // Remove from pending
                    {
                        let mut pending = pending_notifications.lock().unwrap();
                        pending.remove(&notification_id);
                    }
                    
                    Ok(())
                }
                Err(e) => {
                    Err(e)
                }
            }
        }).await
    }
    
    /// Get the delivery status of a notification
    pub async fn get_delivery_status(
        &self,
        notification_id: NotificationId,
    ) -> CommandResult<DeliveryStatus> {
        let status = self.delivery_status.lock().unwrap();
        status
            .get(&notification_id)
            .cloned()
            .ok_or_else(|| CommandError::InvalidRequest("Notification not found".to_string()))
    }
    
    /// Cancel a pending notification
    pub async fn cancel_notification(
        &self,
        notification_id: NotificationId,
    ) -> CommandResult<()> {
        // Remove from pending
        {
            let mut pending = self.pending_notifications.lock().unwrap();
            pending.remove(&notification_id);
        }
        
        // Update status
        {
            let mut status = self.delivery_status.lock().unwrap();
            status.insert(notification_id, DeliveryStatus::Cancelled);
        }
        
        Ok(())
    }
    
    /// Get notification history
    pub async fn get_notification_history(&self) -> CommandResult<Vec<NotificationRecord>> {
        let history = self.notification_history.lock().unwrap();
        Ok(history.clone())
    }
    
    /// Get platform capabilities
    pub fn get_capabilities(&self) -> NotificationCapabilities {
        self.backend.get_capabilities()
    }
    
    /// Check if notifications are supported
    pub fn is_supported(&self) -> bool {
        self.backend.is_supported()
    }
    
    /// Get delivery analytics
    pub fn get_delivery_analytics(&self) -> DeliveryAnalytics {
        self.delivery_service.get_analytics()
    }
    
    /// Get pending queue size
    pub fn get_queue_size(&self) -> usize {
        self.delivery_service.queue_size()
    }
    
    /// Get detailed delivery information for a notification
    pub fn get_delivery_info(&self, notification_id: NotificationId) -> Option<DeliveryInfo> {
        self.delivery_service.get_delivery_info(notification_id)
    }
    
    /// Get all delivery information
    pub fn get_all_deliveries(&self) -> Vec<DeliveryInfo> {
        self.delivery_service.get_all_deliveries()
    }
    
    /// Get notification history with filtering options
    pub async fn get_notification_history_filtered(
        &self,
        filter: NotificationHistoryFilter,
    ) -> CommandResult<Vec<NotificationRecord>> {
        let history = self.notification_history.lock().unwrap();
        
        let filtered: Vec<NotificationRecord> = history
            .iter()
            .filter(|record| {
                // Filter by status
                if let Some(ref status) = filter.status {
                    if !matches_status(&record.status, status) {
                        return false;
                    }
                }
                
                // Filter by sender
                if let Some(ref sender) = filter.sender {
                    if &record.notification.sender != sender {
                        return false;
                    }
                }
                
                // Filter by notification type
                if let Some(ref ntype) = filter.notification_type {
                    if &record.notification.notification_type != ntype {
                        return false;
                    }
                }
                
                // Filter by date range
                if let Some(after) = filter.after {
                    if let Some(delivered_at) = record.delivered_at {
                        if delivered_at < after {
                            return false;
                        }
                    }
                }
                
                if let Some(before) = filter.before {
                    if let Some(delivered_at) = record.delivered_at {
                        if delivered_at > before {
                            return false;
                        }
                    }
                }
                
                true
            })
            .cloned()
            .collect();
        
        Ok(filtered)
    }
    
    /// Clean up old notification history records
    pub fn cleanup_old_history(&self, older_than: std::time::Duration) {
        let cutoff = Utc::now() - chrono::Duration::from_std(older_than).unwrap();
        
        {
            let mut history = self.notification_history.lock().unwrap();
            history.retain(|record| {
                if let Some(delivered_at) = record.delivered_at {
                    delivered_at > cutoff
                } else {
                    true // Keep records without delivery time
                }
            });
        }
        
        // Also cleanup delivery tracker records
        self.delivery_service.cleanup_old_records(older_than);
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new().expect("Failed to create notification manager")
    }
}

/// Helper function to create a notification
pub fn create_notification(
    title: impl Into<String>,
    message: impl Into<String>,
    notification_type: NotificationType,
    sender: PeerId,
) -> Notification {
    Notification {
        notification_id: Uuid::new_v4(),
        title: title.into(),
        message: message.into(),
        notification_type,
        priority: NotificationPriority::Normal,
        duration: None,
        actions: vec![],
        sender,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_notification() {
        let notification = create_notification(
            "Test Title",
            "Test Message",
            NotificationType::Info,
            "test-peer".to_string(),
        );
        
        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test Message");
        assert_eq!(notification.notification_type, NotificationType::Info);
        assert_eq!(notification.sender, "test-peer");
    }
}
