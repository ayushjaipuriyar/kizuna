//! Security Integration for Browser Support
//!
//! This module provides authentication, session management, and security
//! integration for browser clients connecting to Kizuna peers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::security::{Security, PeerId, SessionId};
use crate::security::encryption::EncryptionEngine;
use crate::security::trust::TrustManager;
use crate::security::policy::PolicyEngine;
use crate::browser_support::types::{BrowserSession, BrowserInfo, BrowserPermissions};
use crate::browser_support::error::{BrowserSupportError, BrowserResult};

/// Browser authentication credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserCredentials {
    /// Browser client identifier
    pub client_id: String,
    /// Authentication token or certificate
    pub auth_token: String,
    /// Browser information
    pub browser_info: BrowserInfo,
    /// Optional peer ID for trusted connections
    pub peer_id: Option<String>,
}

/// Browser security session with authentication and encryption
#[derive(Debug, Clone)]
pub struct BrowserSecuritySession {
    /// Unique session identifier
    pub session_id: Uuid,
    /// Browser session information
    pub browser_session: BrowserSession,
    /// Encryption session ID for secure communications
    pub encryption_session_id: SessionId,
    /// Peer ID for this browser client
    pub peer_id: PeerId,
    /// Session creation timestamp
    pub created_at: SystemTime,
    /// Session expiration timestamp
    pub expires_at: SystemTime,
    /// Last activity timestamp
    pub last_activity: SystemTime,
    /// Whether the session is authenticated
    pub is_authenticated: bool,
}

impl BrowserSecuritySession {
    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        SystemTime::now() > self.expires_at
    }
    
    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = SystemTime::now();
    }
    
    /// Check if session needs refresh (within 5 minutes of expiration)
    pub fn needs_refresh(&self) -> bool {
        if let Ok(time_until_expiry) = self.expires_at.duration_since(SystemTime::now()) {
            time_until_expiry < Duration::from_secs(300) // 5 minutes
        } else {
            true // Already expired
        }
    }
}

/// Browser authenticator for managing browser client authentication
pub struct BrowserAuthenticator {
    /// Reference to the security system
    security: Arc<dyn Security>,
    /// Active browser sessions
    sessions: Arc<RwLock<HashMap<Uuid, BrowserSecuritySession>>>,
    /// Session timeout duration
    session_timeout: Duration,
    /// Automatic session refresh enabled
    auto_refresh: bool,
}

impl BrowserAuthenticator {
    /// Create a new browser authenticator
    pub fn new(security: Arc<dyn Security>, session_timeout: Duration) -> Self {
        Self {
            security,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_timeout,
            auto_refresh: true,
        }
    }
    
    /// Create with default settings (1 hour timeout)
    pub fn with_defaults(security: Arc<dyn Security>) -> Self {
        Self::new(security, Duration::from_secs(3600))
    }
    
    /// Authenticate a browser client and create a security session
    pub async fn authenticate_browser_client(
        &self,
        credentials: BrowserCredentials,
    ) -> BrowserResult<BrowserSecuritySession> {
        // Validate credentials
        self.validate_credentials(&credentials).await?;
        
        // Determine peer ID
        let peer_id = if let Some(ref peer_id_str) = credentials.peer_id {
            // Parse existing peer ID
            PeerId::from_string(peer_id_str)
                .map_err(|e| BrowserSupportError::AuthenticationFailed(format!("Invalid peer ID: {}", e)))?
        } else {
            // Generate new peer ID for this browser client
            self.generate_browser_peer_id(&credentials).await?
        };
        
        // Check if peer is trusted (if peer_id was provided)
        if credentials.peer_id.is_some() {
            let is_trusted = self.security.is_trusted(&peer_id).await
                .map_err(|e| BrowserSupportError::AuthenticationFailed(format!("Trust check failed: {}", e)))?;
            
            if !is_trusted {
                return Err(BrowserSupportError::AuthenticationFailed(
                    "Browser client peer is not trusted".to_string()
                ));
            }
        }
        
        // Establish encryption session
        let encryption_session_id = self.security.establish_session(&peer_id).await
            .map_err(|e| BrowserSupportError::EncryptionFailed(format!("Failed to establish session: {}", e)))?;
        
        // Create browser session
        let session_id = Uuid::new_v4();
        let now = SystemTime::now();
        let expires_at = now + self.session_timeout;
        
        let browser_session = BrowserSession {
            session_id,
            browser_info: credentials.browser_info.clone(),
            webrtc_connection: crate::browser_support::types::WebRTCConnection {
                connection_id: Uuid::new_v4(),
                peer_id: peer_id.to_string(),
                data_channels: HashMap::new(),
                connection_state: crate::browser_support::types::ConnectionState::New,
                ice_connection_state: crate::browser_support::types::IceConnectionState::New,
            },
            permissions: BrowserPermissions::default(),
            created_at: now,
            last_activity: now,
        };
        
        let security_session = BrowserSecuritySession {
            session_id,
            browser_session,
            encryption_session_id,
            peer_id: peer_id.clone(),
            created_at: now,
            expires_at,
            last_activity: now,
            is_authenticated: true,
        };
        
        // Store session
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, security_session.clone());
        
        Ok(security_session)
    }
    
    /// Validate browser credentials
    async fn validate_credentials(&self, credentials: &BrowserCredentials) -> BrowserResult<()> {
        // Validate client ID format
        if credentials.client_id.is_empty() {
            return Err(BrowserSupportError::AuthenticationFailed(
                "Client ID cannot be empty".to_string()
            ));
        }
        
        // Validate auth token
        if credentials.auth_token.is_empty() {
            return Err(BrowserSupportError::AuthenticationFailed(
                "Auth token cannot be empty".to_string()
            ));
        }
        
        Ok(())
    }
    
    /// Generate a peer ID for a browser client
    async fn generate_browser_peer_id(&self, credentials: &BrowserCredentials) -> BrowserResult<PeerId> {
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(credentials.client_id.as_bytes());
        hasher.update(credentials.browser_info.user_agent.as_bytes());
        hasher.update(credentials.auth_token.as_bytes());
        
        let fingerprint: [u8; 32] = hasher.finalize().into();
        Ok(PeerId::from_fingerprint(fingerprint))
    }
    
    /// Get a browser security session by ID
    pub async fn get_session(&self, session_id: &Uuid) -> BrowserResult<BrowserSecuritySession> {
        let sessions = self.sessions.read().await;
        
        sessions.get(session_id)
            .cloned()
            .ok_or_else(|| BrowserSupportError::SessionNotFound(session_id.to_string()))
    }
    
    /// Validate a session and check if it's still active
    pub async fn validate_session(&self, session_id: &Uuid) -> BrowserResult<bool> {
        let sessions = self.sessions.read().await;
        
        if let Some(session) = sessions.get(session_id) {
            if session.is_expired() {
                return Ok(false);
            }
            
            if !session.is_authenticated {
                return Ok(false);
            }
            
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Refresh a browser session to extend its lifetime
    pub async fn refresh_session(&self, session_id: &Uuid) -> BrowserResult<BrowserSecuritySession> {
        let mut sessions = self.sessions.write().await;
        
        let session = sessions.get_mut(session_id)
            .ok_or_else(|| BrowserSupportError::SessionNotFound(session_id.to_string()))?;
        
        // Check if session is still valid
        if !session.is_authenticated {
            return Err(BrowserSupportError::AuthenticationFailed(
                "Session is not authenticated".to_string()
            ));
        }
        
        // Extend expiration time
        let now = SystemTime::now();
        session.expires_at = now + self.session_timeout;
        session.last_activity = now;
        
        Ok(session.clone())
    }
    
    /// Update session activity timestamp
    pub async fn update_session_activity(&self, session_id: &Uuid) -> BrowserResult<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(session_id) {
            session.update_activity();
            
            // Auto-refresh if needed
            if self.auto_refresh && session.needs_refresh() {
                let now = SystemTime::now();
                session.expires_at = now + self.session_timeout;
            }
        }
        
        Ok(())
    }
    
    /// Revoke a browser session
    pub async fn revoke_session(&self, session_id: &Uuid) -> BrowserResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        Ok(())
    }
    
    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> BrowserResult<usize> {
        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();
        
        sessions.retain(|_, session| !session.is_expired());
        
        let removed_count = initial_count - sessions.len();
        Ok(removed_count)
    }
    
    /// Get all active sessions
    pub async fn get_active_sessions(&self) -> Vec<BrowserSecuritySession> {
        let sessions = self.sessions.read().await;
        sessions.values()
            .filter(|s| !s.is_expired() && s.is_authenticated)
            .cloned()
            .collect()
    }
    
    /// Get session count
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }
}

/// Browser certificate validator for validating browser client certificates
pub struct BrowserCertificateValidator {
    /// Trusted certificate authorities
    trusted_cas: Arc<RwLock<Vec<String>>>,
    /// Certificate revocation list
    revoked_certs: Arc<RwLock<Vec<String>>>,
}

impl BrowserCertificateValidator {
    /// Create a new certificate validator
    pub fn new() -> Self {
        Self {
            trusted_cas: Arc::new(RwLock::new(Vec::new())),
            revoked_certs: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Add a trusted certificate authority
    pub async fn add_trusted_ca(&self, ca_cert: String) -> BrowserResult<()> {
        let mut cas = self.trusted_cas.write().await;
        if !cas.contains(&ca_cert) {
            cas.push(ca_cert);
        }
        Ok(())
    }
    
    /// Revoke a certificate
    pub async fn revoke_certificate(&self, cert_id: String) -> BrowserResult<()> {
        let mut revoked = self.revoked_certs.write().await;
        if !revoked.contains(&cert_id) {
            revoked.push(cert_id);
        }
        Ok(())
    }
    
    /// Validate a browser certificate
    pub async fn validate_certificate(&self, cert: &str) -> BrowserResult<bool> {
        // Check if certificate is revoked
        let revoked = self.revoked_certs.read().await;
        if revoked.contains(&cert.to_string()) {
            return Ok(false);
        }
        
        Ok(true)
    }
}

impl Default for BrowserCertificateValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Browser permission manager for managing browser client permissions
pub struct BrowserPermissionManager {
    /// Reference to trust manager
    trust_manager: Arc<dyn TrustManager>,
    /// Reference to policy engine
    policy_engine: Arc<dyn PolicyEngine>,
    /// Default permissions for new browser clients
    default_permissions: BrowserPermissions,
}

impl BrowserPermissionManager {
    /// Create a new permission manager
    pub fn new(
        trust_manager: Arc<dyn TrustManager>,
        policy_engine: Arc<dyn PolicyEngine>,
    ) -> Self {
        Self {
            trust_manager,
            policy_engine,
            default_permissions: BrowserPermissions::default(),
        }
    }
    
    /// Set default permissions for new browser clients
    pub fn set_default_permissions(&mut self, permissions: BrowserPermissions) {
        self.default_permissions = permissions;
    }
    
    /// Get permissions for a browser client
    pub async fn get_permissions(&self, peer_id: &PeerId) -> BrowserResult<BrowserPermissions> {
        // Check if peer is trusted
        let is_trusted = self.trust_manager.is_trusted(peer_id).await
            .map_err(|e| BrowserSupportError::PermissionDenied(format!("Trust check failed: {}", e)))?;
        
        if !is_trusted {
            // Return default (restricted) permissions for untrusted peers
            return Ok(BrowserPermissions::default());
        }
        
        // Get permissions from trust manager
        let trust_entry = self.trust_manager.get_trust_entry(peer_id).await
            .map_err(|e| BrowserSupportError::PermissionDenied(format!("Failed to get peer: {}", e)))?;
        
        if let Some(entry) = trust_entry {
            // Convert service permissions to browser permissions
            let permissions = BrowserPermissions {
                file_transfer: entry.permissions.file_transfer,
                clipboard_sync: entry.permissions.clipboard,
                command_execution: entry.permissions.commands,
                camera_streaming: entry.permissions.camera,
                system_info: true, // Always allow system info for trusted peers
            };
            
            Ok(permissions)
        } else {
            Ok(self.default_permissions.clone())
        }
    }
    
    /// Update permissions for a browser client
    pub async fn update_permissions(
        &self,
        peer_id: &PeerId,
        permissions: BrowserPermissions,
    ) -> BrowserResult<()> {
        // Convert browser permissions to service permissions
        let service_permissions = crate::security::trust::ServicePermissions {
            file_transfer: permissions.file_transfer,
            clipboard: permissions.clipboard_sync,
            commands: permissions.command_execution,
            camera: permissions.camera_streaming,
        };
        
        self.trust_manager.update_permissions(peer_id, service_permissions).await
            .map_err(|e| BrowserSupportError::PermissionDenied(format!("Failed to update permissions: {}", e)))?;
        
        Ok(())
    }
    
    /// Validate if a browser client has permission for an operation
    pub async fn validate_permission(
        &self,
        peer_id: &PeerId,
        operation: BrowserOperation,
    ) -> BrowserResult<bool> {
        let permissions = self.get_permissions(peer_id).await?;
        
        let has_permission = match operation {
            BrowserOperation::FileTransfer => permissions.file_transfer,
            BrowserOperation::ClipboardSync => permissions.clipboard_sync,
            BrowserOperation::CommandExecution => permissions.command_execution,
            BrowserOperation::CameraStreaming => permissions.camera_streaming,
            BrowserOperation::SystemInfo => permissions.system_info,
        };
        
        Ok(has_permission)
    }
}

/// Browser operations that require permission validation
#[derive(Debug, Clone, Copy)]
pub enum BrowserOperation {
    FileTransfer,
    ClipboardSync,
    CommandExecution,
    CameraStreaming,
    SystemInfo,
}
