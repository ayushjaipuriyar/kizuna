use std::time::{Duration, SystemTime};
use std::net::SocketAddr;
use std::fmt;
use thiserror::Error;

/// Transport-specific error types with enhanced error handling
#[derive(Debug, Error)]
pub enum TransportError {
    #[error("Connection failed: {reason}")]
    ConnectionFailed { reason: String },
    
    #[error("Protocol not supported: {protocol}")]
    UnsupportedProtocol { protocol: String },
    
    #[error("NAT traversal failed: {method}")]
    NatTraversalFailed { method: String },
    
    #[error("Relay connection failed: {relay_addr}")]
    RelayFailed { relay_addr: SocketAddr },
    
    #[error("Protocol negotiation timeout")]
    NegotiationTimeout,
    
    #[error("Connection timeout after {timeout:?}")]
    ConnectionTimeout { timeout: Duration },
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("QUIC error: {0}")]
    Quic(String),
    
    #[error("WebRTC error: {0}")]
    WebRTC(String),
    
    #[error("WebSocket error: {0}")]
    WebSocket(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Invalid peer address")]
    InvalidPeerAddress,
    
    #[error("Connection already exists")]
    ConnectionExists,
    
    #[error("Connection not found")]
    ConnectionNotFound,
    
    #[error("Transport not available")]
    TransportNotAvailable,
    
    #[error("Resource limit exceeded: {resource}")]
    ResourceLimitExceeded { resource: String },
    
    #[error("Invalid route: {reason}")]
    InvalidRoute { reason: String },
    
    // Enhanced error types for better error handling
    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },
    
    #[error("Certificate validation failed: {reason}")]
    CertificateValidationFailed { reason: String },
    
    #[error("Bandwidth limit exceeded: current={current_bps}, limit={limit_bps}")]
    BandwidthLimitExceeded { current_bps: u64, limit_bps: u64 },
    
    #[error("Connection pool exhausted: active={active}, max={max}")]
    ConnectionPoolExhausted { active: usize, max: usize },
    
    #[error("Protocol version mismatch: local={local}, remote={remote}")]
    ProtocolVersionMismatch { local: String, remote: String },
    
    #[error("Network unreachable: {target}")]
    NetworkUnreachable { target: SocketAddr },
    
    #[error("Graceful shutdown in progress")]
    ShutdownInProgress,
    
    #[error("Configuration error: {field} - {reason}")]
    ConfigurationError { field: String, reason: String },
}

impl TransportError {
    /// Check if this error is recoverable with retry
    pub fn is_recoverable(&self) -> bool {
        match self {
            TransportError::ConnectionTimeout { .. } => true,
            TransportError::NatTraversalFailed { .. } => true,
            TransportError::RelayFailed { .. } => true,
            TransportError::NetworkUnreachable { .. } => true,
            TransportError::BandwidthLimitExceeded { .. } => true,
            TransportError::ConnectionPoolExhausted { .. } => true,
            TransportError::Io(io_err) => match io_err.kind() {
                std::io::ErrorKind::TimedOut => true,
                std::io::ErrorKind::ConnectionRefused => true,
                std::io::ErrorKind::ConnectionReset => true,
                std::io::ErrorKind::ConnectionAborted => true,
                std::io::ErrorKind::NotConnected => true,
                std::io::ErrorKind::WouldBlock => true,
                std::io::ErrorKind::Interrupted => true,
                _ => false,
            },
            _ => false,
        }
    }

    /// Get error severity level
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            TransportError::UnsupportedProtocol { .. } => ErrorSeverity::Warning,
            TransportError::TransportNotAvailable => ErrorSeverity::Warning,
            TransportError::ConnectionTimeout { .. } => ErrorSeverity::Warning,
            TransportError::BandwidthLimitExceeded { .. } => ErrorSeverity::Warning,
            TransportError::ConnectionPoolExhausted { .. } => ErrorSeverity::Warning,
            TransportError::NetworkUnreachable { .. } => ErrorSeverity::Warning,
            TransportError::NegotiationTimeout => ErrorSeverity::Error,
            TransportError::ResourceLimitExceeded { .. } => ErrorSeverity::Error,
            TransportError::AuthenticationFailed { .. } => ErrorSeverity::Error,
            TransportError::CertificateValidationFailed { .. } => ErrorSeverity::Error,
            TransportError::ProtocolVersionMismatch { .. } => ErrorSeverity::Error,
            TransportError::ConfigurationError { .. } => ErrorSeverity::Error,
            TransportError::ShutdownInProgress => ErrorSeverity::Info,
            _ => ErrorSeverity::Critical,
        }
    }

    /// Get suggested retry strategy for this error
    pub fn retry_strategy(&self) -> RetryStrategy {
        match self {
            TransportError::ConnectionTimeout { .. } => RetryStrategy::ExponentialBackoff {
                initial_delay: Duration::from_millis(100),
                max_delay: Duration::from_secs(30),
                max_attempts: 5,
            },
            TransportError::NatTraversalFailed { .. } => RetryStrategy::LinearBackoff {
                delay: Duration::from_secs(2),
                max_attempts: 3,
            },
            TransportError::RelayFailed { .. } => RetryStrategy::ExponentialBackoff {
                initial_delay: Duration::from_millis(500),
                max_delay: Duration::from_secs(60),
                max_attempts: 3,
            },
            TransportError::BandwidthLimitExceeded { .. } => RetryStrategy::LinearBackoff {
                delay: Duration::from_secs(1),
                max_attempts: 10,
            },
            TransportError::ConnectionPoolExhausted { .. } => RetryStrategy::LinearBackoff {
                delay: Duration::from_millis(100),
                max_attempts: 20,
            },
            TransportError::NetworkUnreachable { .. } => RetryStrategy::ExponentialBackoff {
                initial_delay: Duration::from_secs(1),
                max_delay: Duration::from_secs(120),
                max_attempts: 5,
            },
            _ if self.is_recoverable() => RetryStrategy::ExponentialBackoff {
                initial_delay: Duration::from_millis(200),
                max_delay: Duration::from_secs(10),
                max_attempts: 3,
            },
            _ => RetryStrategy::NoRetry,
        }
    }

    /// Get error category for metrics and logging
    pub fn category(&self) -> ErrorCategory {
        match self {
            TransportError::ConnectionFailed { .. } |
            TransportError::ConnectionTimeout { .. } |
            TransportError::NetworkUnreachable { .. } => ErrorCategory::Connection,
            
            TransportError::UnsupportedProtocol { .. } |
            TransportError::NegotiationTimeout |
            TransportError::ProtocolVersionMismatch { .. } => ErrorCategory::Protocol,
            
            TransportError::NatTraversalFailed { .. } |
            TransportError::RelayFailed { .. } => ErrorCategory::Network,
            
            TransportError::AuthenticationFailed { .. } |
            TransportError::CertificateValidationFailed { .. } => ErrorCategory::Security,
            
            TransportError::ResourceLimitExceeded { .. } |
            TransportError::BandwidthLimitExceeded { .. } |
            TransportError::ConnectionPoolExhausted { .. } => ErrorCategory::Resource,
            
            TransportError::ConfigurationError { .. } => ErrorCategory::Configuration,
            
            TransportError::Serialization(_) => ErrorCategory::Data,
            
            _ => ErrorCategory::Other,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Info => write!(f, "INFO"),
            ErrorSeverity::Warning => write!(f, "WARN"),
            ErrorSeverity::Error => write!(f, "ERROR"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Retry strategy for error recovery
#[derive(Debug, Clone, PartialEq)]
pub enum RetryStrategy {
    NoRetry,
    LinearBackoff {
        delay: Duration,
        max_attempts: u32,
    },
    ExponentialBackoff {
        initial_delay: Duration,
        max_delay: Duration,
        max_attempts: u32,
    },
}

impl RetryStrategy {
    /// Calculate delay for the given attempt number (0-based)
    pub fn delay_for_attempt(&self, attempt: u32) -> Option<Duration> {
        match self {
            RetryStrategy::NoRetry => None,
            RetryStrategy::LinearBackoff { delay, max_attempts } => {
                if attempt < *max_attempts {
                    Some(*delay)
                } else {
                    None
                }
            },
            RetryStrategy::ExponentialBackoff { initial_delay, max_delay, max_attempts } => {
                if attempt < *max_attempts {
                    let delay = *initial_delay * 2_u32.pow(attempt);
                    Some(delay.min(*max_delay))
                } else {
                    None
                }
            },
        }
    }

    /// Check if retry is allowed for the given attempt
    pub fn should_retry(&self, attempt: u32) -> bool {
        self.delay_for_attempt(attempt).is_some()
    }
}

/// Error categories for metrics and analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    Connection,
    Protocol,
    Network,
    Security,
    Resource,
    Configuration,
    Data,
    Other,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Connection => write!(f, "connection"),
            ErrorCategory::Protocol => write!(f, "protocol"),
            ErrorCategory::Network => write!(f, "network"),
            ErrorCategory::Security => write!(f, "security"),
            ErrorCategory::Resource => write!(f, "resource"),
            ErrorCategory::Configuration => write!(f, "configuration"),
            ErrorCategory::Data => write!(f, "data"),
            ErrorCategory::Other => write!(f, "other"),
        }
    }
}

/// Error context for enhanced debugging and logging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub timestamp: SystemTime,
    pub peer_id: Option<String>,
    pub protocol: Option<String>,
    pub local_addr: Option<SocketAddr>,
    pub remote_addr: Option<SocketAddr>,
    pub attempt_number: u32,
    pub operation: String,
    pub additional_info: std::collections::HashMap<String, String>,
}

impl ErrorContext {
    pub fn new(operation: String) -> Self {
        Self {
            timestamp: SystemTime::now(),
            peer_id: None,
            protocol: None,
            local_addr: None,
            remote_addr: None,
            attempt_number: 0,
            operation,
            additional_info: std::collections::HashMap::new(),
        }
    }

    pub fn with_peer_id(mut self, peer_id: String) -> Self {
        self.peer_id = Some(peer_id);
        self
    }

    pub fn with_protocol(mut self, protocol: String) -> Self {
        self.protocol = Some(protocol);
        self
    }

    pub fn with_addresses(mut self, local: SocketAddr, remote: SocketAddr) -> Self {
        self.local_addr = Some(local);
        self.remote_addr = Some(remote);
        self
    }

    pub fn with_attempt(mut self, attempt: u32) -> Self {
        self.attempt_number = attempt;
        self
    }

    pub fn add_info<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.additional_info.insert(key.into(), value.into());
        self
    }
}

/// Enhanced error with context for better debugging
#[derive(Debug)]
pub struct ContextualError {
    pub error: TransportError,
    pub context: ErrorContext,
}

impl ContextualError {
    pub fn new(error: TransportError, context: ErrorContext) -> Self {
        Self { error, context }
    }

    /// Create a formatted log message
    pub fn log_message(&self) -> String {
        let mut msg = format!(
            "[{}] {} - {} (attempt {})",
            self.error.severity(),
            self.context.operation,
            self.error,
            self.context.attempt_number + 1
        );

        if let Some(peer_id) = &self.context.peer_id {
            msg.push_str(&format!(" | peer: {}", peer_id));
        }

        if let Some(protocol) = &self.context.protocol {
            msg.push_str(&format!(" | protocol: {}", protocol));
        }

        if let (Some(local), Some(remote)) = (&self.context.local_addr, &self.context.remote_addr) {
            msg.push_str(&format!(" | {}→{}", local, remote));
        }

        if !self.context.additional_info.is_empty() {
            let info: Vec<String> = self.context.additional_info
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            msg.push_str(&format!(" | {}", info.join(", ")));
        }

        msg
    }
}

impl Clone for TransportError {
    fn clone(&self) -> Self {
        match self {
            TransportError::ConnectionFailed { reason } => TransportError::ConnectionFailed { reason: reason.clone() },
            TransportError::UnsupportedProtocol { protocol } => TransportError::UnsupportedProtocol { protocol: protocol.clone() },
            TransportError::NatTraversalFailed { method } => TransportError::NatTraversalFailed { method: method.clone() },
            TransportError::RelayFailed { relay_addr } => TransportError::RelayFailed { relay_addr: *relay_addr },
            TransportError::NegotiationTimeout => TransportError::NegotiationTimeout,
            TransportError::ConnectionTimeout { timeout } => TransportError::ConnectionTimeout { timeout: *timeout },
            TransportError::Io(io_err) => TransportError::Io(std::io::Error::new(io_err.kind(), io_err.to_string())),
            TransportError::Quic(msg) => TransportError::Quic(msg.clone()),
            TransportError::WebRTC(msg) => TransportError::WebRTC(msg.clone()),
            TransportError::WebSocket(msg) => TransportError::WebSocket(msg.clone()),
            TransportError::Serialization(msg) => TransportError::Serialization(msg.clone()),
            TransportError::InvalidPeerAddress => TransportError::InvalidPeerAddress,
            TransportError::ConnectionExists => TransportError::ConnectionExists,
            TransportError::ConnectionNotFound => TransportError::ConnectionNotFound,
            TransportError::TransportNotAvailable => TransportError::TransportNotAvailable,
            TransportError::ResourceLimitExceeded { resource } => TransportError::ResourceLimitExceeded { resource: resource.clone() },
            TransportError::InvalidRoute { reason } => TransportError::InvalidRoute { reason: reason.clone() },
            TransportError::AuthenticationFailed { reason } => TransportError::AuthenticationFailed { reason: reason.clone() },
            TransportError::CertificateValidationFailed { reason } => TransportError::CertificateValidationFailed { reason: reason.clone() },
            TransportError::BandwidthLimitExceeded { current_bps, limit_bps } => TransportError::BandwidthLimitExceeded { current_bps: *current_bps, limit_bps: *limit_bps },
            TransportError::ConnectionPoolExhausted { active, max } => TransportError::ConnectionPoolExhausted { active: *active, max: *max },
            TransportError::ProtocolVersionMismatch { local, remote } => TransportError::ProtocolVersionMismatch { local: local.clone(), remote: remote.clone() },
            TransportError::NetworkUnreachable { target } => TransportError::NetworkUnreachable { target: *target },
            TransportError::ShutdownInProgress => TransportError::ShutdownInProgress,
            TransportError::ConfigurationError { field, reason } => TransportError::ConfigurationError { field: field.clone(), reason: reason.clone() },
        }
    }
}