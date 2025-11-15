// Queue Management Module
//
// Handles transfer queue, scheduling, and prioritization

use crate::file_transfer::{
    error::{FileTransferError, Result},
    types::*,
};
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Queue item wrapper for priority queue ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PriorityQueueItem {
    item: QueueItem,
}

impl PartialEq for PriorityQueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.item.queue_id == other.item.queue_id
    }
}

impl Eq for PriorityQueueItem {}

impl PartialOrd for PriorityQueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityQueueItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority comes first
        match self.item.priority.cmp(&other.item.priority) {
            std::cmp::Ordering::Equal => {
                // If priorities are equal, earlier created_at comes first (FIFO)
                other.item.created_at.cmp(&self.item.created_at)
            }
            ordering => ordering,
        }
    }
}

/// Queue manager implementation
pub struct QueueManagerImpl {
    /// Priority queue for pending transfers
    queue: Arc<RwLock<BinaryHeap<PriorityQueueItem>>>,
    /// Map of queue items by ID for quick lookup
    pub items: Arc<RwLock<HashMap<QueueId, QueueItem>>>,
    /// Queue persistence directory
    persistence_dir: PathBuf,
    /// Maximum concurrent transfers
    max_concurrent: usize,
    /// Currently active transfer count
    active_count: Arc<RwLock<usize>>,
}

impl QueueManagerImpl {
    /// Create a new queue manager with persistence directory
    pub fn new(persistence_dir: PathBuf, max_concurrent: usize) -> Self {
        Self {
            queue: Arc::new(RwLock::new(BinaryHeap::new())),
            items: Arc::new(RwLock::new(HashMap::new())),
            persistence_dir,
            max_concurrent,
            active_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Initialize queue manager and load persisted queue items
    pub async fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.persistence_dir)
            .await
            .map_err(|e| FileTransferError::IoError {
                path: self.persistence_dir.clone(),
                source: e,
            })?;

        self.load_persisted_queue().await?;
        Ok(())
    }

    /// Enqueue a transfer request with priority
    pub async fn enqueue_transfer(
        &self,
        request: TransferRequest,
        priority: Priority,
    ) -> Result<QueueId> {
        let queue_id = Uuid::new_v4();
        let created_at = current_timestamp();

        let queue_item = QueueItem {
            queue_id,
            transfer_request: request,
            priority,
            estimated_start: None,
            state: QueueState::Pending,
            created_at,
        };

        let mut queue = self.queue.write().await;
        let mut items = self.items.write().await;

        queue.push(PriorityQueueItem {
            item: queue_item.clone(),
        });
        items.insert(queue_id, queue_item.clone());

        drop(queue);
        drop(items);

        self.persist_queue_item(&queue_item).await?;
        self.update_estimated_start_times().await?;

        Ok(queue_id)
    }

    /// Get queue item by ID
    pub async fn get_queue_item(&self, queue_id: QueueId) -> Result<QueueItem> {
        let items = self.items.read().await;
        items
            .get(&queue_id)
            .cloned()
            .ok_or_else(|| FileTransferError::QueueItemNotFound {
                queue_id: queue_id.to_string(),
            })
    }

    /// Update queue item state
    pub async fn update_queue_item_state(
        &self,
        queue_id: QueueId,
        new_state: QueueState,
    ) -> Result<()> {
        let mut items = self.items.write().await;

        if let Some(item) = items.get_mut(&queue_id) {
            item.state = new_state;
            self.persist_queue_item(item).await?;
            Ok(())
        } else {
            Err(FileTransferError::QueueItemNotFound {
                queue_id: queue_id.to_string(),
            })
        }
    }

    /// Modify queue item priority
    pub async fn modify_queue_item_priority(
        &self,
        queue_id: QueueId,
        new_priority: Priority,
    ) -> Result<()> {
        let mut queue = self.queue.write().await;
        let mut items = self.items.write().await;

        if let Some(item) = items.get_mut(&queue_id) {
            item.priority = new_priority;

            let all_items: Vec<PriorityQueueItem> = queue.drain().collect();
            for mut pq_item in all_items {
                if pq_item.item.queue_id == queue_id {
                    pq_item.item.priority = new_priority;
                }
                queue.push(pq_item);
            }

            self.persist_queue_item(item).await?;
            Ok(())
        } else {
            Err(FileTransferError::QueueItemNotFound {
                queue_id: queue_id.to_string(),
            })
        }
    }

    /// Cancel a queue item
    pub async fn cancel_queue_item(&self, queue_id: QueueId) -> Result<()> {
        self.update_queue_item_state(queue_id, QueueState::Cancelled)
            .await?;
        self.remove_from_queue(queue_id).await?;
        Ok(())
    }

    /// Remove item from queue
    pub async fn remove_from_queue(&self, queue_id: QueueId) -> Result<()> {
        let mut queue = self.queue.write().await;
        let all_items: Vec<PriorityQueueItem> = queue.drain().collect();
        for pq_item in all_items {
            if pq_item.item.queue_id != queue_id {
                queue.push(pq_item);
            }
        }
        Ok(())
    }

    /// Get all queue items
    pub async fn get_all_queue_items(&self) -> Result<Vec<QueueItem>> {
        let items = self.items.read().await;
        Ok(items.values().cloned().collect())
    }

    /// Get queue items by state
    pub async fn get_queue_items_by_state(&self, state: QueueState) -> Result<Vec<QueueItem>> {
        let items = self.items.read().await;
        Ok(items
            .values()
            .filter(|item| item.state == state)
            .cloned()
            .collect())
    }

    /// Get pending queue items in priority order
    pub async fn get_pending_items(&self) -> Result<Vec<QueueItem>> {
        let queue = self.queue.read().await;
        let items = self.items.read().await;

        let mut pending: Vec<QueueItem> = queue
            .iter()
            .filter(|pq_item| {
                items
                    .get(&pq_item.item.queue_id)
                    .map(|item| item.state == QueueState::Pending)
                    .unwrap_or(false)
            })
            .map(|pq_item| pq_item.item.clone())
            .collect();

        pending.sort_by(|a, b| {
            match b.priority.cmp(&a.priority) {
                std::cmp::Ordering::Equal => a.created_at.cmp(&b.created_at),
                ordering => ordering,
            }
        });

        Ok(pending)
    }

    /// Get next item to process from queue
    pub async fn get_next_item(&self) -> Result<Option<QueueItem>> {
        let mut queue = self.queue.write().await;
        let items = self.items.read().await;

        while let Some(pq_item) = queue.pop() {
            if let Some(item) = items.get(&pq_item.item.queue_id) {
                if item.state == QueueState::Pending {
                    queue.push(pq_item.clone());
                    return Ok(Some(item.clone()));
                }
            }
        }

        Ok(None)
    }

    /// Mark item as scheduled
    pub async fn mark_item_scheduled(&self, queue_id: QueueId) -> Result<()> {
        self.update_queue_item_state(queue_id, QueueState::Scheduled)
            .await?;
        let mut active_count = self.active_count.write().await;
        *active_count += 1;
        Ok(())
    }

    /// Mark item as completed
    pub async fn mark_item_completed(&self, queue_id: QueueId) -> Result<()> {
        self.remove_from_queue(queue_id).await?;
        let mut active_count = self.active_count.write().await;
        *active_count = active_count.saturating_sub(1);
        self.delete_persisted_queue_item(queue_id).await?;
        let mut items = self.items.write().await;
        items.remove(&queue_id);
        Ok(())
    }

    /// Get current active transfer count
    pub async fn get_active_count(&self) -> usize {
        *self.active_count.read().await
    }

    /// Get maximum concurrent transfers
    pub fn get_max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Check if queue has capacity
    pub async fn has_capacity(&self) -> bool {
        let active_count = self.get_active_count().await;
        active_count < self.max_concurrent
    }

    /// Update estimated start times
    async fn update_estimated_start_times(&self) -> Result<()> {
        let pending = self.get_pending_items().await?;
        let active_count = self.get_active_count().await;
        let mut items = self.items.write().await;

        let current_time = current_timestamp();
        let avg_transfer_duration = 300u64;

        for (index, pending_item) in pending.iter().enumerate() {
            if let Some(item) = items.get_mut(&pending_item.queue_id) {
                let position = if index < self.max_concurrent.saturating_sub(active_count) {
                    0
                } else {
                    index - (self.max_concurrent - active_count)
                };

                let estimated_delay = (position as u64) * avg_transfer_duration;
                item.estimated_start = Some(current_time + estimated_delay);
                self.persist_queue_item(item).await?;
            }
        }

        Ok(())
    }

    /// Persist queue item to disk
    pub async fn persist_queue_item(&self, item: &QueueItem) -> Result<()> {
        let item_file = self.get_queue_item_file_path(item.queue_id);
        let item_json = serde_json::to_vec_pretty(item).map_err(|e| {
            FileTransferError::InternalError(format!("Failed to serialize queue item: {}", e))
        })?;

        let mut file = fs::File::create(&item_file).await.map_err(|e| {
            FileTransferError::IoError {
                path: item_file.clone(),
                source: e,
            }
        })?;

        file.write_all(&item_json).await.map_err(|e| {
            FileTransferError::IoError {
                path: item_file.clone(),
                source: e,
            }
        })?;

        file.flush().await.map_err(|e| {
            FileTransferError::IoError {
                path: item_file.clone(),
                source: e,
            }
        })?;

        Ok(())
    }

    /// Load persisted queue items
    async fn load_persisted_queue(&self) -> Result<()> {
        let mut entries = fs::read_dir(&self.persistence_dir)
            .await
            .map_err(|e| FileTransferError::IoError {
                path: self.persistence_dir.clone(),
                source: e,
            })?;

        let mut queue = self.queue.write().await;
        let mut items = self.items.write().await;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            FileTransferError::IoError {
                path: self.persistence_dir.clone(),
                source: e,
            }
        })? {
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }

            match self.load_queue_item_from_file(&path).await {
                Ok(item) => {
                    if item.state == QueueState::Pending {
                        queue.push(PriorityQueueItem { item: item.clone() });
                    }
                    items.insert(item.queue_id, item);
                }
                Err(e) => {
                    eprintln!("Failed to load queue item from {:?}: {}", path, e);
                }
            }
        }

        Ok(())
    }

    /// Load queue item from file
    async fn load_queue_item_from_file(&self, path: &PathBuf) -> Result<QueueItem> {
        let mut file = fs::File::open(path).await.map_err(|e| {
            FileTransferError::IoError {
                path: path.clone(),
                source: e,
            }
        })?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await.map_err(|e| {
            FileTransferError::IoError {
                path: path.clone(),
                source: e,
            }
        })?;

        let item: QueueItem = serde_json::from_slice(&contents).map_err(|e| {
            FileTransferError::InternalError(format!("Failed to deserialize queue item: {}", e))
        })?;

        Ok(item)
    }

    /// Delete persisted queue item
    pub async fn delete_persisted_queue_item(&self, queue_id: QueueId) -> Result<()> {
        let item_file = self.get_queue_item_file_path(queue_id);

        if item_file.exists() {
            fs::remove_file(&item_file).await.map_err(|e| {
                FileTransferError::IoError {
                    path: item_file,
                    source: e,
                }
            })?;
        }

        Ok(())
    }

    /// Get file path for queue item
    fn get_queue_item_file_path(&self, queue_id: QueueId) -> PathBuf {
        self.persistence_dir.join(format!("queue_{}.json", queue_id))
    }

    /// Cleanup old items
    pub async fn cleanup_old_items(&self, max_age_seconds: u64) -> Result<usize> {
        let current_time = current_timestamp();
        let cutoff_time = current_time.saturating_sub(max_age_seconds);

        let items = self.items.read().await;
        let mut removed_count = 0;

        let items_to_remove: Vec<QueueId> = items
            .iter()
            .filter(|(_, item)| {
                matches!(item.state, QueueState::Cancelled)
                    && item.created_at < cutoff_time
            })
            .map(|(id, _)| *id)
            .collect();

        drop(items);

        for queue_id in items_to_remove {
            self.delete_persisted_queue_item(queue_id).await.ok();
            let mut items = self.items.write().await;
            items.remove(&queue_id);
            removed_count += 1;
        }

        Ok(removed_count)
    }
}

/// Queue scheduler handles intelligent queue processing and resource allocation
pub struct QueueScheduler {
    queue_manager: Arc<QueueManagerImpl>,
    connection_slots: Arc<RwLock<usize>>,
    total_bandwidth: Arc<RwLock<Option<u64>>>,
    bandwidth_per_transfer: Arc<RwLock<HashMap<QueueId, u64>>>,
}

impl QueueScheduler {
    pub fn new(queue_manager: Arc<QueueManagerImpl>, connection_slots: usize) -> Self {
        Self {
            queue_manager,
            connection_slots: Arc::new(RwLock::new(connection_slots)),
            total_bandwidth: Arc::new(RwLock::new(None)),
            bandwidth_per_transfer: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set_total_bandwidth(&self, bandwidth: Option<u64>) {
        let mut total_bandwidth = self.total_bandwidth.write().await;
        *total_bandwidth = bandwidth;
        self.reallocate_bandwidth().await.ok();
    }

    pub async fn get_available_slots(&self) -> usize {
        let connection_slots = self.connection_slots.read().await;
        let active_count = self.queue_manager.get_active_count().await;
        connection_slots.saturating_sub(active_count)
    }

    pub async fn has_resources_available(&self) -> bool {
        self.get_available_slots().await > 0
    }

    pub async fn schedule_next_transfer(&self) -> Result<Option<QueueItem>> {
        if !self.has_resources_available().await {
            return Ok(None);
        }

        let next_item = self.queue_manager.get_next_item().await?;

        if let Some(item) = next_item {
            self.allocate_resources(&item).await?;
            self.queue_manager
                .mark_item_scheduled(item.queue_id)
                .await?;
            Ok(Some(item))
        } else {
            Ok(None)
        }
    }

    async fn allocate_resources(&self, item: &QueueItem) -> Result<()> {
        let total_bandwidth = self.total_bandwidth.read().await;
        if let Some(total_bw) = *total_bandwidth {
            let active_count = self.queue_manager.get_active_count().await;
            let slots_to_use = active_count + 1;
            let bandwidth_per_transfer = total_bw / slots_to_use as u64;

            let mut bandwidth_allocations = self.bandwidth_per_transfer.write().await;
            bandwidth_allocations.insert(item.queue_id, bandwidth_per_transfer);

            drop(bandwidth_allocations);
            self.reallocate_bandwidth().await?;
        }

        Ok(())
    }

    pub async fn deallocate_resources(&self, queue_id: QueueId) -> Result<()> {
        let mut bandwidth_allocations = self.bandwidth_per_transfer.write().await;
        bandwidth_allocations.remove(&queue_id);
        drop(bandwidth_allocations);
        self.reallocate_bandwidth().await?;
        Ok(())
    }

    async fn reallocate_bandwidth(&self) -> Result<()> {
        let total_bandwidth = self.total_bandwidth.read().await;
        if let Some(total_bw) = *total_bandwidth {
            let active_count = self.queue_manager.get_active_count().await;
            if active_count > 0 {
                let bandwidth_per_transfer = total_bw / active_count as u64;
                let mut bandwidth_allocations = self.bandwidth_per_transfer.write().await;
                for allocation in bandwidth_allocations.values_mut() {
                    *allocation = bandwidth_per_transfer;
                }
            }
        }

        Ok(())
    }

    pub async fn get_bandwidth_allocation(&self, queue_id: QueueId) -> Option<u64> {
        let bandwidth_allocations = self.bandwidth_per_transfer.read().await;
        bandwidth_allocations.get(&queue_id).copied()
    }

    pub async fn calculate_estimated_start_time(&self, queue_id: QueueId) -> Result<Option<Timestamp>> {
        let item = self.queue_manager.get_queue_item(queue_id).await?;

        if item.state != QueueState::Pending {
            return Ok(None);
        }

        let pending_items = self.queue_manager.get_pending_items().await?;
        let position = pending_items
            .iter()
            .position(|i| i.queue_id == queue_id)
            .ok_or_else(|| FileTransferError::QueueItemNotFound {
                queue_id: queue_id.to_string(),
            })?;

        let available_slots = self.get_available_slots().await;
        let current_time = current_timestamp();

        if position < available_slots {
            return Ok(Some(current_time));
        }

        let avg_transfer_duration = self.estimate_average_transfer_duration().await;
        let items_ahead = position - available_slots;
        let estimated_delay = (items_ahead as u64) * avg_transfer_duration;

        Ok(Some(current_time + estimated_delay))
    }

    async fn estimate_average_transfer_duration(&self) -> u64 {
        300 // 5 minutes
    }

    pub async fn update_all_estimated_start_times(&self) -> Result<()> {
        let pending_items = self.queue_manager.get_pending_items().await?;

        for item in pending_items {
            if let Ok(Some(estimated_start)) = self.calculate_estimated_start_time(item.queue_id).await {
                let mut items = self.queue_manager.items.write().await;
                if let Some(queue_item) = items.get_mut(&item.queue_id) {
                    queue_item.estimated_start = Some(estimated_start);
                    drop(items);
                    self.queue_manager.persist_queue_item(&item).await?;
                }
            }
        }

        Ok(())
    }

    pub async fn get_queue_statistics(&self) -> QueueStatistics {
        let pending_items = self.queue_manager.get_pending_items().await.unwrap_or_default();
        let active_count = self.queue_manager.get_active_count().await;
        let available_slots = self.get_available_slots().await;
        let total_bandwidth = *self.total_bandwidth.read().await;

        QueueStatistics {
            pending_count: pending_items.len(),
            active_count,
            available_slots,
            total_bandwidth,
            avg_estimated_wait: self.estimate_average_transfer_duration().await,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatistics {
    pub pending_count: usize,
    pub active_count: usize,
    pub available_slots: usize,
    pub total_bandwidth: Option<u64>,
    pub avg_estimated_wait: u64,
}

/// Queue operations manager provides user-facing queue manipulation operations
pub struct QueueOperations {
    queue_manager: Arc<QueueManagerImpl>,
    scheduler: Arc<QueueScheduler>,
}

impl QueueOperations {
    pub fn new(queue_manager: Arc<QueueManagerImpl>, scheduler: Arc<QueueScheduler>) -> Self {
        Self {
            queue_manager,
            scheduler,
        }
    }

    pub async fn reorder_queue_item(&self, queue_id: QueueId, new_position: usize) -> Result<()> {
        let item = self.queue_manager.get_queue_item(queue_id).await?;

        if item.state != QueueState::Pending {
            return Err(FileTransferError::InvalidQueueOperation {
                reason: format!("Cannot reorder item in state {:?}", item.state),
            });
        }

        let mut pending_items = self.queue_manager.get_pending_items().await?;

        let current_position = pending_items
            .iter()
            .position(|i| i.queue_id == queue_id)
            .ok_or_else(|| FileTransferError::QueueItemNotFound {
                queue_id: queue_id.to_string(),
            })?;

        if new_position >= pending_items.len() {
            return Err(FileTransferError::InvalidQueueOperation {
                reason: format!("Invalid position: {}", new_position),
            });
        }

        let item_to_move = pending_items.remove(current_position);
        pending_items.insert(new_position, item_to_move);

        let base_priority = Priority::Normal as i32;
        for (index, pending_item) in pending_items.iter().enumerate() {
            let priority_adjustment = (pending_items.len() - index) as i32;
            let new_priority_value = (base_priority + priority_adjustment).clamp(0, 3);
            
            let new_priority = match new_priority_value {
                0 => Priority::Low,
                1 => Priority::Normal,
                2 => Priority::High,
                3 => Priority::Urgent,
                _ => Priority::Normal,
            };

            if pending_item.priority != new_priority {
                self.queue_manager
                    .modify_queue_item_priority(pending_item.queue_id, new_priority)
                    .await?;
            }
        }

        self.scheduler.update_all_estimated_start_times().await?;
        Ok(())
    }

    pub async fn pause_queue_item(&self, queue_id: QueueId) -> Result<()> {
        let item = self.queue_manager.get_queue_item(queue_id).await?;

        if !matches!(item.state, QueueState::Pending | QueueState::Scheduled) {
            return Err(FileTransferError::InvalidQueueOperation {
                reason: format!("Cannot pause item in state {:?}", item.state),
            });
        }

        self.queue_manager
            .update_queue_item_state(queue_id, QueueState::Paused)
            .await?;

        self.queue_manager.remove_from_queue(queue_id).await?;
        self.scheduler.update_all_estimated_start_times().await?;
        Ok(())
    }

    pub async fn resume_queue_item(&self, queue_id: QueueId) -> Result<()> {
        let item = self.queue_manager.get_queue_item(queue_id).await?;

        if item.state != QueueState::Paused {
            return Err(FileTransferError::InvalidQueueOperation {
                reason: format!("Cannot resume item in state {:?}", item.state),
            });
        }

        self.queue_manager
            .update_queue_item_state(queue_id, QueueState::Pending)
            .await?;

        let mut queue = self.queue_manager.queue.write().await;
        queue.push(PriorityQueueItem { item: item.clone() });
        drop(queue);

        self.scheduler.update_all_estimated_start_times().await?;
        Ok(())
    }

    pub async fn cancel_queue_item(&self, queue_id: QueueId) -> Result<()> {
        let item = self.queue_manager.get_queue_item(queue_id).await?;

        if matches!(item.state, QueueState::Cancelled) {
            return Ok(());
        }

        self.queue_manager.cancel_queue_item(queue_id).await?;
        self.scheduler.update_all_estimated_start_times().await?;
        Ok(())
    }

    pub async fn change_priority(&self, queue_id: QueueId, new_priority: Priority) -> Result<()> {
        let item = self.queue_manager.get_queue_item(queue_id).await?;

        if item.state != QueueState::Pending {
            return Err(FileTransferError::InvalidQueueOperation {
                reason: format!("Cannot change priority of item in state {:?}", item.state),
            });
        }

        self.queue_manager
            .modify_queue_item_priority(queue_id, new_priority)
            .await?;

        self.scheduler.update_all_estimated_start_times().await?;
        Ok(())
    }

    pub async fn get_queue_status(&self) -> Result<QueueStatus> {
        let all_items = self.queue_manager.get_all_queue_items().await?;
        let statistics = self.scheduler.get_queue_statistics().await;

        let pending_items: Vec<QueueItem> = all_items
            .iter()
            .filter(|item| item.state == QueueState::Pending)
            .cloned()
            .collect();

        let scheduled_items: Vec<QueueItem> = all_items
            .iter()
            .filter(|item| item.state == QueueState::Scheduled)
            .cloned()
            .collect();

        let paused_items: Vec<QueueItem> = all_items
            .iter()
            .filter(|item| item.state == QueueState::Paused)
            .cloned()
            .collect();

        let cancelled_items: Vec<QueueItem> = all_items
            .iter()
            .filter(|item| item.state == QueueState::Cancelled)
            .cloned()
            .collect();

        Ok(QueueStatus {
            pending_items,
            scheduled_items,
            paused_items,
            cancelled_items,
            statistics,
        })
    }

    pub async fn get_item_status(&self, queue_id: QueueId) -> Result<QueueItemStatus> {
        let item = self.queue_manager.get_queue_item(queue_id).await?;
        let bandwidth_allocation = self.scheduler.get_bandwidth_allocation(queue_id).await;

        let position_in_queue = if item.state == QueueState::Pending {
            let pending_items = self.queue_manager.get_pending_items().await?;
            pending_items
                .iter()
                .position(|i| i.queue_id == queue_id)
                .map(|pos| pos + 1)
        } else {
            None
        };

        Ok(QueueItemStatus {
            item,
            position_in_queue,
            bandwidth_allocation,
        })
    }

    pub async fn clear_cancelled_items(&self) -> Result<usize> {
        let cancelled_items = self
            .queue_manager
            .get_queue_items_by_state(QueueState::Cancelled)
            .await?;

        let mut cleared_count = 0;
        for item in cancelled_items {
            self.queue_manager
                .delete_persisted_queue_item(item.queue_id)
                .await?;
            
            let mut items = self.queue_manager.items.write().await;
            items.remove(&item.queue_id);
            cleared_count += 1;
        }

        Ok(cleared_count)
    }

    pub async fn get_items_by_priority(&self) -> Result<Vec<QueueItem>> {
        let pending_items = self.queue_manager.get_pending_items().await?;
        Ok(pending_items)
    }

    pub async fn get_items_by_state(&self, state: QueueState) -> Result<Vec<QueueItem>> {
        self.queue_manager.get_queue_items_by_state(state).await
    }

    pub async fn bulk_cancel(&self, queue_ids: Vec<QueueId>) -> Result<usize> {
        let mut cancelled_count = 0;

        for queue_id in queue_ids {
            if self.cancel_queue_item(queue_id).await.is_ok() {
                cancelled_count += 1;
            }
        }

        Ok(cancelled_count)
    }

    pub async fn bulk_pause(&self, queue_ids: Vec<QueueId>) -> Result<usize> {
        let mut paused_count = 0;

        for queue_id in queue_ids {
            if self.pause_queue_item(queue_id).await.is_ok() {
                paused_count += 1;
            }
        }

        Ok(paused_count)
    }

    pub async fn bulk_resume(&self, queue_ids: Vec<QueueId>) -> Result<usize> {
        let mut resumed_count = 0;

        for queue_id in queue_ids {
            if self.resume_queue_item(queue_id).await.is_ok() {
                resumed_count += 1;
            }
        }

        Ok(resumed_count)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStatus {
    pub pending_items: Vec<QueueItem>,
    pub scheduled_items: Vec<QueueItem>,
    pub paused_items: Vec<QueueItem>,
    pub cancelled_items: Vec<QueueItem>,
    pub statistics: QueueStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItemStatus {
    pub item: QueueItem,
    pub position_in_queue: Option<usize>,
    pub bandwidth_allocation: Option<u64>,
}
