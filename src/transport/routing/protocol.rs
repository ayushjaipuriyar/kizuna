use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::transport::{PeerId, TransportError};
use super::table::{RoutingTable, Route, RouteMetrics};
use super::mesh::{RouteDiscoveryMessage, RouteAdvertisement, MeshConfig};

/// Protocol for routing table updates and convergence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoutingProtocolMessage {
    /// Full routing table update
    FullUpdate {
        source: PeerId,
        routes: Vec<RouteTableEntry>,
        sequence_number: u64,
        timestamp: SystemTime,
    },
    /// Incremental routing table update
    IncrementalUpdate {
        source: PeerId,
        added_routes: Vec<RouteTableEntry>,
        removed_routes: Vec<RouteTableKey>,
        sequence_number: u64,
        timestamp: SystemTime,
    },
    /// Request for routing table synchronization
    SyncRequest {
        source: PeerId,
        last_known_sequence: u64,
        timestamp: SystemTime,
    },
    /// Response to synchronization request
    SyncResponse {
        source: PeerId,
        routes: Vec<RouteTableEntry>,
        sequence_number: u64,
        timestamp: SystemTime,
    },
    /// Heartbeat to maintain neighbor relationships
    Heartbeat {
        source: PeerId,
        sequence_number: u64,
        timestamp: SystemTime,
    },
    /// Acknowledgment of received update
    UpdateAck {
        source: PeerId,
        acked_sequence: u64,
        timestamp: SystemTime,
    },
}

/// Entry in routing table for protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteTableEntry {
    pub destination: PeerId,
    pub next_hop: PeerId,
    pub cost: u32,
    pub hop_count: u8,
    pub trust_score: u8,
    pub metrics: RouteMetrics,
    pub last_updated: SystemTime,
}

/// Key for identifying routes in updates
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct RouteTableKey {
    pub destination: PeerId,
    pub next_hop: PeerId,
}

/// State of a neighboring peer in the routing protocol
#[derive(Debug, Clone)]
pub struct NeighborState {
    pub peer_id: PeerId,
    pub last_seen: SystemTime,
    pub last_sequence_number: u64,
    pub pending_acks: HashSet<u64>,
    pub is_reliable: bool,
    pub heartbeat_interval: Duration,
    pub missed_heartbeats: u32,
    pub max_missed_heartbeats: u32,
}

impl NeighborState {
    pub fn new(peer_id: PeerId, heartbeat_interval: Duration) -> Self {
        Self {
            peer_id,
            last_seen: SystemTime::now(),
            last_sequence_number: 0,
            pending_acks: HashSet::new(),
            is_reliable: true,
            heartbeat_interval,
            missed_heartbeats: 0,
            max_missed_heartbeats: 3,
        }
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = SystemTime::now();
        self.missed_heartbeats = 0;
    }

    pub fn is_alive(&self) -> bool {
        self.missed_heartbeats < self.max_missed_heartbeats
    }

    pub fn mark_heartbeat_missed(&mut self) {
        self.missed_heartbeats += 1;
        if self.missed_heartbeats >= self.max_missed_heartbeats {
            self.is_reliable = false;
        }
    }

    pub fn should_send_heartbeat(&self) -> bool {
        SystemTime::now()
            .duration_since(self.last_seen)
            .unwrap_or_default() >= self.heartbeat_interval
    }
}

/// Configuration for the routing protocol
#[derive(Debug, Clone)]
pub struct RoutingProtocolConfig {
    /// Interval for sending full routing table updates
    pub full_update_interval: Duration,
    /// Interval for sending heartbeats to neighbors
    pub heartbeat_interval: Duration,
    /// Maximum time to wait for acknowledgments
    pub ack_timeout: Duration,
    /// Maximum number of retries for unacknowledged updates
    pub max_retries: u32,
    /// Enable reliable delivery with acknowledgments
    pub reliable_delivery: bool,
    /// Maximum age for routing information
    pub max_route_age: Duration,
    /// Convergence timeout for routing changes
    pub convergence_timeout: Duration,
}

impl Default for RoutingProtocolConfig {
    fn default() -> Self {
        Self {
            full_update_interval: Duration::from_secs(120), // 2 minutes
            heartbeat_interval: Duration::from_secs(30),
            ack_timeout: Duration::from_secs(5),
            max_retries: 3,
            reliable_delivery: true,
            max_route_age: Duration::from_secs(300), // 5 minutes
            convergence_timeout: Duration::from_secs(60),
        }
    }
}

/// Statistics for routing protocol operations
#[derive(Debug, Clone)]
pub struct RoutingProtocolStats {
    pub full_updates_sent: u64,
    pub incremental_updates_sent: u64,
    pub updates_received: u64,
    pub heartbeats_sent: u64,
    pub heartbeats_received: u64,
    pub sync_requests_sent: u64,
    pub sync_responses_sent: u64,
    pub convergence_events: u64,
    pub neighbor_failures: u64,
    pub active_neighbors: usize,
    pub pending_acks: usize,
}

/// Manager for routing protocol operations
pub struct RoutingProtocolManager {
    /// Local peer ID
    local_peer_id: PeerId,
    /// Routing table reference
    routing_table: Arc<RwLock<RoutingTable>>,
    /// Configuration
    config: RoutingProtocolConfig,
    /// Sequence number for messages
    sequence_number: Arc<RwLock<u64>>,
    /// State of neighboring peers
    neighbors: Arc<RwLock<HashMap<PeerId, NeighborState>>>,
    /// Statistics
    stats: Arc<RwLock<RoutingProtocolStats>>,
    /// Pending acknowledgments
    pending_acks: Arc<RwLock<HashMap<u64, (RoutingProtocolMessage, SystemTime, u32)>>>,
}

impl RoutingProtocolManager {
    /// Create a new routing protocol manager
    pub fn new(
        local_peer_id: PeerId,
        routing_table: Arc<RwLock<RoutingTable>>,
        config: RoutingProtocolConfig,
    ) -> Self {
        Self {
            local_peer_id,
            routing_table,
            config,
            sequence_number: Arc::new(RwLock::new(0)),
            neighbors: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(RoutingProtocolStats {
                full_updates_sent: 0,
                incremental_updates_sent: 0,
                updates_received: 0,
                heartbeats_sent: 0,
                heartbeats_received: 0,
                sync_requests_sent: 0,
                sync_responses_sent: 0,
                convergence_events: 0,
                neighbor_failures: 0,
                active_neighbors: 0,
                pending_acks: 0,
            })),
            pending_acks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a neighbor to track
    pub async fn add_neighbor(&self, peer_id: PeerId) {
        let mut neighbors = self.neighbors.write().await;
        neighbors.insert(
            peer_id.clone(),
            NeighborState::new(peer_id, self.config.heartbeat_interval),
        );
    }

    /// Remove a neighbor
    pub async fn remove_neighbor(&self, peer_id: &PeerId) {
        let mut neighbors = self.neighbors.write().await;
        neighbors.remove(peer_id);
    }

    /// Send full routing table update to all neighbors
    pub async fn send_full_update(&self) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        let routes = self.get_routing_table_entries().await;
        let sequence = self.next_sequence_number().await;
        
        let message = RoutingProtocolMessage::FullUpdate {
            source: self.local_peer_id.clone(),
            routes,
            sequence_number: sequence,
            timestamp: SystemTime::now(),
        };

        if self.config.reliable_delivery {
            self.add_pending_ack(sequence, message.clone()).await;
        }

        let mut stats = self.stats.write().await;
        stats.full_updates_sent += 1;

        Ok(vec![message])
    }

    /// Send incremental update for route changes
    pub async fn send_incremental_update(
        &self,
        added_routes: Vec<RouteTableEntry>,
        removed_routes: Vec<RouteTableKey>,
    ) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        if added_routes.is_empty() && removed_routes.is_empty() {
            return Ok(vec![]);
        }

        let sequence = self.next_sequence_number().await;
        
        let message = RoutingProtocolMessage::IncrementalUpdate {
            source: self.local_peer_id.clone(),
            added_routes,
            removed_routes,
            sequence_number: sequence,
            timestamp: SystemTime::now(),
        };

        if self.config.reliable_delivery {
            self.add_pending_ack(sequence, message.clone()).await;
        }

        let mut stats = self.stats.write().await;
        stats.incremental_updates_sent += 1;

        Ok(vec![message])
    }

    /// Handle incoming routing protocol message
    pub async fn handle_protocol_message(
        &self,
        message: RoutingProtocolMessage,
        from_peer: &PeerId,
    ) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        // Update neighbor state
        self.update_neighbor_state(from_peer).await;

        match message {
            RoutingProtocolMessage::FullUpdate { source, routes, sequence_number, timestamp } => {
                self.handle_full_update(source, routes, sequence_number, timestamp).await
            }
            RoutingProtocolMessage::IncrementalUpdate { 
                source, 
                added_routes, 
                removed_routes, 
                sequence_number, 
                timestamp 
            } => {
                self.handle_incremental_update(source, added_routes, removed_routes, sequence_number, timestamp).await
            }
            RoutingProtocolMessage::SyncRequest { source, last_known_sequence, timestamp } => {
                self.handle_sync_request(source, last_known_sequence, timestamp).await
            }
            RoutingProtocolMessage::SyncResponse { source, routes, sequence_number, timestamp } => {
                self.handle_sync_response(source, routes, sequence_number, timestamp).await
            }
            RoutingProtocolMessage::Heartbeat { source, sequence_number, timestamp } => {
                self.handle_heartbeat(source, sequence_number, timestamp).await
            }
            RoutingProtocolMessage::UpdateAck { source, acked_sequence, timestamp } => {
                self.handle_update_ack(source, acked_sequence, timestamp).await
            }
        }
    }

    /// Handle full routing table update
    async fn handle_full_update(
        &self,
        source: PeerId,
        routes: Vec<RouteTableEntry>,
        sequence_number: u64,
        _timestamp: SystemTime,
    ) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        // Update neighbor sequence number
        {
            let mut neighbors = self.neighbors.write().await;
            if let Some(neighbor) = neighbors.get_mut(&source) {
                neighbor.last_sequence_number = sequence_number;
            }
        }

        // Apply routes to routing table
        {
            let mut table = self.routing_table.write().await;
            for route_entry in routes {
                let route = Route::new(
                    vec![route_entry.next_hop, route_entry.destination.clone()],
                    route_entry.cost,
                    route_entry.trust_score,
                );
                let _ = table.add_route(route_entry.destination, route, route_entry.metrics);
            }
        }

        let mut stats = self.stats.write().await;
        stats.updates_received += 1;

        // Send acknowledgment if reliable delivery is enabled
        if self.config.reliable_delivery {
            let ack_message = RoutingProtocolMessage::UpdateAck {
                source: self.local_peer_id.clone(),
                acked_sequence: sequence_number,
                timestamp: SystemTime::now(),
            };
            Ok(vec![ack_message])
        } else {
            Ok(vec![])
        }
    }

    /// Handle incremental routing table update
    async fn handle_incremental_update(
        &self,
        source: PeerId,
        added_routes: Vec<RouteTableEntry>,
        removed_routes: Vec<RouteTableKey>,
        sequence_number: u64,
        _timestamp: SystemTime,
    ) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        // Update neighbor sequence number
        {
            let mut neighbors = self.neighbors.write().await;
            if let Some(neighbor) = neighbors.get_mut(&source) {
                neighbor.last_sequence_number = sequence_number;
            }
        }

        // Apply changes to routing table
        {
            let mut table = self.routing_table.write().await;
            
            // Add new routes
            for route_entry in added_routes {
                let route = Route::new(
                    vec![route_entry.next_hop, route_entry.destination.clone()],
                    route_entry.cost,
                    route_entry.trust_score,
                );
                let _ = table.add_route(route_entry.destination, route, route_entry.metrics);
            }
            
            // Remove old routes
            for route_key in removed_routes {
                table.remove_route(&route_key.destination, &[route_key.next_hop]);
            }
        }

        let mut stats = self.stats.write().await;
        stats.updates_received += 1;

        // Send acknowledgment if reliable delivery is enabled
        if self.config.reliable_delivery {
            let ack_message = RoutingProtocolMessage::UpdateAck {
                source: self.local_peer_id.clone(),
                acked_sequence: sequence_number,
                timestamp: SystemTime::now(),
            };
            Ok(vec![ack_message])
        } else {
            Ok(vec![])
        }
    }

    /// Handle synchronization request
    async fn handle_sync_request(
        &self,
        source: PeerId,
        _last_known_sequence: u64,
        _timestamp: SystemTime,
    ) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        let routes = self.get_routing_table_entries().await;
        let sequence = self.next_sequence_number().await;
        
        let response = RoutingProtocolMessage::SyncResponse {
            source: self.local_peer_id.clone(),
            routes,
            sequence_number: sequence,
            timestamp: SystemTime::now(),
        };

        let mut stats = self.stats.write().await;
        stats.sync_responses_sent += 1;

        Ok(vec![response])
    }

    /// Handle synchronization response
    async fn handle_sync_response(
        &self,
        source: PeerId,
        routes: Vec<RouteTableEntry>,
        sequence_number: u64,
        _timestamp: SystemTime,
    ) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        // Similar to full update handling
        self.handle_full_update(source, routes, sequence_number, SystemTime::now()).await
    }

    /// Handle heartbeat message
    async fn handle_heartbeat(
        &self,
        source: PeerId,
        sequence_number: u64,
        _timestamp: SystemTime,
    ) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        // Update neighbor state
        {
            let mut neighbors = self.neighbors.write().await;
            if let Some(neighbor) = neighbors.get_mut(&source) {
                neighbor.last_sequence_number = sequence_number;
                neighbor.update_last_seen();
            }
        }

        let mut stats = self.stats.write().await;
        stats.heartbeats_received += 1;

        Ok(vec![])
    }

    /// Handle update acknowledgment
    async fn handle_update_ack(
        &self,
        _source: PeerId,
        acked_sequence: u64,
        _timestamp: SystemTime,
    ) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        // Remove from pending acknowledgments
        {
            let mut pending = self.pending_acks.write().await;
            pending.remove(&acked_sequence);
        }

        Ok(vec![])
    }

    /// Send heartbeat to all neighbors
    pub async fn send_heartbeats(&self) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        let sequence = self.next_sequence_number().await;
        
        let message = RoutingProtocolMessage::Heartbeat {
            source: self.local_peer_id.clone(),
            sequence_number: sequence,
            timestamp: SystemTime::now(),
        };

        let mut stats = self.stats.write().await;
        stats.heartbeats_sent += 1;

        Ok(vec![message])
    }

    /// Check for failed neighbors and clean up routes
    pub async fn check_neighbor_failures(&self) -> Vec<PeerId> {
        let mut failed_neighbors = Vec::new();
        
        {
            let mut neighbors = self.neighbors.write().await;
            let now = SystemTime::now();
            
            for (peer_id, neighbor) in neighbors.iter_mut() {
                if now.duration_since(neighbor.last_seen).unwrap_or_default() 
                   > neighbor.heartbeat_interval * 2 {
                    neighbor.mark_heartbeat_missed();
                    
                    if !neighbor.is_alive() {
                        failed_neighbors.push(peer_id.clone());
                    }
                }
            }
            
            // Remove failed neighbors
            for failed_peer in &failed_neighbors {
                neighbors.remove(failed_peer);
            }
        }

        // Clean up routes through failed neighbors
        if !failed_neighbors.is_empty() {
            let routes_to_remove = {
                let table = self.routing_table.read().await;
                let mut routes_to_remove = Vec::new();
                for failed_peer in &failed_neighbors {
                    let routes_through_peer = table.get_routes_through_peer(failed_peer);
                    for (destination, route) in routes_through_peer {
                        routes_to_remove.push((destination, route.hops.clone()));
                    }
                }
                routes_to_remove
            };
            
            if !routes_to_remove.is_empty() {
                let mut table = self.routing_table.write().await;
                for (destination, hops) in routes_to_remove {
                    table.remove_route(&destination, &hops);
                }
            }
            
            let mut stats = self.stats.write().await;
            stats.neighbor_failures += failed_neighbors.len() as u64;
        }

        failed_neighbors
    }

    /// Retry unacknowledged messages
    pub async fn retry_unacknowledged(&self) -> Result<Vec<RoutingProtocolMessage>, TransportError> {
        let mut messages_to_retry = Vec::new();
        let now = SystemTime::now();
        
        {
            let mut pending = self.pending_acks.write().await;
            let mut to_remove = Vec::new();
            
            for (sequence, (message, sent_at, retry_count)) in pending.iter_mut() {
                if now.duration_since(*sent_at).unwrap_or_default() > self.config.ack_timeout {
                    if *retry_count < self.config.max_retries {
                        messages_to_retry.push(message.clone());
                        *sent_at = now;
                        *retry_count += 1;
                    } else {
                        // Max retries exceeded, give up
                        to_remove.push(*sequence);
                    }
                }
            }
            
            // Remove messages that exceeded max retries
            for sequence in to_remove {
                pending.remove(&sequence);
            }
        }

        Ok(messages_to_retry)
    }

    /// Get routing table entries for protocol messages
    async fn get_routing_table_entries(&self) -> Vec<RouteTableEntry> {
        let table = self.routing_table.read().await;
        let destinations = table.get_destinations();
        
        let mut entries = Vec::new();
        for destination in destinations {
            if let Some(route) = table.get_best_route(&destination) {
                if let Some(next_hop) = route.next_hop() {
                    entries.push(RouteTableEntry {
                        destination,
                        next_hop: next_hop.clone(),
                        cost: route.cost,
                        hop_count: route.hop_count,
                        trust_score: route.trust_score,
                        metrics: RouteMetrics::default_unknown(), // Would be actual metrics
                        last_updated: route.last_updated,
                    });
                }
            }
        }
        
        entries
    }

    /// Get next sequence number
    async fn next_sequence_number(&self) -> u64 {
        let mut seq = self.sequence_number.write().await;
        *seq += 1;
        *seq
    }

    /// Add message to pending acknowledgments
    async fn add_pending_ack(&self, sequence: u64, message: RoutingProtocolMessage) {
        let mut pending = self.pending_acks.write().await;
        pending.insert(sequence, (message, SystemTime::now(), 0));
    }

    /// Update neighbor state
    async fn update_neighbor_state(&self, peer_id: &PeerId) {
        let mut neighbors = self.neighbors.write().await;
        if let Some(neighbor) = neighbors.get_mut(peer_id) {
            neighbor.update_last_seen();
        } else {
            // Add new neighbor
            neighbors.insert(
                peer_id.clone(),
                NeighborState::new(peer_id.clone(), self.config.heartbeat_interval),
            );
        }
    }

    /// Get protocol statistics
    pub async fn get_stats(&self) -> RoutingProtocolStats {
        let stats = self.stats.read().await;
        let mut protocol_stats = stats.clone();
        
        let neighbors = self.neighbors.read().await;
        protocol_stats.active_neighbors = neighbors.len();
        
        let pending = self.pending_acks.read().await;
        protocol_stats.pending_acks = pending.len();
        
        protocol_stats
    }

    /// Start periodic protocol tasks
    pub fn start_protocol_tasks(manager: Arc<RoutingProtocolManager>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut full_update_interval = tokio::time::interval(manager.config.full_update_interval);
            let mut heartbeat_interval = tokio::time::interval(manager.config.heartbeat_interval);
            let mut retry_interval = tokio::time::interval(manager.config.ack_timeout);
            let mut failure_check_interval = tokio::time::interval(Duration::from_secs(30));
            
            loop {
                tokio::select! {
                    _ = full_update_interval.tick() => {
                        let _ = manager.send_full_update().await;
                    }
                    _ = heartbeat_interval.tick() => {
                        let _ = manager.send_heartbeats().await;
                    }
                    _ = retry_interval.tick() => {
                        let _ = manager.retry_unacknowledged().await;
                    }
                    _ = failure_check_interval.tick() => {
                        let _ = manager.check_neighbor_failures().await;
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::routing::table::RoutingTable;

    #[tokio::test]
    async fn test_routing_protocol_manager_creation() {
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(3, Duration::from_secs(300), 5)));
        let config = RoutingProtocolConfig::default();
        let manager = RoutingProtocolManager::new("test-peer".to_string(), routing_table, config);
        
        assert_eq!(manager.local_peer_id, "test-peer");
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_neighbors, 0);
        assert_eq!(stats.pending_acks, 0);
    }

    #[tokio::test]
    async fn test_neighbor_management() {
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(3, Duration::from_secs(300), 5)));
        let config = RoutingProtocolConfig::default();
        let manager = RoutingProtocolManager::new("test-peer".to_string(), routing_table, config);
        
        manager.add_neighbor("neighbor1".to_string()).await;
        manager.add_neighbor("neighbor2".to_string()).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_neighbors, 2);
        
        manager.remove_neighbor(&"neighbor1".to_string()).await;
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.active_neighbors, 1);
    }

    #[tokio::test]
    async fn test_heartbeat_handling() {
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(3, Duration::from_secs(300), 5)));
        let config = RoutingProtocolConfig::default();
        let manager = RoutingProtocolManager::new("test-peer".to_string(), routing_table, config);
        
        let heartbeat = RoutingProtocolMessage::Heartbeat {
            source: "neighbor1".to_string(),
            sequence_number: 1,
            timestamp: SystemTime::now(),
        };
        
        let response = manager.handle_protocol_message(heartbeat, &"neighbor1".to_string()).await;
        assert!(response.is_ok());
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.heartbeats_received, 1);
        assert_eq!(stats.active_neighbors, 1);
    }

    #[tokio::test]
    async fn test_full_update_handling() {
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(3, Duration::from_secs(300), 5)));
        let config = RoutingProtocolConfig::default();
        let manager = RoutingProtocolManager::new("test-peer".to_string(), routing_table, config);
        
        let routes = vec![
            RouteTableEntry {
                destination: "dest1".to_string(),
                next_hop: "neighbor1".to_string(),
                cost: 100,
                hop_count: 2,
                trust_score: 80,
                metrics: RouteMetrics::default_unknown(),
                last_updated: SystemTime::now(),
            }
        ];
        
        let full_update = RoutingProtocolMessage::FullUpdate {
            source: "neighbor1".to_string(),
            routes,
            sequence_number: 1,
            timestamp: SystemTime::now(),
        };
        
        let response = manager.handle_protocol_message(full_update, &"neighbor1".to_string()).await;
        assert!(response.is_ok());
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.updates_received, 1);
    }

    #[tokio::test]
    async fn test_send_heartbeats() {
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(3, Duration::from_secs(300), 5)));
        let config = RoutingProtocolConfig::default();
        let manager = RoutingProtocolManager::new("test-peer".to_string(), routing_table, config);
        
        let messages = manager.send_heartbeats().await;
        assert!(messages.is_ok());
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.heartbeats_sent, 1);
    }

    #[tokio::test]
    async fn test_neighbor_state() {
        let mut neighbor = NeighborState::new("test-peer".to_string(), Duration::from_secs(30));
        
        assert!(neighbor.is_alive());
        assert_eq!(neighbor.missed_heartbeats, 0);
        
        neighbor.mark_heartbeat_missed();
        neighbor.mark_heartbeat_missed();
        neighbor.mark_heartbeat_missed();
        
        assert!(!neighbor.is_alive());
        assert!(!neighbor.is_reliable);
        
        neighbor.update_last_seen();
        assert_eq!(neighbor.missed_heartbeats, 0);
    }
}