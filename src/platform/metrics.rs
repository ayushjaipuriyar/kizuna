// Cross-platform performance metrics and monitoring
//
// This module provides:
// - Platform-specific performance metric collection
// - Performance profiling and debugging tools
// - Automated performance regression testing support

use crate::platform::{PlatformResult, PlatformError};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use async_trait::async_trait;

/// Performance metrics collector trait
#[async_trait]
pub trait MetricsCollector: Send + Sync {
    /// Start collecting metrics
    async fn start(&self) -> PlatformResult<()>;
    
    /// Stop collecting metrics
    async fn stop(&self) -> PlatformResult<()>;
    
    /// Record a metric value
    fn record_metric(&self, name: &str, value: MetricValue) -> PlatformResult<()>;
    
    /// Get current metrics snapshot
    fn get_metrics(&self) -> PlatformResult<MetricsSnapshot>;
    
    /// Get metric history
    fn get_metric_history(&self, name: &str) -> PlatformResult<Vec<MetricValue>>;
    
    /// Clear all metrics
    fn clear_metrics(&self) -> PlatformResult<()>;
}

/// Metric value types
#[derive(Debug, Clone)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Duration(Duration),
    Timestamp(Instant),
}

/// Metrics snapshot at a point in time
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub timestamp: Instant,
    pub metrics: HashMap<String, MetricValue>,
    pub system_metrics: SystemMetrics,
}

/// System-level metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_usage_percent: f64,
    pub network_bytes_sent: u64,
    pub network_bytes_received: u64,
    pub disk_bytes_read: u64,
    pub disk_bytes_written: u64,
    pub thread_count: usize,
    pub open_file_descriptors: usize,
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_bytes: 0,
            memory_usage_percent: 0.0,
            network_bytes_sent: 0,
            network_bytes_received: 0,
            disk_bytes_read: 0,
            disk_bytes_written: 0,
            thread_count: 0,
            open_file_descriptors: 0,
        }
    }
}

/// Performance profiler for detailed analysis
#[async_trait]
pub trait PerformanceProfiler: Send + Sync {
    /// Start profiling session
    async fn start_profiling(&self, config: ProfilingConfig) -> PlatformResult<ProfilingSession>;
    
    /// Stop profiling session
    async fn stop_profiling(&self, session: ProfilingSession) -> PlatformResult<ProfilingReport>;
    
    /// Record a profiling event
    fn record_event(&self, session: &ProfilingSession, event: ProfilingEvent) -> PlatformResult<()>;
    
    /// Get profiling statistics
    fn get_statistics(&self, session: &ProfilingSession) -> PlatformResult<ProfilingStatistics>;
}

/// Profiling configuration
#[derive(Debug, Clone)]
pub struct ProfilingConfig {
    pub sample_rate_hz: u32,
    pub collect_cpu: bool,
    pub collect_memory: bool,
    pub collect_io: bool,
    pub collect_network: bool,
    pub stack_traces: bool,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: 100,
            collect_cpu: true,
            collect_memory: true,
            collect_io: true,
            collect_network: true,
            stack_traces: false,
        }
    }
}

/// Profiling session identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProfilingSession {
    pub id: u64,
    pub start_time: Instant,
}

/// Profiling event
#[derive(Debug, Clone)]
pub struct ProfilingEvent {
    pub timestamp: Instant,
    pub event_type: ProfilingEventType,
    pub duration: Option<Duration>,
    pub metadata: HashMap<String, String>,
}

/// Profiling event types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfilingEventType {
    FunctionCall(String),
    Allocation(usize),
    Deallocation(usize),
    IORead(usize),
    IOWrite(usize),
    NetworkSend(usize),
    NetworkReceive(usize),
    Custom(String),
}

/// Profiling report
#[derive(Debug, Clone)]
pub struct ProfilingReport {
    pub session: ProfilingSession,
    pub duration: Duration,
    pub statistics: ProfilingStatistics,
    pub events: Vec<ProfilingEvent>,
    pub hotspots: Vec<Hotspot>,
}

/// Profiling statistics
#[derive(Debug, Clone)]
pub struct ProfilingStatistics {
    pub total_samples: u64,
    pub cpu_samples: u64,
    pub memory_allocations: u64,
    pub memory_deallocations: u64,
    pub total_allocated_bytes: u64,
    pub total_deallocated_bytes: u64,
    pub io_operations: u64,
    pub network_operations: u64,
}

impl Default for ProfilingStatistics {
    fn default() -> Self {
        Self {
            total_samples: 0,
            cpu_samples: 0,
            memory_allocations: 0,
            memory_deallocations: 0,
            total_allocated_bytes: 0,
            total_deallocated_bytes: 0,
            io_operations: 0,
            network_operations: 0,
        }
    }
}

/// Performance hotspot
#[derive(Debug, Clone)]
pub struct Hotspot {
    pub function_name: String,
    pub sample_count: u64,
    pub percentage: f64,
    pub average_duration: Duration,
}

/// Performance regression tester
#[async_trait]
pub trait RegressionTester: Send + Sync {
    /// Run regression test
    async fn run_test(&self, test: RegressionTest) -> PlatformResult<RegressionResult>;
    
    /// Compare with baseline
    fn compare_with_baseline(&self, current: &MetricsSnapshot, baseline: &MetricsSnapshot) 
        -> PlatformResult<RegressionComparison>;
    
    /// Save baseline
    fn save_baseline(&self, name: &str, metrics: &MetricsSnapshot) -> PlatformResult<()>;
    
    /// Load baseline
    fn load_baseline(&self, name: &str) -> PlatformResult<MetricsSnapshot>;
}

/// Regression test definition
#[derive(Debug, Clone)]
pub struct RegressionTest {
    pub name: String,
    pub baseline_name: String,
    pub thresholds: RegressionThresholds,
    pub iterations: u32,
}

/// Regression thresholds
#[derive(Debug, Clone)]
pub struct RegressionThresholds {
    pub max_cpu_increase_percent: f64,
    pub max_memory_increase_percent: f64,
    pub max_duration_increase_percent: f64,
    pub max_throughput_decrease_percent: f64,
}

impl Default for RegressionThresholds {
    fn default() -> Self {
        Self {
            max_cpu_increase_percent: 10.0,
            max_memory_increase_percent: 10.0,
            max_duration_increase_percent: 10.0,
            max_throughput_decrease_percent: 10.0,
        }
    }
}

/// Regression test result
#[derive(Debug, Clone)]
pub struct RegressionResult {
    pub test_name: String,
    pub passed: bool,
    pub comparison: RegressionComparison,
    pub violations: Vec<RegressionViolation>,
}

/// Regression comparison
#[derive(Debug, Clone)]
pub struct RegressionComparison {
    pub cpu_change_percent: f64,
    pub memory_change_percent: f64,
    pub duration_change_percent: f64,
    pub throughput_change_percent: f64,
}

/// Regression violation
#[derive(Debug, Clone)]
pub struct RegressionViolation {
    pub metric_name: String,
    pub threshold: f64,
    pub actual: f64,
    pub severity: ViolationSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationSeverity {
    Warning,
    Error,
    Critical,
}

/// Metrics collector factory
pub struct MetricsCollectorFactory;

impl MetricsCollectorFactory {
    /// Create metrics collector
    pub fn create() -> Arc<dyn MetricsCollector> {
        Arc::new(DefaultMetricsCollector::new())
    }
}

/// Performance profiler factory
pub struct PerformanceProfilerFactory;

impl PerformanceProfilerFactory {
    /// Create performance profiler
    pub fn create() -> Arc<dyn PerformanceProfiler> {
        Arc::new(DefaultPerformanceProfiler::new())
    }
}

/// Regression tester factory
pub struct RegressionTesterFactory;

impl RegressionTesterFactory {
    /// Create regression tester
    pub fn create() -> Arc<dyn RegressionTester> {
        Arc::new(DefaultRegressionTester::new())
    }
}

// Default implementations

/// Default metrics collector
pub struct DefaultMetricsCollector {
    metrics: Arc<Mutex<HashMap<String, Vec<MetricValue>>>>,
    running: Arc<Mutex<bool>>,
}

impl DefaultMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
            running: Arc::new(Mutex::new(false)),
        }
    }
}

#[async_trait]
impl MetricsCollector for DefaultMetricsCollector {
    async fn start(&self) -> PlatformResult<()> {
        *self.running.lock().unwrap() = true;
        Ok(())
    }
    
    async fn stop(&self) -> PlatformResult<()> {
        *self.running.lock().unwrap() = false;
        Ok(())
    }
    
    fn record_metric(&self, name: &str, value: MetricValue) -> PlatformResult<()> {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(value);
        Ok(())
    }
    
    fn get_metrics(&self) -> PlatformResult<MetricsSnapshot> {
        let metrics = self.metrics.lock().unwrap();
        let mut snapshot_metrics = HashMap::new();
        
        for (name, values) in metrics.iter() {
            if let Some(last_value) = values.last() {
                snapshot_metrics.insert(name.clone(), last_value.clone());
            }
        }
        
        Ok(MetricsSnapshot {
            timestamp: Instant::now(),
            metrics: snapshot_metrics,
            system_metrics: SystemMetrics::default(),
        })
    }
    
    fn get_metric_history(&self, name: &str) -> PlatformResult<Vec<MetricValue>> {
        let metrics = self.metrics.lock().unwrap();
        Ok(metrics.get(name).cloned().unwrap_or_default())
    }
    
    fn clear_metrics(&self) -> PlatformResult<()> {
        self.metrics.lock().unwrap().clear();
        Ok(())
    }
}

/// Default performance profiler
pub struct DefaultPerformanceProfiler {
    sessions: Arc<Mutex<HashMap<u64, ProfilingSessionData>>>,
    next_session_id: Arc<Mutex<u64>>,
}

struct ProfilingSessionData {
    config: ProfilingConfig,
    events: Vec<ProfilingEvent>,
    statistics: ProfilingStatistics,
}

impl DefaultPerformanceProfiler {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            next_session_id: Arc::new(Mutex::new(1)),
        }
    }
}

#[async_trait]
impl PerformanceProfiler for DefaultPerformanceProfiler {
    async fn start_profiling(&self, config: ProfilingConfig) -> PlatformResult<ProfilingSession> {
        let mut next_id = self.next_session_id.lock().unwrap();
        let session_id = *next_id;
        *next_id += 1;
        
        let session = ProfilingSession {
            id: session_id,
            start_time: Instant::now(),
        };
        
        let session_data = ProfilingSessionData {
            config,
            events: Vec::new(),
            statistics: ProfilingStatistics::default(),
        };
        
        self.sessions.lock().unwrap().insert(session_id, session_data);
        
        Ok(session)
    }
    
    async fn stop_profiling(&self, session: ProfilingSession) -> PlatformResult<ProfilingReport> {
        let mut sessions = self.sessions.lock().unwrap();
        let session_data = sessions.remove(&session.id)
            .ok_or_else(|| PlatformError::ConfigurationError("Session not found".to_string()))?;
        
        let duration = Instant::now() - session.start_time;
        
        // Calculate hotspots
        let mut function_samples: HashMap<String, u64> = HashMap::new();
        for event in &session_data.events {
            if let ProfilingEventType::FunctionCall(ref name) = event.event_type {
                *function_samples.entry(name.clone()).or_insert(0) += 1;
            }
        }
        
        let total_samples = function_samples.values().sum::<u64>() as f64;
        let hotspots: Vec<Hotspot> = function_samples.into_iter()
            .map(|(name, count)| Hotspot {
                function_name: name,
                sample_count: count,
                percentage: (count as f64 / total_samples) * 100.0,
                average_duration: Duration::from_secs(0),
            })
            .collect();
        
        Ok(ProfilingReport {
            session,
            duration,
            statistics: session_data.statistics,
            events: session_data.events,
            hotspots,
        })
    }
    
    fn record_event(&self, session: &ProfilingSession, event: ProfilingEvent) -> PlatformResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        let session_data = sessions.get_mut(&session.id)
            .ok_or_else(|| PlatformError::ConfigurationError("Session not found".to_string()))?;
        
        // Update statistics
        session_data.statistics.total_samples += 1;
        match &event.event_type {
            ProfilingEventType::Allocation(size) => {
                session_data.statistics.memory_allocations += 1;
                session_data.statistics.total_allocated_bytes += *size as u64;
            }
            ProfilingEventType::Deallocation(size) => {
                session_data.statistics.memory_deallocations += 1;
                session_data.statistics.total_deallocated_bytes += *size as u64;
            }
            ProfilingEventType::IORead(_) | ProfilingEventType::IOWrite(_) => {
                session_data.statistics.io_operations += 1;
            }
            ProfilingEventType::NetworkSend(_) | ProfilingEventType::NetworkReceive(_) => {
                session_data.statistics.network_operations += 1;
            }
            _ => {}
        }
        
        session_data.events.push(event);
        Ok(())
    }
    
    fn get_statistics(&self, session: &ProfilingSession) -> PlatformResult<ProfilingStatistics> {
        let sessions = self.sessions.lock().unwrap();
        let session_data = sessions.get(&session.id)
            .ok_or_else(|| PlatformError::ConfigurationError("Session not found".to_string()))?;
        Ok(session_data.statistics.clone())
    }
}

/// Default regression tester
pub struct DefaultRegressionTester {
    baselines: Arc<Mutex<HashMap<String, MetricsSnapshot>>>,
}

impl DefaultRegressionTester {
    pub fn new() -> Self {
        Self {
            baselines: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl RegressionTester for DefaultRegressionTester {
    async fn run_test(&self, test: RegressionTest) -> PlatformResult<RegressionResult> {
        let baseline = self.load_baseline(&test.baseline_name)?;
        
        // In production, this would run the actual test and collect metrics
        let current = MetricsSnapshot {
            timestamp: Instant::now(),
            metrics: HashMap::new(),
            system_metrics: SystemMetrics::default(),
        };
        
        let comparison = self.compare_with_baseline(&current, &baseline)?;
        
        let mut violations = Vec::new();
        
        if comparison.cpu_change_percent > test.thresholds.max_cpu_increase_percent {
            violations.push(RegressionViolation {
                metric_name: "cpu_usage".to_string(),
                threshold: test.thresholds.max_cpu_increase_percent,
                actual: comparison.cpu_change_percent,
                severity: ViolationSeverity::Error,
            });
        }
        
        if comparison.memory_change_percent > test.thresholds.max_memory_increase_percent {
            violations.push(RegressionViolation {
                metric_name: "memory_usage".to_string(),
                threshold: test.thresholds.max_memory_increase_percent,
                actual: comparison.memory_change_percent,
                severity: ViolationSeverity::Error,
            });
        }
        
        Ok(RegressionResult {
            test_name: test.name,
            passed: violations.is_empty(),
            comparison,
            violations,
        })
    }
    
    fn compare_with_baseline(&self, current: &MetricsSnapshot, baseline: &MetricsSnapshot) 
        -> PlatformResult<RegressionComparison> {
        let cpu_change = if baseline.system_metrics.cpu_usage_percent > 0.0 {
            ((current.system_metrics.cpu_usage_percent - baseline.system_metrics.cpu_usage_percent) 
                / baseline.system_metrics.cpu_usage_percent) * 100.0
        } else {
            0.0
        };
        
        let memory_change = if baseline.system_metrics.memory_usage_bytes > 0 {
            ((current.system_metrics.memory_usage_bytes as f64 - baseline.system_metrics.memory_usage_bytes as f64) 
                / baseline.system_metrics.memory_usage_bytes as f64) * 100.0
        } else {
            0.0
        };
        
        Ok(RegressionComparison {
            cpu_change_percent: cpu_change,
            memory_change_percent: memory_change,
            duration_change_percent: 0.0,
            throughput_change_percent: 0.0,
        })
    }
    
    fn save_baseline(&self, name: &str, metrics: &MetricsSnapshot) -> PlatformResult<()> {
        self.baselines.lock().unwrap().insert(name.to_string(), metrics.clone());
        Ok(())
    }
    
    fn load_baseline(&self, name: &str) -> PlatformResult<MetricsSnapshot> {
        self.baselines.lock().unwrap()
            .get(name)
            .cloned()
            .ok_or_else(|| PlatformError::ConfigurationError(format!("Baseline '{}' not found", name)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollectorFactory::create();
        assert!(collector.start().await.is_ok());
        
        collector.record_metric("test_counter", MetricValue::Counter(42)).unwrap();
        collector.record_metric("test_gauge", MetricValue::Gauge(3.14)).unwrap();
        
        let snapshot = collector.get_metrics().unwrap();
        assert_eq!(snapshot.metrics.len(), 2);
        
        assert!(collector.stop().await.is_ok());
    }

    #[tokio::test]
    async fn test_performance_profiler() {
        let profiler = PerformanceProfilerFactory::create();
        let config = ProfilingConfig::default();
        
        let session = profiler.start_profiling(config).await.unwrap();
        
        let event = ProfilingEvent {
            timestamp: Instant::now(),
            event_type: ProfilingEventType::FunctionCall("test_function".to_string()),
            duration: Some(Duration::from_millis(10)),
            metadata: HashMap::new(),
        };
        
        profiler.record_event(&session, event).unwrap();
        
        let report = profiler.stop_profiling(session).await.unwrap();
        assert_eq!(report.events.len(), 1);
    }

    #[tokio::test]
    async fn test_regression_tester() {
        let tester = RegressionTesterFactory::create();
        
        let baseline = MetricsSnapshot {
            timestamp: Instant::now(),
            metrics: HashMap::new(),
            system_metrics: SystemMetrics {
                cpu_usage_percent: 50.0,
                memory_usage_bytes: 1024 * 1024 * 100,
                ..Default::default()
            },
        };
        
        tester.save_baseline("test_baseline", &baseline).unwrap();
        
        let test = RegressionTest {
            name: "test_regression".to_string(),
            baseline_name: "test_baseline".to_string(),
            thresholds: RegressionThresholds::default(),
            iterations: 1,
        };
        
        let result = tester.run_test(test).await.unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_metric_value_types() {
        let counter = MetricValue::Counter(100);
        let gauge = MetricValue::Gauge(42.5);
        let duration = MetricValue::Duration(Duration::from_secs(5));
        
        match counter {
            MetricValue::Counter(v) => assert_eq!(v, 100),
            _ => panic!("Wrong metric type"),
        }
        
        match gauge {
            MetricValue::Gauge(v) => assert_eq!(v, 42.5),
            _ => panic!("Wrong metric type"),
        }
        
        match duration {
            MetricValue::Duration(d) => assert_eq!(d, Duration::from_secs(5)),
            _ => panic!("Wrong metric type"),
        }
    }
}
