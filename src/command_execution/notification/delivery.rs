// Notification Delivery and Status Tracking
//
// Provides notification queue management, retry logic, and delivery analytics

use crate::command_execution::error::{CommandError, CommandResult};
use crate::command_execution::types::*;
use chrono::Utc;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time::sleep;

/// Notification delivery queue with retry logic
pub struct NotificationQueue {
    queue: Arc<Mutex<VecDeque<QueuedNotification>>>,
    max_retries: usize,
    retry_delay: Duration,
}

/// Queued notification with retry information
#[derive(Debug, Clone)]
struct QueuedNotification {
    notification: Notification,
    target: PeerId,
    attempts: usize,
    last_attempt: Option<Timestamp>,
    error: Option<String>,
}

impl NotificationQueue {
    /// Create a new notification queue
    pub fn new(max_retries: usize, retry_delay: Duration) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            max_retries,
            retry_delay,
        }
    }
    
    /// Add a notification to the queue
    pub fn enqueue(&self, notification: Notification, target: PeerId) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(QueuedNotification {
            notification,
            target,
            attempts: 0,
            last_attempt: None,
            error: None,
        });
    }
    
    /// Get the next notification to deliver
    pub fn dequeue(&self) -> Option<(Notification, PeerId)> {
        let mut queue = self.queue.lock().unwrap();
        if let Some(queued) = queue.pop_front() {
            Some((queued.notification, queued.target))
        } else {
            None
        }
    }
    
    /// Re-queue a failed notification for retry
    pub fn requeue(&self, notification: Notification, target: PeerId, error: String) -> bool {
        let mut queue = self.queue.lock().unwrap();
        
        // Find if this notification was already queued
        if let Some(queued) = queue.iter_mut().find(|q| q.notification.notification_id == notification.notification_id) {
            queued.attempts += 1;
            queued.last_attempt = Some(Utc::now());
            queued.error = Some(error);
            
            if queued.attempts >= self.max_retries {
                // Remove from queue if max retries reached
                queue.retain(|q| q.notification.notification_id != notification.notification_id);
                return false;
            }
            true
        } else {
            // Add as new queued notification
            queue.push_back(QueuedNotification {
                notification,
                target,
                attempts: 1,
                last_attempt: Some(Utc::now()),
                error: Some(error),
            });
            true
        }
    }
    
    /// Get queue size
    pub fn size(&self) -> usize {
        let queue = self.queue.lock().unwrap();
        queue.len()
    }
    
    /// Clear the queue
    pub fn clear(&self) {
        let mut queue = self.queue.lock().unwrap();
        queue.clear();
    }
    
    /// Get retry delay
    pub fn get_retry_delay(&self) -> Duration {
        self.retry_delay
    }
}

/// Notification delivery tracker
pub struct DeliveryTracker {
    deliveries: Arc<Mutex<HashMap<NotificationId, DeliveryInfo>>>,
    analytics: Arc<Mutex<DeliveryAnalytics>>,
}

/// Delivery information for a notification
#[derive(Debug, Clone)]
pub struct DeliveryInfo {
    pub notification_id: NotificationId,
    pub status: DeliveryStatus,
    pub attempts: usize,
    pub first_attempt: Timestamp,
    pub last_attempt: Option<Timestamp>,
    pub delivered_at: Option<Timestamp>,
    pub error_message: Option<String>,
}

/// Delivery analytics and statistics
#[derive(Debug, Clone, Default)]
pub struct DeliveryAnalytics {
    pub total_sent: usize,
    pub total_delivered: usize,
    pub total_failed: usize,
    pub total_cancelled: usize,
    pub average_delivery_time: Duration,
    pub retry_rate: f64,
}

impl DeliveryTracker {
    /// Create a new delivery tracker
    pub fn new() -> Self {
        Self {
            deliveries: Arc::new(Mutex::new(HashMap::new())),
            analytics: Arc::new(Mutex::new(DeliveryAnalytics::default())),
        }
    }
    
    /// Track a new notification delivery attempt
    pub fn track_attempt(&self, notification_id: NotificationId) {
        let mut deliveries = self.deliveries.lock().unwrap();
        
        if let Some(info) = deliveries.get_mut(&notification_id) {
            info.attempts += 1;
            info.last_attempt = Some(Utc::now());
        } else {
            deliveries.insert(notification_id, DeliveryInfo {
                notification_id,
                status: DeliveryStatus::Pending,
                attempts: 1,
                first_attempt: Utc::now(),
                last_attempt: None,
                delivered_at: None,
                error_message: None,
            });
        }
        
        // Update analytics
        let mut analytics = self.analytics.lock().unwrap();
        analytics.total_sent += 1;
    }
    
    /// Mark a notification as delivered
    pub fn mark_delivered(&self, notification_id: NotificationId) {
        let mut deliveries = self.deliveries.lock().unwrap();
        
        if let Some(info) = deliveries.get_mut(&notification_id) {
            info.status = DeliveryStatus::Delivered;
            info.delivered_at = Some(Utc::now());
            
            // Update analytics
            let mut analytics = self.analytics.lock().unwrap();
            analytics.total_delivered += 1;
            
            // Calculate delivery time
            let delivery_time = info.delivered_at.unwrap()
                .signed_duration_since(info.first_attempt)
                .to_std()
                .unwrap_or(Duration::from_secs(0));
            
            // Update average delivery time
            let total = analytics.total_delivered as f64;
            let current_avg = analytics.average_delivery_time.as_secs_f64();
            let new_avg = (current_avg * (total - 1.0) + delivery_time.as_secs_f64()) / total;
            analytics.average_delivery_time = Duration::from_secs_f64(new_avg);
            
            // Update retry rate
            if info.attempts > 1 {
                analytics.retry_rate = (analytics.retry_rate * (total - 1.0) + 1.0) / total;
            }
        }
    }
    
    /// Mark a notification as failed
    pub fn mark_failed(&self, notification_id: NotificationId, error: String) {
        let mut deliveries = self.deliveries.lock().unwrap();
        
        if let Some(info) = deliveries.get_mut(&notification_id) {
            info.status = DeliveryStatus::Failed(error.clone());
            info.error_message = Some(error);
            
            // Update analytics
            let mut analytics = self.analytics.lock().unwrap();
            analytics.total_failed += 1;
        }
    }
    
    /// Mark a notification as cancelled
    pub fn mark_cancelled(&self, notification_id: NotificationId) {
        let mut deliveries = self.deliveries.lock().unwrap();
        
        if let Some(info) = deliveries.get_mut(&notification_id) {
            info.status = DeliveryStatus::Cancelled;
            
            // Update analytics
            let mut analytics = self.analytics.lock().unwrap();
            analytics.total_cancelled += 1;
        }
    }
    
    /// Get delivery information for a notification
    pub fn get_delivery_info(&self, notification_id: NotificationId) -> Option<DeliveryInfo> {
        let deliveries = self.deliveries.lock().unwrap();
        deliveries.get(&notification_id).cloned()
    }
    
    /// Get delivery analytics
    pub fn get_analytics(&self) -> DeliveryAnalytics {
        let analytics = self.analytics.lock().unwrap();
        analytics.clone()
    }
    
    /// Get all delivery information
    pub fn get_all_deliveries(&self) -> Vec<DeliveryInfo> {
        let deliveries = self.deliveries.lock().unwrap();
        deliveries.values().cloned().collect()
    }
    
    /// Clear old delivery records
    pub fn cleanup_old_records(&self, older_than: Duration) {
        let mut deliveries = self.deliveries.lock().unwrap();
        let cutoff = Utc::now() - chrono::Duration::from_std(older_than).unwrap();
        
        deliveries.retain(|_, info| {
            info.first_attempt > cutoff
        });
    }
}

impl Default for DeliveryTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Notification delivery service with retry logic
pub struct DeliveryService {
    queue: NotificationQueue,
    tracker: DeliveryTracker,
}

impl DeliveryService {
    /// Create a new delivery service
    pub fn new(max_retries: usize, retry_delay: Duration) -> Self {
        Self {
            queue: NotificationQueue::new(max_retries, retry_delay),
            tracker: DeliveryTracker::new(),
        }
    }
    
    /// Queue a notification for delivery
    pub fn queue_notification(&self, notification: Notification, target: PeerId) {
        self.queue.enqueue(notification, target);
    }
    
    /// Process the delivery queue (should be called periodically)
    pub async fn process_queue<F>(&self, mut deliver_fn: F) -> CommandResult<usize>
    where
        F: FnMut(Notification, PeerId) -> CommandResult<()>,
    {
        let mut processed = 0;
        
        while let Some((notification, target)) = self.queue.dequeue() {
            let notification_id = notification.notification_id;
            
            // Track attempt
            self.tracker.track_attempt(notification_id);
            
            // Attempt delivery
            match deliver_fn(notification.clone(), target.clone()) {
                Ok(()) => {
                    self.tracker.mark_delivered(notification_id);
                    processed += 1;
                }
                Err(e) => {
                    let error_msg = e.to_string();
                    
                    // Try to requeue for retry
                    if self.queue.requeue(notification, target, error_msg.clone()) {
                        // Will retry later
                        sleep(self.queue.get_retry_delay()).await;
                    } else {
                        // Max retries reached
                        self.tracker.mark_failed(notification_id, error_msg);
                    }
                }
            }
        }
        
        Ok(processed)
    }
    
    /// Get delivery status
    pub fn get_delivery_status(&self, notification_id: NotificationId) -> Option<DeliveryStatus> {
        self.tracker.get_delivery_info(notification_id)
            .map(|info| info.status)
    }
    
    /// Get delivery analytics
    pub fn get_analytics(&self) -> DeliveryAnalytics {
        self.tracker.get_analytics()
    }
    
    /// Get queue size
    pub fn queue_size(&self) -> usize {
        self.queue.size()
    }
    
    /// Get delivery information for a notification
    pub fn get_delivery_info(&self, notification_id: NotificationId) -> Option<DeliveryInfo> {
        self.tracker.get_delivery_info(notification_id)
    }
    
    /// Get all delivery information
    pub fn get_all_deliveries(&self) -> Vec<DeliveryInfo> {
        self.tracker.get_all_deliveries()
    }
    
    /// Mark a notification as delivered
    pub fn mark_delivered(&self, notification_id: NotificationId) {
        self.tracker.mark_delivered(notification_id);
    }
    
    /// Mark a notification as failed
    pub fn mark_failed(&self, notification_id: NotificationId, error: String) {
        self.tracker.mark_failed(notification_id, error);
    }
    
    /// Clean up old delivery records
    pub fn cleanup_old_records(&self, older_than: Duration) {
        self.tracker.cleanup_old_records(older_than);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_notification() -> Notification {
        Notification {
            notification_id: Uuid::new_v4(),
            title: "Test".to_string(),
            message: "Test message".to_string(),
            notification_type: NotificationType::Info,
            priority: NotificationPriority::Normal,
            duration: None,
            actions: vec![],
            sender: "test-peer".to_string(),
        }
    }

    #[test]
    fn test_notification_queue() {
        let queue = NotificationQueue::new(3, Duration::from_secs(1));
        let notification = create_test_notification();
        
        queue.enqueue(notification.clone(), "target".to_string());
        assert_eq!(queue.size(), 1);
        
        let dequeued = queue.dequeue();
        assert!(dequeued.is_some());
        assert_eq!(queue.size(), 0);
    }

    #[test]
    fn test_delivery_tracker() {
        let tracker = DeliveryTracker::new();
        let notification_id = Uuid::new_v4();
        
        tracker.track_attempt(notification_id);
        let info = tracker.get_delivery_info(notification_id);
        assert!(info.is_some());
        assert_eq!(info.unwrap().attempts, 1);
        
        tracker.mark_delivered(notification_id);
        let info = tracker.get_delivery_info(notification_id);
        assert!(matches!(info.unwrap().status, DeliveryStatus::Delivered));
    }

    #[test]
    fn test_delivery_analytics() {
        let tracker = DeliveryTracker::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        
        tracker.track_attempt(id1);
        tracker.mark_delivered(id1);
        
        tracker.track_attempt(id2);
        tracker.mark_failed(id2, "Test error".to_string());
        
        let analytics = tracker.get_analytics();
        assert_eq!(analytics.total_sent, 2);
        assert_eq!(analytics.total_delivered, 1);
        assert_eq!(analytics.total_failed, 1);
    }
}
