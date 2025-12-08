//! Browser Support Error Types

use thiserror::Error;

/// Browser support system errors
#[derive(Error, Debug)]
pub enum BrowserSupportError {
    #[error("WebRTC connection failed: {reason}")]
    WebRTCError { reason: String },
    
    #[error("Browser compatibility error: {browser} - {issue}")]
    BrowserCompatibilityError { browser: String, issue: String },
    
    #[error("Security error: {message}")]
    SecurityError { message: String },
    
    #[error("API error: {endpoint} - {error}")]
    APIError { endpoint: String, error: String },
    
    #[error("Network error: {details}")]
    NetworkError { details: String },
    
    #[error("PWA error: {operation} failed - {reason}")]
    PWAError { operation: String, reason: String },
    
    #[error("Session error: {session_id} - {error}")]
    SessionError { session_id: String, error: String },
    
    #[error("Configuration error: {parameter} - {issue}")]
    ConfigurationError { parameter: String, issue: String },
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    
    #[error("Certificate validation failed: {0}")]
    CertificateValidationFailed(String),
    
    #[error("HTTPS required: {0}")]
    HTTPSRequired(String),
    
    #[error("Security policy violation: {0}")]
    SecurityPolicyViolation(String),
    
    #[error("Integration error with {system}: {message}")]
    IntegrationError { system: String, message: String },
}

impl BrowserSupportError {
    /// Create an integration error
    pub fn integration(system: impl Into<String>, message: impl Into<String>) -> Self {
        Self::IntegrationError {
            system: system.into(),
            message: message.into(),
        }
    }
    
    /// Create a session not found error
    pub fn session_not_found(session_id: impl Into<String>) -> Self {
        Self::SessionNotFound(session_id.into())
    }
    
    /// Create a permission denied error
    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::PermissionDenied(message.into())
    }
    
    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::SessionNotFound(message.into())
    }
    
    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::ConfigurationError {
            parameter: "validation".to_string(),
            issue: message.into(),
        }
    }
    
    /// Create a not implemented error
    pub fn not_implemented(feature: impl Into<String>) -> Self {
        Self::BrowserCompatibilityError {
            browser: "unknown".to_string(),
            issue: format!("Feature not implemented: {}", feature.into()),
        }
    }
}

impl From<webrtc::Error> for BrowserSupportError {
    fn from(err: webrtc::Error) -> Self {
        BrowserSupportError::WebRTCError {
            reason: err.to_string(),
        }
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for BrowserSupportError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        BrowserSupportError::NetworkError {
            details: err.to_string(),
        }
    }
}

/// Result type for browser support operations
pub type BrowserResult<T> = std::result::Result<T, BrowserSupportError>;