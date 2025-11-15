use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use crate::security::error::{SecurityResult, PolicyError};
use crate::security::identity::PeerId;
use super::{
    SecurityPolicy, ConnectionType, SecurityEvent, SecurityEventType,
    PolicyEngine, PrivateModeController, InviteCode, RateLimiter, SecurityAuditor,
    NetworkPolicyEnforcer, AttackDetector,
};

/// Implementation of the security policy engine
pub struct PolicyEngineImpl {
    /// Current security policy
    policy: Arc<RwLock<SecurityPolicy>>,
    /// Private mode controller
    private_mode: Arc<PrivateModeController>,
    /// Network policy enforcer
    network_policy: Arc<NetworkPolicyEnforcer>,
    /// Rate limiter for connection attempts
    rate_limiter: Arc<RateLimiter>,
    /// Attack detector for suspicious patterns
    attack_detector: Arc<AttackDetector>,
    /// Security auditor for event logging
    auditor: Arc<SecurityAuditor>,
}

impl PolicyEngineImpl {
    /// Create a new policy engine with default configuration
    pub fn new() -> Self {
        Self {
            policy: Arc::new(RwLock::new(SecurityPolicy::default())),
            private_mode: Arc::new(PrivateModeController::new()),
            network_policy: Arc::new(NetworkPolicyEnforcer::new()),
            rate_limiter: Arc::new(RateLimiter::new()),
            attack_detector: Arc::new(AttackDetector::new()),
            auditor: Arc::new(SecurityAuditor::new()),
        }
    }
    
    /// Create a new policy engine with custom policy
    pub fn with_policy(policy: SecurityPolicy) -> Self {
        let engine = Self::new();
        
        // Apply initial policy settings
        if policy.private_mode {
            let _ = engine.private_mode.enable();
        }
        
        if policy.local_only_mode {
            let _ = engine.network_policy.enable_local_only();
        }
        
        *engine.policy.write().unwrap() = policy;
        
        engine
    }
    
    /// Check if a connection type is allowed based on local-only mode
    fn check_local_only_mode(&self, connection_type: &ConnectionType) -> SecurityResult<bool> {
        self.network_policy.is_connection_type_allowed(connection_type)?;
        Ok(true)
    }
    
    /// Detect suspicious activity patterns
    fn detect_suspicious_activity(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        // Record the connection attempt
        self.attack_detector.record_connection_attempt(peer_id)?;
        
        // Check for suspicious patterns
        let patterns = self.attack_detector.detect_suspicious_patterns(peer_id)?;
        
        if !patterns.is_empty() {
            let pattern_names: Vec<String> = patterns.iter()
                .map(|p| format!("{:?}", p))
                .collect();
            
            let event = SecurityEvent::new(
                SecurityEventType::SuspiciousActivity,
                Some(peer_id.clone()),
                format!("Suspicious patterns detected: {}", pattern_names.join(", ")),
            );
            self.auditor.log_event(event)?;
            
            // Check if we should block
            if self.attack_detector.should_block(peer_id)? {
                // Block for 1 hour
                self.attack_detector.block_peer(peer_id, 3600)?;
                
                return Err(PolicyError::SuspiciousActivity(
                    format!("Blocked due to suspicious patterns: {}", pattern_names.join(", "))
                ).into());
            }
        }
        
        Ok(false)
    }
    
    /// Perform periodic cleanup tasks
    pub fn cleanup(&self) -> SecurityResult<()> {
        self.rate_limiter.cleanup()?;
        self.private_mode.cleanup_expired_codes()?;
        self.attack_detector.cleanup()?;
        Ok(())
    }
    
    /// Get the private mode controller
    pub fn private_mode_controller(&self) -> Arc<PrivateModeController> {
        Arc::clone(&self.private_mode)
    }
    
    /// Get the rate limiter
    pub fn rate_limiter(&self) -> Arc<RateLimiter> {
        Arc::clone(&self.rate_limiter)
    }
    
    /// Get the security auditor
    pub fn auditor(&self) -> Arc<SecurityAuditor> {
        Arc::clone(&self.auditor)
    }
    
    /// Get the network policy enforcer
    pub fn network_policy_enforcer(&self) -> Arc<NetworkPolicyEnforcer> {
        Arc::clone(&self.network_policy)
    }
    
    /// Get the attack detector
    pub fn attack_detector(&self) -> Arc<AttackDetector> {
        Arc::clone(&self.attack_detector)
    }
}

impl Default for PolicyEngineImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl PolicyEngine for PolicyEngineImpl {
    async fn is_connection_allowed(
        &self,
        peer_id: &PeerId,
        connection_type: ConnectionType,
    ) -> SecurityResult<bool> {
        // Log connection attempt
        let event = SecurityEvent::new(
            SecurityEventType::ConnectionAttempt,
            Some(peer_id.clone()),
            format!("Connection attempt via {:?}", connection_type),
        );
        self.auditor.log_event(event)?;
        
        // Check rate limiting first
        if let Err(e) = self.rate_limiter.check_rate_limit(peer_id) {
            let event = SecurityEvent::new(
                SecurityEventType::RateLimitExceeded,
                Some(peer_id.clone()),
                "Rate limit exceeded".to_string(),
            );
            self.auditor.log_event(event)?;
            return Err(e);
        }
        
        // Check for suspicious activity
        self.detect_suspicious_activity(peer_id)?;
        
        // Check local-only mode restrictions
        self.check_local_only_mode(&connection_type)?;
        
        // Check private mode restrictions
        if let Err(e) = self.private_mode.should_allow_connection(peer_id) {
            let event = SecurityEvent::new(
                SecurityEventType::PolicyViolation,
                Some(peer_id.clone()),
                "Private mode blocked connection".to_string(),
            );
            self.auditor.log_event(event)?;
            return Err(e);
        }
        
        // All checks passed
        let event = SecurityEvent::new(
            SecurityEventType::ConnectionAccepted,
            Some(peer_id.clone()),
            "Connection allowed".to_string(),
        );
        self.auditor.log_event(event)?;
        
        Ok(true)
    }
    
    async fn get_policy(&self) -> SecurityResult<SecurityPolicy> {
        let policy = self.policy.read().unwrap();
        Ok(policy.clone())
    }
    
    async fn update_policy(&self, new_policy: SecurityPolicy) -> SecurityResult<()> {
        let mut policy = self.policy.write().unwrap();
        
        // Update private mode if changed
        if new_policy.private_mode != policy.private_mode {
            if new_policy.private_mode {
                self.private_mode.enable()?;
            } else {
                self.private_mode.disable()?;
            }
        }
        
        *policy = new_policy;
        Ok(())
    }
    
    async fn log_event(&self, event: SecurityEvent) -> SecurityResult<()> {
        self.auditor.log_event(event)
    }
    
    async fn check_rate_limit(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        self.rate_limiter.check_rate_limit(peer_id)
    }
    
    async fn enable_private_mode(&self) -> SecurityResult<()> {
        self.private_mode.enable()?;
        
        let mut policy = self.policy.write().unwrap();
        policy.private_mode = true;
        
        Ok(())
    }
    
    async fn disable_private_mode(&self) -> SecurityResult<()> {
        self.private_mode.disable()?;
        
        let mut policy = self.policy.write().unwrap();
        policy.private_mode = false;
        
        Ok(())
    }
    
    async fn generate_invite_code(&self, peer_id: PeerId) -> SecurityResult<InviteCode> {
        // Default validity: 24 hours
        let validity_secs = 24 * 3600;
        self.private_mode.generate_invite_code(peer_id, validity_secs)
    }
    
    async fn validate_invite_code(&self, code: &str) -> SecurityResult<Option<PeerId>> {
        self.private_mode.validate_invite_code(code)
    }
    
    async fn enable_local_only_mode(&self) -> SecurityResult<()> {
        self.network_policy.enable_local_only()?;
        
        let mut policy = self.policy.write().unwrap();
        policy.local_only_mode = true;
        
        Ok(())
    }
    
    async fn disable_local_only_mode(&self) -> SecurityResult<()> {
        self.network_policy.disable_local_only()?;
        
        let mut policy = self.policy.write().unwrap();
        policy.local_only_mode = false;
        
        Ok(())
    }
    
    async fn get_audit_log(&self, limit: usize) -> SecurityResult<Vec<SecurityEvent>> {
        let entries = self.auditor.get_recent_entries(limit);
        Ok(entries.into_iter().map(|entry| entry.event).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection_allowed_basic() {
        let engine = PolicyEngineImpl::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Should allow connection by default
        let result = engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_private_mode_blocking() {
        let engine = PolicyEngineImpl::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Enable private mode
        engine.enable_private_mode().await.unwrap();
        
        // Should block connection
        let result = engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await;
        assert!(result.is_err());
        
        // Add peer to allowed list
        engine.private_mode.add_allowed_peer(peer_id.clone()).unwrap();
        
        // Should now allow connection
        let result = engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_local_only_mode() {
        let engine = PolicyEngineImpl::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Enable local-only mode
        engine.enable_local_only_mode().await.unwrap();
        
        // Should allow local network connections
        let result = engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await;
        assert!(result.is_ok());
        
        // Should block relay connections
        let result = engine.is_connection_allowed(&peer_id, ConnectionType::Relay).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_rate_limiting() {
        let engine = PolicyEngineImpl::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Make multiple connection attempts
        for _ in 0..5 {
            let _ = engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await;
        }
        
        // Next attempt should fail due to rate limiting
        let result = engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_invite_code_generation() {
        let engine = PolicyEngineImpl::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Generate invite code
        let invite = engine.generate_invite_code(peer_id.clone()).await.unwrap();
        
        assert_eq!(invite.code().len(), 8);
        assert_eq!(invite.peer_id(), &peer_id);
        
        // Validate the code
        let validated = engine.validate_invite_code(invite.code()).await.unwrap();
        assert_eq!(validated, Some(peer_id));
    }
    
    #[tokio::test]
    async fn test_audit_logging() {
        let engine = PolicyEngineImpl::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Make a connection attempt
        let _ = engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await;
        
        // Check audit log
        let log = engine.get_audit_log(10).await.unwrap();
        assert!(!log.is_empty());
    }
    
    #[tokio::test]
    async fn test_policy_update() {
        let engine = PolicyEngineImpl::new();
        
        let mut policy = engine.get_policy().await.unwrap();
        assert!(!policy.private_mode);
        
        policy.private_mode = true;
        engine.update_policy(policy).await.unwrap();
        
        let updated_policy = engine.get_policy().await.unwrap();
        assert!(updated_policy.private_mode);
        assert!(engine.private_mode.is_enabled());
    }
}
