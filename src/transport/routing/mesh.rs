use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::transport::{
    Connection, ConnectionManager, PeerId, TransportError, PeerAddress, TransportCapabilities,
};
use super::table::{RoutingTable, Route, RouteMetrics};

/// Configuration for mesh routing
#[derive(Debug, Clone)]
pub struct MeshConfig {
    /// Maximum number of hops allowed in routes
    pub max_hop_count: u8,
    /// Maximum age for routes before they expire
    pub max_route_age: Duration,
    /// Maximum number of routes per destination
    pub max_routes_per_destination: usize,
    /// Interval for route discovery broadcasts
    pub route_discovery_interval: Duration,
    /// Interval for route advertisement broadcasts
    pub route_advertisement_interval: Duration,
    /// Maximum time to wait for route discovery responses
    pub route_discovery_timeout: Duration,
    /// Enable hop-by-hop encryption
    pub enable_hop_encryption: bool,
    /// Maximum message size for routing
    pub max_message_size: usize,
}

impl Default for MeshConfig {
    fn default() -> Self {
        Self {
            max_hop_count: 5,
            max_route_age: Duration::from_secs(300), // 5 minutes
            max_routes_per_destination: 3,
            route_discovery_interval: Duration::from_secs(60),
            route_advertisement_interval: Duration::from_secs(30),
            route_discovery_timeout: Duration::from_secs(10),
            enable_hop_encryption: true,
            max_message_size: 64 * 1024, // 64KB
        }
    }
}

/// Message types for route discovery and maintenance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RouteDiscoveryMessage {
    /// Request to discover routes to a destination
    RouteRequest {
        request_id: String,
        destination: PeerId,
        source: PeerId,
        hop_count: u8,
        max_hops: u8,
        timestamp: SystemTime,
    },
    /// Response with route information
    RouteResponse {
        request_id: String,
        destination: PeerId,
        source: PeerId,
        route: Vec<PeerId>,
        cost: u32,
        timestamp: SystemTime,
    },
    /// Advertisement of available routes
    RouteAdvertisement {
        source: PeerId,
        routes: Vec<RouteAdvertisement>,
        timestamp: SystemTime,
    },
    /// Notification of route failure
    RouteError {
        source: PeerId,
        destination: PeerId,
        failed_hop: PeerId,
        error_code: u8,
        timestamp: SystemTime,
    },
}

/// Advertisement of a single route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteAdvertisement {
    pub destination: PeerId,
    pub hop_count: u8,
    pub cost: u32,
    pub trust_score: u8,
    pub capabilities: Option<TransportCapabilities>,
}

/// Encrypted message for hop-by-hop transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedHopMessage {
    /// Next hop in the route
    pub next_hop: PeerId,
    /// Encrypted payload for the next hop
    pub encrypted_payload: Vec<u8>,
    /// Message authentication code
    pub mac: Vec<u8>,
    /// Timestamp for replay protection
    pub timestamp: SystemTime,
}

/// Statistics for mesh routing operations
#[derive(Debug, Clone)]
pub struct MeshStats {
    pub routes_discovered: u64,
    pub route_requests_sent: u64,
    pub route_responses_received: u64,
    pub messages_routed: u64,
    pub routing_failures: u64,
    pub hop_encryption_operations: u64,
    pub active_route_discoveries: usize,
}

/// Main mesh router implementation
pub struct MeshRouter {
    /// Local peer ID
    local_peer_id: PeerId,
    /// Routing table for managing routes
    routing_table: Arc<RwLock<RoutingTable>>,
    /// Connection manager for transport operations
    connection_manager: Arc<ConnectionManager>,
    /// Configuration for mesh routing
    config: MeshConfig,
    /// Active route discovery requests
    active_discoveries: Arc<RwLock<HashMap<String, RouteDiscoveryState>>>,
    /// Statistics
    stats: Arc<RwLock<MeshStats>>,
    /// Encryption keys for hop-by-hop encryption (peer_id -> key)
    hop_encryption_keys: Arc<RwLock<HashMap<PeerId, Vec<u8>>>>,
}

/// State for an active route discovery
#[derive(Debug)]
struct RouteDiscoveryState {
    destination: PeerId,
    started_at: SystemTime,
    responses_received: Vec<RouteDiscoveryMessage>,
    timeout: Duration,
}

impl MeshRouter {
    /// Create a new mesh router
    pub fn new(
        local_peer_id: PeerId,
        connection_manager: Arc<ConnectionManager>,
        config: MeshConfig,
    ) -> Self {
        let routing_table = Arc::new(RwLock::new(RoutingTable::new(
            config.max_routes_per_destination,
            config.max_route_age,
            config.max_hop_count,
        )));

        Self {
            local_peer_id,
            routing_table,
            connection_manager,
            config,
            active_discoveries: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(MeshStats {
                routes_discovered: 0,
                route_requests_sent: 0,
                route_responses_received: 0,
                messages_routed: 0,
                routing_failures: 0,
                hop_encryption_operations: 0,
                active_route_discoveries: 0,
            })),
            hop_encryption_keys: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Route a message to a destination peer through the mesh
    pub async fn route_to_peer(&self, destination: &PeerId, data: &[u8]) -> Result<(), TransportError> {
        if data.len() > self.config.max_message_size {
            return Err(TransportError::InvalidRoute {
                reason: format!("Message size {} exceeds maximum {}", data.len(), self.config.max_message_size),
            });
        }

        // Check if we have a direct connection first
        let connections = self.connection_manager.get_connections(destination).await;
        if !connections.is_empty() {
            // Use direct connection
            return self.send_direct(destination, data).await;
        }

        // Find route through mesh
        let route = {
            let table = self.routing_table.read().await;
            table.get_best_route(destination).cloned()
        };

        let route = match route {
            Some(route) => route,
            None => {
                // No route available, try to discover one
                self.discover_route(destination).await?;
                
                // Try again after discovery
                let table = self.routing_table.read().await;
                table.get_best_route(destination).cloned()
                    .ok_or_else(|| TransportError::InvalidRoute {
                        reason: format!("No route found to destination: {}", destination),
                    })?
            }
        };

        // Route through mesh
        self.route_through_mesh(&route, data).await
    }

    /// Send data directly to a peer
    async fn send_direct(&self, destination: &PeerId, data: &[u8]) -> Result<(), TransportError> {
        // This would use the connection manager to send data directly
        // For now, we'll simulate this operation
        let mut stats = self.stats.write().await;
        stats.messages_routed += 1;
        
        // In a real implementation, this would:
        // 1. Get an active connection to the destination
        // 2. Write the data to the connection
        // 3. Handle any errors appropriately
        
        Ok(())
    }

    /// Route data through the mesh using the specified route
    async fn route_through_mesh(&self, route: &Route, data: &[u8]) -> Result<(), TransportError> {
        if route.hops.is_empty() {
            return Err(TransportError::InvalidRoute {
                reason: "Empty route provided".to_string(),
            });
        }

        let next_hop = route.next_hop().unwrap();
        
        if self.config.enable_hop_encryption {
            // Encrypt data for hop-by-hop transmission
            let encrypted_message = self.encrypt_for_hop(next_hop, data).await?;
            self.send_encrypted_hop_message(next_hop, &encrypted_message).await?;
        } else {
            // Send unencrypted (not recommended for production)
            self.send_direct(next_hop, data).await?;
        }

        // Update route statistics
        {
            let mut table = self.routing_table.write().await;
            table.mark_route_success(&route.destination().unwrap(), &route.hops);
        }

        let mut stats = self.stats.write().await;
        stats.messages_routed += 1;

        Ok(())
    }

    /// Encrypt data for hop-by-hop transmission
    async fn encrypt_for_hop(&self, next_hop: &PeerId, data: &[u8]) -> Result<EncryptedHopMessage, TransportError> {
        let keys = self.hop_encryption_keys.read().await;
        let key = keys.get(next_hop)
            .ok_or_else(|| TransportError::InvalidRoute {
                reason: format!("No encryption key available for hop: {}", next_hop),
            })?;

        // Simple XOR encryption for demonstration (use proper encryption in production)
        let mut encrypted_payload = data.to_vec();
        for (i, byte) in encrypted_payload.iter_mut().enumerate() {
            *byte ^= key[i % key.len()];
        }

        // Simple MAC calculation (use HMAC in production)
        let mac = self.calculate_mac(&encrypted_payload, key);

        let mut stats = self.stats.write().await;
        stats.hop_encryption_operations += 1;

        Ok(EncryptedHopMessage {
            next_hop: next_hop.clone(),
            encrypted_payload,
            mac,
            timestamp: SystemTime::now(),
        })
    }

    /// Calculate message authentication code
    fn calculate_mac(&self, data: &[u8], key: &[u8]) -> Vec<u8> {
        // Simple checksum for demonstration (use proper HMAC in production)
        let mut mac = vec![0u8; 16];
        for (i, &byte) in data.iter().enumerate() {
            mac[i % 16] ^= byte ^ key[i % key.len()];
        }
        mac
    }

    /// Send encrypted hop message
    async fn send_encrypted_hop_message(&self, next_hop: &PeerId, message: &EncryptedHopMessage) -> Result<(), TransportError> {
        let serialized = serde_json::to_vec(message)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;
        
        self.send_direct(next_hop, &serialized).await
    }

    /// Discover a route to a destination
    pub async fn discover_route(&self, destination: &PeerId) -> Result<(), TransportError> {
        let request_id = format!("{}_{}", self.local_peer_id, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis());
        
        let discovery_message = RouteDiscoveryMessage::RouteRequest {
            request_id: request_id.clone(),
            destination: destination.clone(),
            source: self.local_peer_id.clone(),
            hop_count: 0,
            max_hops: self.config.max_hop_count,
            timestamp: SystemTime::now(),
        };

        // Store discovery state
        {
            let mut discoveries = self.active_discoveries.write().await;
            discoveries.insert(request_id.clone(), RouteDiscoveryState {
                destination: destination.clone(),
                started_at: SystemTime::now(),
                responses_received: Vec::new(),
                timeout: self.config.route_discovery_timeout,
            });
        }

        // Broadcast route request to all connected peers
        self.broadcast_route_message(&discovery_message).await?;

        let mut stats = self.stats.write().await;
        stats.route_requests_sent += 1;
        stats.active_route_discoveries += 1;

        Ok(())
    }

    /// Broadcast a route message to all connected peers
    async fn broadcast_route_message(&self, message: &RouteDiscoveryMessage) -> Result<(), TransportError> {
        let serialized = serde_json::to_vec(message)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;

        // Get all active connections and broadcast
        // In a real implementation, this would iterate through all active connections
        // and send the message to each peer
        
        Ok(())
    }

    /// Handle incoming route discovery message
    pub async fn handle_route_message(&self, message: RouteDiscoveryMessage, from_peer: &PeerId) -> Result<(), TransportError> {
        match message {
            RouteDiscoveryMessage::RouteRequest { 
                request_id, 
                destination, 
                source, 
                hop_count, 
                max_hops, 
                timestamp 
            } => {
                self.handle_route_request(request_id, destination, source, hop_count, max_hops, timestamp, from_peer).await
            }
            RouteDiscoveryMessage::RouteResponse { 
                request_id, 
                destination, 
                source, 
                route, 
                cost, 
                timestamp 
            } => {
                self.handle_route_response(request_id, destination, source, route, cost, timestamp).await
            }
            RouteDiscoveryMessage::RouteAdvertisement { 
                source, 
                routes, 
                timestamp 
            } => {
                self.handle_route_advertisement(source, routes, timestamp).await
            }
            RouteDiscoveryMessage::RouteError { 
                source, 
                destination, 
                failed_hop, 
                error_code, 
                timestamp 
            } => {
                self.handle_route_error(source, destination, failed_hop, error_code, timestamp).await
            }
        }
    }

    /// Handle route request message
    async fn handle_route_request(
        &self,
        request_id: String,
        destination: PeerId,
        source: PeerId,
        hop_count: u8,
        max_hops: u8,
        _timestamp: SystemTime,
        from_peer: &PeerId,
    ) -> Result<(), TransportError> {
        // Prevent loops
        if source == self.local_peer_id {
            return Ok(());
        }

        // Check hop limit
        if hop_count >= max_hops {
            return Ok(());
        }

        // If we are the destination, send response
        if destination == self.local_peer_id {
            let response = RouteDiscoveryMessage::RouteResponse {
                request_id,
                destination: destination.clone(),
                source: self.local_peer_id.clone(),
                route: vec![self.local_peer_id.clone()],
                cost: 0,
                timestamp: SystemTime::now(),
            };
            
            return self.send_route_message_to_peer(&response, from_peer).await;
        }

        // Check if we have a route to the destination
        let route_to_destination = {
            let table = self.routing_table.read().await;
            table.get_best_route(&destination).cloned()
        };

        if let Some(route) = route_to_destination {
            // Send response with our route
            let mut full_route = vec![self.local_peer_id.clone()];
            full_route.extend(route.hops);
            
            let response = RouteDiscoveryMessage::RouteResponse {
                request_id,
                destination,
                source: self.local_peer_id.clone(),
                route: full_route,
                cost: route.cost + 10, // Add cost for additional hop
                timestamp: SystemTime::now(),
            };
            
            self.send_route_message_to_peer(&response, from_peer).await?;
        } else {
            // Forward the request to other peers
            let forwarded_request = RouteDiscoveryMessage::RouteRequest {
                request_id,
                destination,
                source,
                hop_count: hop_count + 1,
                max_hops,
                timestamp: SystemTime::now(),
            };
            
            self.forward_route_message(&forwarded_request, from_peer).await?;
        }

        Ok(())
    }

    /// Handle route response message
    async fn handle_route_response(
        &self,
        request_id: String,
        destination: PeerId,
        _source: PeerId,
        route: Vec<PeerId>,
        cost: u32,
        _timestamp: SystemTime,
    ) -> Result<(), TransportError> {
        // Check if this is a response to our request
        let is_our_request = {
            let discoveries = self.active_discoveries.read().await;
            discoveries.contains_key(&request_id)
        };

        if is_our_request {
            // Add route to our routing table
            let route_obj = Route::new(route, cost, 80); // Default trust score
            let metrics = RouteMetrics::default_unknown();
            
            {
                let mut table = self.routing_table.write().await;
                let _ = table.add_route(destination, route_obj, metrics);
            }

            let mut stats = self.stats.write().await;
            stats.route_responses_received += 1;
            stats.routes_discovered += 1;
        }

        Ok(())
    }

    /// Handle route advertisement message
    async fn handle_route_advertisement(
        &self,
        _source: PeerId,
        routes: Vec<RouteAdvertisement>,
        _timestamp: SystemTime,
    ) -> Result<(), TransportError> {
        let mut table = self.routing_table.write().await;
        
        for route_ad in routes {
            let route = Route::new(
                vec![route_ad.destination.clone()],
                route_ad.cost,
                route_ad.trust_score,
            );
            let metrics = RouteMetrics::default_unknown();
            
            let _ = table.add_route(route_ad.destination, route, metrics);
        }

        Ok(())
    }

    /// Handle route error message
    async fn handle_route_error(
        &self,
        _source: PeerId,
        destination: PeerId,
        failed_hop: PeerId,
        _error_code: u8,
        _timestamp: SystemTime,
    ) -> Result<(), TransportError> {
        // Mark routes through the failed hop as failed
        let routes_to_mark = {
            let table = self.routing_table.read().await;
            let routes_through_failed_hop = table.get_routes_through_peer(&failed_hop);
            routes_through_failed_hop.into_iter()
                .filter(|(dest, _)| *dest == destination)
                .map(|(dest, route)| (dest, route.hops.clone()))
                .collect::<Vec<_>>()
        };
        
        if !routes_to_mark.is_empty() {
            let mut table = self.routing_table.write().await;
            for (dest, hops) in routes_to_mark {
                table.mark_route_failed(&dest, &hops);
            }
        }

        let mut stats = self.stats.write().await;
        stats.routing_failures += 1;

        Ok(())
    }

    /// Send route message to a specific peer
    async fn send_route_message_to_peer(&self, message: &RouteDiscoveryMessage, peer: &PeerId) -> Result<(), TransportError> {
        let serialized = serde_json::to_vec(message)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;
        
        self.send_direct(peer, &serialized).await
    }

    /// Forward route message to other peers (excluding the sender)
    async fn forward_route_message(&self, message: &RouteDiscoveryMessage, exclude_peer: &PeerId) -> Result<(), TransportError> {
        // In a real implementation, this would send to all connected peers except exclude_peer
        let serialized = serde_json::to_vec(message)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;
        
        // Placeholder for actual forwarding logic
        Ok(())
    }

    /// Add a trusted peer for routing
    pub async fn add_trusted_peer(&self, peer_id: PeerId) {
        let mut table = self.routing_table.write().await;
        table.add_trusted_peer(peer_id);
    }

    /// Remove a trusted peer
    pub async fn remove_trusted_peer(&self, peer_id: &PeerId) {
        let mut table = self.routing_table.write().await;
        table.remove_trusted_peer(peer_id);
    }

    /// Check if a peer is trusted
    pub async fn is_trusted_peer(&self, peer_id: &PeerId) -> bool {
        let table = self.routing_table.read().await;
        table.is_trusted_peer(peer_id)
    }

    /// Set encryption key for a peer
    pub async fn set_hop_encryption_key(&self, peer_id: PeerId, key: Vec<u8>) {
        let mut keys = self.hop_encryption_keys.write().await;
        keys.insert(peer_id, key);
    }

    /// Remove encryption key for a peer
    pub async fn remove_hop_encryption_key(&self, peer_id: &PeerId) {
        let mut keys = self.hop_encryption_keys.write().await;
        keys.remove(peer_id);
    }

    /// Get routing statistics
    pub async fn get_stats(&self) -> MeshStats {
        let stats = self.stats.read().await;
        let mut mesh_stats = stats.clone();
        
        let discoveries = self.active_discoveries.read().await;
        mesh_stats.active_route_discoveries = discoveries.len();
        
        mesh_stats
    }

    /// Get routing table statistics
    pub async fn get_routing_table_stats(&self) -> super::table::RoutingTableStats {
        let table = self.routing_table.read().await;
        table.get_stats()
    }

    /// Clean up expired routes and discoveries
    pub async fn cleanup_expired(&self) {
        // Clean up routing table
        {
            let mut table = self.routing_table.write().await;
            table.cleanup_expired_routes();
        }

        // Clean up expired route discoveries
        {
            let mut discoveries = self.active_discoveries.write().await;
            let now = SystemTime::now();
            discoveries.retain(|_, state| {
                now.duration_since(state.started_at).unwrap_or_default() < state.timeout
            });
        }
    }

    /// Start periodic maintenance tasks
    pub fn start_maintenance_tasks(router: Arc<MeshRouter>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut cleanup_interval = tokio::time::interval(Duration::from_secs(60));
            let mut advertisement_interval = tokio::time::interval(router.config.route_advertisement_interval);
            
            loop {
                tokio::select! {
                    _ = cleanup_interval.tick() => {
                        router.cleanup_expired().await;
                    }
                    _ = advertisement_interval.tick() => {
                        let _ = router.advertise_routes().await;
                    }
                }
            }
        })
    }

    /// Advertise our known routes to neighbors
    async fn advertise_routes(&self) -> Result<(), TransportError> {
        let table = self.routing_table.read().await;
        let destinations = table.get_destinations();
        
        let mut advertisements = Vec::new();
        for destination in destinations {
            if let Some(route) = table.get_best_route(&destination) {
                advertisements.push(RouteAdvertisement {
                    destination,
                    hop_count: route.hop_count,
                    cost: route.cost,
                    trust_score: route.trust_score,
                    capabilities: None, // Could be populated with destination capabilities
                });
            }
        }

        if !advertisements.is_empty() {
            let message = RouteDiscoveryMessage::RouteAdvertisement {
                source: self.local_peer_id.clone(),
                routes: advertisements,
                timestamp: SystemTime::now(),
            };
            
            self.broadcast_route_message(&message).await?;
        }

        Ok(())
    }

    /// Get all known routes
    pub async fn get_all_routes(&self) -> HashMap<PeerId, Vec<Route>> {
        let table = self.routing_table.read().await;
        let destinations = table.get_destinations();
        
        let mut all_routes = HashMap::new();
        for destination in destinations {
            let routes = table.get_routes(&destination).into_iter().cloned().collect();
            all_routes.insert(destination, routes);
        }
        
        all_routes
    }

    /// Find route to destination
    pub async fn find_route(&self, destination: &PeerId) -> Option<Route> {
        let table = self.routing_table.read().await;
        table.get_best_route(destination).cloned()
    }

    /// Update route metrics based on transmission results
    pub async fn update_route_metrics(&self, destination: &PeerId, hops: &[PeerId], success: bool) {
        let mut table = self.routing_table.write().await;
        if success {
            table.mark_route_success(destination, hops);
        } else {
            table.mark_route_failed(destination, hops);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    fn create_test_connection_manager() -> Arc<ConnectionManager> {
        Arc::new(ConnectionManager::new())
    }

    #[tokio::test]
    async fn test_mesh_router_creation() {
        let connection_manager = create_test_connection_manager();
        let config = MeshConfig::default();
        let router = MeshRouter::new("test-peer".to_string(), connection_manager, config);
        
        assert_eq!(router.local_peer_id, "test-peer");
        
        let stats = router.get_stats().await;
        assert_eq!(stats.routes_discovered, 0);
        assert_eq!(stats.messages_routed, 0);
    }

    #[tokio::test]
    async fn test_trusted_peer_management() {
        let connection_manager = create_test_connection_manager();
        let config = MeshConfig::default();
        let router = MeshRouter::new("test-peer".to_string(), connection_manager, config);
        
        let trusted_peer = "trusted-peer".to_string();
        
        assert!(!router.is_trusted_peer(&trusted_peer).await);
        
        router.add_trusted_peer(trusted_peer.clone()).await;
        assert!(router.is_trusted_peer(&trusted_peer).await);
        
        router.remove_trusted_peer(&trusted_peer).await;
        assert!(!router.is_trusted_peer(&trusted_peer).await);
    }

    #[tokio::test]
    async fn test_hop_encryption_key_management() {
        let connection_manager = create_test_connection_manager();
        let config = MeshConfig::default();
        let router = MeshRouter::new("test-peer".to_string(), connection_manager, config);
        
        let peer_id = "peer1".to_string();
        let key = vec![1, 2, 3, 4, 5, 6, 7, 8];
        
        router.set_hop_encryption_key(peer_id.clone(), key.clone()).await;
        
        // Test encryption
        let data = b"test message";
        let encrypted = router.encrypt_for_hop(&peer_id, data).await;
        assert!(encrypted.is_ok());
        
        router.remove_hop_encryption_key(&peer_id).await;
        
        // Should fail after key removal
        let encrypted = router.encrypt_for_hop(&peer_id, data).await;
        assert!(encrypted.is_err());
    }

    #[tokio::test]
    async fn test_route_discovery_message_handling() {
        let connection_manager = create_test_connection_manager();
        let config = MeshConfig::default();
        let router = MeshRouter::new("test-peer".to_string(), connection_manager, config);
        
        let message = RouteDiscoveryMessage::RouteRequest {
            request_id: "test-request".to_string(),
            destination: "dest-peer".to_string(),
            source: "source-peer".to_string(),
            hop_count: 1,
            max_hops: 5,
            timestamp: SystemTime::now(),
        };
        
        let result = router.handle_route_message(message, &"from-peer".to_string()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_route_advertisement_handling() {
        let connection_manager = create_test_connection_manager();
        let config = MeshConfig::default();
        let router = MeshRouter::new("test-peer".to_string(), connection_manager, config);
        
        let advertisements = vec![
            RouteAdvertisement {
                destination: "dest1".to_string(),
                hop_count: 2,
                cost: 100,
                trust_score: 80,
                capabilities: None,
            },
            RouteAdvertisement {
                destination: "dest2".to_string(),
                hop_count: 1,
                cost: 50,
                trust_score: 90,
                capabilities: None,
            },
        ];
        
        let message = RouteDiscoveryMessage::RouteAdvertisement {
            source: "advertiser".to_string(),
            routes: advertisements,
            timestamp: SystemTime::now(),
        };
        
        let result = router.handle_route_message(message, &"from-peer".to_string()).await;
        assert!(result.is_ok());
        
        // Check that routes were added
        let route = router.find_route(&"dest1".to_string()).await;
        assert!(route.is_some());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let connection_manager = create_test_connection_manager();
        let config = MeshConfig::default();
        let router = MeshRouter::new("test-peer".to_string(), connection_manager, config);
        
        // Should not panic on empty state
        router.cleanup_expired().await;
        
        let stats = router.get_routing_table_stats().await;
        assert_eq!(stats.total_routes, 0);
    }

    #[tokio::test]
    async fn test_message_size_validation() {
        let connection_manager = create_test_connection_manager();
        let mut config = MeshConfig::default();
        config.max_message_size = 100; // Small limit for testing
        
        let router = MeshRouter::new("test-peer".to_string(), connection_manager, config);
        
        let large_data = vec![0u8; 200]; // Exceeds limit
        let result = router.route_to_peer(&"dest".to_string(), &large_data).await;
        
        assert!(result.is_err());
        if let Err(TransportError::InvalidRoute { reason }) = result {
            assert!(reason.contains("Message size"));
        }
    }

    #[tokio::test]
    async fn test_get_all_routes() {
        let connection_manager = create_test_connection_manager();
        let config = MeshConfig::default();
        let router = MeshRouter::new("test-peer".to_string(), connection_manager, config);
        
        let all_routes = router.get_all_routes().await;
        assert!(all_routes.is_empty());
    }
}