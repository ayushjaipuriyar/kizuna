use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::RwLock;
use tokio::time::sleep;

use super::error::{TransportError, ErrorSeverity, RetryStrategy, ErrorCategory, ErrorContext, ContextualError};

/// Comprehensive error handler with retry logic and recovery strategies
#[derive(Debug)]
pub struct ErrorHandler {
    /// Error statistics by category
    error_stats: Arc<RwLock<HashMap<ErrorCategory, ErrorStats>>>,
    /// Circuit breaker states by operation
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    /// Global error handling configuration
    config: ErrorHandlerConfig,
}

/// Configuration for error handling behavior
#[derive(Debug, Clone)]
pub struct ErrorHandlerConfig {
    /// Maximum number of retry attempts across all strategies
    pub global_max_retries: u32,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,
    /// Circuit breaker timeout before attempting reset
    pub circuit_breaker_timeout: Duration,
    /// Enable detailed error logging
    pub detailed_logging: bool,
    /// Error rate threshold for degraded mode
    pub degraded_mode_threshold: f64,
    /// Time window for error rate calculation
    pub error_rate_window: Duration,
}

impl Default for ErrorHandlerConfig {
    fn default() -> Self {
        Self {
            global_max_retries: 10,
            circuit_breaker_threshold: 5,
            circuit_breaker_timeout: Duration::from_secs(60),
            detailed_logging: true,
            degraded_mode_threshold: 0.5, // 50% error rate
            error_rate_window: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Statistics for a specific error category
#[derive(Debug, Clone)]
pub struct ErrorStats {
    pub total_errors: u64,
    pub errors_by_severity: HashMap<ErrorSeverity, u64>,
    pub recent_errors: Vec<ErrorRecord>,
    pub last_error_time: Option<SystemTime>,
    pub error_rate: f64, // Errors per second over recent window
}

impl Default for ErrorStats {
    fn default() -> Self {
        Self {
            total_errors: 0,
            errors_by_severity: HashMap::new(),
            recent_errors: Vec::new(),
            last_error_time: None,
            error_rate: 0.0,
        }
    }
}

/// Record of a specific error occurrence
#[derive(Debug, Clone)]
pub struct ErrorRecord {
    pub timestamp: SystemTime,
    pub error: TransportError,
    pub context: ErrorContext,
    pub resolved: bool,
    pub resolution_time: Option<Duration>,
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub last_failure_time: Option<Instant>,
    pub last_success_time: Option<Instant>,
    pub threshold: u32,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,   // Normal operation
    Open,     // Failing fast
    HalfOpen, // Testing if service recovered
}

impl ErrorHandler {
    /// Create a new error handler with default configuration
    pub fn new() -> Self {
        Self::with_config(ErrorHandlerConfig::default())
    }

    /// Create a new error handler with custom configuration
    pub fn with_config(config: ErrorHandlerConfig) -> Self {
        Self {
            error_stats: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Handle an error with automatic retry logic
    pub async fn handle_error<F, Fut, T>(
        &self,
        operation: &str,
        context: ErrorContext,
        operation_fn: F,
    ) -> Result<T, ContextualError>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, TransportError>>,
    {
        let mut attempt = 0;
        let mut last_error = None;

        // Check circuit breaker
        if self.is_circuit_open(operation).await {
            let error = TransportError::ResourceLimitExceeded {
                resource: format!("Circuit breaker open for operation: {}", operation),
            };
            let contextual_error = ContextualError::new(error, context);
            self.record_error(&contextual_error).await;
            return Err(contextual_error);
        }

        loop {
            let result = operation_fn().await;

            match result {
                Ok(value) => {
                    // Success - record and reset circuit breaker
                    self.record_success(operation).await;
                    if attempt > 0 && self.config.detailed_logging {
                        self.log_recovery(&context, attempt).await;
                    }
                    return Ok(value);
                }
                Err(error) => {
                    let contextual_error = ContextualError::new(
                        error.clone(),
                        context.clone().with_attempt(attempt),
                    );

                    // Record the error
                    self.record_error(&contextual_error).await;

                    // Check if we should retry
                    let retry_strategy = error.retry_strategy();
                    if let Some(delay) = retry_strategy.delay_for_attempt(attempt) {
                        if attempt < self.config.global_max_retries {
                            if self.config.detailed_logging {
                                self.log_retry_attempt(&contextual_error, delay).await;
                            }

                            sleep(delay).await;
                            attempt += 1;
                            last_error = Some(contextual_error);
                            continue;
                        }
                    }

                    // No more retries - update circuit breaker and return error
                    self.record_failure(operation).await;
                    return Err(contextual_error);
                }
            }
        }
    }

    /// Record an error occurrence
    pub async fn record_error(&self, error: &ContextualError) {
        let category = error.error.category();
        let severity = error.error.severity();

        let mut stats = self.error_stats.write().await;
        let error_stats = stats.entry(category).or_insert_with(ErrorStats::default);

        error_stats.total_errors += 1;
        *error_stats.errors_by_severity.entry(severity).or_insert(0) += 1;
        error_stats.last_error_time = Some(error.context.timestamp);

        // Add to recent errors (keep last 100)
        error_stats.recent_errors.push(ErrorRecord {
            timestamp: error.context.timestamp,
            error: error.error.clone(),
            context: error.context.clone(),
            resolved: false,
            resolution_time: None,
        });

        if error_stats.recent_errors.len() > 100 {
            error_stats.recent_errors.remove(0);
        }

        // Update error rate
        self.update_error_rate(error_stats).await;

        if self.config.detailed_logging {
            self.log_error(error).await;
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self, operation: &str) {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(operation) {
            breaker.last_success_time = Some(Instant::now());
            
            match breaker.state {
                CircuitBreakerState::HalfOpen => {
                    breaker.state = CircuitBreakerState::Closed;
                    breaker.failure_count = 0;
                    if self.config.detailed_logging {
                        println!("[INFO] Circuit breaker closed for operation: {}", operation);
                    }
                }
                CircuitBreakerState::Closed => {
                    breaker.failure_count = 0;
                }
                _ => {}
            }
        }
    }

    /// Record a failed operation for circuit breaker
    pub async fn record_failure(&self, operation: &str) {
        let mut breakers = self.circuit_breakers.write().await;
        let breaker = breakers.entry(operation.to_string()).or_insert_with(|| {
            CircuitBreaker {
                state: CircuitBreakerState::Closed,
                failure_count: 0,
                last_failure_time: None,
                last_success_time: None,
                threshold: self.config.circuit_breaker_threshold,
                timeout: self.config.circuit_breaker_timeout,
            }
        });

        breaker.failure_count += 1;
        breaker.last_failure_time = Some(Instant::now());

        if breaker.failure_count >= breaker.threshold {
            breaker.state = CircuitBreakerState::Open;
            if self.config.detailed_logging {
                println!("[WARN] Circuit breaker opened for operation: {} (failures: {})", 
                    operation, breaker.failure_count);
            }
        }
    }

    /// Check if circuit breaker is open for an operation
    pub async fn is_circuit_open(&self, operation: &str) -> bool {
        let mut breakers = self.circuit_breakers.write().await;
        if let Some(breaker) = breakers.get_mut(operation) {
            match breaker.state {
                CircuitBreakerState::Open => {
                    // Check if timeout has passed
                    if let Some(last_failure) = breaker.last_failure_time {
                        if last_failure.elapsed() >= breaker.timeout {
                            breaker.state = CircuitBreakerState::HalfOpen;
                            if self.config.detailed_logging {
                                println!("[INFO] Circuit breaker half-open for operation: {}", operation);
                            }
                            return false;
                        }
                    }
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// Get error statistics for a specific category
    pub async fn get_error_stats(&self, category: ErrorCategory) -> Option<ErrorStats> {
        let stats = self.error_stats.read().await;
        stats.get(&category).cloned()
    }

    /// Get all error statistics
    pub async fn get_all_error_stats(&self) -> HashMap<ErrorCategory, ErrorStats> {
        self.error_stats.read().await.clone()
    }

    /// Check if system is in degraded mode based on error rates
    pub async fn is_degraded_mode(&self) -> bool {
        let stats = self.error_stats.read().await;
        for error_stats in stats.values() {
            if error_stats.error_rate > self.config.degraded_mode_threshold {
                return true;
            }
        }
        false
    }

    /// Get health status of the error handler
    pub async fn get_health_status(&self) -> ErrorHandlerHealth {
        let stats = self.get_all_error_stats().await;
        let breakers = self.circuit_breakers.read().await;

        let total_errors: u64 = stats.values().map(|s| s.total_errors).sum();
        let max_error_rate = stats.values().map(|s| s.error_rate).fold(0.0, f64::max);
        let open_circuits: Vec<String> = breakers
            .iter()
            .filter(|(_, b)| b.state == CircuitBreakerState::Open)
            .map(|(name, _)| name.clone())
            .collect();

        ErrorHandlerHealth {
            total_errors,
            max_error_rate,
            is_degraded: self.is_degraded_mode().await,
            open_circuits,
            error_stats: stats,
        }
    }

    /// Update error rate calculation
    async fn update_error_rate(&self, error_stats: &mut ErrorStats) {
        let now = SystemTime::now();
        let window_start = now - self.config.error_rate_window;

        // Filter recent errors within the time window
        error_stats.recent_errors.retain(|record| record.timestamp >= window_start);

        // Calculate error rate (errors per second)
        let error_count = error_stats.recent_errors.len() as f64;
        let window_seconds = self.config.error_rate_window.as_secs_f64();
        error_stats.error_rate = error_count / window_seconds;
    }

    /// Log error with context
    async fn log_error(&self, error: &ContextualError) {
        println!("{}", error.log_message());
    }

    /// Log retry attempt
    async fn log_retry_attempt(&self, error: &ContextualError, delay: Duration) {
        println!(
            "[INFO] Retrying {} after {:?} due to: {} (attempt {})",
            error.context.operation,
            delay,
            error.error,
            error.context.attempt_number + 2
        );
    }

    /// Log successful recovery after retries
    async fn log_recovery(&self, context: &ErrorContext, attempts: u32) {
        println!(
            "[INFO] Operation {} recovered after {} attempts",
            context.operation,
            attempts + 1
        );
    }
}

/// Health status of the error handler
#[derive(Debug, Clone)]
pub struct ErrorHandlerHealth {
    pub total_errors: u64,
    pub max_error_rate: f64,
    pub is_degraded: bool,
    pub open_circuits: Vec<String>,
    pub error_stats: HashMap<ErrorCategory, ErrorStats>,
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    #[tokio::test]
    async fn test_retry_with_exponential_backoff() {
        let handler = ErrorHandler::new();
        let attempt_count = Arc::new(AtomicU32::new(0));
        let attempt_count_clone = attempt_count.clone();

        let context = ErrorContext::new("test_operation".to_string());

        let result = handler.handle_error(
            "test_op",
            context,
            || {
                let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
                async move {
                    if count < 2 {
                        Err(TransportError::ConnectionTimeout {
                            timeout: Duration::from_secs(5),
                        })
                    } else {
                        Ok("success")
                    }
                }
            },
        ).await;

        assert!(result.is_ok());
        assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let mut config = ErrorHandlerConfig::default();
        config.circuit_breaker_threshold = 2;
        let handler = ErrorHandler::with_config(config);

        let context = ErrorContext::new("test_operation".to_string());

        // First failure
        let result1: Result<&str, _> = handler.handle_error(
            "test_circuit",
            context.clone(),
            || async { Err(TransportError::ConnectionFailed { reason: "test".to_string() }) },
        ).await;
        assert!(result1.is_err());

        // Second failure - should open circuit
        let result2: Result<&str, _> = handler.handle_error(
            "test_circuit",
            context.clone(),
            || async { Err(TransportError::ConnectionFailed { reason: "test".to_string() }) },
        ).await;
        assert!(result2.is_err());

        // Third attempt - should fail fast due to open circuit
        let result3: Result<&str, _> = handler.handle_error(
            "test_circuit",
            context,
            || async { Ok("should not be called") },
        ).await;
        assert!(result3.is_err());
        if let Err(error) = result3 {
            assert!(matches!(error.error, TransportError::ResourceLimitExceeded { .. }));
        }
    }

    #[tokio::test]
    async fn test_error_statistics() {
        let handler = ErrorHandler::new();
        let context = ErrorContext::new("test_operation".to_string());

        // Record some errors
        let error1 = ContextualError::new(
            TransportError::ConnectionTimeout { timeout: Duration::from_secs(5) },
            context.clone(),
        );
        let error2 = ContextualError::new(
            TransportError::NatTraversalFailed { method: "STUN".to_string() },
            context,
        );

        handler.record_error(&error1).await;
        handler.record_error(&error2).await;

        let stats = handler.get_all_error_stats().await;
        assert!(stats.contains_key(&ErrorCategory::Connection));
        assert!(stats.contains_key(&ErrorCategory::Network));

        let connection_stats = stats.get(&ErrorCategory::Connection).unwrap();
        assert_eq!(connection_stats.total_errors, 1);
    }
}