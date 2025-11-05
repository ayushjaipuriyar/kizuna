use crate::discovery::{Discovery, DiscoveryError, ServiceRecord};
use crate::discovery::error::{ErrorContext, ErrorSeverity};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct StrategyConfig {
    pub enable_mdns: bool,
    pub enable_udp: bool,
    pub enable_tcp: bool,
    pub enable_bluetooth: bool,
    pub enable_libp2p: bool,
    pub mdns_config: Option<MdnsConfig>,
    pub udp_config: Option<UdpConfig>,
    pub tcp_config: Option<TcpConfig>,
    pub bluetooth_config: Option<BluetoothConfig>,
    pub libp2p_config: Option<Libp2pConfig>,
}

#[derive(Debug, Clone)]
pub struct MdnsConfig {
    pub peer_id: String,
    pub device_name: String,
    pub port: u16,
}

#[derive(Debug, Clone)]
pub struct UdpConfig {
    pub peer_id: String,
    pub device_name: String,
    pub port: u16,
    pub broadcast_interval: Duration,
}

#[derive(Debug, Clone)]
pub struct TcpConfig {
    pub peer_id: String,
    pub device_name: String,
    pub port_range: std::ops::Range<u16>,
    pub scan_timeout: Duration,
    pub max_concurrent_scans: usize,
}

#[derive(Debug, Clone)]
pub struct BluetoothConfig {
    pub peer_id: String,
    pub device_name: String,
    pub service_uuid: String,
}

#[derive(Debug, Clone)]
pub struct Libp2pConfig {
    pub device_name: String,
    pub bootstrap_nodes: Vec<String>, // Multiaddr strings
}

#[derive(Debug, Clone)]
pub struct StrategyUsage {
    pub name: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub success_rate: f64,
    pub average_response_time: Duration,
    pub total_peers_discovered: u64,
    pub performance_score: f64,
    pub is_recently_failing: bool,
    pub network_latency: Option<Duration>,
    pub availability_score: f64,
}

#[derive(Debug, Clone)]
pub struct StrategyUsageReport {
    pub strategies: Vec<StrategyUsage>,
    pub total_discoveries: u64,
    pub total_peers_discovered: u64,
    pub network_condition: Option<NetworkCondition>,
    pub auto_selection_optimal: bool,
    pub recommended_strategy: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            jitter: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ErrorRecoveryConfig {
    pub enable_graceful_degradation: bool,
    pub circuit_breaker_threshold: u32,
    pub circuit_breaker_timeout: Duration,
    pub health_check_interval: Duration,
    pub error_rate_threshold: f64,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            enable_graceful_degradation: true,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
            health_check_interval: Duration::from_secs(30),
            error_rate_threshold: 0.5, // 50% error rate threshold
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerState {
    pub is_open: bool,
    pub failure_count: u32,
    pub last_failure: Option<SystemTime>,
    pub last_success: Option<SystemTime>,
    pub next_attempt_time: Option<SystemTime>,
}

impl CircuitBreakerState {
    pub fn new() -> Self {
        Self {
            is_open: false,
            failure_count: 0,
            last_failure: None,
            last_success: None,
            next_attempt_time: None,
        }
    }

    pub fn can_attempt(&self) -> bool {
        if !self.is_open {
            return true;
        }

        if let Some(next_attempt) = self.next_attempt_time {
            SystemTime::now() >= next_attempt
        } else {
            true
        }
    }

    pub fn record_success(&mut self) {
        self.is_open = false;
        self.failure_count = 0;
        self.last_success = Some(SystemTime::now());
        self.next_attempt_time = None;
    }

    pub fn record_failure(&mut self, threshold: u32, timeout: Duration) {
        self.failure_count += 1;
        self.last_failure = Some(SystemTime::now());

        if self.failure_count >= threshold {
            self.is_open = true;
            self.next_attempt_time = Some(SystemTime::now() + timeout);
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub total_discoveries: u64,
    pub successful_discoveries: u64,
    pub failed_discoveries: u64,
    pub total_peers_found: u64,
    pub average_discovery_time: Duration,
    pub min_discovery_time: Duration,
    pub max_discovery_time: Duration,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub concurrent_discoveries: u64,
    pub strategy_switches: u64,
    pub last_reset: SystemTime,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            total_discoveries: 0,
            successful_discoveries: 0,
            failed_discoveries: 0,
            total_peers_found: 0,
            average_discovery_time: Duration::ZERO,
            min_discovery_time: Duration::MAX,
            max_discovery_time: Duration::ZERO,
            cache_hits: 0,
            cache_misses: 0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
            concurrent_discoveries: 0,
            strategy_switches: 0,
            last_reset: SystemTime::now(),
        }
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_discoveries == 0 {
            0.0
        } else {
            self.successful_discoveries as f64 / self.total_discoveries as f64
        }
    }

    pub fn cache_hit_rate(&self) -> f64 {
        let total_cache_requests = self.cache_hits + self.cache_misses;
        if total_cache_requests == 0 {
            0.0
        } else {
            self.cache_hits as f64 / total_cache_requests as f64
        }
    }

    pub fn average_peers_per_discovery(&self) -> f64 {
        if self.successful_discoveries == 0 {
            0.0
        } else {
            self.total_peers_found as f64 / self.successful_discoveries as f64
        }
    }
}

#[derive(Debug)]
pub struct PerformanceMonitor {
    pub global_metrics: PerformanceMetrics,
    pub strategy_metrics: HashMap<String, PerformanceMetrics>,
    pub resource_usage: ResourceUsage,
    pub timing_history: Vec<(SystemTime, String, Duration)>, // (timestamp, operation, duration)
    pub peer_cache_stats: CacheStats,
}

#[derive(Debug, Clone)]
pub struct ResourceUsage {
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
    pub network_connections: u32,
    pub open_file_descriptors: u32,
    pub thread_count: u32,
    pub last_updated: SystemTime,
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub active_entries: usize,
    pub expired_entries: usize,
    pub memory_usage_bytes: usize,
    pub average_entry_age: Duration,
    pub cleanup_operations: u64,
    pub last_cleanup: Option<SystemTime>,
}

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            global_metrics: PerformanceMetrics::new(),
            strategy_metrics: HashMap::new(),
            resource_usage: ResourceUsage {
                memory_usage_bytes: 0,
                cpu_usage_percent: 0.0,
                network_connections: 0,
                open_file_descriptors: 0,
                thread_count: 0,
                last_updated: SystemTime::now(),
            },
            timing_history: Vec::new(),
            peer_cache_stats: CacheStats {
                total_entries: 0,
                active_entries: 0,
                expired_entries: 0,
                memory_usage_bytes: 0,
                average_entry_age: Duration::ZERO,
                cleanup_operations: 0,
                last_cleanup: None,
            },
        }
    }

    pub fn record_discovery_attempt(&mut self, strategy: &str) {
        self.global_metrics.total_discoveries += 1;
        
        let strategy_metrics = self.strategy_metrics.entry(strategy.to_string())
            .or_insert_with(PerformanceMetrics::new);
        strategy_metrics.total_discoveries += 1;
    }

    pub fn record_discovery_success(&mut self, strategy: &str, duration: Duration, peer_count: usize) {
        // Update global metrics
        self.global_metrics.successful_discoveries += 1;
        self.global_metrics.total_peers_found += peer_count as u64;
        Self::update_timing_metrics(&mut self.global_metrics, duration);
        
        // Update strategy metrics
        let strategy_metrics = self.strategy_metrics.entry(strategy.to_string())
            .or_insert_with(PerformanceMetrics::new);
        strategy_metrics.successful_discoveries += 1;
        strategy_metrics.total_peers_found += peer_count as u64;
        Self::update_timing_metrics(strategy_metrics, duration);
        
        // Record timing history
        self.timing_history.push((SystemTime::now(), format!("discovery_{}", strategy), duration));
        
        // Keep timing history manageable
        if self.timing_history.len() > 1000 {
            self.timing_history.drain(0..100);
        }
    }

    pub fn record_discovery_failure(&mut self, strategy: &str, duration: Duration) {
        self.global_metrics.failed_discoveries += 1;
        Self::update_timing_metrics(&mut self.global_metrics, duration);
        
        let strategy_metrics = self.strategy_metrics.entry(strategy.to_string())
            .or_insert_with(PerformanceMetrics::new);
        strategy_metrics.failed_discoveries += 1;
        Self::update_timing_metrics(strategy_metrics, duration);
        
        self.timing_history.push((SystemTime::now(), format!("discovery_failed_{}", strategy), duration));
    }

    fn update_timing_metrics(metrics: &mut PerformanceMetrics, duration: Duration) {
        // Update min/max
        if duration < metrics.min_discovery_time {
            metrics.min_discovery_time = duration;
        }
        if duration > metrics.max_discovery_time {
            metrics.max_discovery_time = duration;
        }
        
        // Update average (simple moving average)
        if metrics.total_discoveries == 1 {
            metrics.average_discovery_time = duration;
        } else {
            let total_time = metrics.average_discovery_time * (metrics.total_discoveries - 1) as u32 + duration;
            metrics.average_discovery_time = total_time / metrics.total_discoveries as u32;
        }
    }

    pub fn record_cache_hit(&mut self) {
        self.global_metrics.cache_hits += 1;
    }

    pub fn record_cache_miss(&mut self) {
        self.global_metrics.cache_misses += 1;
    }

    pub fn record_concurrent_discovery(&mut self) {
        self.global_metrics.concurrent_discoveries += 1;
    }

    pub fn record_strategy_switch(&mut self) {
        self.global_metrics.strategy_switches += 1;
    }

    pub fn update_cache_stats(&mut self, stats: CacheStats) {
        self.peer_cache_stats = stats;
    }

    pub fn get_performance_summary(&self) -> PerformanceSummary {
        PerformanceSummary {
            global_metrics: self.global_metrics.clone(),
            top_strategies: self.get_top_performing_strategies(5),
            resource_usage: self.resource_usage.clone(),
            cache_stats: self.peer_cache_stats.clone(),
            recent_timing: self.get_recent_timing_stats(Duration::from_secs(300)), // Last 5 minutes
        }
    }

    fn get_top_performing_strategies(&self, limit: usize) -> Vec<(String, PerformanceMetrics)> {
        let mut strategies: Vec<_> = self.strategy_metrics.iter()
            .map(|(name, metrics)| (name.clone(), metrics.clone()))
            .collect();
        
        strategies.sort_by(|a, b| {
            let score_a = a.1.success_rate() * (1.0 / (a.1.average_discovery_time.as_millis() as f64 + 1.0));
            let score_b = b.1.success_rate() * (1.0 / (b.1.average_discovery_time.as_millis() as f64 + 1.0));
            score_b.partial_cmp(&score_a).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        strategies.into_iter().take(limit).collect()
    }

    fn get_recent_timing_stats(&self, window: Duration) -> Vec<(SystemTime, String, Duration)> {
        let cutoff = SystemTime::now() - window;
        self.timing_history.iter()
            .filter(|(timestamp, _, _)| *timestamp >= cutoff)
            .cloned()
            .collect()
    }

    pub fn reset_metrics(&mut self) {
        self.global_metrics = PerformanceMetrics::new();
        self.strategy_metrics.clear();
        self.timing_history.clear();
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    pub global_metrics: PerformanceMetrics,
    pub top_strategies: Vec<(String, PerformanceMetrics)>,
    pub resource_usage: ResourceUsage,
    pub cache_stats: CacheStats,
    pub recent_timing: Vec<(SystemTime, String, Duration)>,
}

impl Default for StrategyConfig {
    fn default() -> Self {
        Self {
            enable_mdns: true,
            enable_udp: true,
            enable_tcp: true,
            enable_bluetooth: true,
            enable_libp2p: true,
            mdns_config: None,
            udp_config: None,
            tcp_config: None,
            bluetooth_config: None,
            libp2p_config: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StrategyStats {
    pub name: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_success: Option<SystemTime>,
    pub last_failure: Option<SystemTime>,
    pub average_response_time: Duration,
    pub total_peers_discovered: u64,
    pub network_latency: Option<Duration>,
    pub availability_score: f64,
    pub recent_failures: u32,
    pub consecutive_failures: u32,
    pub last_performance_test: Option<SystemTime>,
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
            network_latency: None,
            availability_score: 1.0,
            recent_failures: 0,
            consecutive_failures: 0,
            last_performance_test: None,
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

    pub fn performance_score(&self) -> f64 {
        let success_rate = self.success_rate();
        let latency_score = if let Some(latency) = self.network_latency {
            // Lower latency = higher score (max 1.0 for latency under 100ms)
            (1000.0 / (latency.as_millis() as f64 + 100.0)).min(1.0)
        } else {
            0.5 // Default score if no latency data
        };
        
        let response_time_score = if self.average_response_time.as_millis() > 0 {
            (1000.0 / (self.average_response_time.as_millis() as f64 + 100.0)).min(1.0)
        } else {
            0.5
        };

        let failure_penalty = if self.consecutive_failures > 0 {
            0.9_f64.powi(self.consecutive_failures as i32)
        } else {
            1.0
        };

        // Weighted combination of factors
        (success_rate * 0.4 + latency_score * 0.3 + response_time_score * 0.2 + self.availability_score * 0.1) * failure_penalty
    }

    pub fn is_recently_failing(&self) -> bool {
        self.consecutive_failures >= 3 || self.recent_failures >= 5
    }

    pub fn needs_performance_test(&self) -> bool {
        match self.last_performance_test {
            None => true,
            Some(last_test) => {
                SystemTime::now()
                    .duration_since(last_test)
                    .unwrap_or(Duration::ZERO) > Duration::from_secs(300) // Test every 5 minutes
            }
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
    performance_test_timeout: Duration,
    network_condition_cache: Arc<RwLock<Option<NetworkCondition>>>,
    last_network_check: Arc<RwLock<Option<SystemTime>>>,
    fallback_enabled: bool,
    adaptive_timeout: bool,
    retry_config: RetryConfig,
    error_recovery_config: ErrorRecoveryConfig,
    error_history: Arc<RwLock<Vec<(SystemTime, DiscoveryError, ErrorContext)>>>,
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreakerState>>>,
    performance_monitor: Arc<RwLock<PerformanceMonitor>>,
}

#[derive(Debug, Clone)]
pub struct NetworkCondition {
    pub bandwidth_estimate: Option<f64>, // Mbps
    pub latency_estimate: Option<Duration>,
    pub packet_loss_rate: f64,
    pub connection_stability: f64, // 0.0 to 1.0
    pub last_updated: SystemTime,
}

impl NetworkCondition {
    pub fn new() -> Self {
        Self {
            bandwidth_estimate: None,
            latency_estimate: None,
            packet_loss_rate: 0.0,
            connection_stability: 1.0,
            last_updated: SystemTime::now(),
        }
    }

    pub fn is_poor_network(&self) -> bool {
        self.packet_loss_rate > 0.1 || 
        self.connection_stability < 0.7 ||
        self.latency_estimate.map_or(false, |l| l > Duration::from_millis(500))
    }

    pub fn is_expired(&self, max_age: Duration) -> bool {
        SystemTime::now()
            .duration_since(self.last_updated)
            .unwrap_or(Duration::ZERO) > max_age
    }
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
            performance_test_timeout: Duration::from_secs(10),
            network_condition_cache: Arc::new(RwLock::new(None)),
            last_network_check: Arc::new(RwLock::new(None)),
            fallback_enabled: true,
            adaptive_timeout: true,
            retry_config: RetryConfig::default(),
            error_recovery_config: ErrorRecoveryConfig::default(),
            error_history: Arc::new(RwLock::new(Vec::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            performance_monitor: Arc::new(RwLock::new(PerformanceMonitor::new())),
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

    pub fn set_performance_test_timeout(&mut self, timeout: Duration) {
        self.performance_test_timeout = timeout;
    }

    pub fn set_fallback_enabled(&mut self, enabled: bool) {
        self.fallback_enabled = enabled;
    }

    pub fn set_adaptive_timeout(&mut self, enabled: bool) {
        self.adaptive_timeout = enabled;
    }

    pub fn set_retry_config(&mut self, config: RetryConfig) {
        self.retry_config = config;
    }

    pub fn set_error_recovery_config(&mut self, config: ErrorRecoveryConfig) {
        self.error_recovery_config = config;
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
        let cleanup_start = Instant::now();
        let initial_count = {
            let peers = self.discovered_peers.read().await;
            peers.len()
        };
        
        {
            let mut peers = self.discovered_peers.write().await;
            peers.retain(|_, peer| !peer.is_expired(self.peer_ttl));
        }
        
        let final_count = {
            let peers = self.discovered_peers.read().await;
            peers.len()
        };
        
        let cleanup_duration = cleanup_start.elapsed();
        let removed_count = initial_count - final_count;
        
        // Update performance monitoring
        {
            let mut monitor = self.performance_monitor.write().await;
            monitor.peer_cache_stats.cleanup_operations += 1;
            monitor.peer_cache_stats.last_cleanup = Some(SystemTime::now());
            monitor.timing_history.push((
                SystemTime::now(),
                "cache_cleanup".to_string(),
                cleanup_duration,
            ));
        }
        
        if removed_count > 0 {
            println!("[INFO] Cleaned up {} expired peers in {:?}", removed_count, cleanup_duration);
        }
    }

    async fn discover_with_auto_select(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        // Update network conditions
        self.update_network_conditions().await;
        
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

        // Perform performance tests for strategies that need it
        self.perform_performance_tests(&available_strategies).await;

        // Get the best strategy based on current conditions
        let best_strategy_name = self.get_best_strategy_for_conditions().await;
        
        // Calculate adaptive timeout based on network conditions
        let adaptive_timeout = if self.adaptive_timeout {
            self.calculate_adaptive_timeout(timeout).await
        } else {
            timeout
        };

        // Try the best strategy first
        if let Some(best_name) = best_strategy_name {
            if let Some(strategy) = available_strategies.iter().find(|s| s.strategy_name() == best_name) {
                match self.discover_with_single_strategy(strategy.as_ref(), adaptive_timeout).await {
                    Ok(peers) => {
                        self.update_peer_cache(&peers).await;
                        return Ok(peers);
                    }
                    Err(_) => {
                        // Mark strategy as failing and continue
                        self.mark_strategy_failure(&best_name).await;
                    }
                }
            }
        }

        // Fallback logic if enabled
        if self.fallback_enabled {
            return self.discover_with_fallback_strategies(&available_strategies, adaptive_timeout).await;
        }

        // Fall back to priority-based selection
        let mut sorted_strategies = available_strategies;
        sorted_strategies.sort_by_key(|s| std::cmp::Reverse(s.priority()));

        let mut last_error = None;
        
        for strategy in sorted_strategies {
            // Skip recently failing strategies
            if self.is_strategy_recently_failing(strategy.strategy_name()).await {
                continue;
            }

            match self.discover_with_single_strategy(strategy.as_ref(), adaptive_timeout).await {
                Ok(peers) => {
                    self.update_peer_cache(&peers).await;
                    return Ok(peers);
                }
                Err(e) => {
                    last_error = Some(e);
                    self.mark_strategy_failure(strategy.strategy_name()).await;
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
        let strategy_name = strategy.strategy_name().to_string();
        
        // Check circuit breaker
        if !self.can_attempt_strategy(&strategy_name).await {
            return Err(DiscoveryError::ServiceUnavailable {
                service: strategy_name,
                reason: "Circuit breaker is open".to_string(),
            });
        }

        let result = self.discover_with_retry(strategy, timeout).await;
        
        // Update circuit breaker state
        match &result {
            Ok(_) => self.record_strategy_success(&strategy_name).await,
            Err(_) => self.record_strategy_failure(&strategy_name).await,
        }
        
        result
    }

    async fn discover_with_retry(&self, strategy: &dyn Discovery, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        let strategy_name = strategy.strategy_name().to_string();
        let mut last_error = None;
        
        // Record discovery attempt in performance monitor
        {
            let mut monitor = self.performance_monitor.write().await;
            monitor.record_discovery_attempt(&strategy_name);
        }
        
        for attempt in 0..self.retry_config.max_attempts {
            let start_time = Instant::now();
            
            let result = strategy.discover(timeout).await;
            let elapsed = start_time.elapsed();
            
            // Update strategy statistics
            self.update_strategy_stats(&strategy_name, &result, elapsed, result.as_ref().map(|p| p.len()).unwrap_or(0)).await;
            
            match result {
                Ok(peers) => {
                    // Record success in performance monitor
                    {
                        let mut monitor = self.performance_monitor.write().await;
                        monitor.record_discovery_success(&strategy_name, elapsed, peers.len());
                    }
                    
                    // Success - log and return
                    if attempt > 0 {
                        self.log_retry_success(&strategy_name, attempt).await;
                    }
                    return Ok(peers);
                }
                Err(error) => {
                    // Record failure in performance monitor
                    {
                        let mut monitor = self.performance_monitor.write().await;
                        monitor.record_discovery_failure(&strategy_name, elapsed);
                    }
                    
                    let error_context = ErrorContext::new(format!("discover_{}", strategy_name))
                        .with_strategy(strategy_name.clone())
                        .with_retry_info(attempt, self.retry_config.max_attempts)
                        .with_severity(self.classify_error_severity(&error));
                    
                    // Log the error
                    self.log_error(&error, &error_context).await;
                    
                    // Check if we should retry
                    if attempt < self.retry_config.max_attempts - 1 && self.is_retryable_error(&error) {
                        let delay = self.calculate_retry_delay(attempt);
                        self.log_retry_attempt(&strategy_name, attempt + 1, delay).await;
                        tokio::time::sleep(delay).await;
                        last_error = Some(error);
                        continue;
                    } else {
                        // Final attempt failed or non-retryable error
                        let final_error = if attempt == self.retry_config.max_attempts - 1 {
                            DiscoveryError::TransientError {
                                strategy: strategy_name.clone(),
                                message: error.to_string(),
                                attempt: attempt + 1,
                                max_attempts: self.retry_config.max_attempts,
                            }
                        } else {
                            error
                        };
                        
                        return Err(final_error);
                    }
                }
            }
        }
        
        // This should never be reached, but just in case
        Err(last_error.unwrap_or_else(|| DiscoveryError::FatalError {
            strategy: strategy_name,
            message: "Unexpected retry loop exit".to_string(),
        }))
    }

    fn calculate_retry_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.retry_config.base_delay.as_millis() as f64;
        let multiplier = self.retry_config.backoff_multiplier;
        let delay_ms = base_delay * multiplier.powi(attempt as i32);
        
        let mut delay = Duration::from_millis(delay_ms as u64);
        
        // Cap at max delay
        if delay > self.retry_config.max_delay {
            delay = self.retry_config.max_delay;
        }
        
        // Add jitter if enabled
        if self.retry_config.jitter {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            
            let mut hasher = DefaultHasher::new();
            SystemTime::now().hash(&mut hasher);
            let jitter_factor = (hasher.finish() % 100) as f64 / 100.0; // 0.0 to 0.99
            
            let jitter_amount = delay.as_millis() as f64 * 0.1 * jitter_factor; // Up to 10% jitter
            delay += Duration::from_millis(jitter_amount as u64);
        }
        
        delay
    }

    fn is_retryable_error(&self, error: &DiscoveryError) -> bool {
        match error {
            DiscoveryError::Network(_) => true,
            DiscoveryError::Timeout { .. } => true,
            DiscoveryError::ServiceUnavailable { .. } => true,
            DiscoveryError::TransientError { .. } => true,
            DiscoveryError::RateLimitExceeded { .. } => false, // Don't retry rate limits immediately
            DiscoveryError::AuthenticationFailed { .. } => false,
            DiscoveryError::FatalError { .. } => false,
            DiscoveryError::PermissionDenied { .. } => false,
            _ => true, // Default to retryable for unknown errors
        }
    }

    fn classify_error_severity(&self, error: &DiscoveryError) -> ErrorSeverity {
        match error {
            DiscoveryError::Network(_) => ErrorSeverity::Medium,
            DiscoveryError::Timeout { .. } => ErrorSeverity::Medium,
            DiscoveryError::StrategyUnavailable { .. } => ErrorSeverity::Low,
            DiscoveryError::ServiceUnavailable { .. } => ErrorSeverity::Medium,
            DiscoveryError::TransientError { .. } => ErrorSeverity::Medium,
            DiscoveryError::RateLimitExceeded { .. } => ErrorSeverity::High,
            DiscoveryError::AuthenticationFailed { .. } => ErrorSeverity::High,
            DiscoveryError::FatalError { .. } => ErrorSeverity::Critical,
            DiscoveryError::PermissionDenied { .. } => ErrorSeverity::High,
            _ => ErrorSeverity::Medium,
        }
    }

    async fn update_strategy_stats(&self, strategy_name: &str, result: &Result<Vec<ServiceRecord>, DiscoveryError>, elapsed: Duration, peer_count: usize) {
        let mut stats = self.strategy_stats.write().await;
        if let Some(stat) = stats.get_mut(strategy_name) {
            match result {
                Ok(_) => {
                    stat.success_count += 1;
                    stat.last_success = Some(SystemTime::now());
                    stat.total_peers_discovered += peer_count as u64;
                    stat.consecutive_failures = 0; // Reset consecutive failures on success
                    
                    // Decay recent failures over time
                    if stat.recent_failures > 0 {
                        stat.recent_failures = stat.recent_failures.saturating_sub(1);
                    }
                    
                    // Update average response time (simple moving average)
                    if stat.success_count == 1 {
                        stat.average_response_time = elapsed;
                    } else {
                        let total_time = stat.average_response_time * (stat.success_count - 1) as u32 + elapsed;
                        stat.average_response_time = total_time / stat.success_count as u32;
                    }

                    // Update availability score (exponential moving average)
                    stat.availability_score = stat.availability_score * 0.9 + 0.1;
                }
                Err(_) => {
                    stat.failure_count += 1;
                    stat.last_failure = Some(SystemTime::now());
                    stat.consecutive_failures += 1;
                    stat.recent_failures = (stat.recent_failures + 1).min(10); // Cap at 10
                    
                    // Decrease availability score
                    stat.availability_score = (stat.availability_score * 0.9).max(0.1);
                }
            }
        }
    }

    async fn update_network_conditions(&self) {
        let should_update = {
            let last_check = self.last_network_check.read().await;
            match *last_check {
                None => true,
                Some(last_time) => {
                    SystemTime::now()
                        .duration_since(last_time)
                        .unwrap_or(Duration::ZERO) > Duration::from_secs(60) // Update every minute
                }
            }
        };

        if should_update {
            let network_condition = self.evaluate_network_conditions().await;
            
            {
                let mut cache = self.network_condition_cache.write().await;
                *cache = Some(network_condition);
            }
            
            {
                let mut last_check = self.last_network_check.write().await;
                *last_check = Some(SystemTime::now());
            }
        }
    }

    async fn evaluate_network_conditions(&self) -> NetworkCondition {
        let mut condition = NetworkCondition::new();
        
        // Simple network condition evaluation based on strategy performance
        let stats = self.strategy_stats.read().await;
        
        let mut total_latency = Duration::ZERO;
        let mut latency_count = 0;
        let mut total_success_rate = 0.0;
        let mut strategy_count = 0;

        for stat in stats.values() {
            if stat.success_count > 0 {
                total_latency += stat.average_response_time;
                latency_count += 1;
                total_success_rate += stat.success_rate();
                strategy_count += 1;
            }
        }

        if latency_count > 0 {
            condition.latency_estimate = Some(total_latency / latency_count as u32);
        }

        if strategy_count > 0 {
            let avg_success_rate = total_success_rate / strategy_count as f64;
            condition.packet_loss_rate = 1.0 - avg_success_rate;
            condition.connection_stability = avg_success_rate;
        }

        condition
    }

    async fn perform_performance_tests(&self, strategies: &[&Box<dyn Discovery>]) {
        for strategy in strategies {
            let strategy_name = strategy.strategy_name();
            
            let needs_test = {
                let stats = self.strategy_stats.read().await;
                stats.get(strategy_name)
                    .map(|s| s.needs_performance_test())
                    .unwrap_or(true)
            };

            if needs_test {
                self.perform_single_performance_test(strategy.as_ref()).await;
            }
        }
    }

    async fn perform_single_performance_test(&self, strategy: &dyn Discovery) {
        let strategy_name = strategy.strategy_name();
        let start_time = Instant::now();
        
        // Perform a quick discovery test with short timeout
        let test_result = strategy.discover(self.performance_test_timeout).await;
        let test_latency = start_time.elapsed();
        
        // Update strategy stats with performance test results
        let mut stats = self.strategy_stats.write().await;
        if let Some(stat) = stats.get_mut(strategy_name) {
            stat.last_performance_test = Some(SystemTime::now());
            
            match test_result {
                Ok(_) => {
                    stat.network_latency = Some(test_latency);
                }
                Err(_) => {
                    // Performance test failed, increase latency estimate
                    stat.network_latency = Some(test_latency.max(Duration::from_millis(1000)));
                }
            }
        }
    }

    async fn get_best_strategy_for_conditions(&self) -> Option<String> {
        let network_condition = self.network_condition_cache.read().await;
        let stats = self.strategy_stats.read().await;
        
        let mut best_strategy = None;
        let mut best_score = 0.0;
        
        for (name, stat) in stats.iter() {
            if stat.is_recently_failing() {
                continue; // Skip recently failing strategies
            }
            
            let mut score = stat.performance_score();
            
            // Adjust score based on network conditions
            if let Some(ref condition) = *network_condition {
                if condition.is_poor_network() {
                    // In poor network conditions, prefer strategies with better reliability
                    score = score * 0.7 + stat.success_rate() * 0.3;
                } else {
                    // In good network conditions, prefer faster strategies
                    if let Some(latency) = stat.network_latency {
                        let latency_bonus = (1000.0 / (latency.as_millis() as f64 + 100.0)).min(1.0);
                        score = score * 0.8 + latency_bonus * 0.2;
                    }
                }
            }
            
            if score > best_score {
                best_score = score;
                best_strategy = Some(name.clone());
            }
        }
        
        best_strategy
    }

    async fn calculate_adaptive_timeout(&self, base_timeout: Duration) -> Duration {
        let network_condition = self.network_condition_cache.read().await;
        
        if let Some(ref condition) = *network_condition {
            if condition.is_poor_network() {
                // Increase timeout for poor network conditions
                base_timeout.mul_f64(1.5)
            } else if let Some(latency) = condition.latency_estimate {
                // Adjust timeout based on network latency
                let latency_factor = (latency.as_millis() as f64 / 100.0).max(0.5).min(2.0);
                base_timeout.mul_f64(latency_factor)
            } else {
                base_timeout
            }
        } else {
            base_timeout
        }
    }

    async fn discover_with_fallback_strategies(&self, strategies: &[&Box<dyn Discovery>], timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        // Sort strategies by performance score
        let mut strategy_scores: Vec<_> = Vec::new();
        
        {
            let stats = self.strategy_stats.read().await;
            for strategy in strategies {
                let strategy_name = strategy.strategy_name();
                let score = stats.get(strategy_name)
                    .map(|s| s.performance_score())
                    .unwrap_or(0.0);
                
                if !stats.get(strategy_name).map(|s| s.is_recently_failing()).unwrap_or(false) {
                    strategy_scores.push((strategy, score));
                }
            }
        }
        
        // Sort by score (highest first)
        strategy_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        let mut last_error = None;
        
        for (strategy, _score) in strategy_scores {
            match self.discover_with_single_strategy(strategy.as_ref(), timeout).await {
                Ok(peers) => {
                    self.update_peer_cache(&peers).await;
                    return Ok(peers);
                }
                Err(e) => {
                    last_error = Some(e);
                    self.mark_strategy_failure(strategy.strategy_name()).await;
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| DiscoveryError::StrategyUnavailable {
            strategy: "fallback".to_string(),
        }))
    }

    async fn mark_strategy_failure(&self, strategy_name: &str) {
        let mut stats = self.strategy_stats.write().await;
        if let Some(stat) = stats.get_mut(strategy_name) {
            stat.consecutive_failures += 1;
            stat.recent_failures = (stat.recent_failures + 1).min(10);
            stat.availability_score = (stat.availability_score * 0.8).max(0.1);
        }
    }

    async fn is_strategy_recently_failing(&self, strategy_name: &str) -> bool {
        let stats = self.strategy_stats.read().await;
        stats.get(strategy_name)
            .map(|s| s.is_recently_failing())
            .unwrap_or(false)
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

    pub fn set_active_strategy(&mut self, strategy_name: Option<String>) -> Result<(), DiscoveryError> {
        // Validate that the strategy exists if specified
        if let Some(ref name) = strategy_name {
            let strategy_exists = self.strategies
                .iter()
                .any(|s| s.strategy_name() == name && s.is_available());
            
            if !strategy_exists {
                return Err(DiscoveryError::StrategyUnavailable {
                    strategy: name.clone(),
                });
            }
        }
        
        self.active_strategy = strategy_name;
        Ok(())
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
            if stat.success_count == 0 || stat.is_recently_failing() {
                continue; // Skip strategies that haven't been used or are failing
            }
            
            let score = stat.performance_score();
            
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

    /// Get current network conditions
    pub async fn get_network_conditions(&self) -> Option<NetworkCondition> {
        let condition = self.network_condition_cache.read().await;
        condition.clone()
    }

    /// Force a network condition update
    pub async fn refresh_network_conditions(&self) {
        {
            let mut last_check = self.last_network_check.write().await;
            *last_check = None; // Force update
        }
        self.update_network_conditions().await;
    }

    /// Get strategies sorted by performance score
    pub async fn get_strategies_by_performance(&self) -> Vec<(String, f64)> {
        let stats = self.strategy_stats.read().await;
        let mut strategy_scores: Vec<_> = stats.iter()
            .map(|(name, stat)| (name.clone(), stat.performance_score()))
            .collect();
        
        strategy_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        strategy_scores
    }

    /// Check if auto-selection is working optimally
    pub async fn is_auto_selection_optimal(&self) -> bool {
        let stats = self.strategy_stats.read().await;
        
        // Check if we have at least one well-performing strategy
        let has_good_strategy = stats.values().any(|s| {
            s.success_count > 0 && 
            s.success_rate() > 0.8 && 
            !s.is_recently_failing()
        });

        // Check if network conditions are reasonable
        let network_ok = if let Some(condition) = self.network_condition_cache.read().await.as_ref() {
            !condition.is_poor_network()
        } else {
            true // Assume OK if no data
        };

        has_good_strategy && network_ok
    }

    /// Get recommended strategy for current conditions
    pub async fn get_recommended_strategy(&self) -> Option<String> {
        self.get_best_strategy_for_conditions().await
    }

    async fn can_attempt_strategy(&self, strategy_name: &str) -> bool {
        let circuit_breakers = self.circuit_breakers.read().await;
        circuit_breakers.get(strategy_name)
            .map(|cb| cb.can_attempt())
            .unwrap_or(true)
    }

    async fn record_strategy_success(&self, strategy_name: &str) {
        let mut circuit_breakers = self.circuit_breakers.write().await;
        if let Some(cb) = circuit_breakers.get_mut(strategy_name) {
            cb.record_success();
        }
    }

    async fn record_strategy_failure(&self, strategy_name: &str) {
        let mut circuit_breakers = self.circuit_breakers.write().await;
        let cb = circuit_breakers.entry(strategy_name.to_string())
            .or_insert_with(CircuitBreakerState::new);
        
        cb.record_failure(
            self.error_recovery_config.circuit_breaker_threshold,
            self.error_recovery_config.circuit_breaker_timeout,
        );
    }

    async fn log_error(&self, error: &DiscoveryError, context: &ErrorContext) {
        let timestamp = SystemTime::now();
        
        // Add to error history
        {
            let mut history = self.error_history.write().await;
            history.push((timestamp, error.clone(), context.clone()));
            
            // Keep only recent errors (last 1000)
            if history.len() > 1000 {
                history.drain(0..100); // Remove oldest 100
            }
        }

        // Log based on severity
        match context.severity {
            ErrorSeverity::Low => {
                println!("[INFO] Discovery: {} - {}", context.operation, error);
            }
            ErrorSeverity::Medium => {
                println!("[WARN] Discovery: {} - {} (attempt {}/{})", 
                    context.operation, error, context.retry_count + 1, context.max_retries);
            }
            ErrorSeverity::High => {
                eprintln!("[ERROR] Discovery: {} - {} (attempt {}/{})", 
                    context.operation, error, context.retry_count + 1, context.max_retries);
            }
            ErrorSeverity::Critical => {
                eprintln!("[CRITICAL] Discovery: {} - {} (attempt {}/{})", 
                    context.operation, error, context.retry_count + 1, context.max_retries);
            }
        }
    }

    async fn log_retry_attempt(&self, strategy_name: &str, attempt: u32, delay: Duration) {
        println!("[RETRY] Discovery: Retrying {} (attempt {}) after {:?}", 
            strategy_name, attempt, delay);
    }

    async fn log_retry_success(&self, strategy_name: &str, final_attempt: u32) {
        println!("[SUCCESS] Discovery: {} succeeded after {} retries", 
            strategy_name, final_attempt);
    }

    /// Get error history for analysis
    pub async fn get_error_history(&self) -> Vec<(SystemTime, DiscoveryError, ErrorContext)> {
        let history = self.error_history.read().await;
        history.clone()
    }

    /// Get circuit breaker states
    pub async fn get_circuit_breaker_states(&self) -> HashMap<String, CircuitBreakerState> {
        let circuit_breakers = self.circuit_breakers.read().await;
        circuit_breakers.clone()
    }

    /// Reset circuit breakers for all strategies
    pub async fn reset_circuit_breakers(&self) {
        let mut circuit_breakers = self.circuit_breakers.write().await;
        for cb in circuit_breakers.values_mut() {
            *cb = CircuitBreakerState::new();
        }
    }

    /// Get error rate for a specific strategy
    pub async fn get_strategy_error_rate(&self, strategy_name: &str, window: Duration) -> f64 {
        let history = self.error_history.read().await;
        let cutoff_time = SystemTime::now() - window;
        
        let recent_errors: Vec<_> = history.iter()
            .filter(|(timestamp, _, context)| {
                *timestamp >= cutoff_time && 
                context.strategy.as_ref().map(|s| s == strategy_name).unwrap_or(false)
            })
            .collect();
        
        if recent_errors.is_empty() {
            return 0.0;
        }

        // Calculate error rate based on strategy statistics
        let stats = self.strategy_stats.read().await;
        if let Some(stat) = stats.get(strategy_name) {
            let total_attempts = stat.success_count + stat.failure_count;
            if total_attempts > 0 {
                return stat.failure_count as f64 / total_attempts as f64;
            }
        }

        0.0
    }

    /// Check if graceful degradation should be enabled
    pub async fn should_enable_graceful_degradation(&self) -> bool {
        if !self.error_recovery_config.enable_graceful_degradation {
            return false;
        }

        let stats = self.strategy_stats.read().await;
        let total_strategies = stats.len();
        
        if total_strategies == 0 {
            return false;
        }

        // Count how many strategies are failing
        let failing_strategies = stats.values()
            .filter(|s| s.is_recently_failing())
            .count();

        // Enable graceful degradation if more than half the strategies are failing
        failing_strategies as f64 / total_strategies as f64 > 0.5
    }

    /// Perform health check on all strategies
    pub async fn perform_health_check(&self) -> HashMap<String, bool> {
        let mut health_status = HashMap::new();
        
        for strategy in &self.strategies {
            let strategy_name = strategy.strategy_name().to_string();
            let is_healthy = strategy.is_available() && 
                            self.can_attempt_strategy(&strategy_name).await;
            
            health_status.insert(strategy_name, is_healthy);
        }
        
        health_status
    }

    /// Update peer cache statistics for monitoring
    async fn update_cache_monitoring_stats(&self) {
        let peers = self.discovered_peers.read().await;
        let now = SystemTime::now();
        
        let total_entries = peers.len();
        let active_entries = peers.values()
            .filter(|p| !p.is_expired(self.peer_ttl))
            .count();
        let expired_entries = total_entries - active_entries;
        
        // Calculate average entry age
        let total_age: Duration = peers.values()
            .filter_map(|p| now.duration_since(p.last_seen).ok())
            .sum();
        let average_entry_age = if total_entries > 0 {
            total_age / total_entries as u32
        } else {
            Duration::ZERO
        };
        
        // Estimate memory usage (rough calculation)
        let estimated_memory = total_entries * std::mem::size_of::<ServiceRecord>();
        
        let cache_stats = CacheStats {
            total_entries,
            active_entries,
            expired_entries,
            memory_usage_bytes: estimated_memory,
            average_entry_age,
            cleanup_operations: 0, // This would be tracked separately
            last_cleanup: None,
        };
        
        let mut monitor = self.performance_monitor.write().await;
        monitor.update_cache_stats(cache_stats);
    }

    /// Get comprehensive performance report
    pub async fn get_performance_report(&self) -> PerformanceSummary {
        // Update cache stats before generating report
        self.update_cache_monitoring_stats().await;
        
        let monitor = self.performance_monitor.read().await;
        monitor.get_performance_summary()
    }

    /// Get performance metrics for a specific strategy
    pub async fn get_strategy_performance(&self, strategy_name: &str) -> Option<PerformanceMetrics> {
        let monitor = self.performance_monitor.read().await;
        monitor.strategy_metrics.get(strategy_name).cloned()
    }

    /// Reset all performance metrics
    pub async fn reset_performance_metrics(&self) {
        let mut monitor = self.performance_monitor.write().await;
        monitor.reset_metrics();
    }

    /// Enable/disable performance monitoring
    pub async fn set_performance_monitoring(&self, enabled: bool) {
        // This could be used to enable/disable detailed monitoring
        // For now, monitoring is always enabled
        if enabled {
            println!("[INFO] Performance monitoring enabled");
        } else {
            println!("[INFO] Performance monitoring disabled");
        }
    }

    /// Get resource usage optimization recommendations
    pub async fn get_optimization_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        let monitor = self.performance_monitor.read().await;
        
        // Check cache hit rate
        if monitor.global_metrics.cache_hit_rate() < 0.5 {
            recommendations.push("Consider increasing peer cache TTL to improve cache hit rate".to_string());
        }
        
        // Check discovery success rate
        if monitor.global_metrics.success_rate() < 0.8 {
            recommendations.push("Discovery success rate is low - consider reviewing network configuration".to_string());
        }
        
        // Check average discovery time
        if monitor.global_metrics.average_discovery_time > Duration::from_secs(10) {
            recommendations.push("Discovery times are high - consider reducing timeout values or enabling concurrent discovery".to_string());
        }
        
        // Check strategy performance
        let failing_strategies: Vec<_> = monitor.strategy_metrics.iter()
            .filter(|(_, metrics)| metrics.success_rate() < 0.5)
            .map(|(name, _)| name.clone())
            .collect();
        
        if !failing_strategies.is_empty() {
            recommendations.push(format!(
                "Consider disabling poorly performing strategies: {}",
                failing_strategies.join(", ")
            ));
        }
        
        // Check concurrent discovery usage
        if monitor.global_metrics.concurrent_discoveries == 0 && self.strategies.len() > 1 {
            recommendations.push("Consider enabling concurrent discovery for better performance".to_string());
        }
        
        drop(monitor);
        recommendations
    }

    /// Configure automatic cleanup based on resource usage
    pub async fn configure_automatic_cleanup(&self) -> Result<(), DiscoveryError> {
        let (cache_hit_rate, memory_usage) = {
            let monitor = self.performance_monitor.read().await;
            (monitor.global_metrics.cache_hit_rate(), monitor.peer_cache_stats.memory_usage_bytes)
        };
        
        // Adjust peer TTL based on cache performance
        if cache_hit_rate > 0.8 {
            // High cache hit rate - can afford longer TTL
            // Note: This would require making peer_ttl mutable
            println!("[INFO] High cache hit rate - consider increasing peer TTL");
        } else if cache_hit_rate < 0.3 {
            // Low cache hit rate - shorter TTL might help
            println!("[INFO] Low cache hit rate - consider decreasing peer TTL");
        }
        
        // Adjust cleanup frequency based on memory usage
        if memory_usage > 10 * 1024 * 1024 { // 10MB
            println!("[INFO] High memory usage - performing aggressive cleanup");
            self.cleanup_expired_peers().await;
        }
        
        Ok(())
    }

    /// Register all available discovery strategies
    pub async fn register_all_strategies(&mut self) -> Result<(), DiscoveryError> {
        use crate::discovery::strategies::{
            mdns::MdnsDiscovery,
            udp::UdpDiscovery,
            tcp::TcpDiscovery,
            bluetooth::BluetoothDiscovery,
            libp2p::Libp2pDiscovery,
        };

        let mut registered_count = 0;

        // Register mDNS strategy
        let mdns_strategy = MdnsDiscovery::new();
        if mdns_strategy.is_available() {
            self.add_strategy_async(Box::new(mdns_strategy)).await;
            registered_count += 1;
        }

        // Register UDP broadcast strategy
        let udp_strategy = UdpDiscovery::new();
        if udp_strategy.is_available() {
            self.add_strategy_async(Box::new(udp_strategy)).await;
            registered_count += 1;
        }

        // Register TCP handshake strategy
        let tcp_strategy = TcpDiscovery::new();
        if tcp_strategy.is_available() {
            self.add_strategy_async(Box::new(tcp_strategy)).await;
            registered_count += 1;
        }

        // Register Bluetooth LE strategy
        let bluetooth_strategy = BluetoothDiscovery::new();
        if bluetooth_strategy.is_available() {
            self.add_strategy_async(Box::new(bluetooth_strategy)).await;
            registered_count += 1;
        }

        // Register libp2p strategy (if available)
        // TODO: Fix libp2p strategy compilation issues
        // match Libp2pDiscovery::new() {
        //     Ok(libp2p_strategy) => {
        //         if libp2p_strategy.is_available() {
        //             self.add_strategy_async(Box::new(libp2p_strategy)).await;
        //             registered_count += 1;
        //         }
        //     }
        //     Err(_) => {
        //         // libp2p not available, skip silently
        //     }
        // }

        if registered_count == 0 {
            return Err(DiscoveryError::Configuration(
                "No discovery strategies are available on this platform".to_string()
            ));
        }

        Ok(())
    }

    /// Register strategies with custom configuration
    pub async fn register_strategies_with_config(&mut self, config: StrategyConfig) -> Result<(), DiscoveryError> {
        use crate::discovery::strategies::{
            mdns::MdnsDiscovery,
            udp::UdpDiscovery,
            tcp::TcpDiscovery,
            bluetooth::BluetoothDiscovery,
            libp2p::Libp2pDiscovery,
        };

        let mut registered_count = 0;

        // Register mDNS strategy with config
        if config.enable_mdns {
            let mdns_strategy = if let Some(ref mdns_config) = config.mdns_config {
                MdnsDiscovery::with_config(
                    mdns_config.peer_id.clone(),
                    mdns_config.device_name.clone(),
                    mdns_config.port,
                )
            } else {
                MdnsDiscovery::new()
            };

            if mdns_strategy.is_available() {
                self.add_strategy_async(Box::new(mdns_strategy)).await;
                registered_count += 1;
            }
        }

        // Register UDP strategy with config
        if config.enable_udp {
            let udp_strategy = if let Some(ref udp_config) = config.udp_config {
                UdpDiscovery::with_config(
                    udp_config.port,
                    udp_config.peer_id.clone(),
                    udp_config.device_name.clone(),
                )
            } else {
                UdpDiscovery::new()
            };

            if udp_strategy.is_available() {
                self.add_strategy_async(Box::new(udp_strategy)).await;
                registered_count += 1;
            }
        }

        // Register TCP strategy with config
        if config.enable_tcp {
            let tcp_strategy = if let Some(ref tcp_config) = config.tcp_config {
                TcpDiscovery::with_config(
                    tcp_config.peer_id.clone(),
                    tcp_config.device_name.clone(),
                    tcp_config.port_range.start,
                    tcp_config.port_range.clone().collect(),
                )
            } else {
                TcpDiscovery::new()
            };

            if tcp_strategy.is_available() {
                self.add_strategy_async(Box::new(tcp_strategy)).await;
                registered_count += 1;
            }
        }

        // Register Bluetooth strategy with config
        if config.enable_bluetooth {
            let bluetooth_strategy = if let Some(ref bt_config) = config.bluetooth_config {
                BluetoothDiscovery::with_config(
                    bt_config.peer_id.clone(),
                    bt_config.device_name.clone(),
                    8080, // Default port since Bluetooth doesn't use the UUID parameter
                )
            } else {
                BluetoothDiscovery::new()
            };

            if bluetooth_strategy.is_available() {
                self.add_strategy_async(Box::new(bluetooth_strategy)).await;
                registered_count += 1;
            }
        }

        // Register libp2p strategy with config
        // TODO: Fix libp2p strategy compilation issues
        // if config.enable_libp2p {
        //     let libp2p_result = if let Some(ref libp2p_config) = config.libp2p_config {
        //         // Parse multiaddr strings
        //         let bootstrap_addrs: Vec<_> = libp2p_config.bootstrap_nodes.iter()
        //             .filter_map(|addr_str| addr_str.parse().ok())
        //             .collect();
        //         
        //         Libp2pDiscovery::with_config(
        //             libp2p_config.device_name.clone(),
        //             bootstrap_addrs,
        //         )
        //     } else {
        //         Libp2p Discovery::new()
        //     };
        //
        //     match libp2p_result {
        //         Ok(libp2p_strategy) => {
        //             if libp2p_strategy.is_available() {
        //                 self.add_strategy_async(Box::new(libp2p_strategy)).await;
        //                 registered_count += 1;
        //             }
        //         }
        //         Err(_) => {
        //             // libp2p configuration failed, skip
        //         }
        //     }
        // }

        if registered_count == 0 {
            return Err(DiscoveryError::Configuration(
                "No discovery strategies could be registered with the provided configuration".to_string()
            ));
        }

        Ok(())
    }

    /// Create a DiscoveryManager with all available strategies pre-registered
    pub async fn with_all_strategies() -> Result<Self, DiscoveryError> {
        let mut manager = Self::new();
        manager.register_all_strategies().await?;
        Ok(manager)
    }

    /// Create a DiscoveryManager with custom strategy configuration
    pub async fn with_strategy_config(config: StrategyConfig) -> Result<Self, DiscoveryError> {
        let mut manager = Self::new();
        manager.register_strategies_with_config(config).await?;
        Ok(manager)
    }

    /// Enable concurrent discovery across multiple strategies with deduplication
    pub async fn discover_peers_concurrent_with_dedup(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        // Clean up expired peers before discovery
        self.cleanup_expired_peers().await;

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

        // Update network conditions for better strategy selection
        self.update_network_conditions().await;

        let mut tasks = Vec::new();
        
        for strategy in available_strategies {
            let strategy_ref = strategy.as_ref();
            let task = self.discover_with_single_strategy(strategy_ref, timeout);
            tasks.push(task);
        }

        // Wait for all strategies to complete
        let results = futures::future::join_all(tasks).await;
        
        let mut all_peers = Vec::new();
        let mut successful_strategies = 0;

        for result in results {
            match result {
                Ok(peers) => {
                    all_peers.extend(peers);
                    successful_strategies += 1;
                }
                Err(_) => {
                    // Continue with other results
                }
            }
        }

        if successful_strategies > 0 {
            // Advanced deduplication and merging
            let final_peers = self.deduplicate_and_merge_peers(all_peers).await;
            self.update_peer_cache(&final_peers).await;
            Ok(final_peers)
        } else {
            Err(DiscoveryError::StrategyUnavailable {
                strategy: "concurrent".to_string(),
            })
        }
    }

    /// Advanced peer deduplication and merging logic
    async fn deduplicate_and_merge_peers(&self, peers: Vec<ServiceRecord>) -> Vec<ServiceRecord> {
        let mut unique_peers: HashMap<String, ServiceRecord> = HashMap::new();
        let mut peer_sources: HashMap<String, Vec<String>> = HashMap::new();

        for peer in peers {
            let peer_id = peer.peer_id.clone();
            
            // Track which strategies discovered this peer
            peer_sources.entry(peer_id.clone())
                .or_insert_with(Vec::new)
                .push(peer.discovery_method.clone());

            if let Some(existing) = unique_peers.get_mut(&peer_id) {
                // Merge peer information
                existing.merge(peer);
                
                // Update discovery method to show multiple sources
                let sources = &peer_sources[&peer_id];
                if sources.len() > 1 {
                    existing.discovery_method = format!("multi({})", sources.join(","));
                }
            } else {
                unique_peers.insert(peer_id, peer);
            }
        }

        // Sort by discovery confidence (peers found by multiple strategies first)
        let mut final_peers: Vec<ServiceRecord> = unique_peers.into_values().collect();
        final_peers.sort_by(|a, b| {
            let a_sources = peer_sources.get(&a.peer_id).map(|s| s.len()).unwrap_or(0);
            let b_sources = peer_sources.get(&b.peer_id).map(|s| s.len()).unwrap_or(0);
            b_sources.cmp(&a_sources) // More sources = higher confidence
        });

        final_peers
    }

    /// Get strategy usage statistics
    pub async fn get_strategy_usage_report(&self) -> StrategyUsageReport {
        let stats = self.strategy_stats.read().await;
        let network_condition = self.network_condition_cache.read().await.clone();
        
        let mut strategy_reports = Vec::new();
        let mut total_discoveries = 0;
        let mut total_peers = 0;

        for (name, stat) in stats.iter() {
            let usage = StrategyUsage {
                name: name.clone(),
                success_count: stat.success_count,
                failure_count: stat.failure_count,
                success_rate: stat.success_rate(),
                average_response_time: stat.average_response_time,
                total_peers_discovered: stat.total_peers_discovered,
                performance_score: stat.performance_score(),
                is_recently_failing: stat.is_recently_failing(),
                network_latency: stat.network_latency,
                availability_score: stat.availability_score,
            };
            
            total_discoveries += stat.success_count + stat.failure_count;
            total_peers += stat.total_peers_discovered;
            strategy_reports.push(usage);
        }

        // Sort by performance score
        strategy_reports.sort_by(|a, b| b.performance_score.partial_cmp(&a.performance_score).unwrap_or(std::cmp::Ordering::Equal));

        StrategyUsageReport {
            strategies: strategy_reports,
            total_discoveries,
            total_peers_discovered: total_peers,
            network_condition,
            auto_selection_optimal: self.is_auto_selection_optimal().await,
            recommended_strategy: self.get_recommended_strategy().await,
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

    // Integration tests for enhanced auto-selection and multi-strategy discovery

    #[tokio::test]
    async fn test_auto_selection_with_performance_scoring() {
        let mut manager = DiscoveryManager::new();
        
        let peer1 = ServiceRecord::new("peer-1".to_string(), "Fast Device".to_string(), 8080);
        let peer2 = ServiceRecord::new("peer-2".to_string(), "Slow Device".to_string(), 8080);
        
        // Create strategies with different performance characteristics
        let fast_strategy = MockDiscovery::new("fast", true, 70)
            .with_peers(vec![peer1]);
        let slow_strategy = MockDiscovery::new("slow", true, 80) // Higher priority but will be slower
            .with_peers(vec![peer2]);
        
        manager.add_strategy_async(Box::new(fast_strategy)).await;
        manager.add_strategy_async(Box::new(slow_strategy)).await;
        
        // First discovery - should use priority order (slow strategy first)
        let result1 = manager.discover_peers(Duration::from_secs(5)).await;
        assert!(result1.is_ok());
        let peers1 = result1.unwrap();
        assert_eq!(peers1[0].peer_id, "peer-2"); // Slow strategy has higher priority
        
        // Simulate slow strategy becoming slower by adding artificial delay
        // In a real scenario, this would be measured automatically
        
        // Second discovery - auto-selection should now prefer the faster strategy
        let result2 = manager.discover_peers(Duration::from_secs(5)).await;
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_network_condition_evaluation() {
        let mut manager = DiscoveryManager::new();
        
        // Add strategies with different failure rates
        let reliable_strategy = MockDiscovery::new("reliable", true, 60)
            .with_peers(vec![ServiceRecord::new("peer-1".to_string(), "Device 1".to_string(), 8080)]);
        let unreliable_strategy = MockDiscovery::new("unreliable", true, 80)
            .with_failure();
        
        manager.add_strategy_async(Box::new(reliable_strategy)).await;
        manager.add_strategy_async(Box::new(unreliable_strategy)).await;
        
        // Perform multiple discoveries to build up statistics
        for _ in 0..5 {
            let _ = manager.discover_peers(Duration::from_secs(1)).await;
        }
        
        // Check that network conditions are being evaluated
        let conditions = manager.get_network_conditions().await;
        assert!(conditions.is_some());
        
        // Check strategy statistics
        let stats = manager.get_strategy_stats().await;
        assert!(stats.contains_key("reliable"));
        assert!(stats.contains_key("unreliable"));
        
        let reliable_stats = &stats["reliable"];
        let unreliable_stats = &stats["unreliable"];
        
        assert!(reliable_stats.success_rate() > unreliable_stats.success_rate());
    }

    #[tokio::test]
    async fn test_fallback_strategy_selection() {
        let mut manager = DiscoveryManager::new();
        manager.set_fallback_enabled(true);
        
        let peer = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        
        // Create strategies where the first few fail
        let failing_strategy1 = MockDiscovery::new("failing1", true, 90).with_failure();
        let failing_strategy2 = MockDiscovery::new("failing2", true, 80).with_failure();
        let working_strategy = MockDiscovery::new("working", true, 70)
            .with_peers(vec![peer.clone()]);
        
        manager.add_strategy_async(Box::new(failing_strategy1)).await;
        manager.add_strategy_async(Box::new(failing_strategy2)).await;
        manager.add_strategy_async(Box::new(working_strategy)).await;
        
        // Should eventually fall back to the working strategy
        let result = manager.discover_peers(Duration::from_secs(5)).await;
        assert!(result.is_ok());
        
        let peers = result.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, "peer-123");
    }

    #[tokio::test]
    async fn test_adaptive_timeout_calculation() {
        let mut manager = DiscoveryManager::new();
        manager.set_adaptive_timeout(true);
        
        // Simulate poor network conditions
        {
            let mut network_cache = manager.network_condition_cache.write().await;
            let mut condition = NetworkCondition::new();
            condition.packet_loss_rate = 0.2; // 20% packet loss
            condition.connection_stability = 0.6; // Poor stability
            condition.latency_estimate = Some(Duration::from_millis(800)); // High latency
            *network_cache = Some(condition);
        }
        
        let base_timeout = Duration::from_secs(5);
        let adaptive_timeout = manager.calculate_adaptive_timeout(base_timeout).await;
        
        // Should increase timeout for poor network conditions
        assert!(adaptive_timeout > base_timeout);
    }

    #[tokio::test]
    async fn test_concurrent_discovery_with_advanced_deduplication() {
        let mut manager = DiscoveryManager::new();
        manager.set_concurrent_discovery(true);
        
        // Create the same peer discovered by multiple strategies with different info
        let mut peer1 = ServiceRecord::new("peer-123".to_string(), "Device Base".to_string(), 8080);
        peer1.add_capability("version".to_string(), "1.0.0".to_string());
        
        let mut peer2 = ServiceRecord::new("peer-123".to_string(), "Device Enhanced".to_string(), 8080);
        peer2.add_capability("features".to_string(), "chat,file-transfer".to_string());
        peer2.add_address(std::net::SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(192, 168, 1, 100)), 
            8080
        ));
        
        let strategy1 = MockDiscovery::new("strategy1", true, 50)
            .with_peers(vec![peer1]);
        let strategy2 = MockDiscovery::new("strategy2", true, 60)
            .with_peers(vec![peer2]);
        
        manager.add_strategy_async(Box::new(strategy1)).await;
        manager.add_strategy_async(Box::new(strategy2)).await;
        
        let result = manager.discover_peers_concurrent_with_dedup(Duration::from_secs(5)).await;
        assert!(result.is_ok());
        
        let peers = result.unwrap();
        assert_eq!(peers.len(), 1); // Should be deduplicated
        
        let merged_peer = &peers[0];
        assert_eq!(merged_peer.peer_id, "peer-123");
        assert!(merged_peer.has_capability("version"));
        assert!(merged_peer.has_capability("features"));
        assert!(!merged_peer.addresses.is_empty());
        assert!(merged_peer.discovery_method.contains("multi")); // Should show multiple sources
    }

    #[tokio::test]
    async fn test_strategy_performance_testing() {
        let mut manager = DiscoveryManager::new();
        manager.set_performance_test_timeout(Duration::from_millis(100));
        
        let peer = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        let mock_strategy = MockDiscovery::new("test_strategy", true, 50)
            .with_peers(vec![peer]);
        
        manager.add_strategy_async(Box::new(mock_strategy)).await;
        
        // Perform discovery which should trigger performance testing
        let _result = manager.discover_peers(Duration::from_secs(5)).await;
        
        // Check that performance metrics were recorded
        let stats = manager.get_strategy_stats().await;
        let strategy_stats = &stats["test_strategy"];
        
        assert!(strategy_stats.last_performance_test.is_some());
        assert!(strategy_stats.network_latency.is_some());
    }

    #[tokio::test]
    async fn test_strategy_usage_report() {
        let mut manager = DiscoveryManager::new();
        
        let peer1 = ServiceRecord::new("peer-1".to_string(), "Device 1".to_string(), 8080);
        let peer2 = ServiceRecord::new("peer-2".to_string(), "Device 2".to_string(), 8080);
        
        let strategy1 = MockDiscovery::new("strategy1", true, 70)
            .with_peers(vec![peer1]);
        let strategy2 = MockDiscovery::new("strategy2", true, 60)
            .with_peers(vec![peer2]);
        
        manager.add_strategy_async(Box::new(strategy1)).await;
        manager.add_strategy_async(Box::new(strategy2)).await;
        
        // Perform several discoveries
        for _ in 0..3 {
            let _ = manager.discover_peers(Duration::from_secs(1)).await;
        }
        
        let report = manager.get_strategy_usage_report().await;
        
        assert_eq!(report.strategies.len(), 2);
        assert!(report.total_discoveries > 0);
        assert!(report.total_peers_discovered > 0);
        assert!(report.recommended_strategy.is_some());
        
        // Strategies should be sorted by performance score
        if report.strategies.len() >= 2 {
            assert!(report.strategies[0].performance_score >= report.strategies[1].performance_score);
        }
    }

    #[tokio::test]
    async fn test_strategy_config_integration() {
        let config = StrategyConfig {
            enable_mdns: true,
            enable_udp: true,
            enable_tcp: false, // Disable TCP for this test
            enable_bluetooth: false, // Disable Bluetooth for this test
            enable_libp2p: false, // Disable libp2p for this test
            mdns_config: Some(MdnsConfig {
                peer_id: "test-peer".to_string(),
                device_name: "Test Device".to_string(),
                port: 8080,
            }),
            udp_config: Some(UdpConfig {
                peer_id: "test-peer".to_string(),
                device_name: "Test Device".to_string(),
                port: 8080,
                broadcast_interval: Duration::from_secs(30),
            }),
            tcp_config: None,
            bluetooth_config: None,
            libp2p_config: None,
        };
        
        // This test would require actual strategy implementations to work
        // For now, we just test that the configuration structure is correct
        assert!(config.enable_mdns);
        assert!(config.enable_udp);
        assert!(!config.enable_tcp);
        assert!(config.mdns_config.is_some());
        assert!(config.udp_config.is_some());
    }

    #[tokio::test]
    async fn test_auto_selection_optimization_detection() {
        let mut manager = DiscoveryManager::new();
        
        // Add a well-performing strategy
        let good_peer = ServiceRecord::new("peer-good".to_string(), "Good Device".to_string(), 8080);
        let good_strategy = MockDiscovery::new("good_strategy", true, 80)
            .with_peers(vec![good_peer]);
        
        manager.add_strategy_async(Box::new(good_strategy)).await;
        
        // Perform several successful discoveries
        for _ in 0..5 {
            let _ = manager.discover_peers(Duration::from_secs(1)).await;
        }
        
        // Auto-selection should be considered optimal
        let is_optimal = manager.is_auto_selection_optimal().await;
        assert!(is_optimal);
        
        // Add a failing strategy and test again
        let failing_strategy = MockDiscovery::new("failing_strategy", true, 90)
            .with_failure();
        manager.add_strategy_async(Box::new(failing_strategy)).await;
        
        // Try the failing strategy a few times
        for _ in 0..3 {
            let _ = manager.discover_peers(Duration::from_secs(1)).await;
        }
        
        // Should still be optimal because we have at least one good strategy
        let is_still_optimal = manager.is_auto_selection_optimal().await;
        assert!(is_still_optimal);
    }

    #[tokio::test]
    async fn test_recently_failing_strategy_avoidance() {
        let mut manager = DiscoveryManager::new();
        
        let peer = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        
        // Create a strategy that will fail multiple times
        let mut failing_strategy = MockDiscovery::new("failing", true, 90);
        failing_strategy.should_fail = true;
        
        let working_strategy = MockDiscovery::new("working", true, 50)
            .with_peers(vec![peer.clone()]);
        
        manager.add_strategy_async(Box::new(failing_strategy)).await;
        manager.add_strategy_async(Box::new(working_strategy)).await;
        
        // Cause multiple failures for the failing strategy
        for _ in 0..5 {
            let _ = manager.discover_peers(Duration::from_secs(1)).await;
        }
        
        // Check that the failing strategy is marked as recently failing
        let is_failing = manager.is_strategy_recently_failing("failing").await;
        assert!(is_failing);
        
        // Subsequent discoveries should avoid the failing strategy
        let result = manager.discover_peers(Duration::from_secs(1)).await;
        assert!(result.is_ok());
        
        let peers = result.unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, "peer-123");
    }
}