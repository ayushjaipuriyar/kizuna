// Integration module demonstrating the complete command parsing and execution flow

use crate::cli::error::CLIResult;
use crate::cli::parser::{ClapCommandParser, CommandParser, CommandPipeline};
use crate::cli::types::CommandResult;

/// Complete command execution flow from arguments to result
pub struct CommandExecutor {
    parser: ClapCommandParser,
}

impl CommandExecutor {
    /// Create a new command executor
    pub fn new() -> Self {
        Self {
            parser: ClapCommandParser::new(),
        }
    }

    /// Execute a command from raw arguments
    pub async fn execute_from_args(&self, args: Vec<String>) -> CLIResult<CommandResult> {
        // Step 1: Parse arguments
        let parsed_command = self.parser.parse_args(args).await?;

        // Step 2: Validate command
        let validated_command = self.parser.validate_command(parsed_command).await?;

        // Step 3: Execute through pipeline
        let result = CommandPipeline::execute(validated_command).await?;

        Ok(result)
    }

    /// Execute with automatic error recovery
    pub async fn execute_with_recovery(&self, args: Vec<String>) -> CLIResult<CommandResult> {
        // Step 1: Parse arguments
        let parsed_command = self.parser.parse_args(args).await?;

        // Step 2: Validate command
        let validated_command = self.parser.validate_command(parsed_command).await?;

        // Step 3: Execute through pipeline with recovery
        let result = CommandPipeline::execute_with_recovery(validated_command).await?;

        Ok(result)
    }

    /// Get help for a command
    pub async fn get_help(&self, command: Option<String>) -> CLIResult<String> {
        let help_text = self.parser.generate_help(command).await?;
        Ok(help_text.usage)
    }

    /// Suggest corrections for invalid command
    pub async fn suggest_corrections(&self, invalid_command: String) -> CLIResult<Vec<String>> {
        self.parser.suggest_corrections(invalid_command).await
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_discover_command() {
        let executor = CommandExecutor::new();
        let args = vec![
            "kizuna".to_string(),
            "discover".to_string(),
            "--type".to_string(),
            "desktop".to_string(),
        ];

        let result = executor.execute_from_args(args).await;
        assert!(result.is_ok());

        let cmd_result = result.unwrap();
        assert!(cmd_result.success);
    }

    #[tokio::test]
    async fn test_execute_send_command() {
        let executor = CommandExecutor::new();
        let args = vec![
            "kizuna".to_string(),
            "send".to_string(),
            "Cargo.toml".to_string(), // Use existing file
            "--peer".to_string(),
            "laptop".to_string(),
        ];

        let result = executor.execute_from_args(args).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_command_suggestions() {
        let executor = CommandExecutor::new();
        let suggestions = executor.suggest_corrections("discver".to_string()).await;

        assert!(suggestions.is_ok());
        let suggestions = suggestions.unwrap();
        assert!(!suggestions.is_empty());
        assert!(suggestions.contains(&"discover".to_string()));
    }

    #[tokio::test]
    async fn test_get_help() {
        let executor = CommandExecutor::new();
        let help = executor.get_help(Some("discover".to_string())).await;

        assert!(help.is_ok());
        let help_text = help.unwrap();
        assert!(help_text.contains("discover"));
    }

    #[tokio::test]
    async fn test_execute_with_recovery() {
        let executor = CommandExecutor::new();
        let args = vec![
            "kizuna".to_string(),
            "status".to_string(),
        ];

        let result = executor.execute_with_recovery(args).await;
        assert!(result.is_ok());
    }
}
