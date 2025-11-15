/// Error types for the Developer API system
use thiserror::Error;
use std::fmt;

/// Main error type for Kizuna API operations
#[derive(Debug, Error)]
pub enum KizunaError {
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
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Serialization/deserialization failed
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    /// Generic error for other cases
    #[error("Error: {message}")]
    Other { message: String },
}

impl KizunaError {
    /// Creates a discovery error
    pub fn discovery<S: Into<String>>(reason: S) -> Self {
        Self::DiscoveryError {
            reason: reason.into(),
        }
    }
    
    /// Creates a connection error
    pub fn connection<S: Into<String>>(peer_id: S) -> Self {
        Self::ConnectionError {
            peer_id: peer_id.into(),
        }
    }
    
    /// Creates a transfer error
    pub fn transfer<S: Into<String>>(transfer_id: S) -> Self {
        Self::TransferError {
            transfer_id: transfer_id.into(),
        }
    }
    
    /// Creates a plugin error
    pub fn plugin<S: Into<String>>(plugin_name: S, error: S) -> Self {
        Self::PluginError {
            plugin_name: plugin_name.into(),
            error: error.into(),
        }
    }
    
    /// Creates a configuration error
    pub fn config<S: Into<String>>(message: S) -> Self {
        Self::ConfigError {
            message: message.into(),
        }
    }
    
    /// Creates a state error
    pub fn state<S: Into<String>>(message: S) -> Self {
        Self::StateError {
            message: message.into(),
        }
    }
    
    /// Creates a parameter error
    pub fn parameter<S: Into<String>>(parameter: S, reason: S) -> Self {
        Self::ParameterError {
            parameter: parameter.into(),
            reason: reason.into(),
        }
    }
    
    /// Creates a security error
    pub fn security<S: Into<String>>(message: S) -> Self {
        Self::SecurityError {
            message: message.into(),
        }
    }
    
    /// Creates a network error
    pub fn network<S: Into<String>>(message: S) -> Self {
        Self::NetworkError {
            message: message.into(),
        }
    }
    
    /// Creates a generic error
    pub fn other<S: Into<String>>(message: S) -> Self {
        Self::Other {
            message: message.into(),
        }
    }
}

/// Error context for detailed error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Operation that failed
    pub operation: String,
    
    /// Additional context information
    pub context: std::collections::HashMap<String, String>,
    
    /// Timestamp of the error
    pub timestamp: std::time::SystemTime,
}

impl ErrorContext {
    /// Creates a new error context
    pub fn new<S: Into<String>>(operation: S) -> Self {
        Self {
            operation: operation.into(),
            context: std::collections::HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        }
    }
    
    /// Adds context information
    pub fn with_context<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
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
    
    /// Reports an error with context
    fn report_error(&self, error: &KizunaError, context: ErrorContext);
}
