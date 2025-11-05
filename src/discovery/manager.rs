use crate::discovery::{Discovery, DiscoveryError, ServiceRecord};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct StrategyStats {
    pub name: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_success: Option<SystemTime>,
    pub last_failure: Option<SystemTime>,
    pub average_response_time: Duration,
    pub total_peers_discovered: u64,
}

impl StrategyStats {
    pub fn new(name: String) -> Self {
        Self {
            name,
            success_count: 0,
            failure_count: 0,
            last_success: None,
            last_failure: None,
            average_response_time: Duration::ZERO,
            total_peers_discovered: 0,
        }
    }

    pub fn success_rate(&self) -> f64 {
        let total = self.success_count + self.failure_count;
        if total == 0 {
            0.0
        } else {
            self.success_count as f64 / total as f64
        }
    }
}

pub struct DiscoveryManager {
    strategies: Vec<Box<dyn Discovery>>,
    auto_select: bool,
    active_strategy: Option<String>,
    discovered_peers: Arc<RwLock<HashMap<String, ServiceRecord>>>,
    peer_ttl: Duration,
    strategy_stats: Arc<RwLock<HashMap<String, StrategyStats>>>,
    concurrent_discovery: bool,
    max_concurrent_strategies: usize,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
            auto_select: true,
            active_strategy: None,
            discovered_peers: Arc::new(RwLock::new(HashMap::new())),
            peer_ttl: Duration::from_secs(300), // 5 minutes default TTL
            strategy_stats: Arc::new(RwLock::new(HashMap::new())),
            concurrent_discovery: false,
            max_concurrent_strategies: 3,
        }
    }

    pub fn add_strategy(&mut self, strategy: Box<dyn Discovery>) {
        let strategy_name = strategy.strategy_name().to_string();
        self.strategies.push(strategy);
        
        // Initialize stats for this strategy synchronously
        let stats = Arc::clone(&self.strategy_stats);
        let name = strategy_name.clone();
        tokio::spawn(async move {
            let mut stats_guard = stats.write().await;
            stats_guard.insert(name.clone(), StrategyStats::new(name));
        });
    }

    /// Add strategy and wait for stats initialization (useful for testing)
    pub async fn add_strategy_async(&mut self, strategy: Box<dyn Discovery>) {
        let strategy_name = strategy.strategy_name().to_string();
        self.strategies.push(strategy);
        
        // Initialize stats for this strategy and wait for completion
        let mut stats_guard = self.strategy_stats.write().await;
        stats_guard.insert(strategy_name.clone(), StrategyStats::new(strategy_name));
    }

    pub fn set_auto_select(&mut self, enabled: bool) {
        self.auto_select = enabled;
    }

    pub fn set_peer_ttl(&mut self, ttl: Duration) {
        self.peer_ttl = ttl;
    }

    pub fn set_concurrent_discovery(&mut self, enabled: bool) {
        self.concurrent_discovery = enabled;
    }

    pub fn set_max_concurrent_strategies(&mut self, max: usize) {
        self.max_concurrent_strategies = max.max(1); // At least 1
    }

    pub async fn discover_peers(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        // Clean up expired peers before discovery
        self.cleanup_expired_peers().await;

        if self.concurrent_discovery {
            self.discover_concurrent(timeout).await
        } else if self.auto_select {
            self.discover_with_auto_select(timeout).await
        } else if let Some(strategy_name) = &self.active_strategy {
            self.discover_with_strategy(strategy_name, timeout).await
        } else {
            // Use the first available strategy
            if let Some(strategy) = self.strategies.first() {
                let peers = self.discover_with_single_strategy(strategy.as_ref(), timeout).await?;
                self.update_peer_cache(&peers).await;
                Ok(peers)
            } else {
                Err(DiscoveryError::StrategyUnavailable {
                    strategy: "none".to_string(),
                })
            }
        }
    }

    pub async fn announce_presence(&self) -> Result<(), DiscoveryError> {
        let mut errors = Vec::new();
        
        for strategy in &self.strategies {
            if strategy.is_available() {
                if let Err(e) = strategy.announce().await {
                    errors.push(format!("{}: {}", strategy.strategy_name(), e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(DiscoveryError::Configuration(format!(
                "Some announcements failed: {}",
                errors.join(", ")
            )))
        }
    }

    pub async fn stop_announce(&self) -> Result<(), DiscoveryError> {
        let mut errors = Vec::new();
        
        for strategy in &self.strategies {
            if let Err(e) = strategy.stop_announce().await {
                errors.push(format!("{}: {}", strategy.strategy_name(), e));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(DiscoveryError::Configuration(format!(
                "Some stop operations failed: {}",
                errors.join(", ")
            )))
        }
    }

    pub async fn get_discovered_peers(&self) -> Vec<ServiceRecord> {
        let peers = self.discovered_peers.read().await;
        
        peers
            .values()
            .filter(|peer| !peer.is_expired(self.peer_ttl))
            .cloned()
            .collect()
    }

    pub async fn cleanup_expired_peers(&self) {
        let mut peers = self.discovered_peers.write().await;
        
        peers.retain(|_, peer| !peer.is_expired(self.peer_ttl));
    }

    async fn discover_with_auto_select(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        // Get available strategies
        let available_strategies: Vec<_> = self.strategies
            .iter()
            .filter(|s| s.is_available())
            .collect();

        if available_strategies.is_empty() {
            return Err(DiscoveryError::StrategyUnavailable {
                strategy: "all".to_string(),
            });
        }

        // Try to use the best performing strategy first, then fall back to priority order
        let best_strategy_name = self.get_best_strategy().await;
        
        if let Some(best_name) = best_strategy_name {
            if let Some(strategy) = available_strategies.iter().find(|s| s.strategy_name() == best_name) {
                match self.discover_with_single_strategy(strategy.as_ref(), timeout).await {
                    Ok(peers) => {
                        self.update_peer_cache(&peers).await;
                        return Ok(peers);
                    }
                    Err(_) => {
                        // Continue to priority-based selection
                    }
                }
            }
        }

        // Fall back to priority-based selection
        let mut sorted_strategies = available_strategies;
        sorted_strategies.sort_by_key(|s| std::cmp::Reverse(s.priority()));

        let mut last_error = None;
        
        for strategy in sorted_strategies {
            match self.discover_with_single_strategy(strategy.as_ref(), timeout).await {
                Ok(peers) => {
                    self.update_peer_cache(&peers).await;
                    return Ok(peers);
                }
                Err(e) => {
                    last_error = Some(e);
                    // Continue to next strategy
                }
            }
        }

        // All strategies failed
        Err(last_error.unwrap_or_else(|| DiscoveryError::StrategyUnavailable {
            strategy: "auto-select".to_string(),
        }))
    }

    async fn discover_with_strategy(&self, strategy_name: &str, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        let strategy = self.strategies
            .iter()
            .find(|s| s.strategy_name() == strategy_name)
            .ok_or_else(|| DiscoveryError::StrategyUnavailable {
                strategy: strategy_name.to_string(),
            })?;

        if !strategy.is_available() {
            return Err(DiscoveryError::StrategyUnavailable {
                strategy: strategy_name.to_string(),
            });
        }

        let peers = self.discover_with_single_strategy(strategy.as_ref(), timeout).await?;
        self.update_peer_cache(&peers).await;
        Ok(peers)
    }

    async fn discover_concurrent(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        let available_strategies: Vec<_> = self.strategies
            .iter()
            .filter(|s| s.is_available())
            .take(self.max_concurrent_strategies)
            .collect();

        if available_strategies.is_empty() {
            return Err(DiscoveryError::StrategyUnavailable {
                strategy: "all".to_string(),
            });
        }

        let mut tasks = Vec::new();
        
        for strategy in available_strategies {
            let strategy_ref = strategy.as_ref();
            let task = self.discover_with_single_strategy(strategy_ref, timeout);
            tasks.push(task);
        }

        // Wait for all strategies to complete
        let results = futures::future::join_all(tasks).await;
        
        let mut all_peers = Vec::new();
        let mut had_success = false;

        for result in results {
            match result {
                Ok(peers) => {
                    all_peers.extend(peers);
                    had_success = true;
                }
                Err(_) => {
                    // Continue with other results
                }
            }
        }

        if had_success {
            // Deduplicate peers by peer_id and merge records
            let mut unique_peers: HashMap<String, ServiceRecord> = HashMap::new();
            for peer in all_peers {
                if let Some(existing) = unique_peers.get_mut(&peer.peer_id) {
                    existing.merge(peer);
                } else {
                    unique_peers.insert(peer.peer_id.clone(), peer);
                }
            }

            let final_peers: Vec<ServiceRecord> = unique_peers.into_values().collect();
            self.update_peer_cache(&final_peers).await;
            Ok(final_peers)
        } else {
            Err(DiscoveryError::StrategyUnavailable {
                strategy: "concurrent".to_string(),
            })
        }
    }

    async fn discover_with_single_strategy(&self, strategy: &dyn Discovery, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        let start_time = Instant::now();
        let strategy_name = strategy.strategy_name().to_string();
        
        let result = strategy.discover(timeout).await;
        let elapsed = start_time.elapsed();
        
        // Update strategy statistics
        self.update_strategy_stats(&strategy_name, &result, elapsed, result.as_ref().map(|p| p.len()).unwrap_or(0)).await;
        
        result
    }

    async fn update_strategy_stats(&self, strategy_name: &str, result: &Result<Vec<ServiceRecord>, DiscoveryError>, elapsed: Duration, peer_count: usize) {
        let mut stats = self.strategy_stats.write().await;
        if let Some(stat) = stats.get_mut(strategy_name) {
            match result {
                Ok(_) => {
                    stat.success_count += 1;
                    stat.last_success = Some(SystemTime::now());
                    stat.total_peers_discovered += peer_count as u64;
                    
                    // Update average response time (simple moving average)
                    if stat.success_count == 1 {
                        stat.average_response_time = elapsed;
                    } else {
                        let total_time = stat.average_response_time * (stat.success_count - 1) as u32 + elapsed;
                        stat.average_response_time = total_time / stat.success_count as u32;
                    }
                }
                Err(_) => {
                    stat.failure_count += 1;
                    stat.last_failure = Some(SystemTime::now());
                }
            }
        }
    }

    async fn update_peer_cache(&self, peers: &[ServiceRecord]) {
        let mut cache = self.discovered_peers.write().await;
        
        for peer in peers {
            if let Some(existing) = cache.get_mut(&peer.peer_id) {
                // Merge with existing record
                existing.merge(peer.clone());
            } else {
                // Add new peer
                cache.insert(peer.peer_id.clone(), peer.clone());
            }
        }
    }

    pub fn get_available_strategies(&self) -> Vec<String> {
        self.strategies
            .iter()
            .filter(|s| s.is_available())
            .map(|s| s.strategy_name().to_string())
            .collect()
    }

    pub fn set_active_strategy(&mut self, strategy_name: Option<String>) {
        self.active_strategy = strategy_name;
    }

    /// Get statistics for all strategies
    pub async fn get_strategy_stats(&self) -> HashMap<String, StrategyStats> {
        self.strategy_stats.read().await.clone()
    }

    /// Get the best performing strategy based on success rate and response time
    pub async fn get_best_strategy(&self) -> Option<String> {
        let stats = self.strategy_stats.read().await;
        
        let mut best_strategy = None;
        let mut best_score = 0.0;
        
        for (name, stat) in stats.iter() {
            if stat.success_count == 0 {
                continue; // Skip strategies that haven't been used
            }
            
            let success_rate = stat.success_rate();
            let response_time_score = if stat.average_response_time.as_millis() > 0 {
                1000.0 / stat.average_response_time.as_millis() as f64
            } else {
                1000.0
            };
            
            // Combine success rate (0-1) with response time score
            let score = success_rate * 0.7 + (response_time_score / 1000.0) * 0.3;
            
            if score > best_score {
                best_score = score;
                best_strategy = Some(name.clone());
            }
        }
        
        best_strategy
    }

    /// Get a specific peer by ID
    pub async fn get_peer(&self, peer_id: &str) -> Option<ServiceRecord> {
        let peers = self.discovered_peers.read().await;
        peers.get(peer_id).filter(|p| !p.is_expired(self.peer_ttl)).cloned()
    }

    /// Get peers discovered by a specific strategy
    pub async fn get_peers_by_strategy(&self, strategy_name: &str) -> Vec<ServiceRecord> {
        let peers = self.discovered_peers.read().await;
        peers
            .values()
            .filter(|peer| peer.discovery_method == strategy_name && !peer.is_expired(self.peer_ttl))
            .cloned()
            .collect()
    }

    /// Get the total number of discovered peers (including expired ones)
    pub async fn total_peer_count(&self) -> usize {
        let peers = self.discovered_peers.read().await;
        peers.len()
    }

    /// Get the number of active (non-expired) peers
    pub async fn active_peer_count(&self) -> usize {
        let peers = self.discovered_peers.read().await;
        peers.values().filter(|p| !p.is_expired(self.peer_ttl)).count()
    }

    /// Clear all discovered peers
    pub async fn clear_peers(&self) {
        let mut peers = self.discovered_peers.write().await;
        peers.clear();
    }

    /// Reset all strategy statistics
    pub async fn reset_stats(&self) {
        let mut stats = self.strategy_stats.write().await;
        for stat in stats.values_mut() {
            *stat = StrategyStats::new(stat.name.clone());
        }
    }
}

impl Default for DiscoveryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::strategies::udp::UdpDiscovery;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    // Mock strategy for testing
    struct MockDiscovery {
        name: &'static str,
        available: bool,
        priority: u8,
        should_fail: bool,
        peers_to_return: Vec<ServiceRecord>,
    }

    impl MockDiscovery {
        fn new(name: &'static str, available: bool, priority: u8) -> Self {
            Self {
                name,
                available,
                priority,
                should_fail: false,
                peers_to_return: Vec::new(),
            }
        }

        fn with_peers(mut self, peers: Vec<ServiceRecord>) -> Self {
            self.peers_to_return = peers;
            self
        }

        fn with_failure(mut self) -> Self {
            self.should_fail = true;
            self
        }
    }

    #[async_trait::async_trait]
    impl Discovery for MockDiscovery {
        async fn discover(&self, _timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
            if self.should_fail {
                Err(DiscoveryError::Network(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Mock failure"
                )))
            } else {
                Ok(self.peers_to_return.clone())
            }
        }

        async fn announce(&self) -> Result<(), DiscoveryError> {
            if self.should_fail {
                Err(DiscoveryError::Network(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Mock failure"
                )))
            } else {
                Ok(())
            }
        }

        async fn stop_announce(&self) -> Result<(), DiscoveryError> {
            Ok(())
        }

        fn strategy_name(&self) -> &'static str {
            self.name
        }

        fn is_available(&self) -> bool {
            self.available
        }

        fn priority(&self) -> u8 {
            self.priority
        }
    }

    #[tokio::test]
    async fn test_discovery_manager_creation() {
        let manager = DiscoveryManager::new();
        assert!(manager.auto_select);
        assert!(manager.active_strategy.is_none());
        assert_eq!(manager.get_available_strategies().len(), 0);
    }

    #[tokio::test]
    async fn test_add_strategy() {
        let mut manager = DiscoveryManager::new();
        let mock_strategy = MockDiscovery::new("mock", true, 50);
        
        manager.add_strategy(Box::new(mock_strategy));
        
        let available = manager.get_available_strategies();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0], "mock");
    }

    #[tokio::test]
    async fn test_discover_with_single_strategy() {
        let mut manager = DiscoveryManager::new();
        
        let mut peer = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        peer.add_address(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080));
        
        let mock_strategy = MockDiscovery::new("mock", true, 50)
            .with_peers(vec![peer.clone()]);
        
        manager.add_strategy(Box::new(mock_strategy));
        
        let result = manager.discover_peers(Duration::from_secs(5)).await;
        assert!(result.is_ok());
        
        let peers = result.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, "peer-123");
    }

    #[tokio::test]
    async fn test_auto_select_priority_order() {
        let mut manager = DiscoveryManager::new();
        
        let peer1 = ServiceRecord::new("peer-1".to_string(), "Device 1".to_string(), 8080);
        let peer2 = ServiceRecord::new("peer-2".to_string(), "Device 2".to_string(), 8080);
        
        // Add strategies in reverse priority order
        let low_priority = MockDiscovery::new("low", true, 30)
            .with_peers(vec![peer1]);
        let high_priority = MockDiscovery::new("high", true, 80)
            .with_peers(vec![peer2]);
        
        manager.add_strategy(Box::new(low_priority));
        manager.add_strategy(Box::new(high_priority));
        
        let result = manager.discover_peers(Duration::from_secs(5)).await;
        assert!(result.is_ok());
        
        let peers = result.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, "peer-2"); // Should use high priority strategy
    }

    #[tokio::test]
    async fn test_fallback_on_strategy_failure() {
        let mut manager = DiscoveryManager::new();
        
        let peer = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        
        let failing_strategy = MockDiscovery::new("failing", true, 80).with_failure();
        let working_strategy = MockDiscovery::new("working", true, 50)
            .with_peers(vec![peer.clone()]);
        
        manager.add_strategy(Box::new(failing_strategy));
        manager.add_strategy(Box::new(working_strategy));
        
        let result = manager.discover_peers(Duration::from_secs(5)).await;
        assert!(result.is_ok());
        
        let peers = result.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, "peer-123");
    }

    #[tokio::test]
    async fn test_concurrent_discovery() {
        let mut manager = DiscoveryManager::new();
        manager.set_concurrent_discovery(true);
        
        let peer1 = ServiceRecord::new("peer-1".to_string(), "Device 1".to_string(), 8080);
        let peer2 = ServiceRecord::new("peer-2".to_string(), "Device 2".to_string(), 8080);
        
        let strategy1 = MockDiscovery::new("strategy1", true, 50)
            .with_peers(vec![peer1]);
        let strategy2 = MockDiscovery::new("strategy2", true, 60)
            .with_peers(vec![peer2]);
        
        manager.add_strategy(Box::new(strategy1));
        manager.add_strategy(Box::new(strategy2));
        
        let result = manager.discover_peers(Duration::from_secs(5)).await;
        assert!(result.is_ok());
        
        let peers = result.unwrap();
        assert_eq!(peers.len(), 2); // Should get peers from both strategies
    }

    #[tokio::test]
    async fn test_peer_deduplication_in_concurrent_mode() {
        let mut manager = DiscoveryManager::new();
        manager.set_concurrent_discovery(true);
        
        // Same peer discovered by different strategies
        let peer1 = ServiceRecord::new("peer-123".to_string(), "Device 1".to_string(), 8080);
        let mut peer2 = ServiceRecord::new("peer-123".to_string(), "Device 1 Enhanced".to_string(), 8080);
        peer2.add_capability("version".to_string(), "1.0.0".to_string());
        
        let strategy1 = MockDiscovery::new("strategy1", true, 50)
            .with_peers(vec![peer1]);
        let strategy2 = MockDiscovery::new("strategy2", true, 60)
            .with_peers(vec![peer2]);
        
        manager.add_strategy(Box::new(strategy1));
        manager.add_strategy(Box::new(strategy2));
        
        let result = manager.discover_peers(Duration::from_secs(5)).await;
        assert!(result.is_ok());
        
        let peers = result.unwrap();
        assert_eq!(peers.len(), 1); // Should be deduplicated
        assert!(peers[0].has_capability("version")); // Should be merged
    }

    #[tokio::test]
    async fn test_peer_cache_management() {
        let mut manager = DiscoveryManager::new();
        
        let peer = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        let mock_strategy = MockDiscovery::new("mock", true, 50)
            .with_peers(vec![peer.clone()]);
        
        manager.add_strategy(Box::new(mock_strategy));
        
        // Discover peers
        let _result = manager.discover_peers(Duration::from_secs(5)).await;
        
        // Check cache
        let cached_peers = manager.get_discovered_peers().await;
        assert_eq!(cached_peers.len(), 1);
        assert_eq!(cached_peers[0].peer_id, "peer-123");
        
        // Check specific peer lookup
        let specific_peer = manager.get_peer("peer-123").await;
        assert!(specific_peer.is_some());
        assert_eq!(specific_peer.unwrap().peer_id, "peer-123");
        
        // Check non-existent peer
        let non_existent = manager.get_peer("non-existent").await;
        assert!(non_existent.is_none());
    }

    #[tokio::test]
    async fn test_strategy_statistics() {
        let mut manager = DiscoveryManager::new();
        
        let peer = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        let mock_strategy = MockDiscovery::new("mock", true, 50)
            .with_peers(vec![peer]);
        
        manager.add_strategy_async(Box::new(mock_strategy)).await;
        
        // Perform discovery
        let _result = manager.discover_peers(Duration::from_secs(5)).await;
        
        // Check statistics
        let stats = manager.get_strategy_stats().await;
        assert!(stats.contains_key("mock"));
        
        let mock_stats = &stats["mock"];
        assert_eq!(mock_stats.success_count, 1);
        assert_eq!(mock_stats.failure_count, 0);
        assert_eq!(mock_stats.total_peers_discovered, 1);
        assert!(mock_stats.success_rate() > 0.0);
    }

    #[tokio::test]
    async fn test_unavailable_strategies_ignored() {
        let mut manager = DiscoveryManager::new();
        
        let unavailable_strategy = MockDiscovery::new("unavailable", false, 80);
        let available_strategy = MockDiscovery::new("available", true, 50);
        
        manager.add_strategy(Box::new(unavailable_strategy));
        manager.add_strategy(Box::new(available_strategy));
        
        let available = manager.get_available_strategies();
        assert_eq!(available.len(), 1);
        assert_eq!(available[0], "available");
    }

    #[tokio::test]
    async fn test_no_available_strategies() {
        let mut manager = DiscoveryManager::new();
        
        let unavailable_strategy = MockDiscovery::new("unavailable", false, 80);
        manager.add_strategy(Box::new(unavailable_strategy));
        
        let result = manager.discover_peers(Duration::from_secs(5)).await;
        assert!(result.is_err());
        
        if let Err(DiscoveryError::StrategyUnavailable { strategy }) = result {
            assert_eq!(strategy, "all");
        } else {
            panic!("Expected StrategyUnavailable error");
        }
    }

    #[tokio::test]
    async fn test_peer_expiration() {
        let mut manager = DiscoveryManager::new();
        manager.set_peer_ttl(Duration::from_millis(100)); // Very short TTL for testing
        
        let peer = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        let mock_strategy = MockDiscovery::new("mock", true, 50)
            .with_peers(vec![peer]);
        
        manager.add_strategy(Box::new(mock_strategy));
        
        // Discover peers
        let _result = manager.discover_peers(Duration::from_secs(5)).await;
        
        // Should have peers initially
        let initial_count = manager.active_peer_count().await;
        assert_eq!(initial_count, 1);
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should have no active peers after expiration
        let expired_count = manager.active_peer_count().await;
        assert_eq!(expired_count, 0);
        
        // But total count should still include expired peers
        let total_count = manager.total_peer_count().await;
        assert_eq!(total_count, 1);
        
        // Cleanup should remove expired peers
        manager.cleanup_expired_peers().await;
        let cleaned_count = manager.total_peer_count().await;
        assert_eq!(cleaned_count, 0);
    }
}