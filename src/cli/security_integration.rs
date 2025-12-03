// CLI Security Integration
//
// Provides authentication, authorization, and secure session management
// for CLI operations.
//
// Requirements: 6.5

use crate::cli::error::{CLIError, CLIResult};
use crate::security::api::SecuritySystem;
use crate::security::identity::{DeviceIdentity, PeerId};
use crate::security::policy::{ConnectionType, SecurityPolicy};
use crate::security::trust::ServicePermissions;
use crate::security::trust::{PairingCode, TrustEntry, TrustLevel};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// CLI security integration for authentication and authorization
pub struct CLISecurityIntegration {
    security_system: Arc<SecuritySystem>,
    /// Current authenticated session
    current_session: Arc<RwLock<Option<CLISession>>>,
}

/// CLI session information
#[derive(Clone)]
pub struct CLISession {
    /// Session ID
    pub session_id: Uuid,
    /// Device identity
    pub device_identity: DeviceIdentity,
    /// Peer ID
    pub peer_id: PeerId,
    /// Session start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Session expiry time
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl CLISecurityIntegration {
    /// Create a new CLI security integration
    pub fn new(security_system: Arc<SecuritySystem>) -> Self {
        Self {
            security_system,
            current_session: Arc::new(RwLock::new(None)),
        }
    }

    /// Authenticate and create a CLI session
    pub async fn authenticate(&self) -> CLIResult<CLISession> {
        // Get or create device identity
        let device_identity = self
            .security_system
            .get_or_create_identity()
            .await
            .map_err(|e| CLIError::security(format!("Failed to get device identity: {}", e)))?;

        // Derive peer ID
        let peer_id = device_identity.derive_peer_id();

        // Create session
        let session = CLISession {
            session_id: Uuid::new_v4(),
            device_identity,
            peer_id,
            started_at: chrono::Utc::now(),
            expires_at: chrono::Utc::now() + chrono::Duration::hours(24),
        };

        // Store current session
        *self.current_session.write().await = Some(session.clone());

        Ok(session)
    }

    /// Get current session
    pub async fn get_current_session(&self) -> CLIResult<CLISession> {
        let session = self.current_session.read().await;
        session
            .clone()
            .ok_or_else(|| CLIError::security("No active session. Please authenticate first.".to_string()))
    }

    /// Check if current session is valid
    pub async fn is_session_valid(&self) -> bool {
        let session = self.current_session.read().await;
        if let Some(session) = session.as_ref() {
            chrono::Utc::now() < session.expires_at
        } else {
            false
        }
    }

    /// Logout and clear current session
    pub async fn logout(&self) -> CLIResult<()> {
        *self.current_session.write().await = None;
        Ok(())
    }

    /// Check if a peer is trusted
    pub async fn is_peer_trusted(&self, peer_id: &PeerId) -> CLIResult<bool> {
        self.security_system
            .is_trusted(peer_id)
            .await
            .map_err(|e| CLIError::security(format!("Failed to check trust status: {}", e)))
    }

    /// Add a trusted peer with authorization prompt
    pub async fn add_trusted_peer(&self, peer_id: PeerId, nickname: String) -> CLIResult<()> {
        // Prompt user for confirmation
        println!("Add peer '{}' ({}) to trusted list?", nickname, peer_id);
        println!("This will allow the peer to connect and interact with your device.");
        print!("Confirm (y/n): ");
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if input.trim().to_lowercase() != "y" {
            return Err(CLIError::security("User declined to add trusted peer".to_string()));
        }

        // Add to trust list
        self.security_system
            .add_trusted_peer(peer_id, nickname)
            .await
            .map_err(|e| CLIError::security(format!("Failed to add trusted peer: {}", e)))?;

        println!("Peer added to trusted list successfully.");
        Ok(())
    }

    /// Remove a trusted peer with authorization prompt
    pub async fn remove_trusted_peer(&self, peer_id: &PeerId) -> CLIResult<()> {
        // Prompt user for confirmation
        println!("Remove peer '{}' from trusted list?", peer_id);
        println!("This will prevent the peer from connecting to your device.");
        print!("Confirm (y/n): ");
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if input.trim().to_lowercase() != "y" {
            return Err(CLIError::security("User declined to remove trusted peer".to_string()));
        }

        // Remove from trust list
        self.security_system
            .remove_trusted_peer(peer_id)
            .await
            .map_err(|e| CLIError::security(format!("Failed to remove trusted peer: {}", e)))?;

        println!("Peer removed from trusted list successfully.");
        Ok(())
    }

    /// Get all trusted peers
    pub async fn get_trusted_peers(&self) -> CLIResult<Vec<TrustEntry>> {
        self.security_system
            .get_trusted_peers()
            .await
            .map_err(|e| CLIError::security(format!("Failed to get trusted peers: {}", e)))
    }

    /// Generate a pairing code for peer authentication
    pub async fn generate_pairing_code(&self) -> CLIResult<PairingCode> {
        self.security_system
            .generate_pairing_code()
            .await
            .map_err(|e| CLIError::security(format!("Failed to generate pairing code: {}", e)))
    }

    /// Verify a pairing code and add peer to trust list
    pub async fn verify_and_trust_peer(
        &self,
        code: &PairingCode,
        peer_id: &PeerId,
        nickname: String,
    ) -> CLIResult<bool> {
        let verified = self
            .security_system
            .verify_and_trust_peer(code, peer_id, nickname)
            .await
            .map_err(|e| CLIError::security(format!("Failed to verify pairing code: {}", e)))?;

        if verified {
            println!("Pairing successful! Peer added to trusted list.");
        } else {
            println!("Pairing failed. Invalid code or peer ID.");
        }

        Ok(verified)
    }

    /// Update permissions for a peer with authorization prompt
    pub async fn update_peer_permissions(
        &self,
        peer_id: &PeerId,
        permissions: ServicePermissions,
    ) -> CLIResult<()> {
        // Prompt user for confirmation
        println!("Update permissions for peer '{}'?", peer_id);
        println!("New permissions: {:?}", permissions);
        print!("Confirm (y/n): ");
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        if input.trim().to_lowercase() != "y" {
            return Err(CLIError::security("User declined to update permissions".to_string()));
        }

        // Update permissions
        self.security_system
            .update_peer_permissions(peer_id, permissions)
            .await
            .map_err(|e| CLIError::security(format!("Failed to update permissions: {}", e)))?;

        println!("Permissions updated successfully.");
        Ok(())
    }

    /// Check if a connection is allowed
    pub async fn is_connection_allowed(
        &self,
        peer_id: &PeerId,
        connection_type: ConnectionType,
    ) -> CLIResult<bool> {
        self.security_system
            .is_connection_allowed(peer_id, connection_type)
            .await
            .map_err(|e| CLIError::security(format!("Failed to check connection permission: {}", e)))
    }

    /// Authorize an operation with user prompt
    pub async fn authorize_operation(&self, operation: &str, peer_id: &PeerId) -> CLIResult<bool> {
        // Check if peer is trusted
        if !self.is_peer_trusted(peer_id).await? {
            println!("Warning: Peer '{}' is not trusted!", peer_id);
        }

        // Prompt user for authorization
        println!("Authorize operation: {}", operation);
        println!("Peer: {}", peer_id);
        print!("Allow (y/n): ");
        
        use std::io::{self, Write};
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        
        Ok(input.trim().to_lowercase() == "y")
    }

    /// Get security policy
    pub async fn get_security_policy(&self) -> CLIResult<SecurityPolicy> {
        self.security_system
            .get_policy()
            .await
            .map_err(|e| CLIError::security(format!("Failed to get security policy: {}", e)))
    }

    /// Update security policy
    pub async fn update_security_policy(&self, policy: SecurityPolicy) -> CLIResult<()> {
        self.security_system
            .update_policy(policy)
            .await
            .map_err(|e| CLIError::security(format!("Failed to update security policy: {}", e)))
    }

    /// Enable private mode
    pub async fn enable_private_mode(&self) -> CLIResult<()> {
        self.security_system
            .enable_private_mode()
            .await
            .map_err(|e| CLIError::security(format!("Failed to enable private mode: {}", e)))?;

        println!("Private mode enabled. Only invited peers can connect.");
        Ok(())
    }

    /// Disable private mode
    pub async fn disable_private_mode(&self) -> CLIResult<()> {
        self.security_system
            .disable_private_mode()
            .await
            .map_err(|e| CLIError::security(format!("Failed to disable private mode: {}", e)))?;

        println!("Private mode disabled.");
        Ok(())
    }

    /// Generate an invite code for private mode
    pub async fn generate_invite_code(&self, peer_id: PeerId) -> CLIResult<String> {
        let invite_code = self
            .security_system
            .generate_invite_code(peer_id)
            .await
            .map_err(|e| CLIError::security(format!("Failed to generate invite code: {}", e)))?;

        Ok(invite_code.code().to_string())
    }

    /// Validate an invite code
    pub async fn validate_invite_code(&self, code: &str) -> CLIResult<Option<PeerId>> {
        self.security_system
            .validate_invite_code(code)
            .await
            .map_err(|e| CLIError::security(format!("Failed to validate invite code: {}", e)))
    }

    /// Get audit log
    pub async fn get_audit_log(&self, limit: usize) -> CLIResult<Vec<crate::security::policy::SecurityEvent>> {
        self.security_system
            .get_audit_log(limit)
            .await
            .map_err(|e| CLIError::security(format!("Failed to get audit log: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cli_security_integration_creation() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let cli_security = CLISecurityIntegration::new(security_system);

        // Initially no session
        assert!(!cli_security.is_session_valid().await);
    }

    #[tokio::test]
    async fn test_authentication() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let cli_security = CLISecurityIntegration::new(security_system);

        // Authenticate
        let session = cli_security.authenticate().await.unwrap();
        // Peer ID should be valid
        assert!(session.session_id != Uuid::nil());

        // Session should be valid
        assert!(cli_security.is_session_valid().await);

        // Get current session
        let current = cli_security.get_current_session().await.unwrap();
        assert_eq!(current.session_id, session.session_id);
    }

    #[tokio::test]
    async fn test_logout() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let cli_security = CLISecurityIntegration::new(security_system);

        // Authenticate
        cli_security.authenticate().await.unwrap();
        assert!(cli_security.is_session_valid().await);

        // Logout
        cli_security.logout().await.unwrap();
        assert!(!cli_security.is_session_valid().await);
    }

    #[tokio::test]
    async fn test_get_trusted_peers() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let cli_security = CLISecurityIntegration::new(security_system);

        let peers = cli_security.get_trusted_peers().await.unwrap();
        assert_eq!(peers.len(), 0);
    }

    #[tokio::test]
    async fn test_generate_pairing_code() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let cli_security = CLISecurityIntegration::new(security_system);

        let code = cli_security.generate_pairing_code().await.unwrap();
        assert!(!code.code().is_empty());
    }

    #[tokio::test]
    async fn test_security_policy() {
        let security_system = Arc::new(SecuritySystem::new().unwrap());
        let cli_security = CLISecurityIntegration::new(security_system);

        let policy = cli_security.get_security_policy().await.unwrap();
        assert!(!policy.private_mode);
    }
}
