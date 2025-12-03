// Unified command manager with platform-specific backends
//
// This module provides a unified interface for command execution that automatically
// routes commands to the appropriate platform-specific executor.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::command_execution::{
    error::{CommandError, CommandResult},
    types::{self, *},
    manager::CommandManager,
};

use super::executor::{ExecutionContext, PlatformExecutor};
use super::translator::CommandTranslator;

#[cfg(target_os = "windows")]
use super::windows::WindowsExecutor;

#[cfg(any(target_os = "macos", target_os = "linux"))]
use super::unix::UnixExecutor;

/// Execution queue entry
#[derive(Debug, Clone)]
struct QueuedExecution {
    execution_id: ExecutionId,
    request: CommandRequest,
    status: ExecutionStatus,
    result: Option<types::CommandResult>,
}

/// Unified command manager implementation
pub struct UnifiedCommandManager {
    executor: Arc<dyn PlatformExecutor>,
    translator: CommandTranslator,
    execution_queue: Arc<RwLock<HashMap<ExecutionId, QueuedExecution>>>,
    history: Arc<RwLock<Vec<CommandHistoryEntry>>>,
    max_history_size: usize,
}

impl UnifiedCommandManager {
    /// Create a new unified command manager
    pub fn new() -> CommandResult<Self> {
        let executor = Self::create_platform_executor()?;
        let translator = CommandTranslator::for_current_platform();

        Ok(Self {
            executor,
            translator,
            execution_queue: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 1000,
        })
    }

    /// Create a new manager with custom translator
    pub fn with_translator(translator: CommandTranslator) -> CommandResult<Self> {
        let executor = Self::create_platform_executor()?;

        Ok(Self {
            executor,
            translator,
            execution_queue: Arc::new(RwLock::new(HashMap::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            max_history_size: 1000,
        })
    }

    /// Create the appropriate platform-specific executor
    fn create_platform_executor() -> CommandResult<Arc<dyn PlatformExecutor>> {
        #[cfg(target_os = "windows")]
        {
            let executor = WindowsExecutor::new()?;
            Ok(Arc::new(executor))
        }

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let executor = UnixExecutor::new()?;
            Ok(Arc::new(executor))
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Err(CommandError::platform_error("Unsupported platform"))
        }
    }

    /// Translate command if needed
    fn translate_command_request(&self, request: &CommandRequest) -> CommandResult<(String, Vec<String>)> {
        self.translator.translate_command_line(&request.command, &request.arguments)
    }

    /// Create execution context from command request
    fn create_execution_context(&self, request: &CommandRequest) -> CommandResult<ExecutionContext> {
        let (command, arguments) = self.translate_command_request(request)?;

        let mut context = ExecutionContext::new(command, arguments)
            .with_timeout(request.timeout)
            .with_environment(request.environment.clone());

        if let Some(ref dir) = request.working_directory {
            context = context.with_working_directory(dir.clone());
        }

        Ok(context)
    }

    /// Add execution to queue
    async fn queue_execution(&self, request: CommandRequest) -> ExecutionId {
        let execution_id = Uuid::new_v4();
        let queued = QueuedExecution {
            execution_id,
            request: request.clone(),
            status: ExecutionStatus::Pending,
            result: None,
        };

        let mut queue = self.execution_queue.write().await;
        queue.insert(execution_id, queued);
        execution_id
    }

    /// Update execution status
    async fn update_execution_status(&self, execution_id: ExecutionId, status: ExecutionStatus) {
        let mut queue = self.execution_queue.write().await;
        if let Some(execution) = queue.get_mut(&execution_id) {
            execution.status = status;
        }
    }

    /// Store execution result
    async fn store_execution_result(&self, execution_id: ExecutionId, result: types::CommandResult) {
        let mut queue = self.execution_queue.write().await;
        if let Some(execution) = queue.get_mut(&execution_id) {
            execution.result = Some(result);
            execution.status = ExecutionStatus::Completed;
        }
    }

    /// Add to history
    async fn add_to_history(&self, entry: CommandHistoryEntry) {
        let mut history = self.history.write().await;
        history.push(entry);

        // Trim history if it exceeds max size
        if history.len() > self.max_history_size {
            let excess = history.len() - self.max_history_size;
            history.drain(0..excess);
        }
    }

    /// Remove from queue
    async fn remove_from_queue(&self, execution_id: ExecutionId) {
        let mut queue = self.execution_queue.write().await;
        queue.remove(&execution_id);
    }
}

impl Default for UnifiedCommandManager {
    fn default() -> Self {
        Self::new().expect("Failed to create unified command manager")
    }
}

#[async_trait]
impl CommandManager for UnifiedCommandManager {
    async fn execute_command(&self, request: CommandRequest) -> CommandResult<types::CommandResult> {
        let execution_id = self.queue_execution(request.clone()).await;

        // Update status to executing
        self.update_execution_status(execution_id, ExecutionStatus::Executing).await;

        // Create execution context
        let context = self.create_execution_context(&request)?;

        // Execute the command
        let result = self.executor.execute(context).await;

        // Convert platform result to types::CommandResult
        let cmd_result = match result {
            Ok(exec_result) => {
                let cmd_result = types::CommandResult {
                    request_id: request.request_id,
                    exit_code: exec_result.exit_code,
                    stdout: exec_result.stdout,
                    stderr: exec_result.stderr,
                    execution_time: exec_result.execution_time,
                    resource_usage: exec_result.resource_usage,
                    completed_at: chrono::Utc::now(),
                };

                // Store result
                self.store_execution_result(execution_id, cmd_result.clone()).await;

                // Add to history
                let history_entry = CommandHistoryEntry {
                    entry_id: Uuid::new_v4(),
                    command_request: request.clone(),
                    result: Some(cmd_result.clone()),
                    authorization: AuthorizationRecord {
                        request_id: request.request_id,
                        decision: AuthorizationDecision::Approved,
                        decided_at: chrono::Utc::now(),
                        decided_by: "system".to_string(),
                    },
                    execution_status: ExecutionStatus::Completed,
                    created_at: request.created_at,
                    completed_at: Some(chrono::Utc::now()),
                };
                self.add_to_history(history_entry).await;

                Ok(cmd_result)
            }
            Err(e) => {
                // Update status to failed
                self.update_execution_status(
                    execution_id,
                    ExecutionStatus::Failed(e.to_string())
                ).await;

                // Add to history with error
                let history_entry = CommandHistoryEntry {
                    entry_id: Uuid::new_v4(),
                    command_request: request.clone(),
                    result: None,
                    authorization: AuthorizationRecord {
                        request_id: request.request_id,
                        decision: AuthorizationDecision::Approved,
                        decided_at: chrono::Utc::now(),
                        decided_by: "system".to_string(),
                    },
                    execution_status: ExecutionStatus::Failed(e.to_string()),
                    created_at: request.created_at,
                    completed_at: Some(chrono::Utc::now()),
                };
                self.add_to_history(history_entry).await;

                Err(e)
            }
        };

        // Remove from queue
        self.remove_from_queue(execution_id).await;

        cmd_result
    }

    async fn execute_script(&self, script: ScriptRequest) -> CommandResult<ScriptResult> {
        // Determine which executor method to use based on script language
        let result = match script.language {
            ScriptLanguage::PowerShell => {
                self.executor.execute_powershell(&script.content, script.parameters.clone()).await
            }
            ScriptLanguage::Bash | ScriptLanguage::Batch | ScriptLanguage::Auto => {
                self.executor.execute_shell_script(&script.content, script.parameters.clone()).await
            }
            _ => {
                return Err(CommandError::script_error(format!(
                    "Unsupported script language: {:?}",
                    script.language
                )));
            }
        };

        // Convert platform result to ScriptResult
        match result {
            Ok(exec_result) => {
                let lines_executed = exec_result.stdout.lines().count();
                Ok(ScriptResult {
                    request_id: script.request_id,
                    exit_code: exec_result.exit_code,
                    output: exec_result.stdout,
                    errors: if !exec_result.stderr.is_empty() {
                        vec![ScriptError {
                            line: None,
                            message: exec_result.stderr,
                            error_type: "stderr".to_string(),
                        }]
                    } else {
                        vec![]
                    },
                    execution_time: exec_result.execution_time,
                    lines_executed,
                })
            }
            Err(e) => Err(e),
        }
    }

    async fn query_system_info(&self, _query: SystemInfoQuery) -> CommandResult<SystemInfo> {
        // System info query will be implemented in a later task
        Err(CommandError::Internal("System info query not yet implemented".to_string()))
    }

    async fn send_notification(&self, _notification: Notification) -> CommandResult<NotificationResult> {
        // Notification sending will be implemented in a later task
        Err(CommandError::Internal("Notification sending not yet implemented".to_string()))
    }

    async fn get_execution_status(&self, execution_id: ExecutionId) -> CommandResult<ExecutionStatus> {
        let queue = self.execution_queue.read().await;
        queue
            .get(&execution_id)
            .map(|e| e.status.clone())
            .ok_or_else(|| CommandError::invalid_request("Execution not found"))
    }

    async fn cancel_execution(&self, execution_id: ExecutionId) -> CommandResult<()> {
        self.update_execution_status(execution_id, ExecutionStatus::Cancelled).await;
        self.remove_from_queue(execution_id).await;
        Ok(())
    }

    async fn get_history(&self, limit: Option<usize>) -> CommandResult<Vec<CommandHistoryEntry>> {
        let history = self.history.read().await;
        let entries: Vec<_> = if let Some(limit) = limit {
            history.iter().rev().take(limit).cloned().collect()
        } else {
            history.iter().rev().cloned().collect()
        };
        Ok(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn create_test_request() -> CommandRequest {
        CommandRequest {
            request_id: Uuid::new_v4(),
            command: "echo".to_string(),
            arguments: vec!["test".to_string()],
            working_directory: None,
            environment: HashMap::new(),
            timeout: Duration::from_secs(10),
            sandbox_config: SandboxConfig::default(),
            requester: "test_peer".to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = UnifiedCommandManager::new();
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_queue_execution() {
        let manager = UnifiedCommandManager::new().unwrap();
        let request = create_test_request();
        let execution_id = manager.queue_execution(request).await;
        
        let status = manager.get_execution_status(execution_id).await;
        assert!(status.is_ok());
        assert!(matches!(status.unwrap(), ExecutionStatus::Pending));
    }

    #[tokio::test]
    async fn test_execute_simple_command() {
        let manager = UnifiedCommandManager::new().unwrap();
        let request = create_test_request();
        
        let result = manager.execute_command(request).await;
        assert!(result.is_ok());
        
        let cmd_result = result.unwrap();
        assert_eq!(cmd_result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_history_tracking() {
        let manager = UnifiedCommandManager::new().unwrap();
        let request = create_test_request();
        
        let _ = manager.execute_command(request).await;
        
        let history = manager.get_history(Some(10)).await;
        assert!(history.is_ok());
        assert!(!history.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_cancel_execution() {
        let manager = UnifiedCommandManager::new().unwrap();
        let request = create_test_request();
        let execution_id = manager.queue_execution(request).await;
        
        let cancel_result = manager.cancel_execution(execution_id).await;
        assert!(cancel_result.is_ok());
        
        let status = manager.get_execution_status(execution_id).await;
        // Should be removed from queue, so this should error
        assert!(status.is_err());
    }
}
