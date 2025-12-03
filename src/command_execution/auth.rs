use async_trait::async_trait;
use crate::command_execution::{
    error::{CommandError, CommandResult as CmdResult},
    types::*,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, oneshot};
use tokio::time::{timeout, Duration};
use chrono::Utc;
use regex::Regex;

/// Authorization Manager trait for command authorization and trust management
#[async_trait]
pub trait AuthorizationManager: Send + Sync {
    /// Request authorization for a command execution
    async fn request_authorization(
        &self,
        request: AuthorizationRequest,
    ) -> CmdResult<AuthorizationDecision>;

    /// Add a command pattern to the trusted commands list
    async fn add_trusted_command(
        &self,
        command: CommandPattern,
        peer_id: PeerId,
    ) -> CmdResult<CommandId>;

    /// Remove a command from the trusted commands list
    async fn remove_trusted_command(&self, command_id: CommandId) -> CmdResult<()>;

    /// Check if a command matches any trusted patterns
    async fn is_trusted_command(
        &self,
        command: &str,
        peer_id: &PeerId,
    ) -> CmdResult<bool>;

    /// Update sandbox policy for a specific trust level
    async fn update_sandbox_policy(
        &self,
        risk_level: RiskLevel,
        policy: SandboxConfig,
    ) -> CmdResult<()>;

    /// Get authorization history
    async fn get_authorization_history(&self) -> CmdResult<Vec<AuthorizationRecord>>;

    /// Assess the risk level of a command
    async fn assess_risk_level(&self, command: &CommandRequest) -> CmdResult<RiskLevel>;
}

/// User prompt request for command authorization
#[derive(Debug)]
pub struct UserPromptRequest {
    pub request: AuthorizationRequest,
    pub response_channel: oneshot::Sender<AuthorizationDecision>,
}

/// Trusted command entry with pattern matching
#[derive(Debug, Clone)]
struct TrustedCommandEntry {
    pub id: CommandId,
    pub pattern: CommandPattern,
    pub regex: Regex,
    pub created_at: Timestamp,
}

/// Default implementation of AuthorizationManager
pub struct DefaultAuthorizationManager {
    /// Trusted commands list with pattern matching
    trusted_commands: Arc<RwLock<HashMap<CommandId, TrustedCommandEntry>>>,
    
    /// Sandbox policies for different risk levels
    sandbox_policies: Arc<RwLock<HashMap<RiskLevel, SandboxConfig>>>,
    
    /// Authorization history for audit trail
    authorization_history: Arc<RwLock<Vec<AuthorizationRecord>>>,
    
    /// Channel for sending user prompt requests
    user_prompt_tx: mpsc::Sender<UserPromptRequest>,
    
    /// Default authorization timeout
    default_timeout: Duration,
}

impl DefaultAuthorizationManager {
    /// Create a new DefaultAuthorizationManager
    pub fn new(user_prompt_tx: mpsc::Sender<UserPromptRequest>) -> Self {
        let mut sandbox_policies = HashMap::new();
        
        // Initialize default sandbox policies for each risk level
        sandbox_policies.insert(RiskLevel::Low, SandboxConfig {
            max_cpu_percent: 25,
            max_memory_mb: 256,
            max_execution_time: Duration::from_secs(30),
            allowed_directories: vec![],
            blocked_directories: vec![],
            network_access: NetworkAccess::None,
            environment_isolation: true,
            temp_directory: None,
        });
        
        sandbox_policies.insert(RiskLevel::Medium, SandboxConfig {
            max_cpu_percent: 50,
            max_memory_mb: 512,
            max_execution_time: Duration::from_secs(60),
            allowed_directories: vec![],
            blocked_directories: vec![],
            network_access: NetworkAccess::LocalOnly,
            environment_isolation: true,
            temp_directory: None,
        });
        
        sandbox_policies.insert(RiskLevel::High, SandboxConfig {
            max_cpu_percent: 75,
            max_memory_mb: 1024,
            max_execution_time: Duration::from_secs(120),
            allowed_directories: vec![],
            blocked_directories: vec![],
            network_access: NetworkAccess::Limited(vec![]),
            environment_isolation: true,
            temp_directory: None,
        });
        
        sandbox_policies.insert(RiskLevel::Critical, SandboxConfig {
            max_cpu_percent: 90,
            max_memory_mb: 2048,
            max_execution_time: Duration::from_secs(300),
            allowed_directories: vec![],
            blocked_directories: vec![],
            network_access: NetworkAccess::Full,
            environment_isolation: false,
            temp_directory: None,
        });
        
        Self {
            trusted_commands: Arc::new(RwLock::new(HashMap::new())),
            sandbox_policies: Arc::new(RwLock::new(sandbox_policies)),
            authorization_history: Arc::new(RwLock::new(Vec::new())),
            user_prompt_tx,
            default_timeout: Duration::from_secs(60),
        }
    }
    
    /// Check if a command matches a trusted pattern
    fn matches_pattern(&self, command: &str, entry: &TrustedCommandEntry) -> bool {
        entry.regex.is_match(command)
    }
    
    /// Record an authorization decision in the history
    async fn record_authorization(&self, request_id: RequestId, decision: AuthorizationDecision) {
        let record = AuthorizationRecord {
            request_id,
            decision,
            decided_at: Utc::now(),
            decided_by: "user".to_string(),
        };
        
        let mut history = self.authorization_history.write().await;
        history.push(record);
    }
    
    /// Get all trusted commands
    pub async fn get_trusted_commands(&self) -> Vec<(CommandId, CommandPattern)> {
        let trusted = self.trusted_commands.read().await;
        trusted.iter()
            .map(|(id, entry)| (*id, entry.pattern.clone()))
            .collect()
    }
    
    /// Update a trusted command pattern
    pub async fn update_trusted_command(
        &self,
        command_id: CommandId,
        new_pattern: CommandPattern,
    ) -> CmdResult<()> {
        let mut trusted = self.trusted_commands.write().await;
        
        let entry = trusted.get_mut(&command_id)
            .ok_or_else(|| CommandError::InvalidRequest("Command ID not found".to_string()))?;
        
        // Compile the new pattern as a regex
        let regex = Regex::new(&new_pattern.pattern)
            .map_err(|e| CommandError::InvalidRequest(format!("Invalid pattern: {}", e)))?;
        
        entry.pattern = new_pattern;
        entry.regex = regex;
        
        Ok(())
    }
    
    /// Find matching trusted command for a given command and peer
    pub async fn find_matching_trusted_command(
        &self,
        command: &str,
        peer_id: &PeerId,
    ) -> Option<CommandId> {
        let trusted = self.trusted_commands.read().await;
        
        for (id, entry) in trusted.iter() {
            // Check if the peer is allowed for this pattern
            if !entry.pattern.allowed_peers.is_empty() 
                && !entry.pattern.allowed_peers.contains(peer_id) {
                continue;
            }
            
            // Check if the command matches the pattern
            if self.matches_pattern(command, entry) {
                return Some(*id);
            }
        }
        
        None
    }
    
    /// Get sandbox policy for a specific risk level
    pub async fn get_sandbox_policy(&self, risk_level: RiskLevel) -> CmdResult<SandboxConfig> {
        let policies = self.sandbox_policies.read().await;
        policies.get(&risk_level)
            .cloned()
            .ok_or_else(|| CommandError::Internal(format!("No policy found for risk level {:?}", risk_level)))
    }
}

#[async_trait]
impl AuthorizationManager for DefaultAuthorizationManager {
    async fn request_authorization(
        &self,
        request: AuthorizationRequest,
    ) -> CmdResult<AuthorizationDecision> {
        let request_id = request.request_id;
        let timeout_duration = request.timeout;
        
        // Create a oneshot channel for the response
        let (tx, rx) = oneshot::channel();
        
        // Send the prompt request to the user interface
        let prompt_request = UserPromptRequest {
            request,
            response_channel: tx,
        };
        
        self.user_prompt_tx
            .send(prompt_request)
            .await
            .map_err(|_| CommandError::Internal("Failed to send authorization request".to_string()))?;
        
        // Wait for the user's decision with timeout
        let decision = match timeout(timeout_duration, rx).await {
            Ok(Ok(decision)) => decision,
            Ok(Err(_)) => {
                // Channel was closed without a response
                AuthorizationDecision::Timeout
            }
            Err(_) => {
                // Timeout occurred
                AuthorizationDecision::Timeout
            }
        };
        
        // Record the authorization decision
        self.record_authorization(request_id, decision.clone()).await;
        
        Ok(decision)
    }
    
    async fn add_trusted_command(
        &self,
        command: CommandPattern,
        _peer_id: PeerId,
    ) -> CmdResult<CommandId> {
        let command_id = CommandId::new_v4();
        
        // Compile the pattern as a regex
        let regex = Regex::new(&command.pattern)
            .map_err(|e| CommandError::InvalidRequest(format!("Invalid pattern: {}", e)))?;
        
        let entry = TrustedCommandEntry {
            id: command_id,
            pattern: command,
            regex,
            created_at: Utc::now(),
        };
        
        let mut trusted = self.trusted_commands.write().await;
        trusted.insert(command_id, entry);
        
        Ok(command_id)
    }
    
    async fn remove_trusted_command(&self, command_id: CommandId) -> CmdResult<()> {
        let mut trusted = self.trusted_commands.write().await;
        trusted.remove(&command_id)
            .ok_or_else(|| CommandError::InvalidRequest("Command ID not found".to_string()))?;
        Ok(())
    }
    
    async fn is_trusted_command(
        &self,
        command: &str,
        peer_id: &PeerId,
    ) -> CmdResult<bool> {
        let trusted = self.trusted_commands.read().await;
        
        for entry in trusted.values() {
            // Check if the peer is allowed for this pattern
            if !entry.pattern.allowed_peers.is_empty() 
                && !entry.pattern.allowed_peers.contains(peer_id) {
                continue;
            }
            
            // Check if the command matches the pattern
            if self.matches_pattern(command, entry) {
                return Ok(true);
            }
        }
        
        Ok(false)
    }
    
    async fn update_sandbox_policy(
        &self,
        risk_level: RiskLevel,
        policy: SandboxConfig,
    ) -> CmdResult<()> {
        let mut policies = self.sandbox_policies.write().await;
        policies.insert(risk_level, policy);
        Ok(())
    }
    
    async fn get_authorization_history(&self) -> CmdResult<Vec<AuthorizationRecord>> {
        let history = self.authorization_history.read().await;
        Ok(history.clone())
    }
    
    async fn assess_risk_level(&self, command: &CommandRequest) -> CmdResult<RiskLevel> {
        // Assess risk based on command content and requested permissions
        let command_lower = command.command.to_lowercase();
        let full_command = format!("{} {}", command.command, command.arguments.join(" ")).to_lowercase();
        
        // Critical risk indicators
        let critical_keywords = [
            "rm -rf", "del /f", "format", "mkfs", "dd if=", 
            "sudo", "su ", "chmod 777", "chown root",
            "iptables", "firewall", "setenforce",
            "reboot", "shutdown", "poweroff", "init 0",
        ];
        
        for keyword in &critical_keywords {
            if full_command.contains(keyword) || command_lower.contains(keyword) {
                return Ok(RiskLevel::Critical);
            }
        }
        
        // High risk indicators
        let high_keywords = [
            "rm ", "del ", "remove", "delete", "kill",
            "wget", "curl", "nc ", "netcat", "ssh",
            "chmod", "chown", "systemctl", "service",
            "apt install", "yum install", "dnf install",
            "pip install", "npm install -g",
        ];
        
        for keyword in &high_keywords {
            if full_command.contains(keyword) || command_lower.contains(keyword) {
                return Ok(RiskLevel::High);
            }
        }
        
        // Medium risk indicators
        let medium_keywords = [
            "cp ", "mv ", "copy", "move", "write",
            "echo >", "cat >", "tee", "touch",
            "mkdir", "rmdir",
        ];
        
        for keyword in &medium_keywords {
            if full_command.contains(keyword) || command_lower.contains(keyword) {
                return Ok(RiskLevel::Medium);
            }
        }
        
        // Check network access requirement
        match command.sandbox_config.network_access {
            NetworkAccess::Full => return Ok(RiskLevel::High),
            NetworkAccess::Limited(_) => return Ok(RiskLevel::Medium),
            _ => {}
        }
        
        // Check if command accesses sensitive directories
        if let Some(working_dir) = &command.working_directory {
            let sensitive_paths = ["/etc", "/sys", "/proc", "/boot", "C:\\Windows", "C:\\Program Files"];
            let working_dir_str = working_dir.to_string_lossy().to_lowercase();
            
            for sensitive in &sensitive_paths {
                if working_dir_str.starts_with(&sensitive.to_lowercase()) {
                    return Ok(RiskLevel::High);
                }
            }
        }
        
        // Default to low risk for read-only operations
        Ok(RiskLevel::Low)
    }
}

/// Security policy configuration
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Whether to require authorization for all commands
    pub require_authorization: bool,
    
    /// Whether to allow automatic approval for trusted commands
    pub allow_trusted_auto_approval: bool,
    
    /// Maximum allowed risk level without explicit approval
    pub max_auto_approve_risk: RiskLevel,
    
    /// Default timeout for authorization requests
    pub default_timeout: Duration,
    
    /// Whether to log all authorization decisions
    pub log_all_decisions: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            require_authorization: true,
            allow_trusted_auto_approval: true,
            max_auto_approve_risk: RiskLevel::Low,
            default_timeout: Duration::from_secs(60),
            log_all_decisions: true,
        }
    }
}

/// Authorization manager with security policy enforcement
pub struct PolicyEnforcedAuthorizationManager {
    inner: DefaultAuthorizationManager,
    policy: Arc<RwLock<SecurityPolicy>>,
}

impl PolicyEnforcedAuthorizationManager {
    /// Create a new PolicyEnforcedAuthorizationManager
    pub fn new(user_prompt_tx: mpsc::Sender<UserPromptRequest>, policy: SecurityPolicy) -> Self {
        Self {
            inner: DefaultAuthorizationManager::new(user_prompt_tx),
            policy: Arc::new(RwLock::new(policy)),
        }
    }
    
    /// Update the security policy
    pub async fn update_policy(&self, policy: SecurityPolicy) {
        let mut current_policy = self.policy.write().await;
        *current_policy = policy;
    }
    
    /// Get the current security policy
    pub async fn get_policy(&self) -> SecurityPolicy {
        let policy = self.policy.read().await;
        policy.clone()
    }
    
    /// Check if a command should be auto-approved based on policy
    async fn should_auto_approve(
        &self,
        command: &str,
        peer_id: &PeerId,
        risk_level: RiskLevel,
    ) -> CmdResult<bool> {
        let policy = self.policy.read().await;
        
        // If authorization is not required, auto-approve
        if !policy.require_authorization {
            return Ok(true);
        }
        
        // If trusted auto-approval is disabled, don't auto-approve
        if !policy.allow_trusted_auto_approval {
            return Ok(false);
        }
        
        // Check if risk level is within auto-approve threshold
        if risk_level > policy.max_auto_approve_risk {
            return Ok(false);
        }
        
        // Check if command is trusted
        self.inner.is_trusted_command(command, peer_id).await
    }
}

#[async_trait]
impl AuthorizationManager for PolicyEnforcedAuthorizationManager {
    async fn request_authorization(
        &self,
        request: AuthorizationRequest,
    ) -> CmdResult<AuthorizationDecision> {
        let policy = self.policy.read().await;
        
        // Check if we should auto-approve based on policy
        let should_auto = self.should_auto_approve(
            &request.command_preview,
            &request.requester,
            request.risk_level,
        ).await?;
        
        if should_auto {
            let decision = AuthorizationDecision::Approved;
            
            if policy.log_all_decisions {
                self.inner.record_authorization(request.request_id, decision.clone()).await;
            }
            
            return Ok(decision);
        }
        
        // Otherwise, request user authorization
        self.inner.request_authorization(request).await
    }
    
    async fn add_trusted_command(
        &self,
        command: CommandPattern,
        peer_id: PeerId,
    ) -> CmdResult<CommandId> {
        self.inner.add_trusted_command(command, peer_id).await
    }
    
    async fn remove_trusted_command(&self, command_id: CommandId) -> CmdResult<()> {
        self.inner.remove_trusted_command(command_id).await
    }
    
    async fn is_trusted_command(
        &self,
        command: &str,
        peer_id: &PeerId,
    ) -> CmdResult<bool> {
        self.inner.is_trusted_command(command, peer_id).await
    }
    
    async fn update_sandbox_policy(
        &self,
        risk_level: RiskLevel,
        policy: SandboxConfig,
    ) -> CmdResult<()> {
        self.inner.update_sandbox_policy(risk_level, policy).await
    }
    
    async fn get_authorization_history(&self) -> CmdResult<Vec<AuthorizationRecord>> {
        self.inner.get_authorization_history().await
    }
    
    async fn assess_risk_level(&self, command: &CommandRequest) -> CmdResult<RiskLevel> {
        self.inner.assess_risk_level(command).await
    }
}

#[cfg(test)]
#[path = "auth_test.rs"]
mod auth_test;
