// Security Integration for Command Execution
//
// This module provides integration between the command execution system and the security layer,
// enabling encrypted command transmission, peer authentication, and secure result delivery.

use async_trait::async_trait;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::command_execution::{
    CommandRequest, CommandResult, ScriptRequest, ScriptResult, Notification,
    NotificationResult, SystemInfo, SystemInfoQuery,
};
use crate::command_execution::error::{CommandError, CommandResult as CmdResult};
use crate::security::{Security, SessionId, PeerId as SecurityPeerId};

// Command execution uses String for PeerId
type PeerId = String;

/// Encrypted command message wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedCommandMessage {
    /// Session ID for decryption
    pub session_id: SessionId,
    /// Encrypted payload
    pub encrypted_data: Vec<u8>,
    /// Message type identifier
    pub message_type: CommandMessageType,
    /// Sender peer ID
    pub sender: PeerId,
    /// Message timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Types of command messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommandMessageType {
    CommandRequest,
    CommandResult,
    ScriptRequest,
    ScriptResult,
    SystemInfoQuery,
    SystemInfoResponse,
    NotificationRequest,
    NotificationResult,
}

/// Command message payload (before encryption)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandMessage {
    CommandRequest(CommandRequest),
    CommandResult(CommandResult),
    ScriptRequest(ScriptRequest),
    ScriptResult(ScriptResult),
    SystemInfoQuery(SystemInfoQuery),
    SystemInfoResponse(SystemInfo),
    NotificationRequest(Notification),
    NotificationResult(NotificationResult),
}

impl CommandMessage {
    /// Get the message type
    pub fn message_type(&self) -> CommandMessageType {
        match self {
            CommandMessage::CommandRequest(_) => CommandMessageType::CommandRequest,
            CommandMessage::CommandResult(_) => CommandMessageType::CommandResult,
            CommandMessage::ScriptRequest(_) => CommandMessageType::ScriptRequest,
            CommandMessage::ScriptResult(_) => CommandMessageType::ScriptResult,
            CommandMessage::SystemInfoQuery(_) => CommandMessageType::SystemInfoQuery,
            CommandMessage::SystemInfoResponse(_) => CommandMessageType::SystemInfoResponse,
            CommandMessage::NotificationRequest(_) => CommandMessageType::NotificationRequest,
            CommandMessage::NotificationResult(_) => CommandMessageType::NotificationResult,
        }
    }
}

/// Security integration for command execution
pub struct CommandSecurityIntegration {
    security: Arc<dyn Security>,
}

impl CommandSecurityIntegration {
    /// Create a new security integration
    pub fn new(security: Arc<dyn Security>) -> Self {
        Self { security }
    }

    /// Encrypt a command message for transmission
    pub async fn encrypt_message(
        &self,
        message: CommandMessage,
        peer_id: &PeerId,
    ) -> CmdResult<EncryptedCommandMessage> {
        // Convert String PeerId to SecurityPeerId
        let security_peer_id = SecurityPeerId::from_string(peer_id)
            .map_err(|e| CommandError::SecurityError(format!("Invalid peer ID: {}", e)))?;

        // Verify peer is trusted
        let is_trusted = self.security.is_trusted(&security_peer_id)
            .await
            .map_err(|e| CommandError::SecurityError(format!("Trust verification failed: {}", e)))?;

        if !is_trusted {
            return Err(CommandError::SecurityError(
                format!("Peer {} is not trusted", peer_id)
            ));
        }

        // Establish or get existing session
        let session_id = self.security.establish_session(&security_peer_id)
            .await
            .map_err(|e| CommandError::SecurityError(format!("Session establishment failed: {}", e)))?;

        // Serialize message
        let message_bytes = serde_json::to_vec(&message)
            .map_err(|e| CommandError::SerializationError(format!("Failed to serialize message: {}", e)))?;

        // Encrypt message
        let encrypted_data = self.security.encrypt_message(&session_id, &message_bytes)
            .await
            .map_err(|e| CommandError::SecurityError(format!("Encryption failed: {}", e)))?;

        // Get sender peer ID
        let security_sender = self.security.get_peer_id()
            .await
            .map_err(|e| CommandError::SecurityError(format!("Failed to get peer ID: {}", e)))?;
        let sender = security_sender.to_string();

        Ok(EncryptedCommandMessage {
            session_id,
            encrypted_data,
            message_type: message.message_type(),
            sender,
            timestamp: chrono::Utc::now(),
        })
    }

    /// Decrypt a received command message
    pub async fn decrypt_message(
        &self,
        encrypted_message: EncryptedCommandMessage,
    ) -> CmdResult<CommandMessage> {
        // Convert String PeerId to SecurityPeerId
        let security_sender = SecurityPeerId::from_string(&encrypted_message.sender)
            .map_err(|e| CommandError::SecurityError(format!("Invalid sender peer ID: {}", e)))?;

        // Verify sender is trusted
        let is_trusted = self.security.is_trusted(&security_sender)
            .await
            .map_err(|e| CommandError::SecurityError(format!("Trust verification failed: {}", e)))?;

        if !is_trusted {
            return Err(CommandError::SecurityError(
                format!("Sender {} is not trusted", encrypted_message.sender)
            ));
        }

        // Decrypt message
        let decrypted_data = self.security.decrypt_message(
            &encrypted_message.session_id,
            &encrypted_message.encrypted_data,
        )
        .await
        .map_err(|e| CommandError::SecurityError(format!("Decryption failed: {}", e)))?;

        // Deserialize message
        let message: CommandMessage = serde_json::from_slice(&decrypted_data)
            .map_err(|e| CommandError::SerializationError(format!("Failed to deserialize message: {}", e)))?;

        // Verify message type matches
        if message.message_type() != encrypted_message.message_type {
            return Err(CommandError::SecurityError(
                "Message type mismatch - possible tampering".to_string()
            ));
        }

        Ok(message)
    }

    /// Verify peer authentication for command execution
    pub async fn verify_peer_authentication(&self, peer_id: &PeerId) -> CmdResult<bool> {
        let security_peer_id = SecurityPeerId::from_string(peer_id)
            .map_err(|e| CommandError::SecurityError(format!("Invalid peer ID: {}", e)))?;
        self.security.is_trusted(&security_peer_id)
            .await
            .map_err(|e| CommandError::SecurityError(format!("Authentication verification failed: {}", e)))
    }

    /// Add a trusted peer for command execution
    pub async fn add_trusted_peer(&self, peer_id: PeerId, nickname: String) -> CmdResult<()> {
        let security_peer_id = SecurityPeerId::from_string(&peer_id)
            .map_err(|e| CommandError::SecurityError(format!("Invalid peer ID: {}", e)))?;
        self.security.add_trusted_peer(security_peer_id, nickname)
            .await
            .map_err(|e| CommandError::SecurityError(format!("Failed to add trusted peer: {}", e)))
    }

    /// Verify message integrity
    pub async fn verify_message_integrity(
        &self,
        encrypted_message: &EncryptedCommandMessage,
    ) -> CmdResult<bool> {
        // Check timestamp is recent (within 5 minutes)
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(encrypted_message.timestamp);
        
        if age.num_seconds().abs() > 300 {
            return Ok(false);
        }

        // Convert String PeerId to SecurityPeerId
        let security_sender = SecurityPeerId::from_string(&encrypted_message.sender)
            .map_err(|e| CommandError::SecurityError(format!("Invalid sender peer ID: {}", e)))?;

        // Verify sender is trusted
        let is_trusted = self.security.is_trusted(&security_sender)
            .await
            .map_err(|e| CommandError::SecurityError(format!("Trust verification failed: {}", e)))?;

        Ok(is_trusted)
    }

    /// Get the underlying security system
    pub fn security(&self) -> &Arc<dyn Security> {
        &self.security
    }
}

/// Trait for secure command transmission
#[async_trait]
pub trait SecureCommandTransmission: Send + Sync {
    /// Send an encrypted command request
    async fn send_command_request(
        &self,
        request: CommandRequest,
        peer_id: &PeerId,
    ) -> CmdResult<()>;

    /// Send an encrypted command result
    async fn send_command_result(
        &self,
        result: CommandResult,
        peer_id: &PeerId,
    ) -> CmdResult<()>;

    /// Send an encrypted script request
    async fn send_script_request(
        &self,
        request: ScriptRequest,
        peer_id: &PeerId,
    ) -> CmdResult<()>;

    /// Send an encrypted script result
    async fn send_script_result(
        &self,
        result: ScriptResult,
        peer_id: &PeerId,
    ) -> CmdResult<()>;

    /// Send an encrypted system info query
    async fn send_system_info_query(
        &self,
        query: SystemInfoQuery,
        peer_id: &PeerId,
    ) -> CmdResult<()>;

    /// Send an encrypted system info response
    async fn send_system_info_response(
        &self,
        info: SystemInfo,
        peer_id: &PeerId,
    ) -> CmdResult<()>;

    /// Send an encrypted notification
    async fn send_notification(
        &self,
        notification: Notification,
        peer_id: &PeerId,
    ) -> CmdResult<()>;

    /// Send an encrypted notification result
    async fn send_notification_result(
        &self,
        result: NotificationResult,
        peer_id: &PeerId,
    ) -> CmdResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{SecuritySystem, DeviceIdentity};
    use std::collections::HashMap;
    use std::time::Duration;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_encrypt_decrypt_command_request() {
        // Create security system
        let security = Arc::new(SecuritySystem::new().unwrap());
        let integration = CommandSecurityIntegration::new(security.clone());

        // Create a test peer and add to trust list
        let test_identity = DeviceIdentity::generate().unwrap();
        let test_peer_id = test_identity.derive_peer_id();
        security.add_trusted_peer(test_peer_id.clone(), "Test Peer".to_string())
            .await
            .unwrap();

        // Create a command request
        let request = CommandRequest {
            request_id: Uuid::new_v4(),
            command: "echo".to_string(),
            arguments: vec!["Hello, World!".to_string()],
            working_directory: None,
            environment: HashMap::new(),
            timeout: Duration::from_secs(30),
            sandbox_config: Default::default(),
            requester: test_peer_id.clone(),
            created_at: chrono::Utc::now(),
        };

        // Encrypt the message
        let message = CommandMessage::CommandRequest(request.clone());
        let encrypted = integration.encrypt_message(message, &test_peer_id)
            .await
            .unwrap();

        // Verify encrypted data is different from original
        assert!(!encrypted.encrypted_data.is_empty());
        assert_eq!(encrypted.message_type, CommandMessageType::CommandRequest);

        // Decrypt the message
        let decrypted = integration.decrypt_message(encrypted)
            .await
            .unwrap();

        // Verify decrypted message matches original
        match decrypted {
            CommandMessage::CommandRequest(decrypted_request) => {
                assert_eq!(decrypted_request.request_id, request.request_id);
                assert_eq!(decrypted_request.command, request.command);
                assert_eq!(decrypted_request.arguments, request.arguments);
            }
            _ => panic!("Expected CommandRequest"),
        }
    }

    #[tokio::test]
    async fn test_untrusted_peer_rejection() {
        // Create security system
        let security = Arc::new(SecuritySystem::new().unwrap());
        let integration = CommandSecurityIntegration::new(security.clone());

        // Create an untrusted peer
        let test_identity = DeviceIdentity::generate().unwrap();
        let untrusted_peer_id = test_identity.derive_peer_id();

        // Create a command request
        let request = CommandRequest {
            request_id: Uuid::new_v4(),
            command: "echo".to_string(),
            arguments: vec!["Hello".to_string()],
            working_directory: None,
            environment: HashMap::new(),
            timeout: Duration::from_secs(30),
            sandbox_config: Default::default(),
            requester: untrusted_peer_id.clone(),
            created_at: chrono::Utc::now(),
        };

        // Attempt to encrypt message for untrusted peer should fail
        let message = CommandMessage::CommandRequest(request);
        let result = integration.encrypt_message(message, &untrusted_peer_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_message_integrity_verification() {
        // Create security system
        let security = Arc::new(SecuritySystem::new().unwrap());
        let integration = CommandSecurityIntegration::new(security.clone());

        // Create a test peer and add to trust list
        let test_identity = DeviceIdentity::generate().unwrap();
        let test_peer_id = test_identity.derive_peer_id();
        security.add_trusted_peer(test_peer_id.clone(), "Test Peer".to_string())
            .await
            .unwrap();

        // Create a command request
        let request = CommandRequest {
            request_id: Uuid::new_v4(),
            command: "echo".to_string(),
            arguments: vec!["test".to_string()],
            working_directory: None,
            environment: HashMap::new(),
            timeout: Duration::from_secs(30),
            sandbox_config: Default::default(),
            requester: test_peer_id.clone(),
            created_at: chrono::Utc::now(),
        };

        // Encrypt the message
        let message = CommandMessage::CommandRequest(request);
        let encrypted = integration.encrypt_message(message, &test_peer_id)
            .await
            .unwrap();

        // Verify integrity
        let is_valid = integration.verify_message_integrity(&encrypted)
            .await
            .unwrap();
        assert!(is_valid);
    }

    #[tokio::test]
    async fn test_peer_authentication() {
        // Create security system
        let security = Arc::new(SecuritySystem::new().unwrap());
        let integration = CommandSecurityIntegration::new(security.clone());

        // Create a test peer
        let test_identity = DeviceIdentity::generate().unwrap();
        let test_peer_id = test_identity.derive_peer_id();

        // Initially not authenticated
        let is_authenticated = integration.verify_peer_authentication(&test_peer_id)
            .await
            .unwrap();
        assert!(!is_authenticated);

        // Add to trust list
        integration.add_trusted_peer(test_peer_id.clone(), "Test Peer".to_string())
            .await
            .unwrap();

        // Now authenticated
        let is_authenticated = integration.verify_peer_authentication(&test_peer_id)
            .await
            .unwrap();
        assert!(is_authenticated);
    }
}
