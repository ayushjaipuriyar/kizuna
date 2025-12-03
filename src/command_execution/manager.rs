use async_trait::async_trait;
use crate::command_execution::{
    error::CommandResult as CmdResult,
    types::*,
};

/// Command Manager trait for orchestrating command execution
#[async_trait]
pub trait CommandManager: Send + Sync {
    /// Execute a simple command on the target system
    async fn execute_command(&self, request: CommandRequest) -> CmdResult<CommandResult>;

    /// Execute a script on the target system
    async fn execute_script(&self, script: ScriptRequest) -> CmdResult<ScriptResult>;

    /// Query system information
    async fn query_system_info(&self, query: SystemInfoQuery) -> CmdResult<SystemInfo>;

    /// Send a notification to the target system
    async fn send_notification(&self, notification: Notification) -> CmdResult<NotificationResult>;

    /// Get the status of a command execution
    async fn get_execution_status(&self, execution_id: ExecutionId) -> CmdResult<ExecutionStatus>;

    /// Cancel a pending or running command
    async fn cancel_execution(&self, execution_id: ExecutionId) -> CmdResult<()>;

    /// Get command history
    async fn get_history(&self, limit: Option<usize>) -> CmdResult<Vec<CommandHistoryEntry>>;
}
