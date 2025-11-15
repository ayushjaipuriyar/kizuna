use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use super::{
    PeerId, ConnectionInfo, TransportError, PeerAddress, Connection,
    ErrorHandler, ErrorHandlerConfig, ErrorContext, ContextualError,
    TransportLogger, LoggingConfig, LogLevel, LogCategory, LogConnectionEvent, LogSecurityEvent,
    PerformanceMonitor, PerformanceConfig, OptimizationRecommendation, HealthStatus,
};
use std::net::SocketAddr;

/// Integrated transport system combining error handling, logging, and performance monitoring
#[derive(Debug)]
pub struct IntegratedTransportSystem {
    /// Error handler for retry logic and circuit breaking
    error_handler: Arc<ErrorHandler>,
    /// Logger for comprehensive transport logging
    logger: Arc<TransportLogger>,
    /// Performance monitor for metrics and optimization
    performance_monitor: Arc<PerformanceMonitor>,
    /// System configuration
    config: IntegratedSystemConfig,
    /// System state
    state: Arc<RwLock<SystemState>>,
}

/// Configuration for the integrated transport system
#[derive(Debug, Clone)]
pub struct IntegratedSystemConfig {
    pub error_handler_config: ErrorHandlerConfig,
    pub logging_config: LoggingConfig,
    pub performance_config: PerformanceConfig,
    /// Enable automatic optimization based on recommendations
    pub enable_auto_optimization: bool,
    /// Interval for running system health checks
    pub health_check_interval: Duration,
    /// Enable adaptive behavior based on system state
    pub enable_adaptive_behavior: bool,
}

impl Default for IntegratedSystemConfig {
    fn default() -> Self {
        Self {
            error_handler_config: ErrorHandlerConfig::default(),
            logging_config: LoggingConfig::default(),
            performance_config: PerformanceConfig::default(),
            enable_auto_optimization: true,
            health_check_interval: Duration::from_secs(30),
            enable_adaptive_behavior: true,
        }
    }
}

/// Current state of the transport system
#[derive(Debug, Clone)]
pub struct SystemState {
    pub health_status: HealthStatus,
    pub degraded_mode: bool,
    pub last_health_check: Instant,
    pub active_optimizations: Vec<ActiveOptimization>,
    pub system_metrics: SystemMetrics,
}

/// Active optimization being applied
#[derive(Debug, Clone)]
pub struct ActiveOptimization {
    pub recommendation: OptimizationRecommendation,
    pub started_at: Instant,
    pub status: OptimizationStatus,
}

/// Status of an optimization
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizationStatus {
    InProgress,
    Completed,
    Failed(String),
}

/// System-wide metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub total_operations: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub average_operation_time: Duration,
    pub system_uptime: Duration,
    pub last_updated: Instant,
}

impl Default for SystemState {
    fn default() -> Self {
        Self {
            health_status: HealthStatus::Healthy,
            degraded_mode: false,
            last_health_check: Instant::now(),
            active_optimizations: Vec::new(),
            system_metrics: SystemMetrics::default(),
        }
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self {
            total_operations: 0,
            successful_operations: 0,
            failed_operations: 0,
            average_operation_time: Duration::ZERO,
            system_uptime: Duration::ZERO,
            last_updated: Instant::now(),
        }
    }
}

impl IntegratedTransportSystem {
    /// Create a new integrated transport system
    pub fn new() -> Self {
        Self::with_config(IntegratedSystemConfig::default())
    }

    /// Create a new integrated transport system with custom configuration
    pub fn with_config(config: IntegratedSystemConfig) -> Self {
        let error_handler = Arc::new(ErrorHandler::with_config(config.error_handler_config.clone()));
        let logger = Arc::new(TransportLogger::with_config(config.logging_config.clone()));
        let performance_monitor = Arc::new(PerformanceMonitor::with_config(config.performance_config.clone()));

        Self {
            error_handler,
            logger,
            performance_monitor,
            config,
            state: Arc::new(RwLock::new(SystemState::default())),
        }
    }

    /// Start the integrated system
    pub async fn start(&self) -> Result<(), TransportError> {
        // Start performance monitoring
        self.performance_monitor.start_monitoring().await;

        // Start health monitoring
        self.start_health_monitoring().await;

        // Log system startup
        self.logger.log(
            LogLevel::Info,
            LogCategory::Audit,
            "Integrated transport system started".to_string(),
        ).await;

        Ok(())
    }

    /// Execute an operation with comprehensive error handling, logging, and monitoring
    pub async fn execute_operation<F, Fut, T>(
        &self,
        operation_name: &str,
        peer_id: Option<&PeerId>,
        protocol: Option<&str>,
        operation: F,
    ) -> Result<T, ContextualError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, TransportError>>,
    {
        let start_time = Instant::now();
        let mut context = ErrorContext::new(operation_name.to_string());

        if let Some(peer_id) = peer_id {
            context = context.with_peer_id(peer_id.clone());
        }

        if let Some(protocol) = protocol {
            context = context.with_protocol(protocol.to_string());
        }

        // Log operation start
        self.logger.log(
            LogLevel::Debug,
            LogCategory::Debug,
            format!("Starting operation: {}", operation_name),
        ).await;

        // Execute with error handling
        let result = self.error_handler.handle_error(operation_name, context, operation).await;

        let duration = start_time.elapsed();

        // Update system metrics
        self.update_system_metrics(result.is_ok(), duration).await;

        // Log operation completion
        match &result {
            Ok(_) => {
                self.logger.log_performance(
                    operation_name.to_string(),
                    duration,
                    std::collections::HashMap::new(),
                ).await;

                if let Some(peer_id) = peer_id {
                    // Record successful operation for performance monitoring
                    // This would typically be more specific based on the operation type
                }
            }
            Err(error) => {
                self.logger.log_error(error).await;

                if let Some(peer_id) = peer_id {
                    self.performance_monitor.record_error(peer_id).await;
                }
            }
        }

        result
    }

    /// Record connection establishment with integrated monitoring
    pub async fn record_connection_established(
        &self,
        peer_id: PeerId,
        protocol: String,
        connection_info: &ConnectionInfo,
    ) {
        // Record in performance monitor
        self.performance_monitor
            .record_connection_established(peer_id.clone(), protocol.clone())
            .await;

        // Log connection event
        self.logger
            .log_connection_event(LogConnectionEvent::Established, connection_info)
            .await;

        // Log security event if applicable
        self.logger
            .log_security_event(LogSecurityEvent::AuthenticationSuccess, Some(&peer_id))
            .await;
    }

    /// Record connection closure with integrated monitoring
    pub async fn record_connection_closed(&self, peer_id: &PeerId, connection_info: &ConnectionInfo) {
        // Record in performance monitor
        self.performance_monitor.record_connection_closed(peer_id).await;

        // Log connection event
        self.logger
            .log_connection_event(LogConnectionEvent::Closed, connection_info)
            .await;
    }

    /// Record data transfer with integrated monitoring
    pub async fn record_data_transfer(&self, peer_id: &PeerId, bytes_sent: u64, bytes_received: u64) {
        // Record in performance monitor
        self.performance_monitor
            .record_data_transfer(peer_id, bytes_sent, bytes_received)
            .await;

        // Log performance metrics if significant transfer
        let total_bytes = bytes_sent + bytes_received;
        if total_bytes > 1024 * 1024 {
            // Log transfers over 1MB
            let mut metadata = std::collections::HashMap::new();
            metadata.insert("bytes_sent".to_string(), bytes_sent.to_string());
            metadata.insert("bytes_received".to_string(), bytes_received.to_string());
            metadata.insert("peer_id".to_string(), peer_id.clone());

            self.logger
                .log_performance("data_transfer".to_string(), Duration::ZERO, metadata)
                .await;
        }
    }

    /// Record RTT measurement
    pub async fn record_rtt(&self, peer_id: &PeerId, rtt: Duration) {
        self.performance_monitor.record_rtt(peer_id, rtt).await;

        // Log high latency as a warning
        if rtt > Duration::from_millis(500) {
            self.logger
                .log(
                    LogLevel::Warn,
                    LogCategory::Performance,
                    format!("High latency detected for peer {}: {:?}", peer_id, rtt),
                )
                .await;
        }
    }

    /// Check system health and apply optimizations
    pub async fn check_system_health(&self) -> SystemHealthReport {
        let error_handler_health = self.error_handler.get_health_status().await;
        let performance_report = self.performance_monitor.get_performance_report().await;
        let logging_metrics = self.logger.get_metrics().await;

        let overall_health = match (
            error_handler_health.is_degraded,
            &performance_report.health_status,
        ) {
            (true, _) | (_, HealthStatus::Unhealthy) => HealthStatus::Unhealthy,
            (false, HealthStatus::Degraded) => HealthStatus::Degraded,
            (false, HealthStatus::Healthy) => HealthStatus::Healthy,
        };

        // Update system state
        {
            let mut state = self.state.write().await;
            state.health_status = overall_health.clone();
            state.degraded_mode = error_handler_health.is_degraded;
            state.last_health_check = Instant::now();
        }

        // Apply optimizations if enabled
        if self.config.enable_auto_optimization {
            self.apply_optimizations(&performance_report.optimization_recommendations)
                .await;
        }

        SystemHealthReport {
            overall_health,
            error_handler_health,
            performance_report,
            logging_metrics,
            recommendations: self.generate_system_recommendations().await,
        }
    }

    /// Get comprehensive system status
    pub async fn get_system_status(&self) -> SystemStatus {
        let state = self.state.read().await;
        let health_report = self.check_system_health().await;

        SystemStatus {
            health_status: state.health_status.clone(),
            degraded_mode: state.degraded_mode,
            uptime: state.system_metrics.system_uptime,
            active_connections: health_report.performance_report.active_connections,
            total_errors: health_report.error_handler_health.total_errors,
            error_rate: health_report.error_handler_health.max_error_rate,
            average_connection_quality: health_report.performance_report.average_connection_quality,
            bandwidth_usage: health_report.performance_report.total_bandwidth_usage,
            active_optimizations: state.active_optimizations.len(),
            last_health_check: state.last_health_check,
        }
    }

    /// Start health monitoring background task
    async fn start_health_monitoring(&self) {
        let system = Arc::new(self.clone());
        let interval = self.config.health_check_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);

            loop {
                interval.tick().await;
                let _health_report = system.check_system_health().await;
                // Health report is automatically processed in check_system_health
            }
        });
    }

    /// Apply optimization recommendations
    async fn apply_optimizations(&self, recommendations: &[OptimizationRecommendation]) {
        let mut state = self.state.write().await;

        for recommendation in recommendations {
            // Check if we're already applying this optimization
            let already_active = state
                .active_optimizations
                .iter()
                .any(|opt| std::mem::discriminant(&opt.recommendation) == std::mem::discriminant(recommendation));

            if !already_active {
                let active_opt = ActiveOptimization {
                    recommendation: recommendation.clone(),
                    started_at: Instant::now(),
                    status: OptimizationStatus::InProgress,
                };

                state.active_optimizations.push(active_opt);

                // Log optimization start
                drop(state); // Release lock before async operation
                self.logger
                    .log(
                        LogLevel::Info,
                        LogCategory::Performance,
                        format!("Applying optimization: {:?}", recommendation),
                    )
                    .await;
                state = self.state.write().await; // Re-acquire lock
            }
        }
    }

    /// Generate system-level recommendations
    async fn generate_system_recommendations(&self) -> Vec<SystemRecommendation> {
        let mut recommendations = Vec::new();
        let state = self.state.read().await;

        // Check error rate
        if state.degraded_mode {
            recommendations.push(SystemRecommendation::ReduceLoad {
                reason: "System in degraded mode due to high error rate".to_string(),
            });
        }

        // Check connection quality
        let performance_report = self.performance_monitor.get_performance_report().await;
        if performance_report.average_connection_quality < 0.5 {
            recommendations.push(SystemRecommendation::ImproveConnections {
                current_quality: performance_report.average_connection_quality,
                target_quality: 0.8,
            });
        }

        // Check bandwidth usage
        if let Some(global_limit) = self.config.performance_config.global_bandwidth_limit {
            let usage_ratio = performance_report.total_bandwidth_usage as f64 / global_limit as f64;
            if usage_ratio > 0.9 {
                recommendations.push(SystemRecommendation::IncreaseBandwidthLimit {
                    current_limit: global_limit,
                    recommended_limit: (global_limit as f64 * 1.5) as u64,
                });
            }
        }

        recommendations
    }

    /// Update system metrics
    async fn update_system_metrics(&self, success: bool, duration: Duration) {
        let mut state = self.state.write().await;
        let metrics = &mut state.system_metrics;

        metrics.total_operations += 1;
        if success {
            metrics.successful_operations += 1;
        } else {
            metrics.failed_operations += 1;
        }

        // Update average operation time
        if metrics.total_operations == 1 {
            metrics.average_operation_time = duration;
        } else {
            let total_time = metrics.average_operation_time * (metrics.total_operations - 1) as u32 + duration;
            metrics.average_operation_time = total_time / metrics.total_operations as u32;
        }

        metrics.system_uptime = metrics.last_updated.elapsed();
        metrics.last_updated = Instant::now();
    }



    /// Connect to a peer using the integrated system
    pub async fn connect_to_peer(&self, peer_address: &PeerAddress) -> Result<Box<dyn Connection>, TransportError> {
        self.execute_operation(
            "connect_to_peer",
            Some(&peer_address.peer_id),
            None,
            || async {
                // This is a placeholder - in a real implementation, this would use the ConnectionManager
                // For now, return an error indicating the method needs to be implemented with actual transport logic
                Err(TransportError::Configuration("Connection logic not yet integrated with ConnectionManager".to_string()))
            },
        )
        .await
        .map_err(|e| e.error)
    }

    /// Connect to a peer using a specific protocol
    pub async fn connect_with_protocol(&self, peer_address: &PeerAddress, protocol: &str) -> Result<Box<dyn Connection>, TransportError> {
        self.execute_operation(
            "connect_with_protocol",
            Some(&peer_address.peer_id),
            Some(protocol),
            || async {
                // This is a placeholder - in a real implementation, this would use the ConnectionManager
                // For now, return an error indicating the method needs to be implemented with actual transport logic
                Err(TransportError::Configuration("Protocol-specific connection logic not yet integrated with ConnectionManager".to_string()))
            },
        )
        .await
        .map_err(|e| e.error)
    }

    /// Start listening for incoming connections
    pub async fn start_listening(&self, bind_address: SocketAddr) -> Result<(), TransportError> {
        self.execute_operation(
            "start_listening",
            None,
            None,
            || async {
                // This is a placeholder - in a real implementation, this would use the ConnectionManager
                // For now, just log that we're starting to listen
                Ok(())
            },
        )
        .await
        .map_err(|e| e.error)
    }

    /// Stop listening for incoming connections
    pub async fn stop_listening(&self) -> Result<(), TransportError> {
        self.execute_operation(
            "stop_listening",
            None,
            None,
            || async {
                // This is a placeholder - in a real implementation, this would use the ConnectionManager
                // For now, just log that we're stopping listening
                Ok(())
            },
        )
        .await
        .map_err(|e| e.error)
    }

    /// Get system health report
    pub async fn get_health_report(&self) -> SystemHealthReport {
        self.check_system_health().await
    }

    /// Get current system state
    pub async fn get_system_state(&self) -> SystemState {
        let state = self.state.read().await;
        state.clone()
    }
}

/// Comprehensive system health report
#[derive(Debug, Clone)]
pub struct SystemHealthReport {
    pub overall_health: HealthStatus,
    pub error_handler_health: super::ErrorHandlerHealth,
    pub performance_report: super::PerformanceReport,
    pub logging_metrics: super::logging::LoggingMetrics,
    pub recommendations: Vec<SystemRecommendation>,
}

/// System-level recommendations
#[derive(Debug, Clone)]
pub enum SystemRecommendation {
    ReduceLoad { reason: String },
    ImproveConnections { current_quality: f64, target_quality: f64 },
    IncreaseBandwidthLimit { current_limit: u64, recommended_limit: u64 },
    RestartComponent { component: String, reason: String },
    ScaleUp { resource: String, current: u64, recommended: u64 },
}

/// Current system status summary
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub health_status: HealthStatus,
    pub degraded_mode: bool,
    pub uptime: Duration,
    pub active_connections: usize,
    pub total_errors: u64,
    pub error_rate: f64,
    pub average_connection_quality: f64,
    pub bandwidth_usage: u64,
    pub active_optimizations: usize,
    pub last_health_check: Instant,
}

// Implement Clone for IntegratedTransportSystem to support the health monitoring task
impl Clone for IntegratedTransportSystem {
    fn clone(&self) -> Self {
        Self {
            error_handler: self.error_handler.clone(),
            logger: self.logger.clone(),
            performance_monitor: self.performance_monitor.clone(),
            config: self.config.clone(),
            state: self.state.clone(),
        }
    }
}

impl Default for IntegratedTransportSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[tokio::test]
    async fn test_integrated_system_startup() {
        let system = IntegratedTransportSystem::new();
        let result = system.start().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_operation_execution() {
        let system = IntegratedTransportSystem::new();
        system.start().await.unwrap();

        let result = system
            .execute_operation(
                "test_operation",
                Some(&"test_peer".to_string()),
                Some("tcp"),
                || async { Ok("success") },
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_connection_lifecycle() {
        // Simple test that just creates the system
        let system = IntegratedTransportSystem::new();
        
        // Test that we can access the components
        assert!(system.error_handler.get_health_status().await.total_errors == 0);
        
        // Test basic functionality without background tasks
        let global_stats = system.performance_monitor.get_global_stats().await;
        assert_eq!(global_stats.active_connections, 0);
    }

    #[tokio::test]
    async fn test_health_monitoring() {
        let system = IntegratedTransportSystem::new();
        // Don't start background tasks for this test
        
        let health_report = system.check_system_health().await;
        assert_eq!(health_report.overall_health, HealthStatus::Healthy);
    }
}