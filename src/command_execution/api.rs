// Unified Command Execution API
//
// This module provides a high-level, event-driven API for command execution that abstracts
// platform differences, security complexity, and transport details.

use async_trait::async_trait;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{RwLock, mpsc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::command_execution::{
    CommandRequest, CommandResult, ScriptRequest, ScriptResult, Notification,
    NotificationResult, SystemInfo, SystemInfoQuery, PeerId, ExecutionStatus,
    CommandManager, AuthorizationManager, SandboxEngine, ScriptEngine,
    UnifiedCommandManager,
};
use crate::command_execution::system_info::SystemInfoProvider;
use crate::command_execution::notification::NotificationManager;
use crate::command_execution::error::{CommandError, CommandResult as CmdResult};
use crate::command_execution::security_integration::CommandSecurityIntegration;
use crate::command_execution::transport_integration::{
    CommandTransportIntegration, CommandExecutionConfig,
};
use crate::transport::PeerAddress;

/// Command execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommandExecutionEvent {
    /// Command request received
    CommandReceived {
        request_id: Uuid,
        peer_id: PeerId,
        command: String,
    },
    /// Command authorization requested
    AuthorizationRequested {
        request_id: Uuid,
        peer_id: PeerId,
    },
    /// Command authorized
    CommandAuthorized {
        request_id: Uuid,
    },
    /// Command authorization denied
    CommandDenied {
        request_id: Uuid,
        reason: String,
    },
    /// Command execution started
    ExecutionStarted {
        request_id: Uuid,
    },
    /// Command execution progress update
    ExecutionProgress {
        request_id: Uuid,
        progress: f32,
        message: String,
    },
    /// Command execution completed
    ExecutionCompleted {
        request_id: Uuid,
        exit_code: i32,
    },
    /// Command execution failed
    ExecutionFailed {
        request_id: Uuid,
        error: String,
    },
    /// Script execution started
    ScriptStarted {
        request_id: Uuid,
    },
    /// Script execution completed
    ScriptCompleted {
        request_id: Uuid,
        lines_executed: usize,
    },
    /// System info query received
    SystemInfoQueried {
        query_id: Uuid,
        peer_id: PeerId,
    },
    /// Notification received
    NotificationReceived {
        notification_id: Uuid,
        peer_id: PeerId,
        title: String,
    },
    /// Connection established
    ConnectionEstablished {
        peer_id: PeerId,
    },
    /// Connection lost
    ConnectionLost {
        peer_id: PeerId,
        reason: String,
    },
}

/// Callback trait for command execution events
#[async_trait]
pub trait CommandExecutionCallback: Send + Sync {
    /// Called when a command execution event occurs
    async fn on_event(&self, event: CommandExecutionEvent);
}

/// Unified command execution API
pub struct CommandExecution {
    /// Command manager for local execution
    command_manager: Arc<dyn CommandManager>,
    /// Authorization manager
    authorization_manager: Arc<dyn AuthorizationManager>,
    /// System info provider
    system_info_provider: Arc<SystemInfoProvider>,
    /// Notification manager
    notification_manager: Arc<NotificationManager>,
    /// Transport integration for remote execution
    transport_integration: Arc<CommandTransportIntegration>,
    /// Security integration
    security_integration: Arc<CommandSecurityIntegration>,
    /// Configuration
    config: CommandExecutionConfig,
    /// Event callbacks
    callbacks: Arc<RwLock<Vec<Arc<dyn CommandExecutionCallback>>>>,
    /// Event sender
    event_sender: mpsc::UnboundedSender<CommandExecutionEvent>,
    /// Event receiver
    event_receiver: Arc<RwLock<mpsc::UnboundedReceiver<CommandExecutionEvent>>>,
    /// Active executions
    active_executions: Arc<RwLock<HashMap<Uuid, ExecutionStatus>>>,
}

impl CommandExecution {
    /// Create a new command execution API
    pub fn new(
        command_manager: Arc<dyn CommandManager>,
        authorization_manager: Arc<dyn AuthorizationManager>,
        system_info_provider: Arc<SystemInfoProvider>,
        notification_manager: Arc<NotificationManager>,
        transport_integration: Arc<CommandTransportIntegration>,
        security_integration: Arc<CommandSecurityIntegration>,
        config: CommandExecutionConfig,
    ) -> Self {
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        Self {
            command_manager,
            authorization_manager,
            system_info_provider,
            notification_manager,
            transport_integration,
            security_integration,
            config,
            callbacks: Arc::new(RwLock::new(Vec::new())),
            event_sender,
            event_receiver: Arc::new(RwLock::new(event_receiver)),
            active_executions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an event callback
    pub async fn register_callback(&self, callback: Arc<dyn CommandExecutionCallback>) {
        let mut callbacks = self.callbacks.write().await;
        callbacks.push(callback);
    }

    /// Emit an event to all registered callbacks
    async fn emit_event(&self, event: CommandExecutionEvent) {
        // Send to event channel
        let _ = self.event_sender.send(event.clone());

        // Notify all callbacks
        let callbacks = self.callbacks.read().await;
        for callback in callbacks.iter() {
            callback.on_event(event.clone()).await;
        }
    }

    /// Execute a command on a remote peer
    pub async fn execute_remote_command(
        &self,
        request: CommandRequest,
        peer_address: &PeerAddress,
    ) -> CmdResult<CommandResult> {
        let request_id = request.request_id;

        // Update execution status
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(request_id, ExecutionStatus::Pending);
        }

        // Emit event
        self.emit_event(CommandExecutionEvent::CommandReceived {
            request_id,
            peer_id: peer_address.peer_id.clone(),
            command: request.command.clone(),
        }).await;

        // Send command request
        self.emit_event(CommandExecutionEvent::ExecutionStarted {
            request_id,
        }).await;

        let result = self.transport_integration
            .send_command_request(request, peer_address)
            .await;

        // Update execution status and emit event
        match &result {
            Ok(cmd_result) => {
                {
                    let mut executions = self.active_executions.write().await;
                    executions.insert(request_id, ExecutionStatus::Completed);
                }
                self.emit_event(CommandExecutionEvent::ExecutionCompleted {
                    request_id,
                    exit_code: cmd_result.exit_code,
                }).await;
            }
            Err(e) => {
                {
                    let mut executions = self.active_executions.write().await;
                    executions.insert(request_id, ExecutionStatus::Failed(e.to_string()));
                }
                self.emit_event(CommandExecutionEvent::ExecutionFailed {
                    request_id,
                    error: e.to_string(),
                }).await;
            }
        }

        result
    }

    /// Execute a command locally
    pub async fn execute_local_command(
        &self,
        request: CommandRequest,
    ) -> CmdResult<CommandResult> {
        let request_id = request.request_id;

        // Update execution status
        {
            let mut executions = self.active_executions.write().await;
            executions.insert(request_id, ExecutionStatus::Pending);
        }

        // Emit event
        self.emit_event(CommandExecutionEvent::ExecutionStarted {
            request_id,
        }).await;

        // Execute command
        let result = self.command_manager.execute_command(request).await;

        // Update execution status and emit event
        match &result {
            Ok(cmd_result) => {
                {
                    let mut executions = self.active_executions.write().await;
                    executions.insert(request_id, ExecutionStatus::Completed);
                }
                self.emit_event(CommandExecutionEvent::ExecutionCompleted {
                    request_id,
                    exit_code: cmd_result.exit_code,
                }).await;
            }
            Err(e) => {
                {
                    let mut executions = self.active_executions.write().await;
                    executions.insert(request_id, ExecutionStatus::Failed(e.to_string()));
                }
                self.emit_event(CommandExecutionEvent::ExecutionFailed {
                    request_id,
                    error: e.to_string(),
                }).await;
            }
        }

        result
    }

    /// Execute a script on a remote peer
    pub async fn execute_remote_script(
        &self,
        request: ScriptRequest,
        peer_address: &PeerAddress,
    ) -> CmdResult<ScriptResult> {
        let request_id = request.request_id;

        // Emit event
        self.emit_event(CommandExecutionEvent::ScriptStarted {
            request_id,
        }).await;

        let result = self.transport_integration
            .send_script_request(request, peer_address)
            .await;

        // Emit completion event
        if let Ok(script_result) = &result {
            self.emit_event(CommandExecutionEvent::ScriptCompleted {
                request_id,
                lines_executed: script_result.lines_executed,
            }).await;
        }

        result
    }

    /// Execute a script locally
    pub async fn execute_local_script(
        &self,
        request: ScriptRequest,
    ) -> CmdResult<ScriptResult> {
        let request_id = request.request_id;

        // Emit event
        self.emit_event(CommandExecutionEvent::ScriptStarted {
            request_id,
        }).await;

        // Execute script
        let result = self.command_manager.execute_script(request).await;

        // Emit completion event
        if let Ok(script_result) = &result {
            self.emit_event(CommandExecutionEvent::ScriptCompleted {
                request_id,
                lines_executed: script_result.lines_executed,
            }).await;
        }

        result
    }

    /// Query system information from a remote peer
    pub async fn query_remote_system_info(
        &self,
        query: SystemInfoQuery,
        peer_address: &PeerAddress,
    ) -> CmdResult<SystemInfo> {
        let query_id = query.query_id;

        // Emit event
        self.emit_event(CommandExecutionEvent::SystemInfoQueried {
            query_id,
            peer_id: peer_address.peer_id.clone(),
        }).await;

        self.transport_integration
            .send_system_info_query(query, peer_address)
            .await
    }

    /// Query local system information
    pub async fn query_local_system_info(
        &self,
        query: SystemInfoQuery,
    ) -> CmdResult<SystemInfo> {
        let query_id = query.query_id;

        // Emit event
        self.emit_event(CommandExecutionEvent::SystemInfoQueried {
            query_id,
            peer_id: "local".to_string(),
        }).await;

        self.system_info_provider.get_system_info(query.cache_duration).await
    }

    /// Send a notification to a remote peer
    pub async fn send_remote_notification(
        &self,
        notification: Notification,
        peer_address: &PeerAddress,
    ) -> CmdResult<()> {
        self.transport_integration
            .send_notification(notification, peer_address)
            .await
    }

    /// Display a local notification
    pub async fn send_local_notification(
        &self,
        notification: Notification,
    ) -> CmdResult<uuid::Uuid> {
        let notification_id = notification.notification_id;
        let sender = notification.sender.clone();

        // Emit event
        self.emit_event(CommandExecutionEvent::NotificationReceived {
            notification_id,
            peer_id: sender.clone(),
            title: notification.title.clone(),
        }).await;

        self.notification_manager.send_notification(notification, sender).await
    }

    /// Get execution status
    pub async fn get_execution_status(&self, request_id: &Uuid) -> Option<ExecutionStatus> {
        let executions = self.active_executions.read().await;
        executions.get(request_id).cloned()
    }

    /// Get all active executions
    pub async fn get_active_executions(&self) -> HashMap<Uuid, ExecutionStatus> {
        let executions = self.active_executions.read().await;
        executions.clone()
    }

    /// Cancel an execution
    pub async fn cancel_execution(&self, request_id: &Uuid) -> CmdResult<()> {
        let mut executions = self.active_executions.write().await;
        if let Some(status) = executions.get_mut(request_id) {
            *status = ExecutionStatus::Cancelled;
            Ok(())
        } else {
            Err(CommandError::InvalidRequest(
                format!("Execution {} not found", request_id)
            ))
        }
    }

    /// Disconnect from a peer
    pub async fn disconnect_peer(&self, peer_id: &PeerId) -> CmdResult<()> {
        self.transport_integration.disconnect_peer(peer_id).await?;

        self.emit_event(CommandExecutionEvent::ConnectionLost {
            peer_id: peer_id.clone(),
            reason: "User requested disconnect".to_string(),
        }).await;

        Ok(())
    }

    /// Get connected peers
    pub async fn get_connected_peers(&self) -> Vec<PeerId> {
        self.transport_integration.get_active_peers().await
    }

    /// Check if connected to a peer
    pub async fn is_connected(&self, peer_id: &PeerId) -> bool {
        self.transport_integration.is_connected(peer_id).await
    }

    /// Get configuration
    pub fn config(&self) -> &CommandExecutionConfig {
        &self.config
    }
}

/// Builder for CommandExecution
pub struct CommandExecutionBuilder {
    command_manager: Option<Arc<dyn CommandManager>>,
    authorization_manager: Option<Arc<dyn AuthorizationManager>>,
    system_info_provider: Option<Arc<SystemInfoProvider>>,
    notification_manager: Option<Arc<NotificationManager>>,
    transport_integration: Option<Arc<CommandTransportIntegration>>,
    security_integration: Option<Arc<CommandSecurityIntegration>>,
    config: CommandExecutionConfig,
}

impl CommandExecutionBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            command_manager: None,
            authorization_manager: None,
            system_info_provider: None,
            notification_manager: None,
            transport_integration: None,
            security_integration: None,
            config: CommandExecutionConfig::default(),
        }
    }

    /// Set command manager
    pub fn command_manager(mut self, manager: Arc<dyn CommandManager>) -> Self {
        self.command_manager = Some(manager);
        self
    }

    /// Set authorization manager
    pub fn authorization_manager(mut self, manager: Arc<dyn AuthorizationManager>) -> Self {
        self.authorization_manager = Some(manager);
        self
    }

    /// Set system info provider
    pub fn system_info_provider(mut self, provider: Arc<SystemInfoProvider>) -> Self {
        self.system_info_provider = Some(provider);
        self
    }

    /// Set notification manager
    pub fn notification_manager(mut self, manager: Arc<NotificationManager>) -> Self {
        self.notification_manager = Some(manager);
        self
    }

    /// Set transport integration
    pub fn transport_integration(mut self, integration: Arc<CommandTransportIntegration>) -> Self {
        self.transport_integration = Some(integration);
        self
    }

    /// Set security integration
    pub fn security_integration(mut self, integration: Arc<CommandSecurityIntegration>) -> Self {
        self.security_integration = Some(integration);
        self
    }

    /// Set configuration
    pub fn config(mut self, config: CommandExecutionConfig) -> Self {
        self.config = config;
        self
    }

    /// Build the CommandExecution instance
    pub fn build(self) -> CmdResult<CommandExecution> {
        Ok(CommandExecution::new(
            self.command_manager.ok_or_else(|| {
                CommandError::InvalidRequest("Command manager not set".to_string())
            })?,
            self.authorization_manager.ok_or_else(|| {
                CommandError::InvalidRequest("Authorization manager not set".to_string())
            })?,
            self.system_info_provider.ok_or_else(|| {
                CommandError::InvalidRequest("System info provider not set".to_string())
            })?,
            self.notification_manager.ok_or_else(|| {
                CommandError::InvalidRequest("Notification manager not set".to_string())
            })?,
            self.transport_integration.ok_or_else(|| {
                CommandError::InvalidRequest("Transport integration not set".to_string())
            })?,
            self.security_integration.ok_or_else(|| {
                CommandError::InvalidRequest("Security integration not set".to_string())
            })?,
            self.config,
        ))
    }
}

impl Default for CommandExecutionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_execution_event_serialization() {
        let event = CommandExecutionEvent::CommandReceived {
            request_id: Uuid::new_v4(),
            peer_id: "test-peer".to_string(),
            command: "echo test".to_string(),
        };

        // Test JSON serialization
        let json = serde_json::to_string(&event).expect("Failed to serialize");
        let deserialized: CommandExecutionEvent = 
            serde_json::from_str(&json).expect("Failed to deserialize");

        match (event, deserialized) {
            (
                CommandExecutionEvent::CommandReceived { request_id: id1, peer_id: p1, command: c1 },
                CommandExecutionEvent::CommandReceived { request_id: id2, peer_id: p2, command: c2 },
            ) => {
                assert_eq!(id1, id2);
                assert_eq!(p1, p2);
                assert_eq!(c1, c2);
            }
            _ => panic!("Event type mismatch"),
        }
    }

    #[test]
    fn test_builder_pattern() {
        let builder = CommandExecutionBuilder::new()
            .config(CommandExecutionConfig::default());

        // Verify builder can be created
        assert!(builder.command_manager.is_none());
        assert!(builder.authorization_manager.is_none());
    }
}
