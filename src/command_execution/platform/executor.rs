// Platform executor trait and common execution context

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;
use crate::command_execution::{
    error::{CommandError, CommandResult},
    types::{CommandResult as CmdResult, ResourceUsage},
};

/// Execution context for platform-specific command execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub command: String,
    pub arguments: Vec<String>,
    pub working_directory: Option<PathBuf>,
    pub environment: HashMap<String, String>,
    pub timeout: Duration,
    pub shell: Option<String>,
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(command: String, arguments: Vec<String>) -> Self {
        Self {
            command,
            arguments,
            working_directory: None,
            environment: HashMap::new(),
            timeout: Duration::from_secs(60),
            shell: None,
        }
    }

    /// Set the working directory
    pub fn with_working_directory(mut self, dir: PathBuf) -> Self {
        self.working_directory = Some(dir);
        self
    }

    /// Set environment variables
    pub fn with_environment(mut self, env: HashMap<String, String>) -> Self {
        self.environment = env;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set shell to use
    pub fn with_shell(mut self, shell: String) -> Self {
        self.shell = Some(shell);
        self
    }
}

/// Platform-specific command execution result
#[derive(Debug)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub execution_time: Duration,
    pub resource_usage: ResourceUsage,
}

/// Trait for platform-specific command execution
#[async_trait]
pub trait PlatformExecutor: Send + Sync {
    /// Execute a command with the given context
    async fn execute(&self, context: ExecutionContext) -> CommandResult<ExecutionResult>;

    /// Execute a PowerShell script (Windows-specific, may error on other platforms)
    async fn execute_powershell(&self, script: &str, params: HashMap<String, String>) -> CommandResult<ExecutionResult>;

    /// Execute a shell script (bash/zsh on Unix, cmd on Windows)
    async fn execute_shell_script(&self, script: &str, params: HashMap<String, String>) -> CommandResult<ExecutionResult>;

    /// Get the default shell for this platform
    fn default_shell(&self) -> String;

    /// Get the platform name
    fn platform_name(&self) -> &'static str;

    /// Check if a command exists on this platform
    async fn command_exists(&self, command: &str) -> bool;
}
