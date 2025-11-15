use thiserror::Error;

/// Result type for security operations
pub type SecurityResult<T> = Result<T, SecurityError>;

/// Comprehensive error types for the security system
#[derive(Error, Debug)]
pub enum SecurityError {
    /// Errors related to identity management
    #[error("Identity error: {0}")]
    Identity(#[from] IdentityError),
    
    /// Errors related to trust management
    #[error("Trust error: {0}")]
    Trust(#[from] TrustError),
    
    /// Errors related to encryption operations
    #[error("Encryption error: {0}")]
    Encryption(#[from] EncryptionError),
    
    /// Errors related to security policy violations
    #[error("Policy error: {0}")]
    Policy(#[from] PolicyError),
    
    /// Errors related to authentication
    #[error("Authentication error: {0}")]
    Authentication(#[from] AuthenticationError),
    
    /// Security policy violation
    #[error("Policy violation: {0}")]
    PolicyViolation(String),
    
    /// Generic security error
    #[error("Security error: {0}")]
    Generic(String),
    
    /// Other security errors
    #[error("{0}")]
    Other(String),
}

/// Identity-related errors
#[derive(Error, Debug)]
pub enum IdentityError {
    #[error("Failed to generate device identity: {0}")]
    GenerationFailed(String),
    
    #[error("Failed to load identity from storage: {0}")]
    LoadFailed(String),
    
    #[error("Failed to save identity to storage: {0}")]
    SaveFailed(String),
    
    #[error("Identity corrupted or invalid: {0}")]
    Corrupted(String),
    
    #[error("Keystore error: {0}")]
    KeystoreError(String),
    
    #[error("Invalid peer ID format: {0}")]
    InvalidPeerId(String),
}

/// Trust management errors
#[derive(Error, Debug)]
pub enum TrustError {
    #[error("Peer not found in trust list: {0}")]
    PeerNotFound(String),
    
    #[error("Pairing verification failed: {0}")]
    PairingFailed(String),
    
    #[error("Pairing code expired")]
    PairingExpired,
    
    #[error("Trust database error: {0}")]
    DatabaseError(String),
    
    #[error("Peer not trusted: {0}")]
    NotTrusted(String),
    
    #[error("Invalid pairing code")]
    InvalidPairingCode,
}

/// Encryption-related errors
#[derive(Error, Debug)]
pub enum EncryptionError {
    #[error("Key exchange failed: {0}")]
    KeyExchangeFailed(String),
    
    #[error("Encryption operation failed: {0}")]
    EncryptionFailed(String),
    
    #[error("Decryption operation failed: {0}")]
    DecryptionFailed(String),
    
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    
    #[error("Session expired: {0}")]
    SessionExpired(String),
    
    #[error("Invalid nonce or authentication tag")]
    AuthenticationFailed,
    
    #[error("Key rotation failed: {0}")]
    KeyRotationFailed(String),
}

/// Security policy errors
#[derive(Error, Debug)]
pub enum PolicyError {
    #[error("Connection blocked by private mode")]
    PrivateModeBlocked,
    
    #[error("Connection blocked by local-only mode")]
    LocalOnlyBlocked,
    
    #[error("Peer not in allowlist")]
    NotInAllowlist,
    
    #[error("Access denied: insufficient permissions")]
    AccessDenied,
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Suspicious activity detected: {0}")]
    SuspiciousActivity(String),
}

/// Authentication errors
#[derive(Error, Debug)]
pub enum AuthenticationError {
    #[error("Peer authentication failed: {0}")]
    Failed(String),
    
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Peer identity verification failed")]
    VerificationFailed,
    
    #[error("MITM attack detected")]
    MitmDetected,
}

impl From<std::io::Error> for SecurityError {
    fn from(err: std::io::Error) -> Self {
        SecurityError::Generic(format!("I/O error: {}", err))
    }
}

impl From<serde_json::Error> for SecurityError {
    fn from(err: serde_json::Error) -> Self {
        SecurityError::Generic(format!("Serialization error: {}", err))
    }
}
