use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};

use crate::transport::{PeerId, TransportError};

/// A route to reach a destination peer through intermediate hops
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Route {
    /// Sequence of peer IDs to reach the destination
    pub hops: Vec<PeerId>,
    /// Cost metric for this route (lower is better)
    pub cost: u32,
    /// When this route was last updated
    pub last_updated: SystemTime,
    /// Hop count (derived from hops.len())
    pub hop_count: u8,
    /// Trust score for this route (0-100)
    pub trust_score: u8,
}

impl Route {
    /// Create a new route
    pub fn new(hops: Vec<PeerId>, cost: u32, trust_score: u8) -> Self {
        let hop_count = hops.len() as u8;
        Self {
            hops,
            cost,
            last_updated: SystemTime::now(),
            hop_count,
            trust_score,
        }
    }

    /// Create a direct route (no intermediate hops)
    pub fn direct(peer_id: PeerId, cost: u32) -> Self {
        Self::new(vec![peer_id], cost, 100)
    }

    /// Get the next hop in this route
    pub fn next_hop(&self) -> Option<&PeerId> {
        self.hops.first()
    }

    /// Get the destination peer ID
    pub fn destination(&self) -> Option<&PeerId> {
        self.hops.last()
    }

    /// Check if this route is expired
    pub fn is_expired(&self, max_age: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.last_updated)
            .unwrap_or_default() > max_age
    }

    /// Update the route timestamp
    pub fn refresh(&mut self) {
        self.last_updated = SystemTime::now();
    }

    /// Calculate route quality score (0-100)
    pub fn quality_score(&self) -> u8 {
        let mut score = self.trust_score;
        
        // Penalize longer routes
        if self.hop_count > 3 {
            score = score.saturating_sub(20);
        } else if self.hop_count > 1 {
            score = score.saturating_sub(10);
        }
        
        // Penalize high cost routes
        if self.cost > 1000 {
            score = score.saturating_sub(30);
        } else if self.cost > 100 {
            score = score.saturating_sub(15);
        }
        
        score
    }

    /// Check if route contains a loop
    pub fn has_loop(&self) -> bool {
        let mut seen = std::collections::HashSet::new();
        for hop in &self.hops {
            if !seen.insert(hop) {
                return true;
            }
        }
        false
    }

    /// Prepend a hop to the route
    pub fn prepend_hop(&mut self, peer_id: PeerId) {
        self.hops.insert(0, peer_id);
        self.hop_count = self.hops.len() as u8;
        self.cost += 10; // Add cost for additional hop
        self.refresh();
    }
}

/// Metrics for route selection and maintenance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteMetrics {
    /// Latency in milliseconds
    pub latency_ms: u32,
    /// Bandwidth in bytes per second
    pub bandwidth_bps: u64,
    /// Reliability score (0-100)
    pub reliability: u8,
    /// Number of successful transmissions
    pub success_count: u32,
    /// Number of failed transmissions
    pub failure_count: u32,
    /// Last measurement timestamp
    pub last_measured: SystemTime,
}

impl RouteMetrics {
    /// Create new metrics
    pub fn new(latency_ms: u32, bandwidth_bps: u64, reliability: u8) -> Self {
        Self {
            latency_ms,
            bandwidth_bps,
            reliability,
            success_count: 0,
            failure_count: 0,
            last_measured: SystemTime::now(),
        }
    }

    /// Create default metrics for unknown routes
    pub fn default_unknown() -> Self {
        Self::new(1000, 1024 * 1024, 50) // 1s latency, 1MB/s bandwidth, 50% reliability
    }

    /// Record a successful transmission
    pub fn record_success(&mut self) {
        self.success_count += 1;
        self.update_reliability();
        self.last_measured = SystemTime::now();
    }

    /// Record a failed transmission
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        self.update_reliability();
        self.last_measured = SystemTime::now();
    }

    /// Update reliability based on success/failure ratio
    fn update_reliability(&mut self) {
        let total = self.success_count + self.failure_count;
        if total > 0 {
            self.reliability = ((self.success_count * 100) / total) as u8;
        }
    }

    /// Calculate cost based on metrics
    pub fn calculate_cost(&self) -> u32 {
        let latency_cost = self.latency_ms / 10; // 10ms = 1 cost unit
        let bandwidth_cost = if self.bandwidth_bps > 0 {
            (1_000_000 / self.bandwidth_bps) as u32 // Higher bandwidth = lower cost
        } else {
            1000
        };
        let reliability_cost = (100 - self.reliability as u32) * 2;
        
        latency_cost + bandwidth_cost + reliability_cost
    }

    /// Check if metrics are stale
    pub fn is_stale(&self, max_age: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.last_measured)
            .unwrap_or_default() > max_age
    }
}

/// Entry in the routing table
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// The route information
    pub route: Route,
    /// Performance metrics for this route
    pub metrics: RouteMetrics,
    /// Whether this route is currently active
    pub active: bool,
    /// Number of times this route has been used
    pub usage_count: u32,
}

impl RouteEntry {
    /// Create a new route entry
    pub fn new(route: Route, metrics: RouteMetrics) -> Self {
        Self {
            route,
            metrics,
            active: true,
            usage_count: 0,
        }
    }

    /// Mark route as used
    pub fn mark_used(&mut self) {
        self.usage_count += 1;
        self.route.refresh();
    }

    /// Check if this entry should be removed
    pub fn should_expire(&self, max_age: Duration) -> bool {
        !self.active || self.route.is_expired(max_age) || self.metrics.is_stale(max_age)
    }

    /// Calculate overall route score for selection
    pub fn selection_score(&self) -> u32 {
        if !self.active {
            return 0;
        }
        
        let quality = self.route.quality_score() as u32;
        let reliability = self.metrics.reliability as u32;
        let usage_bonus = (self.usage_count.min(10) * 2) as u32; // Bonus for proven routes
        
        quality + reliability + usage_bonus - self.route.cost
    }
}

/// Routing table for managing routes to peers
#[derive(Debug)]
pub struct RoutingTable {
    /// Routes indexed by destination peer ID
    routes: HashMap<PeerId, Vec<RouteEntry>>,
    /// Maximum number of routes per destination
    max_routes_per_destination: usize,
    /// Maximum route age before expiration
    max_route_age: Duration,
    /// Maximum hop count allowed
    max_hop_count: u8,
    /// Trusted peers for routing
    trusted_peers: std::collections::HashSet<PeerId>,
}

impl RoutingTable {
    /// Create a new routing table
    pub fn new(max_routes_per_destination: usize, max_route_age: Duration, max_hop_count: u8) -> Self {
        Self {
            routes: HashMap::new(),
            max_routes_per_destination,
            max_route_age,
            max_hop_count,
            trusted_peers: std::collections::HashSet::new(),
        }
    }

    /// Add or update a route to a destination
    pub fn add_route(&mut self, destination: PeerId, route: Route, metrics: RouteMetrics) -> Result<(), TransportError> {
        // Validate route
        if route.hop_count > self.max_hop_count {
            return Err(TransportError::InvalidRoute {
                reason: format!("Route exceeds maximum hop count: {} > {}", route.hop_count, self.max_hop_count),
            });
        }

        if route.has_loop() {
            return Err(TransportError::InvalidRoute {
                reason: "Route contains a loop".to_string(),
            });
        }

        // Check if any hop is trusted (for multi-hop routes)
        if route.hop_count > 1 {
            let has_trusted_hop = route.hops.iter().any(|hop| self.trusted_peers.contains(hop));
            if !has_trusted_hop {
                return Err(TransportError::InvalidRoute {
                    reason: "Multi-hop route must contain at least one trusted peer".to_string(),
                });
            }
        }

        let destination_routes = self.routes.entry(destination.clone()).or_insert_with(Vec::new);
        
        // Check if we already have this exact route
        if let Some(existing_entry) = destination_routes.iter_mut().find(|entry| entry.route.hops == route.hops) {
            // Update existing route
            existing_entry.route = route;
            existing_entry.metrics = metrics;
            existing_entry.active = true;
            return Ok(());
        }

        // Add new route
        let entry = RouteEntry::new(route, metrics);
        destination_routes.push(entry);

        // Enforce maximum routes per destination
        if destination_routes.len() > self.max_routes_per_destination {
            // Sort by selection score and keep the best ones
            destination_routes.sort_by(|a, b| b.selection_score().cmp(&a.selection_score()));
            destination_routes.truncate(self.max_routes_per_destination);
        }

        Ok(())
    }

    /// Get the best route to a destination
    pub fn get_best_route(&self, destination: &PeerId) -> Option<&Route> {
        self.routes.get(destination)?
            .iter()
            .filter(|entry| entry.active && !entry.should_expire(self.max_route_age))
            .max_by_key(|entry| entry.selection_score())
            .map(|entry| &entry.route)
    }

    /// Get all routes to a destination
    pub fn get_routes(&self, destination: &PeerId) -> Vec<&Route> {
        self.routes.get(destination)
            .map(|entries| {
                entries.iter()
                    .filter(|entry| entry.active && !entry.should_expire(self.max_route_age))
                    .map(|entry| &entry.route)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Remove a specific route
    pub fn remove_route(&mut self, destination: &PeerId, hops: &[PeerId]) {
        if let Some(destination_routes) = self.routes.get_mut(destination) {
            destination_routes.retain(|entry| entry.route.hops != hops);
            if destination_routes.is_empty() {
                self.routes.remove(destination);
            }
        }
    }

    /// Mark a route as failed
    pub fn mark_route_failed(&mut self, destination: &PeerId, hops: &[PeerId]) {
        if let Some(destination_routes) = self.routes.get_mut(destination) {
            if let Some(entry) = destination_routes.iter_mut().find(|entry| entry.route.hops == hops) {
                entry.metrics.record_failure();
                if entry.metrics.reliability < 20 {
                    entry.active = false;
                }
            }
        }
    }

    /// Mark a route as successful
    pub fn mark_route_success(&mut self, destination: &PeerId, hops: &[PeerId]) {
        if let Some(destination_routes) = self.routes.get_mut(destination) {
            if let Some(entry) = destination_routes.iter_mut().find(|entry| entry.route.hops == hops) {
                entry.metrics.record_success();
                entry.mark_used();
            }
        }
    }

    /// Add a trusted peer
    pub fn add_trusted_peer(&mut self, peer_id: PeerId) {
        self.trusted_peers.insert(peer_id);
    }

    /// Remove a trusted peer
    pub fn remove_trusted_peer(&mut self, peer_id: &PeerId) {
        self.trusted_peers.remove(peer_id);
    }

    /// Check if a peer is trusted
    pub fn is_trusted_peer(&self, peer_id: &PeerId) -> bool {
        self.trusted_peers.contains(peer_id)
    }

    /// Clean up expired routes
    pub fn cleanup_expired_routes(&mut self) {
        let mut destinations_to_remove = Vec::new();
        
        for (destination, routes) in &mut self.routes {
            routes.retain(|entry| !entry.should_expire(self.max_route_age));
            if routes.is_empty() {
                destinations_to_remove.push(destination.clone());
            }
        }
        
        for destination in destinations_to_remove {
            self.routes.remove(&destination);
        }
    }

    /// Get all known destinations
    pub fn get_destinations(&self) -> Vec<PeerId> {
        self.routes.keys().cloned().collect()
    }

    /// Get routing table statistics
    pub fn get_stats(&self) -> RoutingTableStats {
        let total_routes = self.routes.values().map(|routes| routes.len()).sum();
        let active_routes = self.routes.values()
            .flat_map(|routes| routes.iter())
            .filter(|entry| entry.active && !entry.should_expire(self.max_route_age))
            .count();
        
        let destinations = self.routes.len();
        let trusted_peers = self.trusted_peers.len();
        
        let mut hop_distribution = HashMap::new();
        for routes in self.routes.values() {
            for entry in routes {
                if entry.active {
                    *hop_distribution.entry(entry.route.hop_count).or_insert(0) += 1;
                }
            }
        }

        RoutingTableStats {
            total_routes,
            active_routes,
            destinations,
            trusted_peers,
            hop_distribution,
        }
    }

    /// Clear all routes
    pub fn clear(&mut self) {
        self.routes.clear();
    }

    /// Set maximum hop count
    pub fn set_max_hop_count(&mut self, max_hops: u8) {
        self.max_hop_count = max_hops;
        
        // Remove routes that exceed the new limit
        for routes in self.routes.values_mut() {
            routes.retain(|entry| entry.route.hop_count <= max_hops);
        }
    }

    /// Get routes through a specific peer
    pub fn get_routes_through_peer(&self, peer_id: &PeerId) -> Vec<(PeerId, &Route)> {
        let mut routes_through_peer = Vec::new();
        
        for (destination, routes) in &self.routes {
            for entry in routes {
                if entry.active && 
                   !entry.should_expire(self.max_route_age) &&
                   entry.route.hops.contains(peer_id) {
                    routes_through_peer.push((destination.clone(), &entry.route));
                }
            }
        }
        
        routes_through_peer
    }
}

/// Statistics about the routing table
#[derive(Debug, Clone)]
pub struct RoutingTableStats {
    pub total_routes: usize,
    pub active_routes: usize,
    pub destinations: usize,
    pub trusted_peers: usize,
    pub hop_distribution: HashMap<u8, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_creation() {
        let hops = vec!["peer1".to_string(), "peer2".to_string()];
        let route = Route::new(hops.clone(), 100, 80);
        
        assert_eq!(route.hops, hops);
        assert_eq!(route.cost, 100);
        assert_eq!(route.hop_count, 2);
        assert_eq!(route.trust_score, 80);
        assert_eq!(route.next_hop(), Some(&"peer1".to_string()));
        assert_eq!(route.destination(), Some(&"peer2".to_string()));
    }

    #[test]
    fn test_direct_route() {
        let route = Route::direct("peer1".to_string(), 50);
        
        assert_eq!(route.hops.len(), 1);
        assert_eq!(route.hop_count, 1);
        assert_eq!(route.cost, 50);
        assert_eq!(route.trust_score, 100);
        assert_eq!(route.next_hop(), Some(&"peer1".to_string()));
        assert_eq!(route.destination(), Some(&"peer1".to_string()));
    }

    #[test]
    fn test_route_loop_detection() {
        let hops_with_loop = vec!["peer1".to_string(), "peer2".to_string(), "peer1".to_string()];
        let route_with_loop = Route::new(hops_with_loop, 100, 80);
        assert!(route_with_loop.has_loop());
        
        let hops_no_loop = vec!["peer1".to_string(), "peer2".to_string(), "peer3".to_string()];
        let route_no_loop = Route::new(hops_no_loop, 100, 80);
        assert!(!route_no_loop.has_loop());
    }

    #[test]
    fn test_route_metrics() {
        let mut metrics = RouteMetrics::new(100, 1024 * 1024, 90);
        
        assert_eq!(metrics.latency_ms, 100);
        assert_eq!(metrics.bandwidth_bps, 1024 * 1024);
        assert_eq!(metrics.reliability, 90);
        
        metrics.record_success();
        assert_eq!(metrics.success_count, 1);
        assert_eq!(metrics.reliability, 100); // 1 success, 0 failures = 100%
        
        metrics.record_failure();
        assert_eq!(metrics.failure_count, 1);
        assert_eq!(metrics.reliability, 50); // 1 success, 1 failure = 50%
    }

    #[test]
    fn test_routing_table_basic_operations() {
        let mut table = RoutingTable::new(3, Duration::from_secs(300), 5);
        
        let destination = "dest1".to_string();
        let route = Route::direct("peer1".to_string(), 100);
        let metrics = RouteMetrics::new(50, 1024 * 1024, 95);
        
        assert!(table.add_route(destination.clone(), route, metrics).is_ok());
        
        let best_route = table.get_best_route(&destination);
        assert!(best_route.is_some());
        assert_eq!(best_route.unwrap().next_hop(), Some(&"peer1".to_string()));
        
        let stats = table.get_stats();
        assert_eq!(stats.total_routes, 1);
        assert_eq!(stats.active_routes, 1);
        assert_eq!(stats.destinations, 1);
    }

    #[test]
    fn test_routing_table_trusted_peers() {
        let mut table = RoutingTable::new(3, Duration::from_secs(300), 5);
        
        table.add_trusted_peer("trusted1".to_string());
        assert!(table.is_trusted_peer(&"trusted1".to_string()));
        assert!(!table.is_trusted_peer(&"untrusted".to_string()));
        
        table.remove_trusted_peer(&"trusted1".to_string());
        assert!(!table.is_trusted_peer(&"trusted1".to_string()));
    }

    #[test]
    fn test_routing_table_multi_hop_validation() {
        let mut table = RoutingTable::new(3, Duration::from_secs(300), 5);
        
        // Multi-hop route without trusted peers should fail
        let destination = "dest1".to_string();
        let route = Route::new(vec!["peer1".to_string(), "peer2".to_string()], 200, 80);
        let metrics = RouteMetrics::new(100, 1024 * 1024, 90);
        
        let result = table.add_route(destination.clone(), route, metrics);
        assert!(result.is_err());
        
        // Add trusted peer and try again
        table.add_trusted_peer("peer1".to_string());
        let route = Route::new(vec!["peer1".to_string(), "peer2".to_string()], 200, 80);
        let metrics = RouteMetrics::new(100, 1024 * 1024, 90);
        
        let result = table.add_route(destination, route, metrics);
        assert!(result.is_ok());
    }

    #[test]
    fn test_routing_table_hop_limit() {
        let mut table = RoutingTable::new(3, Duration::from_secs(300), 2);
        
        let destination = "dest1".to_string();
        let route = Route::new(
            vec!["peer1".to_string(), "peer2".to_string(), "peer3".to_string()], 
            300, 
            80
        );
        let metrics = RouteMetrics::new(150, 1024 * 1024, 85);
        
        let result = table.add_route(destination, route, metrics);
        assert!(result.is_err()); // Should fail due to hop count > 2
    }

    #[test]
    fn test_route_quality_scoring() {
        let high_quality_route = Route::direct("peer1".to_string(), 50);
        assert!(high_quality_route.quality_score() > 80);
        
        let low_quality_route = Route::new(
            vec!["peer1".to_string(), "peer2".to_string(), "peer3".to_string(), "peer4".to_string()],
            2000,
            30
        );
        assert!(low_quality_route.quality_score() < 50);
    }
}