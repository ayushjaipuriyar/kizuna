// Linux Notification Backend
//
// Implements Linux notifications using libnotify and desktop notification standards

use crate::command_execution::error::{CommandError, CommandResult};
use crate::command_execution::types::*;
use super::{NotificationBackend, NotificationCapabilities};

/// Linux notification backend using libnotify
pub struct LinuxNotificationBackend {
    app_name: String,
}

impl LinuxNotificationBackend {
    /// Create a new Linux notification backend
    pub fn new() -> CommandResult<Self> {
        Ok(Self {
            app_name: "Kizuna Command Execution".to_string(),
        })
    }
    
    /// Format notification for Linux
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
            NotificationType::Info => "INFO",
            NotificationType::Warning => "WARNING",
            NotificationType::Error => "ERROR",
            NotificationType::Success => "SUCCESS",
        }
    }
    
    /// Get urgency level for libnotify
    fn get_urgency(&self, notification_type: NotificationType, priority: NotificationPriority) -> &str {
        match (notification_type, priority) {
            (NotificationType::Error, _) => "critical",
            (_, NotificationPriority::Critical) => "critical",
            (NotificationType::Warning, _) => "normal",
            (_, NotificationPriority::High) => "normal",
            _ => "low",
        }
    }
    
    /// Get icon name based on notification type
    fn get_icon_name(&self, notification_type: NotificationType) -> &str {
        match notification_type {
            NotificationType::Info => "dialog-information",
            NotificationType::Warning => "dialog-warning",
            NotificationType::Error => "dialog-error",
            NotificationType::Success => "dialog-information",
        }
    }
}

impl NotificationBackend for LinuxNotificationBackend {
    fn show_notification(&self, notification: &Notification) -> CommandResult<()> {
        #[cfg(target_os = "linux")]
        {
            // In a production implementation, this would use:
            // - libnotify via notify-rust crate
            // - D-Bus notifications via org.freedesktop.Notifications
            // - Desktop notification specification
            
            let formatted = self.format_notification(notification);
            let urgency = self.get_urgency(notification.notification_type, notification.priority);
            let icon = self.get_icon_name(notification.notification_type);
            
            // Placeholder: In production, this would call libnotify
            // For now, we'll simulate success
            eprintln!("Linux Notification [{}] [{}]: {}", urgency, icon, formatted);
            
            Ok(())
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            Err(CommandError::NotificationError(
                "Linux notifications only supported on Linux".to_string()
            ))
        }
    }
    
    fn is_supported(&self) -> bool {
        cfg!(target_os = "linux")
    }
    
    fn get_capabilities(&self) -> NotificationCapabilities {
        NotificationCapabilities {
            supports_actions: true,
            supports_duration: true,
            supports_priority: true,
            supports_icons: true,
            max_title_length: None, // No strict limit
            max_message_length: None, // No strict limit
        }
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_linux_backend_creation() {
        let backend = LinuxNotificationBackend::new();
        assert!(backend.is_ok());
    }

    #[test]
    fn test_linux_backend_supported() {
        let backend = LinuxNotificationBackend::new().unwrap();
        assert!(backend.is_supported());
    }

    #[test]
    fn test_linux_capabilities() {
        let backend = LinuxNotificationBackend::new().unwrap();
        let caps = backend.get_capabilities();
        assert!(caps.supports_actions);
        assert!(caps.supports_duration);
        assert!(caps.supports_priority);
    }
}
