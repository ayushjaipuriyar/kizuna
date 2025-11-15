// Integration tests for file transfer queue management

use kizuna::file_transfer::{
    queue::{QueueManagerImpl, QueueOperations, QueueScheduler},
    types::*,
};
use std::sync::Arc;
use tempfile::TempDir;

fn create_test_manifest() -> TransferManifest {
    TransferManifest::new("test_peer".to_string())
}

fn create_test_request() -> TransferRequest {
    TransferRequest {
        manifest: create_test_manifest(),
        peer_id: "peer1".to_string(),
        transport_preference: Some(TransportProtocol::Tcp),
        bandwidth_limit: None,
    }
}

#[tokio::test]
async fn test_queue_enqueue_and_get() {
    let temp_dir = TempDir::new().unwrap();
    let queue_manager = Arc::new(QueueManagerImpl::new(temp_dir.path().to_path_buf(), 4));
    queue_manager.initialize().await.unwrap();

    let request = create_test_request();
    let queue_id = queue_manager
        .enqueue_transfer(request, Priority::Normal)
        .await
        .unwrap();

    let item = queue_manager.get_queue_item(queue_id).await.unwrap();
    assert_eq!(item.queue_id, queue_id);
    assert_eq!(item.priority, Priority::Normal);
    assert_eq!(item.state, QueueState::Pending);
}

#[tokio::test]
async fn test_queue_priority_ordering() {
    let temp_dir = TempDir::new().unwrap();
    let queue_manager = Arc::new(QueueManagerImpl::new(temp_dir.path().to_path_buf(), 4));
    queue_manager.initialize().await.unwrap();

    // Enqueue items with different priorities
    let low_id = queue_manager
        .enqueue_transfer(create_test_request(), Priority::Low)
        .await
        .unwrap();

    let high_id = queue_manager
        .enqueue_transfer(create_test_request(), Priority::High)
        .await
        .unwrap();

    let normal_id = queue_manager
        .enqueue_transfer(create_test_request(), Priority::Normal)
        .await
        .unwrap();

    // Get pending items (should be ordered by priority)
    let pending = queue_manager.get_pending_items().await.unwrap();
    assert_eq!(pending.len(), 3);
    
    // High priority should be first
    assert_eq!(pending[0].queue_id, high_id);
    assert_eq!(pending[1].queue_id, normal_id);
    assert_eq!(pending[2].queue_id, low_id);
}

#[tokio::test]
async fn test_queue_cancel_item() {
    let temp_dir = TempDir::new().unwrap();
    let queue_manager = Arc::new(QueueManagerImpl::new(temp_dir.path().to_path_buf(), 4));
    queue_manager.initialize().await.unwrap();

    let queue_id = queue_manager
        .enqueue_transfer(create_test_request(), Priority::Normal)
        .await
        .unwrap();

    queue_manager.cancel_queue_item(queue_id).await.unwrap();

    let item = queue_manager.get_queue_item(queue_id).await.unwrap();
    assert_eq!(item.state, QueueState::Cancelled);
}

#[tokio::test]
async fn test_queue_scheduler() {
    let temp_dir = TempDir::new().unwrap();
    let queue_manager = Arc::new(QueueManagerImpl::new(temp_dir.path().to_path_buf(), 2));
    queue_manager.initialize().await.unwrap();

    let scheduler = Arc::new(QueueScheduler::new(queue_manager.clone(), 2));

    // Enqueue some items
    queue_manager
        .enqueue_transfer(create_test_request(), Priority::Normal)
        .await
        .unwrap();

    queue_manager
        .enqueue_transfer(create_test_request(), Priority::High)
        .await
        .unwrap();

    // Schedule next transfer
    let next_item = scheduler.schedule_next_transfer().await.unwrap();
    assert!(next_item.is_some());

    // Check that item was marked as scheduled
    let item = next_item.unwrap();
    let updated_item = queue_manager.get_queue_item(item.queue_id).await.unwrap();
    assert_eq!(updated_item.state, QueueState::Scheduled);
}

#[tokio::test]
async fn test_queue_operations_pause_resume() {
    let temp_dir = TempDir::new().unwrap();
    let queue_manager = Arc::new(QueueManagerImpl::new(temp_dir.path().to_path_buf(), 4));
    queue_manager.initialize().await.unwrap();

    let scheduler = Arc::new(QueueScheduler::new(queue_manager.clone(), 4));
    let operations = QueueOperations::new(queue_manager.clone(), scheduler);

    let queue_id = queue_manager
        .enqueue_transfer(create_test_request(), Priority::Normal)
        .await
        .unwrap();

    // Pause the item
    operations.pause_queue_item(queue_id).await.unwrap();
    let item = queue_manager.get_queue_item(queue_id).await.unwrap();
    assert_eq!(item.state, QueueState::Paused);

    // Resume the item
    operations.resume_queue_item(queue_id).await.unwrap();
    let item = queue_manager.get_queue_item(queue_id).await.unwrap();
    assert_eq!(item.state, QueueState::Pending);
}

#[tokio::test]
async fn test_queue_operations_change_priority() {
    let temp_dir = TempDir::new().unwrap();
    let queue_manager = Arc::new(QueueManagerImpl::new(temp_dir.path().to_path_buf(), 4));
    queue_manager.initialize().await.unwrap();

    let scheduler = Arc::new(QueueScheduler::new(queue_manager.clone(), 4));
    let operations = QueueOperations::new(queue_manager.clone(), scheduler);

    let queue_id = queue_manager
        .enqueue_transfer(create_test_request(), Priority::Normal)
        .await
        .unwrap();

    // Change priority
    operations
        .change_priority(queue_id, Priority::Urgent)
        .await
        .unwrap();

    let item = queue_manager.get_queue_item(queue_id).await.unwrap();
    assert_eq!(item.priority, Priority::Urgent);
}

#[tokio::test]
async fn test_queue_persistence() {
    let temp_dir = TempDir::new().unwrap();
    let persistence_dir = temp_dir.path().to_path_buf();

    let queue_id = {
        let queue_manager = Arc::new(QueueManagerImpl::new(persistence_dir.clone(), 4));
        queue_manager.initialize().await.unwrap();

        queue_manager
            .enqueue_transfer(create_test_request(), Priority::Normal)
            .await
            .unwrap()
    };

    // Create new manager and verify item was loaded
    let queue_manager = Arc::new(QueueManagerImpl::new(persistence_dir, 4));
    queue_manager.initialize().await.unwrap();

    let item = queue_manager.get_queue_item(queue_id).await.unwrap();
    assert_eq!(item.queue_id, queue_id);
    assert_eq!(item.state, QueueState::Pending);
}

#[tokio::test]
async fn test_queue_statistics() {
    let temp_dir = TempDir::new().unwrap();
    let queue_manager = Arc::new(QueueManagerImpl::new(temp_dir.path().to_path_buf(), 2));
    queue_manager.initialize().await.unwrap();

    let scheduler = Arc::new(QueueScheduler::new(queue_manager.clone(), 2));

    // Enqueue some items
    queue_manager
        .enqueue_transfer(create_test_request(), Priority::Normal)
        .await
        .unwrap();

    queue_manager
        .enqueue_transfer(create_test_request(), Priority::High)
        .await
        .unwrap();

    let stats = scheduler.get_queue_statistics().await;
    assert_eq!(stats.pending_count, 2);
    assert_eq!(stats.active_count, 0);
    assert_eq!(stats.available_slots, 2);
}
