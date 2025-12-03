// Streaming error types and result aliases

use thiserror::Error;

/// Result type for streaming operations
pub type StreamResult<T> = Result<T, StreamError>;

/// Comprehensive error types for the streaming system
/// 
/// Covers all error scenarios including hardware failures, network issues,
/// permission problems, and resource constraints.
#[derive(Debug, Error)]
pub enum StreamError {
    /// Camera or screen capture hardware failures
    #[error("Capture error: {0}")]
    Capture(String),
    
    /// Camera or screen access permission denied
    #[error("Permission denied: {0}")]
    Permission(String),
    
    /// Video encoding or decoding failures
    #[error("Encoding error: {0}")]
    Encoding(String),
    
    /// Video decoding failures
    #[error("Decoding error: {0}")]
    Decoding(String),
    
    /// Hardware acceleration failures
    #[error("Hardware acceleration error: {0}")]
    HardwareAcceleration(String),
    
    /// Stream transmission and connectivity issues
    #[error("Network error: {0}")]
    Network(String),
    
    /// Insufficient CPU, memory, or bandwidth
    #[error("Resource error: {0}")]
    Resource(String),
    
    /// Stream session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    
    /// Viewer management errors
    #[error("Viewer error: {0}")]
    Viewer(String),
    
    /// Recording errors
    #[error("Recording error: {0}")]
    Recording(String),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// Device not found or unavailable
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    
    /// Unsupported operation or feature
    #[error("Unsupported: {0}")]
    Unsupported(String),
    
    /// Invalid state for operation
    #[error("Invalid state: {0}")]
    InvalidState(String),
    
    /// Initialization error
    #[error("Initialization error: {0}")]
    Initialization(String),
    
    /// Timeout during operation
    #[error("Timeout: {0}")]
    Timeout(String),
    
    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl StreamError {
    /// Create a capture error
    pub fn capture(msg: impl Into<String>) -> Self {
        Self::Capture(msg.into())
    }
    
    /// Create a permission error
    pub fn permission(msg: impl Into<String>) -> Self {
        Self::Permission(msg.into())
    }
    
    /// Create an encoding error
    pub fn encoding(msg: impl Into<String>) -> Self {
        Self::Encoding(msg.into())
    }
    
    /// Create a decoding error
    pub fn decoding(msg: impl Into<String>) -> Self {
        Self::Decoding(msg.into())
    }
    
    /// Create a network error
    pub fn network(msg: impl Into<String>) -> Self {
        Self::Network(msg.into())
    }
    
    /// Create a resource error
    pub fn resource(msg: impl Into<String>) -> Self {
        Self::Resource(msg.into())
    }
    
    /// Create a session not found error
    pub fn session_not_found(session_id: impl std::fmt::Display) -> Self {
        Self::SessionNotFound(format!("{}", session_id))
    }
    
    /// Create a viewer error
    pub fn viewer(msg: impl Into<String>) -> Self {
        Self::Viewer(msg.into())
    }
    
    /// Create a recording error
    pub fn recording(msg: impl Into<String>) -> Self {
        Self::Recording(msg.into())
    }
    
    /// Create a configuration error
    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }
    
    /// Create a device not found error
    pub fn device_not_found(device: impl Into<String>) -> Self {
        Self::DeviceNotFound(device.into())
    }
    
    /// Create an unsupported error
    pub fn unsupported(msg: impl Into<String>) -> Self {
        Self::Unsupported(msg.into())
    }
    
    /// Create an invalid state error
    pub fn invalid_state(msg: impl Into<String>) -> Self {
        Self::InvalidState(msg.into())
    }
    
    /// Create a timeout error
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self::Timeout(msg.into())
    }
    
    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
    
    /// Create an initialization error
    pub fn initialization(msg: impl Into<String>) -> Self {
        Self::Initialization(msg.into())
    }
}
