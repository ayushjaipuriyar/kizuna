/// Error recovery mechanisms and best practices for the Developer API
use super::error::{KizunaError, ErrorAction, ErrorHandler, RetryStrategy};
use super::logging::{LogLevel, LogRecord, Logger};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

/// Error recovery manager
pub struct ErrorRecoveryManager {
    handler: Arc<dyn ErrorHandler>,
    logger: Option<Arc<dyn Logger>>,
    default_retry_strategy: RetryStrategy,
}

impl ErrorRecoveryManager {
    /// Creates a new error recovery manager
    pub fn new(handler: Arc<dyn ErrorHandler>) -> Self {
        Self {
            handler,
            logger: None,
            default_retry_strategy: RetryStrategy::new(),
        }
    }
    
    /// Sets the logger
    pub fn with_logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }
    
    /// Sets the default retry strategy
    pub fn with_retry_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.default_retry_strategy = strategy;
        self
    }
    
    /// Executes an operation with error recovery
    pub async fn execute_with_recovery<F, Fut, T>(
        &self,
        operation: F,
        operation_name: &str,
    ) -> Result<T, KizunaError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, KizunaError>>,
    {
        self.execute_with_custom_retry(operation, operation_name, &self.default_retry_strategy)
            .await
    }
    
    /// Executes an operation with custom retry strategy
    pub async fn execute_with_custom_retry<F, Fut, T>(
        &self,
        operation: F,
        operation_name: &str,
        retry_strategy: &RetryStrategy,
    ) -> Result<T, KizunaError>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, KizunaError>>,
    {
        let mut attempt = 0;
        
        loop {
            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        self.log_recovery_success(operation_name, attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    let action = self.handler.handle_error(&error);
                    
                    match action {
                        ErrorAction::Retry if retry_strategy.should_retry(attempt) => {
                            self.log_retry_attempt(operation_name, attempt, &error);
                            
                            let delay = retry_strategy.delay_for_attempt(attempt);
                            if delay > Duration::from_millis(0) {
                                tokio::time::sleep(delay).await;
                            }
                            
                            attempt += 1;
                        }
                        ErrorAction::Abort | ErrorAction::Retry => {
                            self.handler.report_error(&error);
                            return Err(error);
                        }
                        ErrorAction::Fallback => {
                            self.log_fallback(operation_name, &error);
                            return Err(error);
                        }
                        ErrorAction::Continue => {
                            self.log_continue(operation_name, &error);
                            return Err(error);
                        }
                    }
                }
            }
        }
    }
    
    fn log_retry_attempt(&self, operation: &str, attempt: u32, error: &KizunaError) {
        if let Some(logger) = &self.logger {
            let record = LogRecord::new(
                LogLevel::Warn,
                format!(
                    "Retrying operation '{}' (attempt {}): {}",
                    operation, attempt + 1, error
                ),
                "error_recovery".to_string(),
            );
            logger.log(&record);
        }
    }
    
    fn log_recovery_success(&self, operation: &str, attempts: u32) {
        if let Some(logger) = &self.logger {
            let record = LogRecord::new(
                LogLevel::Info,
                format!(
                    "Operation '{}' succeeded after {} retries",
                    operation, attempts
                ),
                "error_recovery".to_string(),
            );
            logger.log(&record);
        }
    }
    
    fn log_fallback(&self, operation: &str, error: &KizunaError) {
        if let Some(logger) = &self.logger {
            let record = LogRecord::new(
                LogLevel::Warn,
                format!("Using fallback for operation '{}': {}", operation, error),
                "error_recovery".to_string(),
            );
            logger.log(&record);
        }
    }
    
    fn log_continue(&self, operation: &str, error: &KizunaError) {
        if let Some(logger) = &self.logger {
            let record = LogRecord::new(
                LogLevel::Info,
                format!("Continuing despite error in operation '{}': {}", operation, error),
                "error_recovery".to_string(),
            );
            logger.log(&record);
        }
    }
}

/// Circuit breaker for preventing cascading failures
pub struct CircuitBreaker {
    /// Failure threshold before opening the circuit
    failure_threshold: u32,
    
    /// Success threshold before closing the circuit
    success_threshold: u32,
    
    /// Timeout before attempting to close the circuit
    timeout: Duration,
    
    /// Current state
    state: Arc<std::sync::Mutex<CircuitBreakerState>>,
}

#[derive(Debug, Clone)]
struct CircuitBreakerState {
    failures: u32,
    successes: u32,
    last_failure_time: Option<std::time::Instant>,
    is_open: bool,
}

impl CircuitBreaker {
    /// Creates a new circuit breaker
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            success_threshold,
            timeout,
            state: Arc::new(std::sync::Mutex::new(CircuitBreakerState {
                failures: 0,
                successes: 0,
                last_failure_time: None,
                is_open: false,
            })),
        }
    }
    
    /// Executes an operation through the circuit breaker
    pub async fn execute<F, Fut, T>(&self, operation: F) -> Result<T, KizunaError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, KizunaError>>,
    {
        // Check if circuit is open
        {
            let mut state = self.state.lock().unwrap();
            if state.is_open {
                if let Some(last_failure) = state.last_failure_time {
                    if last_failure.elapsed() < self.timeout {
                        return Err(KizunaError::state("Circuit breaker is open"));
                    } else {
                        // Try to close the circuit
                        state.is_open = false;
                        state.failures = 0;
                        state.successes = 0;
                    }
                }
            }
        }
        
        // Execute operation
        match operation().await {
            Ok(result) => {
                self.record_success();
                Ok(result)
            }
            Err(error) => {
                self.record_failure();
                Err(error)
            }
        }
    }
    
    fn record_success(&self) {
        let mut state = self.state.lock().unwrap();
        state.successes += 1;
        state.failures = 0;
        
        if state.successes >= self.success_threshold {
            state.is_open = false;
            state.successes = 0;
        }
    }
    
    fn record_failure(&self) {
        let mut state = self.state.lock().unwrap();
        state.failures += 1;
        state.successes = 0;
        state.last_failure_time = Some(std::time::Instant::now());
        
        if state.failures >= self.failure_threshold {
            state.is_open = true;
        }
    }
    
    /// Returns whether the circuit is open
    pub fn is_open(&self) -> bool {
        self.state.lock().unwrap().is_open
    }
    
    /// Resets the circuit breaker
    pub fn reset(&self) {
        let mut state = self.state.lock().unwrap();
        state.failures = 0;
        state.successes = 0;
        state.last_failure_time = None;
        state.is_open = false;
    }
}

/// Error reporting and analytics
pub mod reporting {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;
    
    /// Error statistics
    #[derive(Debug, Clone)]
    pub struct ErrorStatistics {
        /// Total number of errors
        pub total_errors: u64,
        
        /// Errors by type
        pub errors_by_type: HashMap<String, u64>,
        
        /// Errors by severity
        pub errors_by_severity: HashMap<String, u64>,
        
        /// Recent errors
        pub recent_errors: Vec<ErrorReport>,
    }
    
    /// Error report
    #[derive(Debug, Clone)]
    pub struct ErrorReport {
        /// Error message
        pub message: String,
        
        /// Error type
        pub error_type: String,
        
        /// Timestamp
        pub timestamp: std::time::SystemTime,
        
        /// Context
        pub context: HashMap<String, String>,
    }
    
    /// Error reporter for collecting and analyzing errors
    pub struct ErrorReporter {
        statistics: Arc<Mutex<ErrorStatistics>>,
        max_recent_errors: usize,
    }
    
    impl ErrorReporter {
        /// Creates a new error reporter
        pub fn new() -> Self {
            Self {
                statistics: Arc::new(Mutex::new(ErrorStatistics {
                    total_errors: 0,
                    errors_by_type: HashMap::new(),
                    errors_by_severity: HashMap::new(),
                    recent_errors: Vec::new(),
                })),
                max_recent_errors: 100,
            }
        }
        
        /// Reports an error
        pub fn report(&self, error: &KizunaError) {
            let mut stats = self.statistics.lock().unwrap();
            
            stats.total_errors += 1;
            
            let error_type = format!("{:?}", error.kind());
            *stats.errors_by_type.entry(error_type.clone()).or_insert(0) += 1;
            
            let severity = format!("{:?}", error.context.severity);
            *stats.errors_by_severity.entry(severity).or_insert(0) += 1;
            
            let report = ErrorReport {
                message: error.to_string(),
                error_type,
                timestamp: std::time::SystemTime::now(),
                context: error.context.fields.clone(),
            };
            
            stats.recent_errors.push(report);
            
            // Keep only the most recent errors
            if stats.recent_errors.len() > self.max_recent_errors {
                stats.recent_errors.remove(0);
            }
        }
        
        /// Gets error statistics
        pub fn statistics(&self) -> ErrorStatistics {
            self.statistics.lock().unwrap().clone()
        }
        
        /// Clears error statistics
        pub fn clear(&self) {
            let mut stats = self.statistics.lock().unwrap();
            stats.total_errors = 0;
            stats.errors_by_type.clear();
            stats.errors_by_severity.clear();
            stats.recent_errors.clear();
        }
    }
    
    impl Default for ErrorReporter {
        fn default() -> Self {
            Self::new()
        }
    }
}

/// Best practices documentation
pub mod best_practices {
    /// Best practices for error handling in Kizuna API
    pub const ERROR_HANDLING_GUIDE: &str = r#"
# Error Handling Best Practices

## General Guidelines

1. **Always handle errors explicitly**
   - Never ignore errors or use unwrap() in production code
   - Use proper error propagation with the ? operator
   - Provide meaningful error messages

2. **Use appropriate error types**
   - Use specific error types for different failure modes
   - Include context information in errors
   - Preserve error chains with source errors

3. **Implement retry logic for transient failures**
   - Use exponential backoff for retries
   - Set reasonable retry limits
   - Add jitter to prevent thundering herd

4. **Log errors appropriately**
   - Log errors at appropriate severity levels
   - Include context and trace information
   - Avoid logging sensitive information

5. **Provide user-friendly error messages**
   - Use clear, actionable error messages
   - Suggest resolution steps
   - Avoid technical jargon when possible

## Error Recovery Patterns

### Retry Pattern
```rust
let result = recovery_manager
    .execute_with_recovery(|| async {
        // Your operation here
    }, "operation_name")
    .await?;
```

### Circuit Breaker Pattern
```rust
let circuit_breaker = CircuitBreaker::new(5, 2, Duration::from_secs(30));
let result = circuit_breaker
    .execute(|| async {
        // Your operation here
    })
    .await?;
```

### Fallback Pattern
```rust
match primary_operation().await {
    Ok(result) => result,
    Err(error) => {
        log_error(&error);
        fallback_operation().await?
    }
}
```

## Common Pitfalls

1. **Swallowing errors**
   - Don't ignore errors silently
   - Always log or propagate errors

2. **Infinite retries**
   - Always set a maximum retry count
   - Use exponential backoff

3. **Missing context**
   - Add relevant context to errors
   - Include operation names and parameters

4. **Poor error messages**
   - Provide actionable error messages
   - Include resolution steps

5. **Not handling specific error types**
   - Handle different error types appropriately
   - Use pattern matching for error handling
"#;
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_retry_strategy() {
        let strategy = RetryStrategy::exponential_backoff(3, 100);
        
        assert!(strategy.should_retry(0));
        assert!(strategy.should_retry(1));
        assert!(strategy.should_retry(2));
        assert!(!strategy.should_retry(3));
        
        let delay0 = strategy.delay_for_attempt(0);
        let delay1 = strategy.delay_for_attempt(1);
        assert!(delay1 > delay0);
    }
    
    #[tokio::test]
    async fn test_circuit_breaker() {
        let breaker = CircuitBreaker::new(3, 2, Duration::from_millis(100));
        
        // Record failures to open circuit
        for _ in 0..3 {
            let _ = breaker
                .execute(|| async { Err::<(), _>(KizunaError::other("test error")) })
                .await;
        }
        
        assert!(breaker.is_open());
        
        // Circuit should be open
        let result = breaker
            .execute(|| async { Ok::<(), KizunaError>(()) })
            .await;
        assert!(result.is_err());
        
        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Circuit should allow attempts again
        let result = breaker
            .execute(|| async { Ok::<(), KizunaError>(()) })
            .await;
        assert!(result.is_ok());
    }
}
