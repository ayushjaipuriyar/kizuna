use thiserror::Error;
use std::io;

/// Result type for command execution operations
pub type CommandResult<T> = std::result::Result<T, CommandError>;

/// Errors that can occur during command execution
#[derive(Error, Debug)]
pub enum CommandError {
    /// Authorization was denied for the command
    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),

    /// Authorization request timed out
    #[error("Authorization timeout")]
    AuthorizationTimeout,

    /// Sandbox creation or configuration failed
    #[error("Sandbox error: {0}")]
    SandboxError(String),

    /// Command execution failed
    #[error("Execution error: {0}")]
    ExecutionError(String),

    /// Insufficient permissions for command execution
    #[error("Permission denied: {0}")]
    PermissionError(String),

    /// Platform-specific execution environment issues
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// Command execution timed out
    #[error("Command timeout after {0:?}")]
    Timeout(std::time::Duration),

    /// Resource limit exceeded
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    /// Script parsing or validation error
    #[error("Script error: {0}")]
    ScriptError(String),

    /// System information query failed
    #[error("System info error: {0}")]
    SystemInfoError(String),

    /// Notification delivery failed
    #[error("Notification error: {0}")]
    NotificationError(String),

    /// IO error occurred
    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Security-related error
    #[error("Security error: {0}")]
    SecurityError(String),

    /// Transport-related error
    #[error("Transport error: {0}")]
    TransportError(String),

    /// Invalid command or request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Command not found
    #[error("Command not found: {0}")]
    CommandNotFound(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Template not found
    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    /// Template validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Schedule error
    #[error("Schedule error: {0}")]
    ScheduleError(String),
}

impl CommandError {
    /// Create a new authorization denied error
    pub fn authorization_denied(reason: impl Into<String>) -> Self {
        Self::AuthorizationDenied(reason.into())
    }

    /// Create a new sandbox error
    pub fn sandbox_error(reason: impl Into<String>) -> Self {
        Self::SandboxError(reason.into())
    }

    /// Create a new execution error
    pub fn execution_error(reason: impl Into<String>) -> Self {
        Self::ExecutionError(reason.into())
    }

    /// Create a new permission error
    pub fn permission_error(reason: impl Into<String>) -> Self {
        Self::PermissionError(reason.into())
    }

    /// Create a new platform error
    pub fn platform_error(reason: impl Into<String>) -> Self {
        Self::PlatformError(reason.into())
    }

    /// Create a new script error
    pub fn script_error(reason: impl Into<String>) -> Self {
        Self::ScriptError(reason.into())
    }

    /// Create a new invalid request error
    pub fn invalid_request(reason: impl Into<String>) -> Self {
        Self::InvalidRequest(reason.into())
    }

    /// Create a new security error
    pub fn security_error(reason: impl Into<String>) -> Self {
        Self::SecurityError(reason.into())
    }

    /// Create a new transport error
    pub fn transport_error(reason: impl Into<String>) -> Self {
        Self::TransportError(reason.into())
    }
}

impl From<serde_json::Error> for CommandError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}
