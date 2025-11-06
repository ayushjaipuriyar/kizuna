use std::time::Duration;

#[derive(Debug, Clone, thiserror::Error)]
pub enum DiscoveryError {
    #[error("Network error: {0}")]
    Network(String),
    
    #[error("Strategy not available: {strategy}")]
    StrategyUnavailable { strategy: String },
    
    #[error("Discovery timeout after {timeout:?}")]
    Timeout { timeout: Duration },
    
    #[error("Invalid service record: {reason}")]
    InvalidServiceRecord { reason: String },
    
    #[error("Bluetooth error: {0}")]
    Bluetooth(String),
    

    #[error("Parse error: {0}")]
    Parse(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },
    
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },
    
    #[error("Service unavailable: {service} - {reason}")]
    ServiceUnavailable { service: String, reason: String },
    
    #[error("Rate limit exceeded for {strategy}: {limit} requests per {window:?}")]
    RateLimitExceeded { strategy: String, limit: u32, window: Duration },
    
    #[error("Authentication failed for {strategy}: {reason}")]
    AuthenticationFailed { strategy: String, reason: String },
    
    #[error("Protocol error in {strategy}: {message}")]
    ProtocolError { strategy: String, message: String },
    
    #[error("Transient error in {strategy}: {message} (retry {attempt}/{max_attempts})")]
    TransientError { 
        strategy: String, 
        message: String, 
        attempt: u32, 
        max_attempts: u32 
    },
    
    #[error("Fatal error in {strategy}: {message}")]
    FatalError { strategy: String, message: String },
    
    #[error("Multiple errors occurred: {errors:?}")]
    MultipleErrors { errors: Vec<DiscoveryError> },
    
    #[error("Initialization failed for {component}: {reason}")]
    InitializationFailed { component: String, reason: String },
    
    #[error("Shutdown error for {component}: {reason}")]
    ShutdownError { component: String, reason: String },
}

#[derive(Debug, Clone)]
pub enum ErrorSeverity {
    Low,      // Informational, can be ignored
    Medium,   // Warning, should be logged
    High,     // Error, affects functionality
    Critical, // Fatal error, requires immediate attention
}

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub strategy: Option<String>,
    pub operation: String,
    pub severity: ErrorSeverity,
    pub retry_count: u32,
    pub max_retries: u32,
    pub timestamp: std::time::SystemTime,
    pub additional_info: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(operation: String) -> Self {
        Self {
            strategy: None,
            operation,
            severity: ErrorSeverity::Medium,
            retry_count: 0,
            max_retries: 3,
            timestamp: std::time::SystemTime::now(),
            additional_info: std::collections::HashMap::new(),
        }
    }

    pub fn with_strategy(mut self, strategy: String) -> Self {
        self.strategy = Some(strategy);
        self
    }

    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_retry_info(mut self, retry_count: u32, max_retries: u32) -> Self {
        self.retry_count = retry_count;
        self.max_retries = max_retries;
        self
    }

    pub fn add_info(mut self, key: String, value: String) -> Self {
        self.additional_info.insert(key, value);
        self
    }

    pub fn is_retryable(&self) -> bool {
        self.retry_count < self.max_retries && 
        matches!(self.severity, ErrorSeverity::Low | ErrorSeverity::Medium)
    }
}

impl From<anyhow::Error> for DiscoveryError {
    fn from(err: anyhow::Error) -> Self {
        DiscoveryError::Network(err.to_string())
    }
}

impl From<std::io::Error> for DiscoveryError {
    fn from(err: std::io::Error) -> Self {
        DiscoveryError::Network(err.to_string())
    }
}