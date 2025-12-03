// Windows-specific command execution using PowerShell and CMD

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::time::timeout;

use crate::command_execution::{
    error::{CommandError, CommandResult},
    types::ResourceUsage,
};

use super::executor::{ExecutionContext, ExecutionResult, PlatformExecutor};

/// Windows command executor using PowerShell and CMD
pub struct WindowsExecutor {
    powershell_path: PathBuf,
    cmd_path: PathBuf,
}

impl WindowsExecutor {
    /// Create a new Windows executor
    pub fn new() -> CommandResult<Self> {
        Ok(Self {
            powershell_path: Self::find_powershell()?,
            cmd_path: PathBuf::from("C:\\Windows\\System32\\cmd.exe"),
        })
    }

    /// Find PowerShell executable path
    fn find_powershell() -> CommandResult<PathBuf> {
        // Try PowerShell Core (pwsh) first, then Windows PowerShell
        let pwsh = PathBuf::from("C:\\Program Files\\PowerShell\\7\\pwsh.exe");
        if pwsh.exists() {
            return Ok(pwsh);
        }

        let powershell = PathBuf::from("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe");
        if powershell.exists() {
            return Ok(powershell);
        }

        // Fallback to just "powershell" and let PATH resolution handle it
        Ok(PathBuf::from("powershell.exe"))
    }

    /// Normalize Windows path separators
    fn normalize_path(path: &Path) -> String {
        path.to_string_lossy().replace('/', "\\")
    }

    /// Escape PowerShell special characters
    fn escape_powershell_string(s: &str) -> String {
        s.replace('`', "``")
            .replace('$', "`$")
            .replace('"', "`\"")
            .replace('\'', "`'")
    }

    /// Build environment variable string for PowerShell
    fn build_env_string(env: &HashMap<String, String>) -> String {
        env.iter()
            .map(|(k, v)| format!("$env:{} = '{}'", k, Self::escape_powershell_string(v)))
            .collect::<Vec<_>>()
            .join("; ")
    }

    /// Execute command using CMD
    async fn execute_cmd(&self, context: &ExecutionContext) -> CommandResult<ExecutionResult> {
        let start = Instant::now();
        
        let mut cmd = Command::new(&self.cmd_path);
        cmd.arg("/C"); // Execute command and terminate
        
        // Build command string
        let mut cmd_string = context.command.clone();
        if !context.arguments.is_empty() {
            cmd_string.push(' ');
            cmd_string.push_str(&context.arguments.join(" "));
        }
        cmd.arg(&cmd_string);

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
            .map_err(|e| CommandError::execution_error(format!("Failed to spawn CMD process: {}", e)))?;

        let output = timeout(context.timeout, child.wait_with_output())
            .await
            .map_err(|_| CommandError::Timeout(context.timeout))?
            .map_err(|e| CommandError::execution_error(format!("CMD execution failed: {}", e)))?;

        let execution_time = start.elapsed();

        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            resource_usage: ResourceUsage::default(),
        })
    }

    /// Execute command using PowerShell
    async fn execute_ps(&self, context: &ExecutionContext) -> CommandResult<ExecutionResult> {
        let start = Instant::now();
        
        let mut cmd = Command::new(&self.powershell_path);
        
        // PowerShell arguments for non-interactive execution
        cmd.arg("-NoProfile");
        cmd.arg("-NonInteractive");
        cmd.arg("-ExecutionPolicy");
        cmd.arg("Bypass");
        cmd.arg("-Command");
        
        // Build PowerShell command
        let mut ps_command = String::new();
        
        // Add environment variables
        if !context.environment.is_empty() {
            ps_command.push_str(&Self::build_env_string(&context.environment));
            ps_command.push_str("; ");
        }

        // Add the actual command
        ps_command.push_str(&context.command);
        if !context.arguments.is_empty() {
            ps_command.push(' ');
            ps_command.push_str(&context.arguments.join(" "));
        }

        cmd.arg(&ps_command);

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
            .map_err(|e| CommandError::execution_error(format!("Failed to spawn PowerShell process: {}", e)))?;

        let output = timeout(context.timeout, child.wait_with_output())
            .await
            .map_err(|_| CommandError::Timeout(context.timeout))?
            .map_err(|e| CommandError::execution_error(format!("PowerShell execution failed: {}", e)))?;

        let execution_time = start.elapsed();

        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            resource_usage: ResourceUsage::default(),
        })
    }
}

impl Default for WindowsExecutor {
    fn default() -> Self {
        Self::new().expect("Failed to create Windows executor")
    }
}

#[async_trait]
impl PlatformExecutor for WindowsExecutor {
    async fn execute(&self, context: ExecutionContext) -> CommandResult<ExecutionResult> {
        // Determine which shell to use
        match context.shell.as_deref() {
            Some("cmd") | Some("cmd.exe") => self.execute_cmd(&context).await,
            Some("powershell") | Some("pwsh") | Some("powershell.exe") | Some("pwsh.exe") => {
                self.execute_ps(&context).await
            }
            None => {
                // Default to PowerShell for better scripting capabilities
                self.execute_ps(&context).await
            }
            Some(shell) => {
                Err(CommandError::platform_error(format!("Unsupported shell: {}", shell)))
            }
        }
    }

    async fn execute_powershell(&self, script: &str, params: HashMap<String, String>) -> CommandResult<ExecutionResult> {
        let start = Instant::now();
        
        let mut cmd = Command::new(&self.powershell_path);
        
        cmd.arg("-NoProfile");
        cmd.arg("-NonInteractive");
        cmd.arg("-ExecutionPolicy");
        cmd.arg("Bypass");
        cmd.arg("-Command");
        
        // Build script with parameter substitution
        let mut ps_script = String::new();
        
        // Define parameters as variables
        for (key, value) in &params {
            ps_script.push_str(&format!("${} = '{}'; ", key, Self::escape_powershell_string(value)));
        }
        
        // Add the script content
        ps_script.push_str(script);
        
        cmd.arg(&ps_script);
        
        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // Execute with default timeout
        let timeout_duration = Duration::from_secs(300); // 5 minutes for scripts
        let child = cmd.spawn()
            .map_err(|e| CommandError::execution_error(format!("Failed to spawn PowerShell: {}", e)))?;

        let output = timeout(timeout_duration, child.wait_with_output())
            .await
            .map_err(|_| CommandError::Timeout(timeout_duration))?
            .map_err(|e| CommandError::execution_error(format!("PowerShell script execution failed: {}", e)))?;

        let execution_time = start.elapsed();

        Ok(ExecutionResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            execution_time,
            resource_usage: ResourceUsage::default(),
        })
    }

    async fn execute_shell_script(&self, script: &str, params: HashMap<String, String>) -> CommandResult<ExecutionResult> {
        // On Windows, shell scripts are executed via CMD batch files
        let start = Instant::now();
        
        let mut cmd = Command::new(&self.cmd_path);
        cmd.arg("/C");
        
        // Build batch script with parameter substitution
        let mut batch_script = String::new();
        
        // Set parameters as environment variables
        for (key, value) in &params {
            batch_script.push_str(&format!("set {}={}\n", key, value));
        }
        
        // Add the script content
        batch_script.push_str(script);
        
        cmd.arg(&batch_script);
        
        // Configure stdio
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::null());

        // Execute with default timeout
        let timeout_duration = Duration::from_secs(300);
        let child = cmd.spawn()
            .map_err(|e| CommandError::execution_error(format!("Failed to spawn CMD: {}", e)))?;

        let output = timeout(timeout_duration, child.wait_with_output())
            .await
            .map_err(|_| CommandError::Timeout(timeout_duration))?
            .map_err(|e| CommandError::execution_error(format!("CMD script execution failed: {}", e)))?;

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
        "powershell".to_string()
    }

    fn platform_name(&self) -> &'static str {
        "windows"
    }

    async fn command_exists(&self, command: &str) -> bool {
        // Use 'where' command to check if command exists
        let result = Command::new("where")
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
    fn test_escape_powershell_string() {
        assert_eq!(
            WindowsExecutor::escape_powershell_string("test$var"),
            "test`$var"
        );
        assert_eq!(
            WindowsExecutor::escape_powershell_string("test`command"),
            "test``command"
        );
    }

    #[test]
    fn test_normalize_path() {
        let path = Path::new("C:/Users/test/file.txt");
        assert_eq!(
            WindowsExecutor::normalize_path(path),
            "C:\\Users\\test\\file.txt"
        );
    }

    #[tokio::test]
    #[cfg(target_os = "windows")]
    async fn test_windows_executor_creation() {
        let executor = WindowsExecutor::new();
        assert!(executor.is_ok());
    }

    #[tokio::test]
    #[cfg(target_os = "windows")]
    async fn test_simple_command_execution() {
        let executor = WindowsExecutor::new().unwrap();
        let context = ExecutionContext::new("echo".to_string(), vec!["Hello".to_string()]);
        
        let result = executor.execute(context).await;
        assert!(result.is_ok());
        
        let output = result.unwrap();
        assert_eq!(output.exit_code, 0);
        assert!(output.stdout.contains("Hello"));
    }

    #[tokio::test]
    #[cfg(target_os = "windows")]
    async fn test_command_exists() {
        let executor = WindowsExecutor::new().unwrap();
        assert!(executor.command_exists("cmd").await);
        assert!(executor.command_exists("powershell").await);
        assert!(!executor.command_exists("nonexistent_command_xyz").await);
    }
}
