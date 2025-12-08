//! Command Execution Integration for Browser Support
//!
//! Integrates browser command execution with the existing command system,
//! enabling browser clients to execute commands on native peers with proper
//! authorization and security controls.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use uuid::Uuid;

use crate::browser_support::{BrowserResult, BrowserSupportError, BrowserSession};
use crate::command_execution::{
    CommandExecution, CommandRequest, CommandResult, ExecutionStatus,
    PeerId, RiskLevel,
};
use crate::browser_support::webrtc::data_channel::DataChannelManager;

/// Browser command execution integration
pub struct BrowserCommandIntegration {
    /// Core command execution system
    command_system: Arc<CommandExecution>,
    /// Data channel manager for WebRTC communication
    data_channel_manager: Arc<RwLock<DataChannelManager>>,
    /// Browser command sessions
    browser_command_sessions: Arc<RwLock<HashMap<Uuid, BrowserCommandSession>>>,
    /// Authorization levels by browser session
    authorization_levels: Arc<RwLock<HashMap<String, RiskLevel>>>,
}

/// Browser command session information
#[derive(Debug, Clone)]
pub struct BrowserCommandSession {
    /// Command request ID
    pub request_id: Uuid,
    /// Browser session ID
    pub browser_session_id: String,
    /// Peer ID where command is executed
    pub peer_id: PeerId,
    /// Command being executed
    pub command: String,
    /// Command status
    pub status: ExecutionStatus,
    /// Authorization level used
    pub authorization_level: RiskLevel,
    /// Output buffer
    pub output: Vec<String>,
    /// Error output buffer
    pub error_output: Vec<String>,
    /// Exit code (if completed)
    pub exit_code: Option<i32>,
}

impl BrowserCommandIntegration {
    /// Create a new browser command integration
    pub fn new(
        command_system: Arc<CommandExecution>,
        data_channel_manager: Arc<RwLock<DataChannelManager>>,
    ) -> Self {
        Self {
            command_system,
            data_channel_manager,
            browser_command_sessions: Arc::new(RwLock::new(HashMap::new())),
            authorization_levels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Request command execution authorization for browser session
    pub async fn request_authorization(
        &self,
        browser_session: &BrowserSession,
        requested_level: RiskLevel,
    ) -> BrowserResult<RiskLevel> {
        // In a real implementation, this would:
        // 1. Prompt user for authorization
        // 2. Verify browser session identity
        // 3. Check security policies
        // 4. Grant appropriate level

        // For now, grant limited authorization by default
        let granted_level = match requested_level {
            RiskLevel::Critical => RiskLevel::High, // Downgrade for security
            RiskLevel::High => RiskLevel::Medium,
            level => level,
        };

        // Store authorization
        {
            let mut auth_levels = self.authorization_levels.write().await;
            auth_levels.insert(browser_session.session_id.to_string(), granted_level);
        }

        Ok(granted_level)
    }

    /// Get authorization level for browser session
    pub async fn get_authorization(&self, session_id: &str) -> Option<RiskLevel> {
        let auth_levels = self.authorization_levels.read().await;
        auth_levels.get(session_id).cloned()
    }

    /// Revoke authorization for browser session
    pub async fn revoke_authorization(&self, session_id: &str) -> BrowserResult<()> {
        let mut auth_levels = self.authorization_levels.write().await;
        auth_levels.remove(session_id);
        Ok(())
    }

    /// Execute command from browser
    pub async fn execute_browser_command(
        &self,
        browser_session: &BrowserSession,
        command: String,
        peer_id: PeerId,
        working_directory: Option<String>,
        environment: Option<HashMap<String, String>>,
    ) -> BrowserResult<Uuid> {
        // Check authorization
        let auth_level = self.get_authorization(&browser_session.session_id.to_string()).await
            .ok_or_else(|| BrowserSupportError::permission_denied("No command execution authorization"))?;

        // Validate command based on authorization level
        self.validate_command(&command, &auth_level)?;

        // Create command request
        let request_id = Uuid::new_v4();
        let request = CommandRequest {
            request_id,
            command: command.clone(),
            arguments: vec![],
            working_directory: working_directory.map(|s| std::path::PathBuf::from(s)),
            environment: environment.unwrap_or_default(),
            timeout: std::time::Duration::from_secs(300), // 5 minute default
            sandbox_config: crate::command_execution::SandboxConfig::default(),
            requester: browser_session.session_id.to_string(),
            created_at: chrono::Utc::now(),
        };

        // Execute through command system (local execution for now)
        let result = self.command_system
            .execute_local_command(request)
            .await
            .map_err(|e| BrowserSupportError::integration("command_execution", format!("Failed to execute: {}", e)))?;

        // Create browser command session
        let session = BrowserCommandSession {
            request_id,
            browser_session_id: browser_session.session_id.to_string(),
            peer_id,
            command,
            status: ExecutionStatus::Completed,
            authorization_level: auth_level,
            output: result.stdout.lines().map(|s| s.to_string()).collect(),
            error_output: result.stderr.lines().map(|s| s.to_string()).collect(),
            exit_code: Some(result.exit_code),
        };

        // Store session
        {
            let mut sessions = self.browser_command_sessions.write().await;
            sessions.insert(request_id, session);
        }

        Ok(request_id)
    }

    /// Validate command based on authorization level
    fn validate_command(&self, command: &str, auth_level: &RiskLevel) -> BrowserResult<()> {
        match auth_level {
            RiskLevel::Low => {
                // Only allow read-only commands
                let dangerous_patterns = ["rm", "del", "format", "mkfs", "dd", ">", ">>"];
                for pattern in &dangerous_patterns {
                    if command.contains(pattern) {
                        return Err(BrowserSupportError::permission_denied(
                            format!("Command contains forbidden pattern: {}", pattern)
                        ));
                    }
                }
                Ok(())
            }
            RiskLevel::Medium => {
                // Block highly dangerous commands
                let forbidden_patterns = ["rm -rf /", "format c:", "mkfs", "dd if="];
                for pattern in &forbidden_patterns {
                    if command.contains(pattern) {
                        return Err(BrowserSupportError::permission_denied(
                            format!("Command contains forbidden pattern: {}", pattern)
                        ));
                    }
                }
                Ok(())
            }
            RiskLevel::High | RiskLevel::Critical => {
                // Allow most commands (but this should rarely be granted to browsers)
                Ok(())
            }
        }
    }

    /// Get command execution result
    pub async fn get_command_result(&self, request_id: Uuid) -> BrowserResult<BrowserCommandSession> {
        let sessions = self.browser_command_sessions.read().await;
        sessions.get(&request_id)
            .cloned()
            .ok_or_else(|| BrowserSupportError::not_found("Command session not found"))
    }

    /// Stream command output to browser
    pub async fn stream_command_output(
        &self,
        request_id: Uuid,
        output_line: String,
        is_error: bool,
    ) -> BrowserResult<()> {
        let mut sessions = self.browser_command_sessions.write().await;
        
        if let Some(session) = sessions.get_mut(&request_id) {
            if is_error {
                session.error_output.push(output_line);
            } else {
                session.output.push(output_line);
            }

            // In a real implementation, this would send through WebRTC data channel
            // to provide real-time output streaming to the browser

            Ok(())
        } else {
            Err(BrowserSupportError::not_found("Command session not found"))
        }
    }

    /// Cancel command execution
    pub async fn cancel_browser_command(&self, request_id: Uuid) -> BrowserResult<()> {
        // Get session info
        let session = {
            let sessions = self.browser_command_sessions.read().await;
            sessions.get(&request_id).cloned()
        };

        if let Some(_session) = session {
            // Update session status
            let mut sessions = self.browser_command_sessions.write().await;
            if let Some(s) = sessions.get_mut(&request_id) {
                s.status = ExecutionStatus::Cancelled;
            }

            // In a real implementation, this would send a cancel signal
            // through the command execution system

            Ok(())
        } else {
            Err(BrowserSupportError::not_found("Command session not found"))
        }
    }

    /// Get command history for browser session
    pub async fn get_command_history(&self, browser_session_id: &str) -> Vec<BrowserCommandSession> {
        let sessions = self.browser_command_sessions.read().await;
        sessions.values()
            .filter(|s| s.browser_session_id == browser_session_id)
            .cloned()
            .collect()
    }

    /// Get all active command sessions
    pub async fn get_active_commands(&self) -> Vec<BrowserCommandSession> {
        let sessions = self.browser_command_sessions.read().await;
        sessions.values()
            .filter(|s| matches!(s.status, ExecutionStatus::Executing | ExecutionStatus::Pending))
            .cloned()
            .collect()
    }

    /// Execute saved command template
    pub async fn execute_template(
        &self,
        browser_session: &BrowserSession,
        template_id: String,
        parameters: HashMap<String, String>,
        peer_id: PeerId,
    ) -> BrowserResult<Uuid> {
        // Check authorization
        let _auth_level = self.get_authorization(&browser_session.session_id.to_string()).await
            .ok_or_else(|| BrowserSupportError::permission_denied("No command execution authorization"))?;

        // In a real implementation, this would:
        // 1. Get template from command system
        // 2. Instantiate template with parameters
        // 3. Execute the instantiated command

        // For now, return an error indicating this feature is not yet implemented
        Err(BrowserSupportError::not_implemented("Template execution not yet implemented"))
    }

    /// Get available command templates
    pub async fn get_available_templates(&self) -> BrowserResult<Vec<String>> {
        // In a real implementation, this would query the command system for templates
        // For now, return an empty list
        Ok(vec![])
    }

    /// Clean up completed command sessions
    pub async fn cleanup_completed_sessions(&self, max_age_seconds: u64) -> usize {
        let mut sessions = self.browser_command_sessions.write().await;
        let initial_count = sessions.len();

        // Remove old completed sessions
        let now = std::time::SystemTime::now();
        sessions.retain(|_, session| {
            // Keep running/pending sessions
            if matches!(session.status, ExecutionStatus::Executing | ExecutionStatus::Pending) {
                return true;
            }

            // Keep recent completed sessions (for history)
            // In a real implementation, we'd track completion time
            true
        });

        initial_count - sessions.len()
    }

    /// Clean up sessions for disconnected browsers
    pub async fn cleanup_disconnected_sessions(&self, active_sessions: &[String]) -> usize {
        let mut sessions = self.browser_command_sessions.write().await;
        let initial_count = sessions.len();

        // Remove sessions for disconnected browsers
        sessions.retain(|_, session| {
            active_sessions.contains(&session.browser_session_id)
        });

        // Also clean up authorization levels
        let mut auth_levels = self.authorization_levels.write().await;
        auth_levels.retain(|session_id, _| active_sessions.contains(session_id));

        initial_count - sessions.len()
    }
}

/// Trait for browser command operations
#[async_trait]
pub trait BrowserCommand: Send + Sync {
    /// Request authorization
    async fn request_authorization(
        &self,
        browser_session: &BrowserSession,
        requested_level: RiskLevel,
    ) -> BrowserResult<RiskLevel>;

    /// Execute command
    async fn execute_command(
        &self,
        browser_session: &BrowserSession,
        command: String,
        peer_id: PeerId,
        working_directory: Option<String>,
        environment: Option<HashMap<String, String>>,
    ) -> BrowserResult<Uuid>;

    /// Get command result
    async fn get_result(&self, request_id: Uuid) -> BrowserResult<BrowserCommandSession>;

    /// Cancel command
    async fn cancel_command(&self, request_id: Uuid) -> BrowserResult<()>;

    /// Get command history
    async fn get_history(&self, browser_session_id: &str) -> Vec<BrowserCommandSession>;
}

#[async_trait]
impl BrowserCommand for BrowserCommandIntegration {
    async fn request_authorization(
        &self,
        browser_session: &BrowserSession,
        requested_level: RiskLevel,
    ) -> BrowserResult<RiskLevel> {
        self.request_authorization(browser_session, requested_level).await
    }

    async fn execute_command(
        &self,
        browser_session: &BrowserSession,
        command: String,
        peer_id: PeerId,
        working_directory: Option<String>,
        environment: Option<HashMap<String, String>>,
    ) -> BrowserResult<Uuid> {
        self.execute_browser_command(browser_session, command, peer_id, working_directory, environment).await
    }

    async fn get_result(&self, request_id: Uuid) -> BrowserResult<BrowserCommandSession> {
        self.get_command_result(request_id).await
    }

    async fn cancel_command(&self, request_id: Uuid) -> BrowserResult<()> {
        self.cancel_browser_command(request_id).await
    }

    async fn get_history(&self, browser_session_id: &str) -> Vec<BrowserCommandSession> {
        self.get_command_history(browser_session_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_command_session_creation() {
        let request_id = Uuid::new_v4();
        let session = BrowserCommandSession {
            request_id,
            browser_session_id: "test-session".to_string(),
            peer_id: "test-peer".to_string(),
            command: "ls -la".to_string(),
            status: ExecutionStatus::Executing,
            authorization_level: RiskLevel::Low,
            output: vec!["file1.txt".to_string(), "file2.txt".to_string()],
            error_output: vec![],
            exit_code: None,
        };

        assert_eq!(session.request_id, request_id);
        assert_eq!(session.browser_session_id, "test-session");
        assert_eq!(session.command, "ls -la");
        assert_eq!(session.output.len(), 2);
    }

    #[test]
    fn test_authorization_level_comparison() {
        assert_eq!(RiskLevel::Low, RiskLevel::Low);
        assert_ne!(RiskLevel::Low, RiskLevel::Medium);
    }
}
