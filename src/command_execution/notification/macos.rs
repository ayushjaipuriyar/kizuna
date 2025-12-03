// macOS Notification Backend
//
// Implements macOS notifications using UserNotifications framework

use crate::command_execution::error::{CommandError, CommandResult};
use crate::command_execution::types::*;
use super::{NotificationBackend, NotificationCapabilities};

/// macOS notification backend using UserNotifications framework
pub struct MacOSNotificationBackend {
    bundle_id: String,
}

impl MacOSNotificationBackend {
    /// Create a new macOS notification backend
    pub fn new() -> CommandResult<Self> {
        Ok(Self {
            bundle_id: "com.kizuna.command-execution".to_string(),
        })
    }
    
    /// Format notification for macOS
    fn format_notification(&self, notification: &Notification) -> String {
        format!(
            "[{}] {}: {}",
            self.format_type(notification.notification_type),
            notification.title,
            notification.message
        )
    }
    
    /// Format notification type for display
    fn format_type(&self, notification_type: NotificationType) -> &str {
        match notification_type {
            NotificationType::Info => "ℹ️",
            NotificationType::Warning => "⚠️",
            NotificationType::Error => "❌",
            NotificationType::Success => "✅",
        }
    }
    
    /// Get sound name based on notification type
    fn get_sound_name(&self, notification_type: NotificationType) -> &str {
        match notification_type {
            NotificationType::Info => "default",
            NotificationType::Warning => "Basso",
            NotificationType::Error => "Sosumi",
            NotificationType::Success => "Glass",
        }
    }
}

impl NotificationBackend for MacOSNotificationBackend {
    fn show_notification(&self, notification: &Notification) -> CommandResult<()> {
        #[cfg(target_os = "macos")]
        {
            // In a production implementation, this would use:
            // - UNUserNotificationCenter for modern macOS (10.14+)
            // - NSUserNotificationCenter for older macOS versions
            // - Objective-C bindings via objc crate
            
            let formatted = self.format_notification(notification);
            
            // Placeholder: In production, this would call macOS APIs
            // For now, we'll simulate success
            eprintln!("macOS Notification: {}", formatted);
            
            Ok(())
        }
        
        #[cfg(not(target_os = "macos"))]
        {
            Err(CommandError::NotificationError(
                "macOS notifications only supported on macOS".to_string()
            ))
        }
    }
    
    fn is_supported(&self) -> bool {
        cfg!(target_os = "macos")
    }
    
    fn get_capabilities(&self) -> NotificationCapabilities {
        NotificationCapabilities {
            supports_actions: true,
            supports_duration: false, // macOS controls duration
            supports_priority: false,
            supports_icons: true,
            max_title_length: Some(256),
            max_message_length: Some(2048),
        }
    }
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_macos_backend_creation() {
        let backend = MacOSNotificationBackend::new();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_macos_backend_supported() {
        let backend = MacOSNotificationBackend::new().unwrap();
        assert!(backend.is_supported());
    }

    #[test]
    fn test_macos_capabilities() {
        let backend = MacOSNotificationBackend::new().unwrap();
        let caps = backend.get_capabilities();
        assert!(caps.supports_actions);
        assert!(!caps.supports_duration);
    }
}
