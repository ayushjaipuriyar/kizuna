use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::security::{Security, SecurityResult, SecurityError};
use crate::security::identity::{
    DeviceIdentity, PeerId, DisposableIdentity, IdentityStore, DisposableIdentityManager,
};
use crate::security::encryption::{EncryptionEngine, EncryptionEngineImpl, SessionId};
use crate::security::trust::{
    TrustManager, TrustManagerImpl, TrustEntry, PairingCode, ServicePermissions, TrustLevel,
};
use crate::security::policy::{
    PolicyEngine, PolicyEngineImpl, SecurityPolicy, ConnectionType, SecurityEvent, InviteCode,
};

/// Unified security system implementation
pub struct SecuritySystem {
    /// Identity store for device identity
    identity_store: IdentityStore,
    /// Disposable identity manager
    disposable_manager: Arc<DisposableIdentityManager>,
    /// Encryption engine
    encryption_engine: Arc<EncryptionEngineImpl>,
    /// Trust manager
    trust_manager: Arc<TrustManagerImpl>,
    /// Policy engine
    policy_engine: Arc<PolicyEngineImpl>,
}

impl SecuritySystem {
    /// Create a new security system with default configuration
    pub fn new() -> SecurityResult<Self> {
        Self::with_config(SecuritySystemConfig::default())
    }
    
    /// Create a new security system with custom configuration
    pub fn with_config(config: SecuritySystemConfig) -> SecurityResult<Self> {
        // Initialize identity store
        let identity_store = if let Some(service_name) = config.keystore_service_name {
            IdentityStore::new(service_name, whoami::username())
        } else {
            IdentityStore::default()
        };
        
        // Initialize disposable identity manager
        let disposable_manager = Arc::new(DisposableIdentityManager::new(
            config.disposable_identity_lifetime.as_secs()
        ));
        
        // Initialize encryption engine
        let encryption_engine = Arc::new(EncryptionEngineImpl::new(
            config.session_timeout,
            config.key_rotation_interval,
        ));
        
        // Initialize trust manager
        let trust_db_path = config.trust_db_path.unwrap_or_else(|| {
            let mut path = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
            path.push("kizuna");
            path.push("trust.db");
            path
        });
        
        // Ensure parent directory exists
        if let Some(parent) = trust_db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SecurityError::Other(format!("Failed to create trust database directory: {}", e))
            })?;
        }
        
        let trust_manager = Arc::new(TrustManagerImpl::new(trust_db_path)?);
        
        // Initialize policy engine
        let policy_engine = Arc::new(PolicyEngineImpl::with_policy(config.security_policy));
        
        Ok(Self {
            identity_store,
            disposable_manager,
            encryption_engine,
            trust_manager,
            policy_engine,
        })
    }
    
    /// Get the encryption engine
    pub fn encryption_engine(&self) -> Arc<EncryptionEngineImpl> {
        Arc::clone(&self.encryption_engine)
    }
    
    /// Get the trust manager
    pub fn trust_manager(&self) -> Arc<TrustManagerImpl> {
        Arc::clone(&self.trust_manager)
    }
    
    /// Get the policy engine
    pub fn policy_engine(&self) -> Arc<PolicyEngineImpl> {
        Arc::clone(&self.policy_engine)
    }
    
    /// Get or create device identity
    pub async fn get_or_create_identity(&self) -> SecurityResult<DeviceIdentity> {
        self.identity_store.get_or_create_identity()
    }
    
    /// Create a disposable identity
    pub async fn create_disposable_identity(&self) -> SecurityResult<DisposableIdentity> {
        self.disposable_manager.create_identity().await
    }
    
    /// Activate a disposable identity
    pub async fn activate_disposable_identity(&self, id: &str) -> SecurityResult<()> {
        self.disposable_manager.activate_identity(id).await
    }
    
    /// Get the currently active disposable identity
    pub async fn get_active_disposable_identity(&self) -> SecurityResult<Option<DisposableIdentity>> {
        self.disposable_manager.get_active_identity().await
    }
    
    /// Cleanup expired disposable identities
    pub async fn cleanup_expired_identities(&self) -> SecurityResult<usize> {
        self.disposable_manager.cleanup_expired().await
    }
    
    /// Add a trusted peer
    pub async fn add_trusted_peer(&self, peer_id: PeerId, nickname: String) -> SecurityResult<()> {
        self.trust_manager.add_trusted_peer(peer_id, nickname).await
    }
    
    /// Remove a trusted peer
    pub async fn remove_trusted_peer(&self, peer_id: &PeerId) -> SecurityResult<()> {
        self.trust_manager.remove_trusted_peer(peer_id).await
    }
    
    /// Check if a peer is trusted
    pub async fn is_trusted(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        self.trust_manager.is_trusted(peer_id).await
    }
    
    /// Get all trusted peers
    pub async fn get_trusted_peers(&self) -> SecurityResult<Vec<TrustEntry>> {
        self.trust_manager.get_all_trusted_peers().await
    }
    
    /// Generate a pairing code
    pub async fn generate_pairing_code(&self) -> SecurityResult<PairingCode> {
        self.trust_manager.generate_pairing_code().await
    }
    
    /// Verify a pairing code and add peer to trust list
    pub async fn verify_and_trust_peer(
        &self,
        code: &PairingCode,
        peer_id: &PeerId,
        nickname: String,
    ) -> SecurityResult<bool> {
        let verified = self.trust_manager.verify_pairing_code(code, peer_id).await?;
        
        if verified {
            // Add peer to trust list with Verified trust level
            let entry = TrustEntry::new(peer_id.clone(), nickname, TrustLevel::Verified);
            self.trust_manager.trust_database().add_peer(entry)?;
        }
        
        Ok(verified)
    }
    
    /// Update permissions for a peer
    pub async fn update_peer_permissions(
        &self,
        peer_id: &PeerId,
        permissions: ServicePermissions,
    ) -> SecurityResult<()> {
        self.trust_manager.update_permissions(peer_id, permissions).await
    }
    
    /// Get security policy
    pub async fn get_policy(&self) -> SecurityResult<SecurityPolicy> {
        self.policy_engine.get_policy().await
    }
    
    /// Update security policy
    pub async fn update_policy(&self, policy: SecurityPolicy) -> SecurityResult<()> {
        self.policy_engine.update_policy(policy).await
    }
    
    /// Enable private mode
    pub async fn enable_private_mode(&self) -> SecurityResult<()> {
        self.policy_engine.enable_private_mode().await
    }
    
    /// Disable private mode
    pub async fn disable_private_mode(&self) -> SecurityResult<()> {
        self.policy_engine.disable_private_mode().await
    }
    
    /// Generate an invite code for private mode
    pub async fn generate_invite_code(&self, peer_id: PeerId) -> SecurityResult<InviteCode> {
        self.policy_engine.generate_invite_code(peer_id).await
    }
    
    /// Validate an invite code
    pub async fn validate_invite_code(&self, code: &str) -> SecurityResult<Option<PeerId>> {
        self.policy_engine.validate_invite_code(code).await
    }
    
    /// Enable local-only mode
    pub async fn enable_local_only_mode(&self) -> SecurityResult<()> {
        self.policy_engine.enable_local_only_mode().await
    }
    
    /// Disable local-only mode
    pub async fn disable_local_only_mode(&self) -> SecurityResult<()> {
        self.policy_engine.disable_local_only_mode().await
    }
    
    /// Check if a connection is allowed
    pub async fn is_connection_allowed(
        &self,
        peer_id: &PeerId,
        connection_type: ConnectionType,
    ) -> SecurityResult<bool> {
        self.policy_engine.is_connection_allowed(peer_id, connection_type).await
    }
    
    /// Get audit log
    pub async fn get_audit_log(&self, limit: usize) -> SecurityResult<Vec<SecurityEvent>> {
        self.policy_engine.get_audit_log(limit).await
    }
    
    /// Cleanup expired sessions
    pub async fn cleanup_expired_sessions(&self) -> SecurityResult<usize> {
        self.encryption_engine.cleanup_expired_sessions().await
    }
    
    /// Get session count
    pub async fn session_count(&self) -> usize {
        self.encryption_engine.session_count().await
    }
}

impl Default for SecuritySystem {
    fn default() -> Self {
        Self::new().expect("Failed to create default security system")
    }
}

#[async_trait]
impl Security for SecuritySystem {
    async fn get_device_identity(&self) -> SecurityResult<DeviceIdentity> {
        self.get_or_create_identity().await
    }
    
    async fn get_peer_id(&self) -> SecurityResult<PeerId> {
        let identity = self.get_device_identity().await?;
        Ok(identity.derive_peer_id())
    }
    
    async fn establish_session(&self, peer_id: &PeerId) -> SecurityResult<SessionId> {
        self.encryption_engine.establish_session(peer_id).await
    }
    
    async fn encrypt_message(&self, session_id: &SessionId, data: &[u8]) -> SecurityResult<Vec<u8>> {
        self.encryption_engine.encrypt_message(session_id, data).await
    }
    
    async fn decrypt_message(&self, session_id: &SessionId, data: &[u8]) -> SecurityResult<Vec<u8>> {
        self.encryption_engine.decrypt_message(session_id, data).await
    }
    
    async fn is_trusted(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        self.trust_manager.is_trusted(peer_id).await
    }
    
    async fn add_trusted_peer(&self, peer_id: PeerId, nickname: String) -> SecurityResult<()> {
        self.trust_manager.add_trusted_peer(peer_id, nickname).await
    }
}

/// Configuration for the security system
#[derive(Clone, Debug)]
pub struct SecuritySystemConfig {
    /// Custom keystore service name (None = use default)
    pub keystore_service_name: Option<String>,
    /// Path to trust database (None = use default)
    pub trust_db_path: Option<PathBuf>,
    /// Session timeout duration
    pub session_timeout: Duration,
    /// Key rotation interval
    pub key_rotation_interval: Duration,
    /// Disposable identity lifetime
    pub disposable_identity_lifetime: Duration,
    /// Security policy
    pub security_policy: SecurityPolicy,
}

impl Default for SecuritySystemConfig {
    fn default() -> Self {
        Self {
            keystore_service_name: None,
            trust_db_path: None,
            session_timeout: Duration::from_secs(3600), // 1 hour
            key_rotation_interval: Duration::from_secs(900), // 15 minutes
            disposable_identity_lifetime: Duration::from_secs(86400), // 24 hours
            security_policy: SecurityPolicy::default(),
        }
    }
}

/// Builder for SecuritySystem
pub struct SecuritySystemBuilder {
    config: SecuritySystemConfig,
}

impl SecuritySystemBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: SecuritySystemConfig::default(),
        }
    }
    
    /// Set custom keystore service name
    pub fn keystore_service_name(mut self, name: impl Into<String>) -> Self {
        self.config.keystore_service_name = Some(name.into());
        self
    }
    
    /// Set trust database path
    pub fn trust_db_path(mut self, path: PathBuf) -> Self {
        self.config.trust_db_path = Some(path);
        self
    }
    
    /// Set session timeout
    pub fn session_timeout(mut self, timeout: Duration) -> Self {
        self.config.session_timeout = timeout;
        self
    }
    
    /// Set key rotation interval
    pub fn key_rotation_interval(mut self, interval: Duration) -> Self {
        self.config.key_rotation_interval = interval;
        self
    }
    
    /// Set disposable identity lifetime
    pub fn disposable_identity_lifetime(mut self, lifetime: Duration) -> Self {
        self.config.disposable_identity_lifetime = lifetime;
        self
    }
    
    /// Set security policy
    pub fn security_policy(mut self, policy: SecurityPolicy) -> Self {
        self.config.security_policy = policy;
        self
    }
    
    /// Build the security system
    pub fn build(self) -> SecurityResult<SecuritySystem> {
        SecuritySystem::with_config(self.config)
    }
}

impl Default for SecuritySystemBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_security_system_creation() {
        let security = SecuritySystem::new().unwrap();
        
        // Get device identity
        let identity = security.get_device_identity().await.unwrap();
        let peer_id = identity.derive_peer_id();
        
        // Verify peer ID matches
        let system_peer_id = security.get_peer_id().await.unwrap();
        assert_eq!(peer_id, system_peer_id);
    }
    
    #[tokio::test]
    async fn test_trust_management() {
        let security = SecuritySystem::new().unwrap();
        
        // Create a test peer
        let test_identity = DeviceIdentity::generate().unwrap();
        let test_peer_id = test_identity.derive_peer_id();
        
        // Initially not trusted
        assert!(!security.is_trusted(&test_peer_id).await.unwrap());
        
        // Add to trust list
        security.add_trusted_peer(test_peer_id.clone(), "Test Peer".to_string())
            .await
            .unwrap();
        
        // Now should be trusted
        assert!(security.is_trusted(&test_peer_id).await.unwrap());
        
        // Remove from trust list
        security.remove_trusted_peer(&test_peer_id).await.unwrap();
        
        // Should no longer be trusted
        assert!(!security.is_trusted(&test_peer_id).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_encryption_session() {
        let security = SecuritySystem::new().unwrap();
        
        // Create a test peer
        let test_identity = DeviceIdentity::generate().unwrap();
        let test_peer_id = test_identity.derive_peer_id();
        
        // Establish session
        let session_id = security.establish_session(&test_peer_id).await.unwrap();
        
        // Encrypt data
        let plaintext = b"Hello, secure world!";
        let ciphertext = security.encrypt_message(&session_id, plaintext).await.unwrap();
        
        // Verify ciphertext is different from plaintext
        assert_ne!(ciphertext.as_slice(), plaintext);
        
        // Decrypt data
        let decrypted = security.decrypt_message(&session_id, &ciphertext).await.unwrap();
        
        // Verify decrypted matches original
        assert_eq!(decrypted.as_slice(), plaintext);
    }
    
    #[tokio::test]
    async fn test_policy_management() {
        let security = SecuritySystem::new().unwrap();
        
        // Get initial policy
        let policy = security.get_policy().await.unwrap();
        assert!(!policy.private_mode);
        assert!(!policy.local_only_mode);
        
        // Enable private mode
        security.enable_private_mode().await.unwrap();
        let policy = security.get_policy().await.unwrap();
        assert!(policy.private_mode);
        
        // Disable private mode
        security.disable_private_mode().await.unwrap();
        let policy = security.get_policy().await.unwrap();
        assert!(!policy.private_mode);
    }
    
    #[tokio::test]
    async fn test_disposable_identity() {
        let security = SecuritySystem::new().unwrap();
        
        // Create disposable identity
        let disposable = security.create_disposable_identity().await.unwrap();
        assert!(!disposable.is_active());
        
        // Activate it
        security.activate_disposable_identity(disposable.id()).await.unwrap();
        
        // Get active identity
        let active = security.get_active_disposable_identity().await.unwrap();
        assert!(active.is_some());
        assert_eq!(active.unwrap().id(), disposable.id());
    }
}
