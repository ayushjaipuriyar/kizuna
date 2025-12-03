# Discovery Performance Monitoring Implementation

## Overview

This document describes the performance monitoring functionality implemented for the Discovery Layer as part of task 9.2.

## Features Implemented

### 1. Timing Metrics for Each Discovery Strategy

The system now tracks comprehensive timing metrics for each discovery strategy:

- **Average discovery time**: Mean time taken for discovery operations
- **Minimum discovery time**: Fastest discovery operation recorded
- **Maximum discovery time**: Slowest discovery operation recorded
- **Total attempts**: Number of discovery attempts made
- **Successful attempts**: Number of successful discoveries

**API Methods:**
- `get_strategy_timing_metrics(strategy_name: &str) -> Option<TimingMetrics>`
- `get_global_timing_metrics() -> TimingMetrics`

### 2. Peer Cache Management with TTL and Cleanup

Enhanced peer cache management with automatic expiration and cleanup:

- **Configurable TTL**: Set custom time-to-live for cached peers
- **Automatic cleanup**: Remove expired peers from cache
- **Cache statistics tracking**: Monitor cache performance
  - Total entries
  - Active (non-expired) entries
  - Expired entries
  - Memory usage estimation
  - Average entry age
  - Cleanup operation count

**API Methods:**
- `set_peer_ttl(ttl: Duration)`
- `cleanup_expired_peers()`
- `get_cache_stats() -> CacheStats`
- `update_cache_monitoring_stats()`
- `configure_automatic_cleanup() -> Result<(), DiscoveryError>`

### 3. Resource Usage Monitoring and Optimization

Track resource consumption and provide optimization recommendations:

- **Resource metrics**:
  - Memory usage (bytes)
  - CPU usage percentage (placeholder for platform-specific implementation)
  - Network connections count
  - Open file descriptors
  - Thread count

- **Optimization recommendations**:
  - Cache hit rate analysis
  - Discovery success rate evaluation
  - Average discovery time assessment
  - Strategy performance evaluation
  - Concurrent discovery suggestions

**API Methods:**
- `get_resource_usage() -> ResourceUsage`
- `update_resource_usage()`
- `get_optimization_recommendations() -> Vec<String>`

### 4. Configuration Options for Timeouts and Retry Behavior

Comprehensive configuration system for fine-tuning discovery behavior:

#### Timeout Configuration
```rust
pub struct TimeoutConfig {
    pub performance_test_timeout: Duration,
    pub adaptive_timeout_enabled: bool,
}
```

**API Methods:**
- `get_timeout_config() -> TimeoutConfig`
- `set_timeout_config(config: TimeoutConfig)`

#### Retry Configuration
```rust
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}
```

**API Methods:**
- `get_retry_config() -> RetryConfig`
- `set_retry_config(config: RetryConfig)`

#### Monitoring Configuration
```rust
pub struct MonitoringConfig {
    pub peer_ttl: Duration,
    pub concurrent_discovery: bool,
    pub max_concurrent_strategies: usize,
    pub performance_test_timeout: Duration,
    pub adaptive_timeout: bool,
    pub fallback_enabled: bool,
    pub retry_config: RetryConfig,
    pub error_recovery_config: ErrorRecoveryConfig,
}
```

**API Methods:**
- `get_monitoring_config() -> MonitoringConfig`
- `set_monitoring_config(config: MonitoringConfig)`

## Performance Monitoring Infrastructure

### PerformanceMonitor Structure

The `PerformanceMonitor` tracks:
- Global metrics across all strategies
- Per-strategy metrics
- Resource usage statistics
- Timing history (last 1000 operations)
- Peer cache statistics

### PerformanceMetrics Structure

Tracks detailed metrics:
- Total discoveries (successful + failed)
- Success/failure counts
- Peer discovery counts
- Timing statistics (average, min, max)
- Cache hit/miss rates
- Network bytes sent/received
- Concurrent discovery count
- Strategy switch count

### Automatic Performance Monitoring

Start continuous monitoring with periodic updates:

```rust
let handle = manager.start_performance_monitoring(Duration::from_secs(60)).await;
```

This spawns a background task that:
- Updates resource usage every interval
- Performs automatic cleanup
- Updates cache monitoring statistics

## Usage Examples

### Basic Timing Metrics

```rust
// Get timing metrics for a specific strategy
let metrics = manager.get_strategy_timing_metrics("mdns").await;
if let Some(metrics) = metrics {
    println!("Average time: {:?}", metrics.average_time);
    println!("Success rate: {}/{}", 
        metrics.successful_attempts, 
        metrics.total_attempts);
}

// Get global timing metrics
let global = manager.get_global_timing_metrics().await;
println!("Global average: {:?}", global.average_time);
```

### Cache Management

```rust
// Set custom TTL
manager.set_peer_ttl(Duration::from_secs(600)); // 10 minutes

// Get cache statistics
let cache_stats = manager.get_cache_stats().await;
println!("Active peers: {}", cache_stats.active_entries);
println!("Memory usage: {} bytes", cache_stats.memory_usage_bytes);

// Manual cleanup
manager.cleanup_expired_peers().await;
```

### Configuration

```rust
// Configure timeouts
let timeout_config = TimeoutConfig {
    performance_test_timeout: Duration::from_secs(5),
    adaptive_timeout_enabled: true,
};
manager.set_timeout_config(timeout_config);

// Configure retry behavior
let retry_config = RetryConfig {
    max_attempts: 5,
    base_delay: Duration::from_millis(200),
    max_delay: Duration::from_secs(60),
    backoff_multiplier: 2.0,
    jitter: true,
};
manager.set_retry_config(retry_config);

// Configure comprehensive monitoring
let monitoring_config = MonitoringConfig {
    peer_ttl: Duration::from_secs(600),
    concurrent_discovery: true,
    max_concurrent_strategies: 5,
    performance_test_timeout: Duration::from_secs(15),
    adaptive_timeout: true,
    fallback_enabled: true,
    retry_config: RetryConfig::default(),
    error_recovery_config: ErrorRecoveryConfig::default(),
};
manager.set_monitoring_config(monitoring_config);
```

### Performance Reports

```rust
// Get comprehensive performance report
let report = manager.get_performance_report().await;
println!("Total discoveries: {}", report.global_metrics.total_discoveries);
println!("Success rate: {:.2}%", report.global_metrics.success_rate() * 100.0);
println!("Cache hit rate: {:.2}%", report.global_metrics.cache_hit_rate() * 100.0);

// Get top performing strategies
for (name, metrics) in report.top_strategies {
    println!("Strategy: {}, Score: {:.2}", name, 
        metrics.success_rate() * (1.0 / (metrics.average_discovery_time.as_millis() as f64 + 1.0)));
}

// Get optimization recommendations
let recommendations = manager.get_optimization_recommendations().await;
for recommendation in recommendations {
    println!("Recommendation: {}", recommendation);
}
```

## Testing

Comprehensive test suite covering:

1. **Timing metrics collection** - Verifies accurate timing measurement
2. **Global timing metrics** - Tests aggregation across strategies
3. **Cache statistics** - Validates cache tracking
4. **Resource usage monitoring** - Tests resource measurement
5. **Timeout configuration** - Verifies configuration management
6. **Retry configuration** - Tests retry behavior settings
7. **Monitoring configuration** - Validates comprehensive config
8. **Performance report generation** - Tests report creation
9. **Strategy performance metrics** - Verifies per-strategy tracking
10. **Optimization recommendations** - Tests recommendation engine
11. **Performance metrics reset** - Validates metric clearing

All tests are located in `src/discovery/manager.rs` under the `tests` module.

## Requirements Validation

This implementation satisfies the requirements from task 9.2:

✅ **Implement timing metrics for each discovery strategy**
- Per-strategy and global timing metrics
- Average, min, max timing tracking
- Success/failure rate tracking

✅ **Add peer cache management with TTL and cleanup**
- Configurable TTL
- Automatic expiration checking
- Manual and automatic cleanup
- Comprehensive cache statistics

✅ **Create resource usage monitoring and optimization**
- Memory usage tracking
- Resource usage statistics
- Optimization recommendation engine
- Performance report generation

✅ **Add configuration options for timeouts and retry behavior**
- TimeoutConfig for timeout management
- RetryConfig for retry behavior
- MonitoringConfig for comprehensive settings
- Getter/setter methods for all configurations

## Integration with Requirements

This implementation addresses:
- **Requirement 1.3**: Performance monitoring and optimization
- **Requirement 8.5**: Configuration and tuning capabilities

## Future Enhancements

Potential improvements for future iterations:

1. **Platform-specific resource monitoring**: Implement actual CPU, network, and file descriptor tracking using platform-specific APIs
2. **Persistent metrics**: Store metrics to disk for historical analysis
3. **Alerting system**: Trigger alerts when metrics exceed thresholds
4. **Metrics export**: Export metrics in standard formats (Prometheus, StatsD)
5. **Real-time dashboards**: Web-based monitoring dashboard
6. **Machine learning**: Predictive optimization based on historical patterns
