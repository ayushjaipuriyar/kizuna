// Integration tests for notification system
//
// These tests verify the complete notification workflow

#[cfg(test)]
mod tests {
    use crate::command_execution::notification::*;
    use crate::command_execution::types::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_notification_manager_creation() {
        let manager = NotificationManager::new();
        assert!(manager.is_ok());
        
        let manager = manager.unwrap();
        assert!(manager.is_supported());
    }

    #[tokio::test]
    async fn test_notification_builder() {
        let notification = NotificationBuilder::new(
            "Test Title",
            "Test Message",
            "test-peer".to_string()
        )
        .notification_type(NotificationType::Info)
        .priority(NotificationPriority::Normal)
        .duration(Duration::from_secs(5))
        .add_action("ok", "OK")
        .add_action("cancel", "Cancel")
        .build();

        assert_eq!(notification.title, "Test Title");
        assert_eq!(notification.message, "Test Message");
        assert_eq!(notification.notification_type, NotificationType::Info);
        assert_eq!(notification.priority, NotificationPriority::Normal);
        assert_eq!(notification.actions.len(), 2);
        assert_eq!(notification.duration, Some(Duration::from_secs(5)));
    }

    #[tokio::test]
    async fn test_notification_formatting() {
        let formatter = NotificationFormatter::new(Some(50), Some(200));
        
        let notification = NotificationBuilder::new(
            "Test",
            "Message",
            "peer".to_string()
        )
        .notification_type(NotificationType::Warning)
        .build();

        let formatted = formatter.format(&notification);
        assert!(formatted.is_ok());
        
        let formatted = formatted.unwrap();
        assert!(formatted.title.contains("Test"));
        assert_eq!(formatted.message, "Message");
    }

    #[tokio::test]
    async fn test_notification_validation() {
        let formatter = NotificationFormatter::new(Some(50), Some(200));
        
        // Valid notification
        let valid = NotificationBuilder::new("Title", "Message", "peer".to_string()).build();
        assert!(formatter.validate(&valid).is_ok());
        
        // Empty title
        let empty_title = NotificationBuilder::new("", "Message", "peer".to_string()).build();
        assert!(formatter.validate(&empty_title).is_err());
        
        // Empty message
        let empty_message = NotificationBuilder::new("Title", "", "peer".to_string()).build();
        assert!(formatter.validate(&empty_message).is_err());
    }

    #[tokio::test]
    async fn test_delivery_queue() {
        let queue = NotificationQueue::new(3, Duration::from_secs(1));
        
        let notification = create_notification(
            "Test",
            "Message",
            NotificationType::Info,
            "peer".to_string()
        );
        
        queue.enqueue(notification.clone(), "target".to_string());
        assert_eq!(queue.size(), 1);
        
        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(queue.size(), 0);
    }

    #[tokio::test]
    async fn test_delivery_tracker() {
        let tracker = DeliveryTracker::new();
        let notification_id = uuid::Uuid::new_v4();
        
        // Track attempt
        tracker.track_attempt(notification_id);
        let info = tracker.get_delivery_info(notification_id);
        assert!(info.is_some());
        assert_eq!(info.unwrap().attempts, 1);
        
        // Mark delivered
        tracker.mark_delivered(notification_id);
        let info = tracker.get_delivery_info(notification_id);
        assert!(matches!(info.unwrap().status, DeliveryStatus::Delivered));
        
        // Check analytics
        let analytics = tracker.get_analytics();
        assert_eq!(analytics.total_sent, 1);
        assert_eq!(analytics.total_delivered, 1);
    }

    #[tokio::test]
    async fn test_delivery_analytics() {
        let tracker = DeliveryTracker::new();
        
        // Simulate multiple deliveries
        for i in 0..5 {
            let id = uuid::Uuid::new_v4();
            tracker.track_attempt(id);
            
            if i < 3 {
                tracker.mark_delivered(id);
            } else {
                tracker.mark_failed(id, "Test error".to_string());
            }
        }
        
        let analytics = tracker.get_analytics();
        assert_eq!(analytics.total_sent, 5);
        assert_eq!(analytics.total_delivered, 3);
        assert_eq!(analytics.total_failed, 2);
    }

    #[tokio::test]
    async fn test_notification_manager_capabilities() {
        let manager = NotificationManager::new().unwrap();
        let capabilities = manager.get_capabilities();
        
        // Capabilities should be platform-specific
        #[cfg(target_os = "linux")]
        {
            assert!(capabilities.supports_actions);
            assert!(capabilities.supports_duration);
            assert!(capabilities.supports_priority);
        }
        
        #[cfg(target_os = "windows")]
        {
            assert!(capabilities.supports_actions);
            assert!(capabilities.supports_duration);
        }
        
        #[cfg(target_os = "macos")]
        {
            assert!(capabilities.supports_actions);
        }
    }
}
