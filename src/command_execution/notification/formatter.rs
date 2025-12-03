// Notification Formatting and Customization
//
// Provides notification formatting, styling, and customization capabilities

use crate::command_execution::error::{CommandError, CommandResult};
use crate::command_execution::types::*;
use std::time::Duration;

/// Notification formatter for styling and customization
pub struct NotificationFormatter {
    max_title_length: Option<usize>,
    max_message_length: Option<usize>,
}

impl NotificationFormatter {
    /// Create a new notification formatter
    pub fn new(max_title_length: Option<usize>, max_message_length: Option<usize>) -> Self {
        Self {
            max_title_length,
            max_message_length,
        }
    }
    
    /// Format a notification with appropriate styling
    pub fn format(&self, notification: &Notification) -> CommandResult<FormattedNotification> {
        let title = self.format_title(&notification.title, notification.notification_type)?;
        let message = self.format_message(&notification.message)?;
        let style = self.get_style(notification.notification_type, notification.priority);
        
        Ok(FormattedNotification {
            title,
            message,
            style,
            duration: notification.duration,
            actions: notification.actions.clone(),
        })
    }
    
    /// Format notification title with truncation if needed
    fn format_title(&self, title: &str, notification_type: NotificationType) -> CommandResult<String> {
        let prefix = self.get_type_prefix(notification_type);
        let formatted = format!("{} {}", prefix, title);
        
        if let Some(max_len) = self.max_title_length {
            if formatted.len() > max_len {
                Ok(format!("{}...", &formatted[..max_len.saturating_sub(3)]))
            } else {
                Ok(formatted)
            }
        } else {
            Ok(formatted)
        }
    }
    
    /// Format notification message with truncation if needed
    fn format_message(&self, message: &str) -> CommandResult<String> {
        if let Some(max_len) = self.max_message_length {
            if message.len() > max_len {
                Ok(format!("{}...", &message[..max_len.saturating_sub(3)]))
            } else {
                Ok(message.to_string())
            }
        } else {
            Ok(message.to_string())
        }
    }
    
    /// Get type prefix for notification
    fn get_type_prefix(&self, notification_type: NotificationType) -> &str {
        match notification_type {
            NotificationType::Info => "ℹ️",
            NotificationType::Warning => "⚠️",
            NotificationType::Error => "❌",
            NotificationType::Success => "✅",
        }
    }
    
    /// Get notification style based on type and priority
    fn get_style(&self, notification_type: NotificationType, priority: NotificationPriority) -> NotificationStyle {
        let color = match notification_type {
            NotificationType::Info => NotificationColor::Blue,
            NotificationType::Warning => NotificationColor::Yellow,
            NotificationType::Error => NotificationColor::Red,
            NotificationType::Success => NotificationColor::Green,
        };
        
        let urgency = match priority {
            NotificationPriority::Low => NotificationUrgency::Low,
            NotificationPriority::Normal => NotificationUrgency::Normal,
            NotificationPriority::High => NotificationUrgency::High,
            NotificationPriority::Critical => NotificationUrgency::Critical,
        };
        
        NotificationStyle {
            color,
            urgency,
            sound: self.should_play_sound(notification_type, priority),
        }
    }
    
    /// Determine if sound should be played
    fn should_play_sound(&self, notification_type: NotificationType, priority: NotificationPriority) -> bool {
        matches!(
            (notification_type, priority),
            (NotificationType::Error, _) |
            (NotificationType::Warning, NotificationPriority::High | NotificationPriority::Critical) |
            (_, NotificationPriority::Critical)
        )
    }
    
    /// Validate notification content
    pub fn validate(&self, notification: &Notification) -> CommandResult<()> {
        if notification.title.is_empty() {
            return Err(CommandError::InvalidRequest("Notification title cannot be empty".to_string()));
        }
        
        if notification.message.is_empty() {
            return Err(CommandError::InvalidRequest("Notification message cannot be empty".to_string()));
        }
        
        if let Some(max_len) = self.max_title_length {
            if notification.title.len() > max_len * 2 {
                return Err(CommandError::InvalidRequest(
                    format!("Notification title too long (max: {})", max_len)
                ));
            }
        }
        
        if let Some(max_len) = self.max_message_length {
            if notification.message.len() > max_len * 2 {
                return Err(CommandError::InvalidRequest(
                    format!("Notification message too long (max: {})", max_len)
                ));
            }
        }
        
        Ok(())
    }
}

/// Formatted notification ready for display
#[derive(Debug, Clone)]
pub struct FormattedNotification {
    pub title: String,
    pub message: String,
    pub style: NotificationStyle,
    pub duration: Option<Duration>,
    pub actions: Vec<NotificationAction>,
}

/// Notification visual style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NotificationStyle {
    pub color: NotificationColor,
    pub urgency: NotificationUrgency,
    pub sound: bool,
}

/// Notification color scheme
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationColor {
    Blue,
    Green,
    Yellow,
    Red,
}

/// Notification urgency level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationUrgency {
    Low,
    Normal,
    High,
    Critical,
}

/// Notification builder for easy customization
pub struct NotificationBuilder {
    title: String,
    message: String,
    notification_type: NotificationType,
    priority: NotificationPriority,
    duration: Option<Duration>,
    actions: Vec<NotificationAction>,
    sender: PeerId,
}

impl NotificationBuilder {
    /// Create a new notification builder
    pub fn new(title: impl Into<String>, message: impl Into<String>, sender: PeerId) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            notification_type: NotificationType::Info,
            priority: NotificationPriority::Normal,
            duration: None,
            actions: vec![],
            sender,
        }
    }
    
    /// Set notification type
    pub fn notification_type(mut self, notification_type: NotificationType) -> Self {
        self.notification_type = notification_type;
        self
    }
    
    /// Set notification priority
    pub fn priority(mut self, priority: NotificationPriority) -> Self {
        self.priority = priority;
        self
    }
    
    /// Set notification duration
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }
    
    /// Add an action button
    pub fn add_action(mut self, id: impl Into<String>, label: impl Into<String>) -> Self {
        self.actions.push(NotificationAction {
            id: id.into(),
            label: label.into(),
        });
        self
    }
    
    /// Build the notification
    pub fn build(self) -> Notification {
        Notification {
            notification_id: uuid::Uuid::new_v4(),
            title: self.title,
            message: self.message,
            notification_type: self.notification_type,
            priority: self.priority,
            duration: self.duration,
            actions: self.actions,
            sender: self.sender,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_formatter() {
        let formatter = NotificationFormatter::new(Some(50), Some(200));
        let notification = NotificationBuilder::new("Test", "Message", "peer".to_string())
            .notification_type(NotificationType::Info)
            .build();
        
        let formatted = formatter.format(&notification).unwrap();
        assert!(formatted.title.contains("Test"));
        assert_eq!(formatted.message, "Message");
    }

    #[test]
    fn test_notification_builder() {
        let notification = NotificationBuilder::new("Title", "Message", "peer".to_string())
            .notification_type(NotificationType::Warning)
            .priority(NotificationPriority::High)
            .duration(Duration::from_secs(5))
            .add_action("ok", "OK")
            .build();
        
        assert_eq!(notification.title, "Title");
        assert_eq!(notification.message, "Message");
        assert_eq!(notification.notification_type, NotificationType::Warning);
        assert_eq!(notification.priority, NotificationPriority::High);
        assert_eq!(notification.actions.len(), 1);
    }

    #[test]
    fn test_title_truncation() {
        let formatter = NotificationFormatter::new(Some(20), None);
        let long_title = "This is a very long title that should be truncated";
        let notification = NotificationBuilder::new(long_title, "Message", "peer".to_string()).build();
        
        let formatted = formatter.format(&notification).unwrap();
        assert!(formatted.title.len() <= 20);
        assert!(formatted.title.ends_with("..."));
    }

    #[test]
    fn test_validation() {
        let formatter = NotificationFormatter::new(Some(50), Some(200));
        
        let valid = NotificationBuilder::new("Title", "Message", "peer".to_string()).build();
        assert!(formatter.validate(&valid).is_ok());
        
        let empty_title = NotificationBuilder::new("", "Message", "peer".to_string()).build();
        assert!(formatter.validate(&empty_title).is_err());
    }
}
