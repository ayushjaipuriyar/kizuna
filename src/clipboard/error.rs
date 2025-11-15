//! Error types for the clipboard system

use thiserror::Error;

/// Result type for clipboard operations
pub type ClipboardResult<T> = Result<T, ClipboardError>;

/// Comprehensive error types for clipboard operations
#[derive(Error, Debug)]
pub enum ClipboardError {
    /// Platform-specific clipboard API failures
    #[error("Platform clipboard error: {message}")]
    PlatformError { message: String },
    
    /// Clipboard access permission denied
    #[error("Clipboard access permission denied: {reason}")]
    PermissionError { reason: String },
    
    /// Invalid or corrupted clipboard content
    #[error("Invalid clipboard content: {details}")]
    ContentError { details: String },
    
    /// Network or peer communication failures
    #[error("Sync operation failed: {operation} - {reason}")]
    SyncError { operation: String, reason: String },
    
    /// Privacy policy violations or sensitive content detection
    #[error("Privacy policy violation: {violation}")]
    PrivacyError { violation: String },
    
    /// Content size exceeds limits
    #[error("Content size {size} exceeds limit {limit}")]
    SizeError { size: usize, limit: usize },
    
    /// Unsupported content format
    #[error("Unsupported content format: {format}")]
    FormatError { format: String },
    
    /// IO operations failed
    #[error("IO operation failed: {operation}")]
    IoError {
        operation: String,
        #[source]
        source: std::io::Error,
    },
    
    /// Database operations failed
    #[error("Database operation failed: {operation}")]
    DatabaseError {
        operation: String,
        #[source]
        source: rusqlite::Error,
    },
    
    /// Serialization/deserialization errors
    #[error("Serialization error: {context}")]
    SerializationError {
        context: String,
        #[source]
        source: serde_json::Error,
    },
    
    /// Timeout errors
    #[error("Operation timed out: {operation} after {timeout_ms}ms")]
    TimeoutError { operation: String, timeout_ms: u64 },
    
    /// Configuration errors
    #[error("Configuration error: {setting} - {reason}")]
    ConfigError { setting: String, reason: String },
    
    /// Generic internal errors
    #[error("Internal error: {message}")]
    InternalError { message: String },
}

impl ClipboardError {
    /// Create a platform error
    pub fn platform(message: impl Into<String>) -> Self {
        Self::PlatformError {
            message: message.into(),
        }
    }
    
    /// Create a permission error
    pub fn permission(reason: impl Into<String>) -> Self {
        Self::PermissionError {
            reason: reason.into(),
        }
    }
    
    /// Create a content error
    pub fn content(details: impl Into<String>) -> Self {
        Self::ContentError {
            details: details.into(),
        }
    }
    
    /// Create a sync error
    pub fn sync(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::SyncError {
            operation: operation.into(),
            reason: reason.into(),
        }
    }
    
    /// Create a privacy error
    pub fn privacy(violation: impl Into<String>) -> Self {
        Self::PrivacyError {
            violation: violation.into(),
        }
    }
    
    /// Create a security error (maps to sync error for security-related issues)
    pub fn security(message: impl Into<String>) -> Self {
        Self::SyncError {
            operation: "security".into(),
            reason: message.into(),
        }
    }
    
    /// Create a size error
    pub fn size(size: usize, limit: usize) -> Self {
        Self::SizeError { size, limit }
    }
    
    /// Create a format error
    pub fn format(format: impl Into<String>) -> Self {
        Self::FormatError {
            format: format.into(),
        }
    }
    
    /// Create an IO error
    pub fn io(operation: impl Into<String>, source: std::io::Error) -> Self {
        Self::IoError {
            operation: operation.into(),
            source,
        }
    }
    
    /// Create a database error
    pub fn database(operation: impl Into<String>, source: rusqlite::Error) -> Self {
        Self::DatabaseError {
            operation: operation.into(),
            source,
        }
    }
    
    /// Create a serialization error
    pub fn serialization(context: impl Into<String>, source: serde_json::Error) -> Self {
        Self::SerializationError {
            context: context.into(),
            source,
        }
    }
    
    /// Create a timeout error
    pub fn timeout(operation: impl Into<String>, timeout_ms: u64) -> Self {
        Self::TimeoutError {
            operation: operation.into(),
            timeout_ms,
        }
    }
    
    /// Create a config error
    pub fn config(setting: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ConfigError {
            setting: setting.into(),
            reason: reason.into(),
        }
    }
    
    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }
    
    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            ClipboardError::PlatformError { .. } => true,
            ClipboardError::PermissionError { .. } => false,
            ClipboardError::ContentError { .. } => false,
            ClipboardError::SyncError { .. } => true,
            ClipboardError::PrivacyError { .. } => false,
            ClipboardError::SizeError { .. } => false,
            ClipboardError::FormatError { .. } => false,
            ClipboardError::IoError { .. } => true,
            ClipboardError::DatabaseError { .. } => true,
            ClipboardError::SerializationError { .. } => false,
            ClipboardError::TimeoutError { .. } => true,
            ClipboardError::ConfigError { .. } => false,
            ClipboardError::InternalError { .. } => false,
        }
    }
    
    /// Get error category for logging and metrics
    pub fn category(&self) -> &'static str {
        match self {
            ClipboardError::PlatformError { .. } => "platform",
            ClipboardError::PermissionError { .. } => "permission",
            ClipboardError::ContentError { .. } => "content",
            ClipboardError::SyncError { .. } => "sync",
            ClipboardError::PrivacyError { .. } => "privacy",
            ClipboardError::SizeError { .. } => "size",
            ClipboardError::FormatError { .. } => "format",
            ClipboardError::IoError { .. } => "io",
            ClipboardError::DatabaseError { .. } => "database",
            ClipboardError::SerializationError { .. } => "serialization",
            ClipboardError::TimeoutError { .. } => "timeout",
            ClipboardError::ConfigError { .. } => "config",
            ClipboardError::InternalError { .. } => "internal",
        }
    }
}

/// Convert from arboard errors
impl From<arboard::Error> for ClipboardError {
    fn from(err: arboard::Error) -> Self {
        ClipboardError::platform(format!("Arboard error: {}", err))
    }
}

/// Convert from IO errors
impl From<std::io::Error> for ClipboardError {
    fn from(err: std::io::Error) -> Self {
        ClipboardError::io("IO operation", err)
    }
}

/// Convert from database errors
impl From<rusqlite::Error> for ClipboardError {
    fn from(err: rusqlite::Error) -> Self {
        ClipboardError::database("Database operation", err)
    }
}

/// Convert from serialization errors
impl From<serde_json::Error> for ClipboardError {
    fn from(err: serde_json::Error) -> Self {
        ClipboardError::serialization("JSON operation", err)
    }
}

/// Convert from image errors
impl From<image::ImageError> for ClipboardError {
    fn from(err: image::ImageError) -> Self {
        ClipboardError::content(format!("Image processing error: {}", err))
    }
}