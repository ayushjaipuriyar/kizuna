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