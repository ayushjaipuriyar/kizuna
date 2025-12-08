/// Error types for the Developer API system
use thiserror::Error;
use std::backtrace::Backtrace;
use std::collections::HashMap;
use std::time::SystemTime;

/// Main error type for Kizuna API operations with comprehensive context
#[derive(Debug)]
pub struct KizunaError {
    /// The error kind
    pub kind: ErrorKind,
    
    /// Error context with additional information
    pub context: ErrorContext,
    
    /// Stack trace captured at error creation
    pub backtrace: Backtrace,
    
    /// Source error if this error wraps another
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

/// Error kind enumeration
#[derive(Debug, Clone, Error)]
pub enum ErrorKind {
    /// Discovery operation failed
    #[error("Discovery failed: {reason}")]
    DiscoveryError { reason: String },
    
    /// Connection to peer failed
    #[error("Connection failed: {peer_id}")]
    ConnectionError { peer_id: String },
    
    /// File transfer operation failed
    #[error("Transfer failed: {transfer_id}")]
    TransferError { transfer_id: String },
    
    /// Plugin operation failed
    #[error("Plugin error: {plugin_name} - {error}")]
    PluginError { plugin_name: String, error: String },
    
    /// Configuration error
    #[error("Configuration error: {message}")]
    ConfigError { message: String },
    
    /// Invalid API state
    #[error("Invalid state: {message}")]
    StateError { message: String },
    
    /// Invalid parameters provided
    #[error("Invalid parameter: {parameter} - {reason}")]
    ParameterError { parameter: String, reason: String },
    
    /// Security operation failed
    #[error("Security error: {message}")]
    SecurityError { message: String },
    
    /// Network operation failed
    #[error("Network error: {message}")]
    NetworkError { message: String },
    
    /// I/O operation failed
    #[error("I/O error: {message}")]
    IoError { message: String },
    
    /// Serialization/deserialization failed
    #[error("Serialization error: {message}")]
    SerializationError { message: String },
    
    /// Timeout occurred
    #[error("Timeout: {operation} exceeded {duration_ms}ms")]
    TimeoutError { operation: String, duration_ms: u64 },
    
    /// Resource exhausted
    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },
    
    /// Permission denied
    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },
    
    /// Not found
    #[error("Not found: {resource}")]
    NotFound { resource: String },
    
    /// Already exists
    #[error("Already exists: {resource}")]
    AlreadyExists { resource: String },
    
    /// Operation cancelled
    #[error("Operation cancelled: {operation}")]
    Cancelled { operation: String },
    
    /// Generic error for other cases
    #[error("Error: {message}")]
    Other { message: String },
}

impl KizunaError {
    /// Creates a new error with the given kind
    pub fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            context: ErrorContext::new(),
            backtrace: Backtrace::capture(),
            source: None,
        }
    }
    
    /// Creates an error with context
    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = context;
        self
    }
    
    /// Adds a context field to the error
    pub fn add_context<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.context.add(key, value);
        self
    }
    
    /// Sets the source error
    pub fn with_error_source<E: std::error::Error + Send + Sync + 'static>(mut self, source: E) -> Self {
        self.source = Some(Box::new(source));
        self
    }
    
    /// Gets the source error
    pub fn error_source(&self) -> Option<&(dyn std::error::Error + Send + Sync)> {
        self.source.as_ref().map(|e| e.as_ref())
    }
    
    /// Creates a discovery error
    pub fn discovery<S: Into<String>>(reason: S) -> Self {
        Self::new(ErrorKind::DiscoveryError {
            reason: reason.into(),
        })
    }
    
    /// Creates a connection error
    pub fn connection<S: Into<String>>(peer_id: S) -> Self {
        Self::new(ErrorKind::ConnectionError {
            peer_id: peer_id.into(),
        })
    }
    
    /// Creates a transfer error
    pub fn transfer<S: Into<String>>(transfer_id: S) -> Self {
        Self::new(ErrorKind::TransferError {
            transfer_id: transfer_id.into(),
        })
    }
    
    /// Creates a plugin error
    pub fn plugin<S: Into<String>>(plugin_name: S, error: S) -> Self {
        Self::new(ErrorKind::PluginError {
            plugin_name: plugin_name.into(),
            error: error.into(),
        })
    }
    
    /// Creates a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorKind::ConfigError {
            message: message.into(),
        })
    }
    
    /// Creates a state error
    pub fn state<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorKind::StateError {
            message: message.into(),
        })
    }
    
    /// Creates a parameter error
    pub fn parameter<S: Into<String>>(parameter: S, reason: S) -> Self {
        Self::new(ErrorKind::ParameterError {
            parameter: parameter.into(),
            reason: reason.into(),
        })
    }
    
    /// Creates a security error
    pub fn security<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorKind::SecurityError {
            message: message.into(),
        })
    }
    
    /// Creates a network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorKind::NetworkError {
            message: message.into(),
        })
    }
    
    /// Creates a timeout error
    pub fn timeout<S: Into<String>>(operation: S, duration_ms: u64) -> Self {
        Self::new(ErrorKind::TimeoutError {
            operation: operation.into(),
            duration_ms,
        })
    }
    
    /// Creates a resource exhausted error
    pub fn resource_exhausted<S: Into<String>>(resource: S) -> Self {
        Self::new(ErrorKind::ResourceExhausted {
            resource: resource.into(),
        })
    }
    
    /// Creates a permission denied error
    pub fn permission_denied<S: Into<String>>(operation: S) -> Self {
        Self::new(ErrorKind::PermissionDenied {
            operation: operation.into(),
        })
    }
    
    /// Creates a not found error
    pub fn not_found<S: Into<String>>(resource: S) -> Self {
        Self::new(ErrorKind::NotFound {
            resource: resource.into(),
        })
    }
    
    /// Creates an already exists error
    pub fn already_exists<S: Into<String>>(resource: S) -> Self {
        Self::new(ErrorKind::AlreadyExists {
            resource: resource.into(),
        })
    }
    
    /// Creates a cancelled error
    pub fn cancelled<S: Into<String>>(operation: S) -> Self {
        Self::new(ErrorKind::Cancelled {
            operation: operation.into(),
        })
    }
    
    /// Creates a generic error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::new(ErrorKind::Other {
            message: message.into(),
        })
    }
    
    /// Creates a transport error
    pub fn transport<S: Into<String>>(message: S) -> Self {
        Self::network(message)
    }
    
    /// Creates a file transfer error
    pub fn file_transfer<S: Into<String>>(message: S) -> Self {
        Self::transfer(message)
    }
    
    /// Creates a streaming error
    pub fn streaming<S: Into<String>>(message: S) -> Self {
        Self::other(format!("Streaming error: {}", message.into()))
    }
    
    /// Creates a clipboard error
    pub fn clipboard<S: Into<String>>(message: S) -> Self {
        Self::other(format!("Clipboard error: {}", message.into()))
    }
    
    /// Creates a command execution error
    pub fn command_execution<S: Into<String>>(message: S) -> Self {
        Self::other(format!("Command execution error: {}", message.into()))
    }
    
    /// Returns the error kind
    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
    
    /// Returns whether this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.kind,
            ErrorKind::NetworkError { .. }
                | ErrorKind::TimeoutError { .. }
                | ErrorKind::ResourceExhausted { .. }
        )
    }
    
    /// Returns a user-friendly error message with resolution steps
    pub fn user_message(&self) -> String {
        match &self.kind {
            ErrorKind::DiscoveryError { reason } => {
                format!("Failed to discover peers: {}. Check your network connection and firewall settings.", reason)
            }
            ErrorKind::ConnectionError { peer_id } => {
                format!("Failed to connect to peer '{}'. Ensure the peer is online and accessible.", peer_id)
            }
            ErrorKind::TransferError { transfer_id } => {
                format!("File transfer '{}' failed. Check available disk space and network stability.", transfer_id)
            }
            ErrorKind::PluginError { plugin_name, error } => {
                format!("Plugin '{}' encountered an error: {}. Try disabling and re-enabling the plugin.", plugin_name, error)
            }
            ErrorKind::ConfigError { message } => {
                format!("Configuration error: {}. Review your configuration file for errors.", message)
            }
            ErrorKind::StateError { message } => {
                format!("Invalid operation: {}. Ensure the API is properly initialized.", message)
            }
            ErrorKind::ParameterError { parameter, reason } => {
                format!("Invalid parameter '{}': {}. Check the API documentation for valid values.", parameter, reason)
            }
            ErrorKind::SecurityError { message } => {
                format!("Security error: {}. Verify your credentials and permissions.", message)
            }
            ErrorKind::NetworkError { message } => {
                format!("Network error: {}. Check your internet connection and try again.", message)
            }
            ErrorKind::IoError { message } => {
                format!("I/O error: {}. Check file permissions and available disk space.", message)
            }
            ErrorKind::SerializationError { message } => {
                format!("Data serialization error: {}. The data format may be corrupted.", message)
            }
            ErrorKind::TimeoutError { operation, duration_ms } => {
                format!("Operation '{}' timed out after {}ms. Try again or increase the timeout value.", operation, duration_ms)
            }
            ErrorKind::ResourceExhausted { resource } => {
                format!("Resource exhausted: {}. Free up resources and try again.", resource)
            }
            ErrorKind::PermissionDenied { operation } => {
                format!("Permission denied for operation '{}'. Check your access rights.", operation)
            }
            ErrorKind::NotFound { resource } => {
                format!("Resource not found: {}. Verify the resource exists and the path is correct.", resource)
            }
            ErrorKind::AlreadyExists { resource } => {
                format!("Resource already exists: {}. Use a different name or remove the existing resource.", resource)
            }
            ErrorKind::Cancelled { operation } => {
                format!("Operation '{}' was cancelled.", operation)
            }
            ErrorKind::Other { message } => {
                format!("Error: {}. Contact support if the problem persists.", message)
            }
        }
    }
}

impl std::fmt::Display for KizunaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)?;
        if !self.context.fields.is_empty() {
            write!(f, " (context: {:?})", self.context.fields)?;
        }
        Ok(())
    }
}

impl std::error::Error for KizunaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

// Conversion from std::io::Error
impl From<std::io::Error> for KizunaError {
    fn from(err: std::io::Error) -> Self {
        Self::new(ErrorKind::IoError {
            message: err.to_string(),
        })
        .with_error_source(err)
    }
}

// Conversion from serde_json::Error
impl From<serde_json::Error> for KizunaError {
    fn from(err: serde_json::Error) -> Self {
        Self::new(ErrorKind::SerializationError {
            message: err.to_string(),
        })
        .with_error_source(err)
    }
}

/// Error context for detailed error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Operation that failed
    pub operation: Option<String>,
    
    /// Additional context information
    pub fields: HashMap<String, String>,
    
    /// Timestamp of the error
    pub timestamp: SystemTime,
    
    /// Error severity level
    pub severity: ErrorSeverity,
    
    /// Error category for grouping
    pub category: ErrorCategory,
}

impl ErrorContext {
    /// Creates a new empty error context
    pub fn new() -> Self {
        Self {
            operation: None,
            fields: HashMap::new(),
            timestamp: SystemTime::now(),
            severity: ErrorSeverity::Error,
            category: ErrorCategory::General,
        }
    }
    
    /// Creates a new error context with an operation
    pub fn with_operation<S: Into<String>>(operation: S) -> Self {
        Self {
            operation: Some(operation.into()),
            fields: HashMap::new(),
            timestamp: SystemTime::now(),
            severity: ErrorSeverity::Error,
            category: ErrorCategory::General,
        }
    }
    
    /// Adds context information
    pub fn add<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
        self.fields.insert(key.into(), value.into());
    }
    
    /// Adds context information (builder pattern)
    pub fn with_field<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
    
    /// Sets the severity level
    pub fn with_severity(mut self, severity: ErrorSeverity) -> Self {
        self.severity = severity;
        self
    }
    
    /// Sets the error category
    pub fn with_category(mut self, category: ErrorCategory) -> Self {
        self.category = category;
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Error severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Informational - not really an error
    Info,
    /// Warning - something unexpected but not critical
    Warning,
    /// Error - operation failed but system is stable
    Error,
    /// Critical - system stability may be affected
    Critical,
}

/// Error category for grouping and filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// General uncategorized error
    General,
    /// Network-related error
    Network,
    /// Security-related error
    Security,
    /// Configuration-related error
    Configuration,
    /// Plugin-related error
    Plugin,
    /// Resource-related error
    Resource,
    /// API usage error
    ApiUsage,
}

/// Action to take when an error occurs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorAction {
    /// Retry the operation
    Retry,
    
    /// Use a fallback mechanism
    Fallback,
    
    /// Abort the operation
    Abort,
    
    /// Continue despite the error
    Continue,
}

/// Error handler trait for custom error handling
pub trait ErrorHandler: Send + Sync {
    /// Handles an error and returns the action to take
    fn handle_error(&self, error: &KizunaError) -> ErrorAction;
    
    /// Reports an error
    fn report_error(&self, error: &KizunaError);
}

/// Default error handler implementation
pub struct DefaultErrorHandler;

impl ErrorHandler for DefaultErrorHandler {
    fn handle_error(&self, error: &KizunaError) -> ErrorAction {
        // Determine action based on error type and retryability
        if error.is_retryable() {
            ErrorAction::Retry
        } else {
            match &error.kind {
                ErrorKind::Cancelled { .. } => ErrorAction::Abort,
                ErrorKind::ParameterError { .. } => ErrorAction::Abort,
                ErrorKind::StateError { .. } => ErrorAction::Abort,
                _ => ErrorAction::Abort,
            }
        }
    }
    
    fn report_error(&self, error: &KizunaError) {
        eprintln!("Error: {}", error);
        if let Some(source) = error.error_source() {
            eprintln!("Caused by: {}", source);
        }
    }
}

/// Retry strategy for error recovery
#[derive(Debug, Clone)]
pub struct RetryStrategy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    
    /// Initial delay between retries in milliseconds
    pub initial_delay_ms: u64,
    
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
    
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f64,
    
    /// Whether to use jitter to avoid thundering herd
    pub use_jitter: bool,
}

impl RetryStrategy {
    /// Creates a new retry strategy with default values
    pub fn new() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            use_jitter: true,
        }
    }
    
    /// Creates a retry strategy with no retries
    pub fn no_retry() -> Self {
        Self {
            max_attempts: 0,
            initial_delay_ms: 0,
            max_delay_ms: 0,
            backoff_multiplier: 1.0,
            use_jitter: false,
        }
    }
    
    /// Creates a retry strategy with fixed delay
    pub fn fixed_delay(max_attempts: u32, delay_ms: u64) -> Self {
        Self {
            max_attempts,
            initial_delay_ms: delay_ms,
            max_delay_ms: delay_ms,
            backoff_multiplier: 1.0,
            use_jitter: false,
        }
    }
    
    /// Creates a retry strategy with exponential backoff
    pub fn exponential_backoff(max_attempts: u32, initial_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            initial_delay_ms,
            max_delay_ms: initial_delay_ms * 32, // 2^5
            backoff_multiplier: 2.0,
            use_jitter: true,
        }
    }
    
    /// Calculates the delay for a given attempt number
    pub fn delay_for_attempt(&self, attempt: u32) -> std::time::Duration {
        if attempt >= self.max_attempts {
            return std::time::Duration::from_millis(0);
        }
        
        let mut delay = self.initial_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        delay = delay.min(self.max_delay_ms as f64);
        
        if self.use_jitter {
            // Add random jitter (±25%)
            use std::collections::hash_map::RandomState;
            use std::hash::{BuildHasher, Hash, Hasher};
            let mut hasher = RandomState::new().build_hasher();
            attempt.hash(&mut hasher);
            let jitter = (hasher.finish() % 50) as f64 / 100.0 - 0.25;
            delay *= 1.0 + jitter;
        }
        
        std::time::Duration::from_millis(delay as u64)
    }
    
    /// Returns whether another retry should be attempted
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self::new()
    }
}

/// Language-specific error mapping for FFI boundaries
pub mod language_mapping {
    use super::*;
    
    /// Error code for FFI boundaries
    #[repr(i32)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum ErrorCode {
        Success = 0,
        DiscoveryError = 1,
        ConnectionError = 2,
        TransferError = 3,
        PluginError = 4,
        ConfigError = 5,
        StateError = 6,
        ParameterError = 7,
        SecurityError = 8,
        NetworkError = 9,
        IoError = 10,
        SerializationError = 11,
        TimeoutError = 12,
        ResourceExhausted = 13,
        PermissionDenied = 14,
        NotFound = 15,
        AlreadyExists = 16,
        Cancelled = 17,
        Other = 99,
    }
    
    impl From<&ErrorKind> for ErrorCode {
        fn from(kind: &ErrorKind) -> Self {
            match kind {
                ErrorKind::DiscoveryError { .. } => ErrorCode::DiscoveryError,
                ErrorKind::ConnectionError { .. } => ErrorCode::ConnectionError,
                ErrorKind::TransferError { .. } => ErrorCode::TransferError,
                ErrorKind::PluginError { .. } => ErrorCode::PluginError,
                ErrorKind::ConfigError { .. } => ErrorCode::ConfigError,
                ErrorKind::StateError { .. } => ErrorCode::StateError,
                ErrorKind::ParameterError { .. } => ErrorCode::ParameterError,
                ErrorKind::SecurityError { .. } => ErrorCode::SecurityError,
                ErrorKind::NetworkError { .. } => ErrorCode::NetworkError,
                ErrorKind::IoError { .. } => ErrorCode::IoError,
                ErrorKind::SerializationError { .. } => ErrorCode::SerializationError,
                ErrorKind::TimeoutError { .. } => ErrorCode::TimeoutError,
                ErrorKind::ResourceExhausted { .. } => ErrorCode::ResourceExhausted,
                ErrorKind::PermissionDenied { .. } => ErrorCode::PermissionDenied,
                ErrorKind::NotFound { .. } => ErrorCode::NotFound,
                ErrorKind::AlreadyExists { .. } => ErrorCode::AlreadyExists,
                ErrorKind::Cancelled { .. } => ErrorCode::Cancelled,
                ErrorKind::Other { .. } => ErrorCode::Other,
            }
        }
    }
    
    /// JavaScript/TypeScript error representation
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    pub struct JsError {
        pub code: i32,
        pub message: String,
        pub kind: String,
        pub context: HashMap<String, String>,
        pub timestamp: u64,
        pub severity: String,
        pub user_message: String,
    }
    
    impl From<&KizunaError> for JsError {
        fn from(error: &KizunaError) -> Self {
            let code = ErrorCode::from(&error.kind) as i32;
            let timestamp = error.context.timestamp
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            Self {
                code,
                message: error.kind.to_string(),
                kind: format!("{:?}", error.kind),
                context: error.context.fields.clone(),
                timestamp,
                severity: format!("{:?}", error.context.severity),
                user_message: error.user_message(),
            }
        }
    }
    
    /// Python exception representation
    #[derive(Debug, Clone)]
    pub struct PyError {
        pub exception_type: String,
        pub message: String,
        pub context: HashMap<String, String>,
        pub user_message: String,
    }
    
    impl From<&KizunaError> for PyError {
        fn from(error: &KizunaError) -> Self {
            let exception_type = match &error.kind {
                ErrorKind::ParameterError { .. } => "ValueError",
                ErrorKind::NotFound { .. } => "LookupError",
                ErrorKind::PermissionDenied { .. } => "PermissionError",
                ErrorKind::IoError { .. } => "IOError",
                ErrorKind::TimeoutError { .. } => "TimeoutError",
                ErrorKind::ConnectionError { .. } => "ConnectionError",
                _ => "RuntimeError",
            };
            
            Self {
                exception_type: exception_type.to_string(),
                message: error.kind.to_string(),
                context: error.context.fields.clone(),
                user_message: error.user_message(),
            }
        }
    }
    
    /// Dart/Flutter exception representation
    #[derive(Debug, Clone)]
    pub struct DartError {
        pub error_type: String,
        pub message: String,
        pub details: HashMap<String, String>,
        pub user_message: String,
    }
    
    impl From<&KizunaError> for DartError {
        fn from(error: &KizunaError) -> Self {
            Self {
                error_type: format!("{:?}", error.kind),
                message: error.kind.to_string(),
                details: error.context.fields.clone(),
                user_message: error.user_message(),
            }
        }
    }
}
