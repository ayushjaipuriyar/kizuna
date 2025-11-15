mod engine;
mod private_mode;
mod rate_limiter;
mod audit;
mod network_policy;
mod attack_detector;

pub use engine::PolicyEngineImpl;
pub use private_mode::{PrivateModeController, InviteCode};
pub use rate_limiter::RateLimiter;
pub use audit::{SecurityAuditor, AuditLog};
pub use network_policy::{NetworkPolicyEnforcer, NetworkMode};
pub use attack_detector::{AttackDetector, SuspiciousPattern, AttackDetectorConfig};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::security::error::SecurityResult;
use crate::security::identity::PeerId;

/// Security policy configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub private_mode: bool,
    pub local_only_mode: bool,
    pub require_pairing: bool,
    pub auto_accept_trusted: bool,
    pub session_timeout: Duration,
    pub key_rotation_interval: Duration,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            private_mode: false,
            local_only_mode: false,
            require_pairing: true,
            auto_accept_trusted: true,
            session_timeout: Duration::from_secs(3600), // 1 hour
            key_rotation_interval: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Connection type for policy enforcement
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionType {
    LocalNetwork,
    Relay,
    Direct,
}

/// Security event for audit logging
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub timestamp: u64,
    pub event_type: SecurityEventType,
    pub peer_id: Option<PeerId>,
    pub details: String,
}

impl SecurityEvent {
    pub fn new(event_type: SecurityEventType, peer_id: Option<PeerId>, details: String) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            timestamp,
            event_type,
            peer_id,
            details,
        }
    }
}

/// Types of security events
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SecurityEventType {
    ConnectionAttempt,
    ConnectionAccepted,
    ConnectionRejected,
    PairingAttempt,
    PairingSuccess,
    PairingFailure,
    RateLimitExceeded,
    SuspiciousActivity,
    PolicyViolation,
}

/// Security policy engine trait
#[async_trait]
pub trait PolicyEngine: Send + Sync {
    /// Check if a connection is allowed
    async fn is_connection_allowed(
        &self,
        peer_id: &PeerId,
        connection_type: ConnectionType,
    ) -> SecurityResult<bool>;
    
    /// Get the current security policy
    async fn get_policy(&self) -> SecurityResult<SecurityPolicy>;
    
    /// Update the security policy
    async fn update_policy(&self, policy: SecurityPolicy) -> SecurityResult<()>;
    
    /// Log a security event
    async fn log_event(&self, event: SecurityEvent) -> SecurityResult<()>;
    
    /// Check rate limiting for a peer
    async fn check_rate_limit(&self, peer_id: &PeerId) -> SecurityResult<bool>;
    
    /// Enable private mode
    async fn enable_private_mode(&self) -> SecurityResult<()>;
    
    /// Disable private mode
    async fn disable_private_mode(&self) -> SecurityResult<()>;
    
    /// Generate an invite code for private mode
    async fn generate_invite_code(&self, peer_id: PeerId) -> SecurityResult<InviteCode>;
    
    /// Validate an invite code
    async fn validate_invite_code(&self, code: &str) -> SecurityResult<Option<PeerId>>;
    
    /// Enable local-only mode
    async fn enable_local_only_mode(&self) -> SecurityResult<()>;
    
    /// Disable local-only mode
    async fn disable_local_only_mode(&self) -> SecurityResult<()>;
    
    /// Get audit log
    async fn get_audit_log(&self, limit: usize) -> SecurityResult<Vec<SecurityEvent>>;
}
