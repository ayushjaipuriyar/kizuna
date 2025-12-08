/// Diagnostic and monitoring tools for the Developer API
use super::error::KizunaError;
use super::logging::{LogLevel, LogRecord, Logger};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// System health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// System is healthy
    Healthy,
    /// System is degraded but operational
    Degraded,
    /// System is unhealthy
    Unhealthy,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Degraded => write!(f, "Degraded"),
            HealthStatus::Unhealthy => write!(f, "Unhealthy"),
        }
    }
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// Component name
    pub component: String,
    
    /// Health status
    pub status: HealthStatus,
    
    /// Status message
    pub message: String,
    
    /// Timestamp of the check
    pub timestamp: SystemTime,
    
    /// Additional details
    pub details: HashMap<String, String>,
}

impl HealthCheck {
    /// Creates a new health check
    pub fn new<S: Into<String>>(component: S, status: HealthStatus, message: S) -> Self {
        Self {
            component: component.into(),
            status,
            message: message.into(),
            timestamp: SystemTime::now(),
            details: HashMap::new(),
        }
    }
    
    /// Adds a detail to the health check
    pub fn with_detail<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.details.insert(key.into(), value.into());
        self
    }
}

/// Health monitor for tracking system health
pub struct HealthMonitor {
    checks: Arc<Mutex<Vec<HealthCheck>>>,
    logger: Option<Arc<dyn Logger>>,
}

impl HealthMonitor {
    /// Creates a new health monitor
    pub fn new() -> Self {
        Self {
            checks: Arc::new(Mutex::new(Vec::new())),
            logger: None,
        }
    }
    
    /// Sets the logger
    pub fn with_logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }
    
    /// Records a health check
    pub fn record_check(&self, check: HealthCheck) {
        if let Some(logger) = &self.logger {
            let level = match check.status {
                HealthStatus::Healthy => LogLevel::Info,
                HealthStatus::Degraded => LogLevel::Warn,
                HealthStatus::Unhealthy => LogLevel::Error,
            };
            
            let record = LogRecord::new(
                level,
                format!("Health check for {}: {} - {}", check.component, check.status, check.message),
                "health_monitor".to_string(),
            );
            logger.log(&record);
        }
        
        self.checks.lock().unwrap().push(check);
    }
    
    /// Gets all health checks
    pub fn get_checks(&self) -> Vec<HealthCheck> {
        self.checks.lock().unwrap().clone()
    }
    
    /// Gets the overall health status
    pub fn overall_status(&self) -> HealthStatus {
        let checks = self.checks.lock().unwrap();
        
        if checks.is_empty() {
            return HealthStatus::Healthy;
        }
        
        let has_unhealthy = checks.iter().any(|c| c.status == HealthStatus::Unhealthy);
        let has_degraded = checks.iter().any(|c| c.status == HealthStatus::Degraded);
        
        if has_unhealthy {
            HealthStatus::Unhealthy
        } else if has_degraded {
            HealthStatus::Degraded
        } else {
            HealthStatus::Healthy
        }
    }
    
    /// Clears all health checks
    pub fn clear(&self) {
        self.checks.lock().unwrap().clear();
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Operation name
    pub operation: String,
    
    /// Number of executions
    pub count: u64,
    
    /// Total duration
    pub total_duration: Duration,
    
    /// Minimum duration
    pub min_duration: Duration,
    
    /// Maximum duration
    pub max_duration: Duration,
    
    /// Average duration
    pub avg_duration: Duration,
    
    /// Success count
    pub success_count: u64,
    
    /// Failure count
    pub failure_count: u64,
}

impl PerformanceMetrics {
    /// Creates new performance metrics
    pub fn new<S: Into<String>>(operation: S) -> Self {
        Self {
            operation: operation.into(),
            count: 0,
            total_duration: Duration::from_secs(0),
            min_duration: Duration::from_secs(u64::MAX),
            max_duration: Duration::from_secs(0),
            avg_duration: Duration::from_secs(0),
            success_count: 0,
            failure_count: 0,
        }
    }
    
    /// Records a successful operation
    pub fn record_success(&mut self, duration: Duration) {
        self.count += 1;
        self.success_count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        self.avg_duration = self.total_duration / self.count as u32;
    }
    
    /// Records a failed operation
    pub fn record_failure(&mut self, duration: Duration) {
        self.count += 1;
        self.failure_count += 1;
        self.total_duration += duration;
        self.min_duration = self.min_duration.min(duration);
        self.max_duration = self.max_duration.max(duration);
        self.avg_duration = self.total_duration / self.count as u32;
    }
    
    /// Returns the success rate
    pub fn success_rate(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.success_count as f64 / self.count as f64
        }
    }
}

/// Performance monitor for tracking operation performance
pub struct PerformanceMonitor {
    metrics: Arc<Mutex<HashMap<String, PerformanceMetrics>>>,
    logger: Option<Arc<dyn Logger>>,
}

impl PerformanceMonitor {
    /// Creates a new performance monitor
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
            logger: None,
        }
    }
    
    /// Sets the logger
    pub fn with_logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }
    
    /// Starts timing an operation
    pub fn start_operation<S: Into<String>>(&self, operation: S) -> OperationTimer {
        OperationTimer::new(operation.into(), self.metrics.clone(), self.logger.clone())
    }
    
    /// Gets metrics for an operation
    pub fn get_metrics(&self, operation: &str) -> Option<PerformanceMetrics> {
        self.metrics.lock().unwrap().get(operation).cloned()
    }
    
    /// Gets all metrics
    pub fn get_all_metrics(&self) -> HashMap<String, PerformanceMetrics> {
        self.metrics.lock().unwrap().clone()
    }
    
    /// Clears all metrics
    pub fn clear(&self) {
        self.metrics.lock().unwrap().clear();
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Timer for measuring operation duration
pub struct OperationTimer {
    operation: String,
    start_time: Instant,
    metrics: Arc<Mutex<HashMap<String, PerformanceMetrics>>>,
    logger: Option<Arc<dyn Logger>>,
}

impl OperationTimer {
    fn new(
        operation: String,
        metrics: Arc<Mutex<HashMap<String, PerformanceMetrics>>>,
        logger: Option<Arc<dyn Logger>>,
    ) -> Self {
        Self {
            operation,
            start_time: Instant::now(),
            metrics,
            logger,
        }
    }
    
    /// Completes the operation successfully
    pub fn complete_success(self) {
        let duration = self.start_time.elapsed();
        
        let mut metrics = self.metrics.lock().unwrap();
        let entry = metrics
            .entry(self.operation.clone())
            .or_insert_with(|| PerformanceMetrics::new(self.operation.clone()));
        entry.record_success(duration);
        
        if let Some(logger) = &self.logger {
            let record = LogRecord::new(
                LogLevel::Debug,
                format!("Operation '{}' completed in {:?}", self.operation, duration),
                "performance_monitor".to_string(),
            );
            logger.log(&record);
        }
    }
    
    /// Completes the operation with failure
    pub fn complete_failure(self) {
        let duration = self.start_time.elapsed();
        
        let mut metrics = self.metrics.lock().unwrap();
        let entry = metrics
            .entry(self.operation.clone())
            .or_insert_with(|| PerformanceMetrics::new(self.operation.clone()));
        entry.record_failure(duration);
        
        if let Some(logger) = &self.logger {
            let record = LogRecord::new(
                LogLevel::Warn,
                format!("Operation '{}' failed after {:?}", self.operation, duration),
                "performance_monitor".to_string(),
            );
            logger.log(&record);
        }
    }
}

/// Diagnostic tools for troubleshooting
pub struct DiagnosticTools {
    health_monitor: HealthMonitor,
    performance_monitor: PerformanceMonitor,
    logger: Option<Arc<dyn Logger>>,
}

impl DiagnosticTools {
    /// Creates new diagnostic tools
    pub fn new() -> Self {
        Self {
            health_monitor: HealthMonitor::new(),
            performance_monitor: PerformanceMonitor::new(),
            logger: None,
        }
    }
    
    /// Sets the logger
    pub fn with_logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger.clone());
        self.health_monitor = self.health_monitor.with_logger(logger.clone());
        self.performance_monitor = self.performance_monitor.with_logger(logger);
        self
    }
    
    /// Gets the health monitor
    pub fn health_monitor(&self) -> &HealthMonitor {
        &self.health_monitor
    }
    
    /// Gets the performance monitor
    pub fn performance_monitor(&self) -> &PerformanceMonitor {
        &self.performance_monitor
    }
    
    /// Generates a diagnostic report
    pub fn generate_report(&self) -> DiagnosticReport {
        DiagnosticReport {
            timestamp: SystemTime::now(),
            overall_health: self.health_monitor.overall_status(),
            health_checks: self.health_monitor.get_checks(),
            performance_metrics: self.performance_monitor.get_all_metrics(),
        }
    }
    
    /// Runs all health checks
    pub fn run_health_checks(&self) {
        // Check API health
        self.health_monitor.record_check(
            HealthCheck::new("api", HealthStatus::Healthy, "API is operational")
        );
        
        // Check performance
        let metrics = self.performance_monitor.get_all_metrics();
        let slow_operations: Vec<_> = metrics
            .iter()
            .filter(|(_, m)| m.avg_duration > Duration::from_secs(1))
            .collect();
        
        if !slow_operations.is_empty() {
            let mut check = HealthCheck::new(
                "performance",
                HealthStatus::Degraded,
                &format!("{} operations are slow", slow_operations.len()),
            );
            
            for (op, metrics) in slow_operations {
                check = check.with_detail(op, format!("avg: {:?}", metrics.avg_duration));
            }
            
            self.health_monitor.record_check(check);
        } else {
            self.health_monitor.record_check(
                HealthCheck::new("performance", HealthStatus::Healthy, "Performance is good")
            );
        }
    }
}

impl Default for DiagnosticTools {
    fn default() -> Self {
        Self::new()
    }
}

/// Diagnostic report
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    /// Report timestamp
    pub timestamp: SystemTime,
    
    /// Overall health status
    pub overall_health: HealthStatus,
    
    /// Health checks
    pub health_checks: Vec<HealthCheck>,
    
    /// Performance metrics
    pub performance_metrics: HashMap<String, PerformanceMetrics>,
}

impl DiagnosticReport {
    /// Formats the report as a string
    pub fn format(&self) -> String {
        let mut output = String::new();
        
        output.push_str("=== Diagnostic Report ===\n\n");
        output.push_str(&format!("Overall Health: {}\n\n", self.overall_health));
        
        output.push_str("Health Checks:\n");
        for check in &self.health_checks {
            output.push_str(&format!(
                "  - {}: {} - {}\n",
                check.component, check.status, check.message
            ));
            for (key, value) in &check.details {
                output.push_str(&format!("    {}: {}\n", key, value));
            }
        }
        
        output.push_str("\nPerformance Metrics:\n");
        for (operation, metrics) in &self.performance_metrics {
            output.push_str(&format!(
                "  - {}: {} calls, avg {:?}, success rate {:.2}%\n",
                operation,
                metrics.count,
                metrics.avg_duration,
                metrics.success_rate() * 100.0
            ));
        }
        
        output
    }
}

/// Error resolution guide
pub struct ErrorResolutionGuide;

impl ErrorResolutionGuide {
    /// Gets resolution steps for an error
    pub fn get_resolution_steps(error: &KizunaError) -> Vec<String> {
        let mut steps = Vec::new();
        
        match error.kind() {
            super::error::ErrorKind::DiscoveryError { .. } => {
                steps.push("Check your network connection".to_string());
                steps.push("Verify firewall settings allow peer discovery".to_string());
                steps.push("Ensure discovery service is running".to_string());
                steps.push("Try restarting the discovery process".to_string());
            }
            super::error::ErrorKind::ConnectionError { .. } => {
                steps.push("Verify the peer is online and accessible".to_string());
                steps.push("Check network connectivity to the peer".to_string());
                steps.push("Ensure firewall allows connections".to_string());
                steps.push("Try reconnecting after a short delay".to_string());
            }
            super::error::ErrorKind::TransferError { .. } => {
                steps.push("Check available disk space".to_string());
                steps.push("Verify network stability".to_string());
                steps.push("Ensure file permissions are correct".to_string());
                steps.push("Try resuming the transfer if supported".to_string());
            }
            super::error::ErrorKind::PluginError { .. } => {
                steps.push("Disable and re-enable the plugin".to_string());
                steps.push("Check plugin configuration".to_string());
                steps.push("Update the plugin to the latest version".to_string());
                steps.push("Check plugin logs for more details".to_string());
            }
            super::error::ErrorKind::ConfigError { .. } => {
                steps.push("Review your configuration file for errors".to_string());
                steps.push("Check configuration syntax".to_string());
                steps.push("Verify all required fields are present".to_string());
                steps.push("Reset to default configuration if needed".to_string());
            }
            super::error::ErrorKind::TimeoutError { .. } => {
                steps.push("Check network latency".to_string());
                steps.push("Increase timeout value if appropriate".to_string());
                steps.push("Verify the remote service is responsive".to_string());
                steps.push("Try the operation again".to_string());
            }
            super::error::ErrorKind::PermissionDenied { .. } => {
                steps.push("Check your access permissions".to_string());
                steps.push("Verify you have the required credentials".to_string());
                steps.push("Contact an administrator if needed".to_string());
            }
            _ => {
                steps.push("Check the error message for details".to_string());
                steps.push("Review the API documentation".to_string());
                steps.push("Contact support if the problem persists".to_string());
            }
        }
        
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_health_monitor() {
        let monitor = HealthMonitor::new();
        
        monitor.record_check(HealthCheck::new("test", HealthStatus::Healthy, "OK"));
        assert_eq!(monitor.overall_status(), HealthStatus::Healthy);
        
        monitor.record_check(HealthCheck::new("test2", HealthStatus::Degraded, "Slow"));
        assert_eq!(monitor.overall_status(), HealthStatus::Degraded);
        
        monitor.record_check(HealthCheck::new("test3", HealthStatus::Unhealthy, "Failed"));
        assert_eq!(monitor.overall_status(), HealthStatus::Unhealthy);
    }
    
    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new("test_operation");
        
        metrics.record_success(Duration::from_millis(100));
        metrics.record_success(Duration::from_millis(200));
        metrics.record_failure(Duration::from_millis(150));
        
        assert_eq!(metrics.count, 3);
        assert_eq!(metrics.success_count, 2);
        assert_eq!(metrics.failure_count, 1);
        assert_eq!(metrics.success_rate(), 2.0 / 3.0);
    }
    
    #[test]
    fn test_performance_monitor() {
        let monitor = PerformanceMonitor::new();
        
        let timer = monitor.start_operation("test_op");
        std::thread::sleep(Duration::from_millis(10));
        timer.complete_success();
        
        let metrics = monitor.get_metrics("test_op").unwrap();
        assert_eq!(metrics.count, 1);
        assert_eq!(metrics.success_count, 1);
    }
}
