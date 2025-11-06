use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tokio::time::interval;

use super::{PeerId, ConnectionInfo, TransportError};

/// Comprehensive performance monitoring system for transport connections
#[derive(Debug)]
pub struct PerformanceMonitor {
    /// Connection metrics by peer ID
    connection_metrics: Arc<RwLock<HashMap<PeerId, ConnectionMetrics>>>,
    /// Global performance statistics
    global_stats: Arc<RwLock<GlobalPerformanceStats>>,
    /// Performance monitoring configuration
    config: PerformanceConfig,
    /// Bandwidth throttling manager
    bandwidth_manager: Arc<RwLock<BandwidthManager>>,
    /// Connection pool optimizer
    pool_optimizer: Arc<RwLock<ConnectionPoolOptimizer>>,
}

/// Configuration for performance monitoring
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
    /// Interval for collecting metrics
    pub metrics_collection_interval: Duration,
    /// Window size for moving averages
    pub metrics_window_size: usize,
    /// Enable bandwidth throttling
    pub enable_bandwidth_throttling: bool,
    /// Global bandwidth limit in bytes per second
    pub global_bandwidth_limit: Option<u64>,
    /// Per-connection bandwidth limit in bytes per second
    pub per_connection_bandwidth_limit: Option<u64>,
    /// Enable connection pool optimization
    pub enable_pool_optimization: bool,
    /// Idle connection timeout
    pub idle_connection_timeout: Duration,
    /// Connection quality threshold for optimization
    pub quality_threshold: f64,
    /// Enable adaptive protocol selection
    pub enable_adaptive_protocol_selection: bool,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            metrics_collection_interval: Duration::from_secs(1),
            metrics_window_size: 60, // 60 samples for 1-minute window
            enable_bandwidth_throttling: true,
            global_bandwidth_limit: None,
            per_connection_bandwidth_limit: Some(10 * 1024 * 1024), // 10 MB/s per connection
            enable_pool_optimization: true,
            idle_connection_timeout: Duration::from_secs(300), // 5 minutes
            quality_threshold: 0.7, // 70% quality threshold
            enable_adaptive_protocol_selection: true,
        }
    }
}

/// Detailed metrics for a specific connection
#[derive(Debug, Clone)]
pub struct ConnectionMetrics {
    pub peer_id: PeerId,
    pub protocol: String,
    pub established_at: SystemTime,
    pub last_activity: Instant,
    
    // Throughput metrics
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    
    // Latency metrics
    pub rtt_samples: VecDeque<Duration>,
    pub average_rtt: Duration,
    pub min_rtt: Duration,
    pub max_rtt: Duration,
    
    // Bandwidth metrics
    pub bandwidth_samples: VecDeque<BandwidthSample>,
    pub current_bandwidth: u64, // bytes per second
    pub peak_bandwidth: u64,
    pub average_bandwidth: u64,
    
    // Quality metrics
    pub packet_loss_rate: f64,
    pub connection_stability: f64,
    pub quality_score: f64,
    
    // Error metrics
    pub error_count: u64,
    pub last_error_time: Option<SystemTime>,
    pub consecutive_errors: u32,
    
    // Resource usage
    pub memory_usage: u64,
    pub cpu_usage: f64,
}

/// Bandwidth measurement sample
#[derive(Debug, Clone)]
pub struct BandwidthSample {
    pub timestamp: Instant,
    pub bytes_transferred: u64,
    pub duration: Duration,
}

/// Global performance statistics across all connections
#[derive(Debug, Clone)]
pub struct GlobalPerformanceStats {
    pub total_connections: u64,
    pub active_connections: u64,
    pub total_bytes_transferred: u64,
    pub total_messages: u64,
    pub average_connection_quality: f64,
    pub global_bandwidth_usage: u64,
    pub peak_concurrent_connections: u64,
    pub connection_success_rate: f64,
    pub protocol_distribution: HashMap<String, u64>,
    pub error_rate: f64,
    pub uptime: Duration,
    pub last_updated: SystemTime,
}

impl Default for GlobalPerformanceStats {
    fn default() -> Self {
        Self {
            total_connections: 0,
            active_connections: 0,
            total_bytes_transferred: 0,
            total_messages: 0,
            average_connection_quality: 0.0,
            global_bandwidth_usage: 0,
            peak_concurrent_connections: 0,
            connection_success_rate: 1.0,
            protocol_distribution: HashMap::new(),
            error_rate: 0.0,
            uptime: Duration::ZERO,
            last_updated: SystemTime::now(),
        }
    }
}

/// Bandwidth management and throttling
#[derive(Debug)]
pub struct BandwidthManager {
    /// Global bandwidth usage tracker
    global_usage: BandwidthTracker,
    /// Per-connection bandwidth trackers
    connection_trackers: HashMap<PeerId, BandwidthTracker>,
    /// Bandwidth allocation strategy
    allocation_strategy: BandwidthAllocationStrategy,
    /// Configuration
    config: PerformanceConfig,
}

/// Bandwidth tracking for throttling decisions
#[derive(Debug, Clone)]
pub struct BandwidthTracker {
    pub current_usage: u64, // bytes per second
    pub allocated_limit: u64, // bytes per second
    pub usage_history: VecDeque<BandwidthSample>,
    pub last_reset: Instant,
    pub bytes_this_second: u64,
}

/// Strategy for allocating bandwidth among connections
#[derive(Debug, Clone)]
pub enum BandwidthAllocationStrategy {
    /// Equal allocation among all connections
    Equal,
    /// Priority-based allocation
    Priority(HashMap<PeerId, u8>),
    /// Quality-based allocation (higher quality gets more bandwidth)
    QualityBased,
    /// Adaptive allocation based on connection needs
    Adaptive,
}

/// Connection pool optimization manager
#[derive(Debug)]
pub struct ConnectionPoolOptimizer {
    /// Connection usage statistics
    usage_stats: HashMap<PeerId, ConnectionUsageStats>,
    /// Optimization recommendations
    recommendations: Vec<OptimizationRecommendation>,
    /// Last optimization run
    last_optimization: Instant,
    /// Configuration
    config: PerformanceConfig,
}

/// Usage statistics for connection pool optimization
#[derive(Debug, Clone)]
pub struct ConnectionUsageStats {
    pub peer_id: PeerId,
    pub last_used: Instant,
    pub usage_frequency: f64, // uses per hour
    pub average_session_duration: Duration,
    pub data_transfer_rate: u64, // bytes per session
    pub connection_overhead: Duration, // time to establish
    pub reliability_score: f64,
}

/// Optimization recommendation for connection management
#[derive(Debug, Clone)]
pub enum OptimizationRecommendation {
    /// Close idle connection
    CloseIdleConnection { peer_id: PeerId, idle_time: Duration },
    /// Upgrade connection protocol
    UpgradeProtocol { peer_id: PeerId, from: String, to: String, reason: String },
    /// Reduce bandwidth allocation
    ReduceBandwidth { peer_id: PeerId, current: u64, recommended: u64 },
    /// Increase bandwidth allocation
    IncreaseBandwidth { peer_id: PeerId, current: u64, recommended: u64 },
    /// Preemptively establish connection
    PreestablishConnection { peer_id: PeerId, reason: String },
    /// Switch to different transport
    SwitchTransport { peer_id: PeerId, current: String, recommended: String },
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self::with_config(PerformanceConfig::default())
    }

    /// Create a new performance monitor with custom configuration
    pub fn with_config(config: PerformanceConfig) -> Self {
        Self {
            connection_metrics: Arc::new(RwLock::new(HashMap::new())),
            global_stats: Arc::new(RwLock::new(GlobalPerformanceStats::default())),
            bandwidth_manager: Arc::new(RwLock::new(BandwidthManager::new(config.clone()))),
            pool_optimizer: Arc::new(RwLock::new(ConnectionPoolOptimizer::new(config.clone()))),
            config,
        }
    }

    /// Start the performance monitoring background task
    pub async fn start_monitoring(&self) {
        let metrics = self.connection_metrics.clone();
        let global_stats = self.global_stats.clone();
        let bandwidth_manager = self.bandwidth_manager.clone();
        let pool_optimizer = self.pool_optimizer.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(config.metrics_collection_interval);
            
            loop {
                interval.tick().await;
                
                // Update global statistics
                Self::update_global_stats(&metrics, &global_stats).await;
                
                // Update bandwidth tracking
                if config.enable_bandwidth_throttling {
                    Self::update_bandwidth_tracking(&bandwidth_manager).await;
                }
                
                // Run connection pool optimization
                if config.enable_pool_optimization {
                    Self::run_pool_optimization(&metrics, &pool_optimizer).await;
                }
            }
        });
    }

    /// Record connection establishment
    pub async fn record_connection_established(&self, peer_id: PeerId, protocol: String) {
        let mut metrics = self.connection_metrics.write().await;
        let connection_metrics = ConnectionMetrics::new(peer_id.clone(), protocol.clone());
        metrics.insert(peer_id, connection_metrics);

        // Update global stats
        let mut global_stats = self.global_stats.write().await;
        global_stats.total_connections += 1;
        global_stats.active_connections += 1;
        *global_stats.protocol_distribution.entry(protocol).or_insert(0) += 1;
        
        if global_stats.active_connections > global_stats.peak_concurrent_connections {
            global_stats.peak_concurrent_connections = global_stats.active_connections;
        }
    }

    /// Record connection closure
    pub async fn record_connection_closed(&self, peer_id: &PeerId) {
        let mut metrics = self.connection_metrics.write().await;
        if let Some(connection_metrics) = metrics.remove(peer_id) {
            // Update global stats
            let mut global_stats = self.global_stats.write().await;
            global_stats.active_connections = global_stats.active_connections.saturating_sub(1);
            global_stats.total_bytes_transferred += connection_metrics.bytes_sent + connection_metrics.bytes_received;
            global_stats.total_messages += connection_metrics.messages_sent + connection_metrics.messages_received;
        }

        // Remove from bandwidth manager
        let mut bandwidth_manager = self.bandwidth_manager.write().await;
        bandwidth_manager.connection_trackers.remove(peer_id);
    }

    /// Record data transfer
    pub async fn record_data_transfer(&self, peer_id: &PeerId, bytes_sent: u64, bytes_received: u64) {
        let mut metrics = self.connection_metrics.write().await;
        if let Some(connection_metrics) = metrics.get_mut(peer_id) {
            connection_metrics.bytes_sent += bytes_sent;
            connection_metrics.bytes_received += bytes_received;
            connection_metrics.last_activity = Instant::now();
            
            // Update bandwidth samples
            let total_bytes = bytes_sent + bytes_received;
            if total_bytes > 0 {
                let sample = BandwidthSample {
                    timestamp: Instant::now(),
                    bytes_transferred: total_bytes,
                    duration: Duration::from_secs(1), // Approximate
                };
                
                connection_metrics.bandwidth_samples.push_back(sample);
                if connection_metrics.bandwidth_samples.len() > self.config.metrics_window_size {
                    connection_metrics.bandwidth_samples.pop_front();
                }
                
                // Update current bandwidth estimate
                connection_metrics.update_bandwidth_metrics();
            }
        }
    }

    /// Record RTT measurement
    pub async fn record_rtt(&self, peer_id: &PeerId, rtt: Duration) {
        let mut metrics = self.connection_metrics.write().await;
        if let Some(connection_metrics) = metrics.get_mut(peer_id) {
            connection_metrics.rtt_samples.push_back(rtt);
            if connection_metrics.rtt_samples.len() > self.config.metrics_window_size {
                connection_metrics.rtt_samples.pop_front();
            }
            
            connection_metrics.update_rtt_metrics();
        }
    }

    /// Record connection error
    pub async fn record_error(&self, peer_id: &PeerId) {
        let mut metrics = self.connection_metrics.write().await;
        if let Some(connection_metrics) = metrics.get_mut(peer_id) {
            connection_metrics.error_count += 1;
            connection_metrics.last_error_time = Some(SystemTime::now());
            connection_metrics.consecutive_errors += 1;
            
            // Update quality score based on errors
            connection_metrics.update_quality_score();
        }
    }

    /// Check if bandwidth is available for a connection
    pub async fn check_bandwidth_availability(&self, peer_id: &PeerId, requested_bytes: u64) -> bool {
        if !self.config.enable_bandwidth_throttling {
            return true;
        }

        let bandwidth_manager = self.bandwidth_manager.read().await;
        bandwidth_manager.check_bandwidth_availability(peer_id, requested_bytes)
    }

    /// Allocate bandwidth for a connection
    pub async fn allocate_bandwidth(&self, peer_id: &PeerId, bytes: u64) -> Result<(), TransportError> {
        if !self.config.enable_bandwidth_throttling {
            return Ok(());
        }

        let mut bandwidth_manager = self.bandwidth_manager.write().await;
        bandwidth_manager.allocate_bandwidth(peer_id, bytes)
    }

    /// Get connection metrics for a specific peer
    pub async fn get_connection_metrics(&self, peer_id: &PeerId) -> Option<ConnectionMetrics> {
        let metrics = self.connection_metrics.read().await;
        metrics.get(peer_id).cloned()
    }

    /// Get global performance statistics
    pub async fn get_global_stats(&self) -> GlobalPerformanceStats {
        self.global_stats.read().await.clone()
    }

    /// Get optimization recommendations
    pub async fn get_optimization_recommendations(&self) -> Vec<OptimizationRecommendation> {
        let optimizer = self.pool_optimizer.read().await;
        optimizer.recommendations.clone()
    }

    /// Get performance report
    pub async fn get_performance_report(&self) -> PerformanceReport {
        let global_stats = self.get_global_stats().await;
        let connection_metrics = self.connection_metrics.read().await;
        let recommendations = self.get_optimization_recommendations().await;

        let connection_count = connection_metrics.len();
        let average_quality = if connection_count > 0 {
            connection_metrics.values().map(|m| m.quality_score).sum::<f64>() / connection_count as f64
        } else {
            0.0
        };

        let total_bandwidth = connection_metrics.values().map(|m| m.current_bandwidth).sum::<u64>();

        PerformanceReport {
            timestamp: SystemTime::now(),
            global_stats,
            active_connections: connection_count,
            average_connection_quality: average_quality,
            total_bandwidth_usage: total_bandwidth,
            optimization_recommendations: recommendations,
            health_status: self.calculate_health_status(&connection_metrics).await,
        }
    }

    /// Calculate overall health status
    async fn calculate_health_status(&self, metrics: &HashMap<PeerId, ConnectionMetrics>) -> HealthStatus {
        let total_connections = metrics.len();
        if total_connections == 0 {
            return HealthStatus::Healthy;
        }

        let healthy_connections = metrics.values()
            .filter(|m| m.quality_score >= self.config.quality_threshold)
            .count();

        let health_ratio = healthy_connections as f64 / total_connections as f64;

        match health_ratio {
            r if r >= 0.9 => HealthStatus::Healthy,
            r if r >= 0.7 => HealthStatus::Degraded,
            _ => HealthStatus::Unhealthy,
        }
    }

    /// Update global statistics
    async fn update_global_stats(
        metrics: &Arc<RwLock<HashMap<PeerId, ConnectionMetrics>>>,
        global_stats: &Arc<RwLock<GlobalPerformanceStats>>,
    ) {
        let metrics = metrics.read().await;
        let mut stats = global_stats.write().await;

        stats.active_connections = metrics.len() as u64;
        
        if !metrics.is_empty() {
            stats.average_connection_quality = metrics.values()
                .map(|m| m.quality_score)
                .sum::<f64>() / metrics.len() as f64;
            
            stats.global_bandwidth_usage = metrics.values()
                .map(|m| m.current_bandwidth)
                .sum::<u64>();
        }

        stats.last_updated = SystemTime::now();
    }

    /// Update bandwidth tracking
    async fn update_bandwidth_tracking(bandwidth_manager: &Arc<RwLock<BandwidthManager>>) {
        let mut manager = bandwidth_manager.write().await;
        manager.update_tracking();
    }

    /// Run connection pool optimization
    async fn run_pool_optimization(
        metrics: &Arc<RwLock<HashMap<PeerId, ConnectionMetrics>>>,
        optimizer: &Arc<RwLock<ConnectionPoolOptimizer>>,
    ) {
        let metrics = metrics.read().await;
        let mut optimizer = optimizer.write().await;
        optimizer.analyze_and_recommend(&metrics);
    }
}

impl ConnectionMetrics {
    /// Create new connection metrics
    pub fn new(peer_id: PeerId, protocol: String) -> Self {
        Self {
            peer_id,
            protocol,
            established_at: SystemTime::now(),
            last_activity: Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
            messages_sent: 0,
            messages_received: 0,
            rtt_samples: VecDeque::new(),
            average_rtt: Duration::ZERO,
            min_rtt: Duration::MAX,
            max_rtt: Duration::ZERO,
            bandwidth_samples: VecDeque::new(),
            current_bandwidth: 0,
            peak_bandwidth: 0,
            average_bandwidth: 0,
            packet_loss_rate: 0.0,
            connection_stability: 1.0,
            quality_score: 1.0,
            error_count: 0,
            last_error_time: None,
            consecutive_errors: 0,
            memory_usage: 0,
            cpu_usage: 0.0,
        }
    }

    /// Update RTT metrics from samples
    pub fn update_rtt_metrics(&mut self) {
        if self.rtt_samples.is_empty() {
            return;
        }

        self.average_rtt = Duration::from_nanos(
            self.rtt_samples.iter().map(|d| d.as_nanos()).sum::<u128>() as u64 / self.rtt_samples.len() as u64
        );

        self.min_rtt = *self.rtt_samples.iter().min().unwrap_or(&Duration::ZERO);
        self.max_rtt = *self.rtt_samples.iter().max().unwrap_or(&Duration::ZERO);
    }

    /// Update bandwidth metrics from samples
    pub fn update_bandwidth_metrics(&mut self) {
        if self.bandwidth_samples.is_empty() {
            return;
        }

        // Calculate current bandwidth (bytes per second)
        let recent_samples: Vec<_> = self.bandwidth_samples
            .iter()
            .rev()
            .take(5) // Use last 5 samples for current estimate
            .collect();

        if !recent_samples.is_empty() {
            let total_bytes: u64 = recent_samples.iter().map(|s| s.bytes_transferred).sum();
            let total_duration: Duration = recent_samples.iter().map(|s| s.duration).sum();
            
            if total_duration.as_secs_f64() > 0.0 {
                self.current_bandwidth = (total_bytes as f64 / total_duration.as_secs_f64()) as u64;
            }
        }

        // Update peak bandwidth
        if self.current_bandwidth > self.peak_bandwidth {
            self.peak_bandwidth = self.current_bandwidth;
        }

        // Calculate average bandwidth
        let total_bytes: u64 = self.bandwidth_samples.iter().map(|s| s.bytes_transferred).sum();
        let total_duration: Duration = self.bandwidth_samples.iter().map(|s| s.duration).sum();
        
        if total_duration.as_secs_f64() > 0.0 {
            self.average_bandwidth = (total_bytes as f64 / total_duration.as_secs_f64()) as u64;
        }
    }

    /// Update connection quality score
    pub fn update_quality_score(&mut self) {
        let mut score = 1.0;

        // Factor in RTT (lower is better)
        if !self.rtt_samples.is_empty() {
            let rtt_penalty = (self.average_rtt.as_millis() as f64 / 1000.0).min(0.5);
            score -= rtt_penalty;
        }

        // Factor in error rate
        let connection_age = self.established_at.elapsed().unwrap_or_default().as_secs().max(1);
        let error_rate = self.error_count as f64 / connection_age as f64;
        score -= (error_rate * 10.0).min(0.3);

        // Factor in consecutive errors
        if self.consecutive_errors > 0 {
            score -= (self.consecutive_errors as f64 * 0.1).min(0.2);
        }

        // Factor in bandwidth utilization (higher is better, up to a point)
        if self.current_bandwidth > 0 {
            let bandwidth_score = (self.current_bandwidth as f64 / (10 * 1024 * 1024) as f64).min(1.0) * 0.1;
            score += bandwidth_score;
        }

        self.quality_score = score.max(0.0).min(1.0);
    }
}

impl BandwidthManager {
    /// Create a new bandwidth manager
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            global_usage: BandwidthTracker::new(config.global_bandwidth_limit.unwrap_or(u64::MAX)),
            connection_trackers: HashMap::new(),
            allocation_strategy: BandwidthAllocationStrategy::Adaptive,
            config,
        }
    }

    /// Check if bandwidth is available
    pub fn check_bandwidth_availability(&self, peer_id: &PeerId, requested_bytes: u64) -> bool {
        // Check global limit
        if let Some(global_limit) = self.config.global_bandwidth_limit {
            if self.global_usage.current_usage + requested_bytes > global_limit {
                return false;
            }
        }

        // Check per-connection limit
        if let Some(per_conn_limit) = self.config.per_connection_bandwidth_limit {
            if let Some(tracker) = self.connection_trackers.get(peer_id) {
                if tracker.current_usage + requested_bytes > per_conn_limit {
                    return false;
                }
            }
        }

        true
    }

    /// Allocate bandwidth for a connection
    pub fn allocate_bandwidth(&mut self, peer_id: &PeerId, bytes: u64) -> Result<(), TransportError> {
        if !self.check_bandwidth_availability(peer_id, bytes) {
            return Err(TransportError::BandwidthLimitExceeded {
                current_bps: self.global_usage.current_usage,
                limit_bps: self.config.global_bandwidth_limit.unwrap_or(u64::MAX),
            });
        }

        // Update global usage
        self.global_usage.current_usage += bytes;

        // Update connection usage
        let tracker = self.connection_trackers.entry(peer_id.clone()).or_insert_with(|| {
            BandwidthTracker::new(self.config.per_connection_bandwidth_limit.unwrap_or(u64::MAX))
        });
        tracker.current_usage += bytes;

        Ok(())
    }

    /// Update bandwidth tracking (called periodically)
    pub fn update_tracking(&mut self) {
        let now = Instant::now();
        
        // Reset global usage if a second has passed
        if now.duration_since(self.global_usage.last_reset) >= Duration::from_secs(1) {
            self.global_usage.current_usage = 0;
            self.global_usage.last_reset = now;
        }

        // Reset connection usage
        for tracker in self.connection_trackers.values_mut() {
            if now.duration_since(tracker.last_reset) >= Duration::from_secs(1) {
                tracker.current_usage = 0;
                tracker.last_reset = now;
            }
        }
    }
}

impl BandwidthTracker {
    /// Create a new bandwidth tracker
    pub fn new(limit: u64) -> Self {
        Self {
            current_usage: 0,
            allocated_limit: limit,
            usage_history: VecDeque::new(),
            last_reset: Instant::now(),
            bytes_this_second: 0,
        }
    }
}

impl ConnectionPoolOptimizer {
    /// Create a new connection pool optimizer
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            usage_stats: HashMap::new(),
            recommendations: Vec::new(),
            last_optimization: Instant::now(),
            config,
        }
    }

    /// Analyze connections and generate recommendations
    pub fn analyze_and_recommend(&mut self, metrics: &HashMap<PeerId, ConnectionMetrics>) {
        self.recommendations.clear();
        let now = Instant::now();

        for (peer_id, connection_metrics) in metrics {
            // Check for idle connections
            let idle_time = now.duration_since(connection_metrics.last_activity);
            if idle_time > self.config.idle_connection_timeout {
                self.recommendations.push(OptimizationRecommendation::CloseIdleConnection {
                    peer_id: peer_id.clone(),
                    idle_time,
                });
            }

            // Check for low-quality connections that might benefit from protocol upgrade
            if connection_metrics.quality_score < self.config.quality_threshold {
                if connection_metrics.protocol == "tcp" && connection_metrics.average_rtt > Duration::from_millis(100) {
                    self.recommendations.push(OptimizationRecommendation::UpgradeProtocol {
                        peer_id: peer_id.clone(),
                        from: "tcp".to_string(),
                        to: "quic".to_string(),
                        reason: "High latency detected".to_string(),
                    });
                }
            }

            // Check bandwidth allocation efficiency
            if let Some(per_conn_limit) = self.config.per_connection_bandwidth_limit {
                let usage_ratio = connection_metrics.current_bandwidth as f64 / per_conn_limit as f64;
                
                if usage_ratio > 0.9 {
                    // Connection is using most of its allocated bandwidth
                    self.recommendations.push(OptimizationRecommendation::IncreaseBandwidth {
                        peer_id: peer_id.clone(),
                        current: per_conn_limit,
                        recommended: (per_conn_limit as f64 * 1.5) as u64,
                    });
                } else if usage_ratio < 0.1 && connection_metrics.current_bandwidth > 0 {
                    // Connection is using very little bandwidth
                    self.recommendations.push(OptimizationRecommendation::ReduceBandwidth {
                        peer_id: peer_id.clone(),
                        current: per_conn_limit,
                        recommended: (per_conn_limit as f64 * 0.5) as u64,
                    });
                }
            }
        }

        self.last_optimization = now;
    }
}

/// Performance report containing comprehensive metrics
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub timestamp: SystemTime,
    pub global_stats: GlobalPerformanceStats,
    pub active_connections: usize,
    pub average_connection_quality: f64,
    pub total_bandwidth_usage: u64,
    pub optimization_recommendations: Vec<OptimizationRecommendation>,
    pub health_status: HealthStatus,
}

/// Overall health status of the transport system
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_performance_monitoring() {
        let monitor = PerformanceMonitor::new();
        
        // Record connection establishment
        monitor.record_connection_established("peer1".to_string(), "tcp".to_string()).await;
        
        // Record data transfer
        monitor.record_data_transfer(&"peer1".to_string(), 1024, 2048).await;
        
        // Record RTT
        monitor.record_rtt(&"peer1".to_string(), Duration::from_millis(50)).await;
        
        // Get metrics
        let metrics = monitor.get_connection_metrics(&"peer1".to_string()).await;
        assert!(metrics.is_some());
        
        let metrics = metrics.unwrap();
        assert_eq!(metrics.bytes_sent, 1024);
        assert_eq!(metrics.bytes_received, 2048);
        assert_eq!(metrics.rtt_samples.len(), 1);
    }

    #[tokio::test]
    async fn test_bandwidth_throttling() {
        let mut config = PerformanceConfig::default();
        config.global_bandwidth_limit = Some(1000); // 1KB/s limit
        let monitor = PerformanceMonitor::with_config(config);
        
        // Check bandwidth availability
        let available = monitor.check_bandwidth_availability(&"peer1".to_string(), 500).await;
        assert!(available);
        
        // Allocate bandwidth
        let result = monitor.allocate_bandwidth(&"peer1".to_string(), 500).await;
        assert!(result.is_ok());
        
        // Try to allocate more than limit
        let result = monitor.allocate_bandwidth(&"peer1".to_string(), 600).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_quality_calculation() {
        let mut metrics = ConnectionMetrics::new("peer1".to_string(), "tcp".to_string());
        
        // Initially should have perfect quality
        assert_eq!(metrics.quality_score, 1.0);
        
        // Add some RTT samples
        metrics.rtt_samples.push_back(Duration::from_millis(100));
        metrics.rtt_samples.push_back(Duration::from_millis(150));
        metrics.update_rtt_metrics();
        metrics.update_quality_score();
        
        // Quality should decrease due to higher RTT
        assert!(metrics.quality_score < 1.0);
        
        // Add errors
        metrics.error_count = 5;
        metrics.consecutive_errors = 2;
        metrics.update_quality_score();
        
        // Quality should decrease further due to errors
        assert!(metrics.quality_score < 0.8);
    }

    #[tokio::test]
    async fn test_optimization_recommendations() {
        let mut config = PerformanceConfig::default();
        config.idle_connection_timeout = Duration::from_secs(1);
        let monitor = PerformanceMonitor::with_config(config);
        
        // Create a connection and let it become idle
        monitor.record_connection_established("peer1".to_string(), "tcp".to_string()).await;
        
        // Wait for it to become idle
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Manually trigger optimization analysis
        let connection_metrics = monitor.connection_metrics.read().await;
        let mut optimizer = monitor.pool_optimizer.write().await;
        optimizer.analyze_and_recommend(&connection_metrics);
        drop(optimizer);
        drop(connection_metrics);
        
        // Get recommendations
        let recommendations = monitor.get_optimization_recommendations().await;
        
        // Should recommend closing idle connection
        assert!(!recommendations.is_empty());
        assert!(matches!(recommendations[0], OptimizationRecommendation::CloseIdleConnection { .. }));
    }
}