// Unix-specific command execution using bash/zsh (macOS and Linux)

use async_trait::async_trait;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::process::Command;
use tokio::time::timeout;

use crate::command_execution::{
    error::{CommandError, CommandResult},
    types::ResourceUsage,
};

use super::executor::{ExecutionContext, ExecutionResult, PlatformExecutor};

/// Unix command executor using bash/zsh
pub struct UnixExecutor {
    shell_path: PathBuf,
    shell_name: String,
}

impl UnixExecutor {
    /// Create a new Unix executor
    pub fn new() -> CommandResult<Self> {
        let (shell_path, shell_name) = Self::find_shell()?;
        Ok(Self {
            shell_path,
            shell_name,
        })
    }

    /// Find the best available shell (prefer bash, fallback to sh)
    fn find_shell() -> CommandResult<(PathBuf, String)> {
        // Try to find bash first
        let bash_paths = vec![
            PathBuf::from("/bin/bash"),
            PathBuf::from("/usr/bin/bash"),
            PathBuf::from("/usr/local/bin/bash"),
        ];

        for path in bash_paths {
            if path.exists() {
                return Ok((path, "bash".to_string()));
            }
        }

        // Try zsh (common on macOS)
        let zsh_paths = vec![
            PathBuf::from("/bin/zsh"),
            PathBuf::from("/usr/bin/zsh"),
            PathBuf::from("/usr/local/bin/zsh"),
        ];

        for path in zsh_paths {
            if path.exists() {
                return Ok((path, "zsh".to_string()));
            }
        }

        // Fallback to sh (should always exist)
        let sh_path = PathBuf::from("/bin/sh");
        if sh_path.exists() {
            return Ok((sh_path, "sh".to_string()));
        }

        Err(CommandError::platform_error("No shell found on system"))
    }

    /// Escape shell special characters for safe execution
    fn escape_shell_string(s: &str) -> String {
        // Use single quotes and escape any single quotes in the string
        format!("'{}'", s.replace('\'', "'\\''"))
    }

    /// Build environment variable export string
    fn build_env_exports(env: &HashMap<String, String>) -> String {
        env.iter()
            .map(|(k, v)| format!("export {}={}", k, Self::escape_shell_string(v)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Execute command directly (without shell interpretation)
    async fn execute_direct(&self, context: &ExecutionContext) -> CommandResult<ExecutionResult> {
        let start = Instant::now();
        
        let mut cmd = Command::new(&context.command);
        
        // Add arguments
        for arg in &context.arguments {
            cmd.arg(arg);
        }

        // Set working directory
        if let Some(ref dir) = context.working_directory {
            cmd.current_dir(dir);
        }

        // Set environment variables
        for (key, value) in &context.environment {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // Execute with timeout
        let child = cmd.spawn()
            .map_err(|e| CommandError::execution_error(format!("Failed to spawn process: {}", e)))?;

        let output = timeout(context.timeout, child.wait_with_output())
            .await
            .map_err(|_| CommandError::Timeout(context.timeout))?
            .map_err(|e| CommandError::execution_error(format!("Command execution failed: {}", e)))?;

        let execution_time = start.elapsed();

        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            resource_usage: ResourceUsage::default(),
        })
    }

    /// Execute command through shell
    async fn execute_shell(&self, context: &ExecutionContext) -> CommandResult<ExecutionResult> {
        let start = Instant::now();
        
        let mut cmd = Command::new(&self.shell_path);
        cmd.arg("-c"); // Execute command string
        
        // Build command string
        let mut cmd_string = String::new();
        
        // Add environment variables
        if !context.environment.is_empty() {
            cmd_string.push_str(&Self::build_env_exports(&context.environment));
            cmd_string.push_str("\n");
        }

        // Add the actual command
        cmd_string.push_str(&context.command);
        if !context.arguments.is_empty() {
            cmd_string.push(' ');
            // Properly escape arguments
            cmd_string.push_str(
                &context.arguments
                    .iter()
                    .map(|arg| Self::escape_shell_string(arg))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }

        cmd.arg(&cmd_string);

        // Set working directory
        if let Some(ref dir) = context.working_directory {
            cmd.current_dir(dir);
        }

        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // Execute with timeout
        let child = cmd.spawn()
            .map_err(|e| CommandError::execution_error(format!("Failed to spawn shell: {}", e)))?;

        let output = timeout(context.timeout, child.wait_with_output())
            .await
            .map_err(|_| CommandError::Timeout(context.timeout))?
            .map_err(|e| CommandError::execution_error(format!("Shell execution failed: {}", e)))?;

        let execution_time = start.elapsed();

        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            resource_usage: ResourceUsage::default(),
        })
    }

    /// Check if a path is executable
    async fn is_executable(path: &PathBuf) -> bool {
        if let Ok(metadata) = fs::metadata(path).await {
            let permissions = metadata.permissions();
            return permissions.mode() & 0o111 != 0;
        }
        false
    }
}

impl Default for UnixExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create Unix executor")
    }
}

#[async_trait]
impl PlatformExecutor for UnixExecutor {
    async fn execute(&self, context: ExecutionContext) -> CommandResult<ExecutionResult> {
        // Determine execution method based on shell specification
        match context.shell.as_deref() {
            Some("direct") | Some("none") => {
                // Execute directly without shell
                self.execute_direct(&context).await
            }
            Some(_) | None => {
                // Execute through shell (default)
                self.execute_shell(&context).await
            }
        }
    }

    async fn execute_powershell(&self, _script: &str, _params: HashMap<String, String>) -> CommandResult<ExecutionResult> {
        // PowerShell is not natively available on Unix systems
        Err(CommandError::platform_error(
            "PowerShell is not supported on Unix systems. Use execute_shell_script instead."
        ))
    }

    async fn execute_shell_script(&self, script: &str, params: HashMap<String, String>) -> CommandResult<ExecutionResult> {
        let start = Instant::now();
        
        let mut cmd = Command::new(&self.shell_path);
        cmd.arg("-c");
        
        // Build script with parameter substitution
        let mut shell_script = String::new();
        
        // Export parameters as environment variables
        for (key, value) in &params {
            shell_script.push_str(&format!("export {}={}\n", key, Self::escape_shell_string(value)));
        }
        
        // Add the script content
        shell_script.push_str(script);
        
        cmd.arg(&shell_script);
        
        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // Execute with default timeout
        let timeout_duration = Duration::from_secs(300); // 5 minutes for scripts
        let child = cmd.spawn()
            .map_err(|e| CommandError::execution_error(format!("Failed to spawn shell: {}", e)))?;

        let output = timeout(timeout_duration, child.wait_with_output())
            .await
            .map_err(|_| CommandError::Timeout(timeout_duration))?
            .map_err(|e| CommandError::execution_error(format!("Shell script execution failed: {}", e)))?;

        let execution_time = start.elapsed();

        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            resource_usage: ResourceUsage::default(),
        })
    }

    fn default_shell(&self) -> String {
        self.shell_name.clone()
    }

    fn platform_name(&self) -> &'static str {
        #[cfg(target_os = "macos")]
        return "macos";
        
        #[cfg(target_os = "linux")]
        return "linux";
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        return "unix";
    }

    async fn command_exists(&self, command: &str) -> bool {
        // Use 'which' command to check if command exists
        let result = Command::new("which")
            .arg(command)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        matches!(result, Ok(status) if status.success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_shell_string() {
        assert_eq!(
            UnixExecutor::escape_shell_string("test'string"),
            "'test'\\''string'"
        );
        assert_eq!(
            UnixExecutor::escape_shell_string("normal"),
            "'normal'"
        );
    }

    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    async fn test_unix_executor_creation() {
        let executor = UnixExecutor::new();
        assert!(executor.is_ok());
    }

    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    async fn test_simple_command_execution() {
        let executor = UnixExecutor::new().unwrap();
        let context = ExecutionContext::new("echo".to_string(), vec!["Hello".to_string()]);
        
        let result = executor.execute(context).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("Hello"));
    }

    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    async fn test_command_exists() {
        let executor = UnixExecutor::new().unwrap();
        assert!(executor.command_exists("ls").await);
        assert!(executor.command_exists("echo").await);
        assert!(!executor.command_exists("nonexistent_command_xyz").await);
    }

    #[tokio::test]
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    async fn test_shell_script_execution() {
        let executor = UnixExecutor::new().unwrap();
        let script = "echo $TEST_VAR";
        let mut params = HashMap::new();
        params.insert("TEST_VAR".to_string(), "test_value".to_string());
        
        let result = executor.execute_shell_script(script, params).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("test_value"));
    }
}
