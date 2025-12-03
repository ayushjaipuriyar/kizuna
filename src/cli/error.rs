// CLI error types and handling

use thiserror::Error;

/// Result type for CLI operations
pub type CLIResult<T> = Result<T, CLIError>;

/// CLI-specific error types
#[derive(Error, Debug)]
pub enum CLIError {
    /// Command-line argument parsing and validation failures
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Configuration file parsing and validation errors
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Terminal UI rendering and interaction failures
    #[error("TUI error: {0}")]
    TUIError(String),

    /// Integration with Kizuna core system failures
    #[error("Integration error: {0}")]
    IntegrationError(String),

    /// File system and terminal I/O errors
    #[error("I/O error: {0}")]
    IOError(#[from] std::io::Error),

    /// Invalid command or subcommand
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    /// Missing required argument
    #[error("Missing required argument: {0}")]
    MissingArgument(String),

    /// Invalid argument value
    #[error("Invalid argument value for {arg}: {reason}")]
    InvalidArgumentValue { arg: String, reason: String },

    /// Command execution failed
    #[error("Command execution failed: {0}")]
    ExecutionError(String),

    /// Operation cancelled by user
    #[error("Operation cancelled")]
    Cancelled,

    /// Output formatting errors
    #[error("Format error: {0}")]
    FormatError(String),

    /// Generic error with context
    #[error("{0}")]
    Other(String),
}

impl CLIError {
    /// Create a parse error with context
    pub fn parse(msg: impl Into<String>) -> Self {
        CLIError::ParseError(msg.into())
    }

    /// Create a config error with context
    pub fn config(msg: impl Into<String>) -> Self {
        CLIError::ConfigError(msg.into())
    }

    /// Create a TUI error with context
    pub fn tui(msg: impl Into<String>) -> Self {
        CLIError::TUIError(msg.into())
    }

    /// Create an integration error with context
    pub fn integration(msg: impl Into<String>) -> Self {
        CLIError::IntegrationError(msg.into())
    }

    /// Create an execution error with context
    pub fn execution(msg: impl Into<String>) -> Self {
        CLIError::ExecutionError(msg.into())
    }

    /// Create a generic error with context
    pub fn other(msg: impl Into<String>) -> Self {
        CLIError::Other(msg.into())
    }

    /// Create a discovery error with context
    pub fn discovery(msg: impl Into<String>) -> Self {
        CLIError::IntegrationError(format!("Discovery: {}", msg.into()))
    }

    /// Create a transfer error with context
    pub fn transfer(msg: impl Into<String>) -> Self {
        CLIError::IntegrationError(format!("Transfer: {}", msg.into()))
    }

    /// Create a clipboard error with context
    pub fn clipboard(msg: impl Into<String>) -> Self {
        CLIError::IntegrationError(format!("Clipboard: {}", msg.into()))
    }

    /// Create a streaming error with context
    pub fn streaming(msg: impl Into<String>) -> Self {
        CLIError::IntegrationError(format!("Streaming: {}", msg.into()))
    }

    /// Create a file not found error
    pub fn file_not_found(path: impl Into<String>) -> Self {
        CLIError::IOError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File not found: {}", path.into()),
        ))
    }

    /// Create a batch operation error with context
    pub fn batch_operation(msg: impl Into<String>) -> Self {
        CLIError::IntegrationError(format!("Batch operation: {}", msg.into()))
    }

    /// Create a security error with context
    pub fn security(msg: impl Into<String>) -> Self {
        CLIError::IntegrationError(format!("Security: {}", msg.into()))
    }

    /// Create a not found error
    pub fn not_found(msg: impl Into<String>) -> Self {
        CLIError::Other(format!("Not found: {}", msg.into()))
    }
}
