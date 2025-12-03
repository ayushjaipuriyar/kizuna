// Command routing and handler dispatch system

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::parser::ValidatedCommand;
use crate::cli::types::{CommandOutput, CommandResult, CommandType};
use std::time::{Duration, Instant};

/// Command execution context
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub validated_command: ValidatedCommand,
    pub start_time: Instant,
    pub verbose: bool,
    pub quiet: bool,
}

impl CommandContext {
    /// Create a new command context
    pub fn new(validated_command: ValidatedCommand) -> Self {
        let verbose = validated_command.command.has_flag("verbose");
        let quiet = validated_command.command.has_flag("quiet");

        Self {
            validated_command,
            start_time: Instant::now(),
            verbose,
            quiet,
        }
    }

    /// Get the command type
    pub fn command_type(&self) -> CommandType {
        self.validated_command.command.command
    }

    /// Get elapsed time since command started
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Check if a flag is set
    pub fn has_flag(&self, flag: &str) -> bool {
        self.validated_command.command.has_flag(flag)
    }

    /// Get an option value
    pub fn get_option(&self, key: &str) -> Option<&String> {
        self.validated_command.command.get_option(key)
    }

    /// Get all arguments
    pub fn arguments(&self) -> &[String] {
        &self.validated_command.command.arguments
    }

    /// Get subcommand
    pub fn subcommand(&self) -> Option<&str> {
        self.validated_command.command.subcommand.as_deref()
    }
}

/// Command router that dispatches commands to appropriate handlers
pub struct CommandRouter;

impl CommandRouter {
    /// Route a validated command to the appropriate handler
    pub async fn route(context: CommandContext) -> CLIResult<CommandResult> {
        // Display validation warnings if not in quiet mode
        if !context.quiet && !context.validated_command.validation_warnings.is_empty() {
            for warning in &context.validated_command.validation_warnings {
                eprintln!("{}", warning);
            }
        }

        // Route to appropriate handler based on command type
        let result = match context.command_type() {
            CommandType::Discover => Self::route_discover(context).await,
            CommandType::Send => Self::route_send(context).await,
            CommandType::Receive => Self::route_receive(context).await,
            CommandType::Stream => Self::route_stream(context).await,
            CommandType::Exec => Self::route_exec(context).await,
            CommandType::Peers => Self::route_peers(context).await,
            CommandType::Status => Self::route_status(context).await,
            CommandType::Clipboard => Self::route_clipboard(context).await,
            CommandType::TUI => Self::route_tui(context).await,
            CommandType::Config => Self::route_config(context).await,
        };

        result
    }

    async fn route_discover(context: CommandContext) -> CLIResult<CommandResult> {
        // Placeholder implementation - will be replaced by actual handler
        let execution_time = context.elapsed();

        Ok(CommandResult {
            success: true,
            output: CommandOutput::Text(format!(
                "Discover command executed (placeholder)\nFilters: type={:?}, name={:?}, timeout={:?}",
                context.get_option("type"),
                context.get_option("name"),
                context.get_option("timeout")
            )),
            execution_time,
            exit_code: 0,
        })
    }

    async fn route_send(context: CommandContext) -> CLIResult<CommandResult> {
        // Placeholder implementation - will be replaced by actual handler
        let execution_time = context.elapsed();

        let files = context.arguments();
        let peer = context.get_option("peer");

        Ok(CommandResult {
            success: true,
            output: CommandOutput::Text(format!(
                "Send command executed (placeholder)\nFiles: {:?}\nPeer: {:?}\nCompression: {}\nEncryption: {}",
                files,
                peer,
                !context.has_flag("no-compression"),
                !context.has_flag("no-encryption")
            )),
            execution_time,
            exit_code: 0,
        })
    }

    async fn route_receive(context: CommandContext) -> CLIResult<CommandResult> {
        // Placeholder implementation - will be replaced by actual handler
        let execution_time = context.elapsed();

        Ok(CommandResult {
            success: true,
            output: CommandOutput::Text(format!(
                "Receive command executed (placeholder)\nOutput: {:?}\nAuto-accept: {}",
                context.get_option("output"),
                context.has_flag("auto-accept")
            )),
            execution_time,
            exit_code: 0,
        })
    }

    async fn route_stream(context: CommandContext) -> CLIResult<CommandResult> {
        // Placeholder implementation - will be replaced by actual handler
        let execution_time = context.elapsed();

        let subcommand = context.subcommand().unwrap_or("unknown");

        Ok(CommandResult {
            success: true,
            output: CommandOutput::Text(format!(
                "Stream command executed (placeholder)\nSubcommand: {}\nQuality: {:?}\nRecord: {}",
                subcommand,
                context.get_option("quality"),
                context.has_flag("record")
            )),
            execution_time,
            exit_code: 0,
        })
    }

    async fn route_exec(context: CommandContext) -> CLIResult<CommandResult> {
        // Placeholder implementation - will be replaced by actual handler
        let execution_time = context.elapsed();

        let command = context.arguments().first().map(|s| s.as_str()).unwrap_or("");
        let peer = context.get_option("peer");

        Ok(CommandResult {
            success: true,
            output: CommandOutput::Text(format!(
                "Exec command executed (placeholder)\nCommand: {}\nPeer: {:?}\nInteractive: {}",
                command,
                peer,
                context.has_flag("interactive")
            )),
            execution_time,
            exit_code: 0,
        })
    }

    async fn route_peers(context: CommandContext) -> CLIResult<CommandResult> {
        // Placeholder implementation - will be replaced by actual handler
        let execution_time = context.elapsed();

        Ok(CommandResult {
            success: true,
            output: CommandOutput::Text(format!(
                "Peers command executed (placeholder)\nWatch: {}\nFilter: {:?}",
                context.has_flag("watch"),
                context.get_option("filter")
            )),
            execution_time,
            exit_code: 0,
        })
    }

    async fn route_status(context: CommandContext) -> CLIResult<CommandResult> {
        // Placeholder implementation - will be replaced by actual handler
        let execution_time = context.elapsed();

        Ok(CommandResult {
            success: true,
            output: CommandOutput::Text(format!(
                "Status command executed (placeholder)\nDetailed: {}",
                context.has_flag("detailed")
            )),
            execution_time,
            exit_code: 0,
        })
    }

    async fn route_clipboard(context: CommandContext) -> CLIResult<CommandResult> {
        // Placeholder implementation - will be replaced by actual handler
        let execution_time = context.elapsed();

        let subcommand = context.subcommand().unwrap_or("unknown");

        Ok(CommandResult {
            success: true,
            output: CommandOutput::Text(format!(
                "Clipboard command executed (placeholder)\nSubcommand: {}\nEnable: {}\nDisable: {}",
                subcommand,
                context.has_flag("enable"),
                context.has_flag("disable")
            )),
            execution_time,
            exit_code: 0,
        })
    }

    async fn route_tui(context: CommandContext) -> CLIResult<CommandResult> {
        // Placeholder implementation - will be replaced by actual handler
        let execution_time = context.elapsed();

        Ok(CommandResult {
            success: true,
            output: CommandOutput::Text(
                "TUI command executed (placeholder)\nLaunching interactive mode...".to_string(),
            ),
            execution_time,
            exit_code: 0,
        })
    }

    async fn route_config(context: CommandContext) -> CLIResult<CommandResult> {
        // Placeholder implementation - will be replaced by actual handler
        let execution_time = context.elapsed();

        let subcommand = context.subcommand().unwrap_or("unknown");
        let args = context.arguments();

        Ok(CommandResult {
            success: true,
            output: CommandOutput::Text(format!(
                "Config command executed (placeholder)\nSubcommand: {}\nArguments: {:?}",
                subcommand, args
            )),
            execution_time,
            exit_code: 0,
        })
    }
}

/// Command execution pipeline
pub struct CommandPipeline;

impl CommandPipeline {
    /// Execute a command through the full pipeline
    pub async fn execute(validated_command: ValidatedCommand) -> CLIResult<CommandResult> {
        // Create execution context
        let context = CommandContext::new(validated_command);

        // Route and execute command
        let result = CommandRouter::route(context).await?;

        Ok(result)
    }

    /// Execute with error handling and recovery
    pub async fn execute_with_recovery(
        validated_command: ValidatedCommand,
    ) -> CLIResult<CommandResult> {
        match Self::execute(validated_command.clone()).await {
            Ok(result) => Ok(result),
            Err(e) => {
                // Attempt recovery based on error type
                match &e {
                    CLIError::IntegrationError(_) => {
                        // Could retry with exponential backoff
                        Err(e)
                    }
                    CLIError::IOError(_) => {
                        // Could suggest alternative paths
                        Err(e)
                    }
                    _ => Err(e),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::parser::ValidatedCommand;
    use crate::cli::types::ParsedCommand;

    #[tokio::test]
    async fn test_route_discover() {
        let parsed = ParsedCommand::new(CommandType::Discover)
            .with_option("type", "desktop")
            .with_option("timeout", "10");

        let validated = ValidatedCommand::new(parsed);
        let context = CommandContext::new(validated);

        let result = CommandRouter::route_discover(context).await;
        assert!(result.is_ok());

        let cmd_result = result.unwrap();
        assert!(cmd_result.success);
        assert_eq!(cmd_result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_route_send() {
        let parsed = ParsedCommand::new(CommandType::Send)
            .with_argument("file.txt")
            .with_option("peer", "laptop");

        let validated = ValidatedCommand::new(parsed);
        let context = CommandContext::new(validated);

        let result = CommandRouter::route_send(context).await;
        assert!(result.is_ok());

        let cmd_result = result.unwrap();
        assert!(cmd_result.success);
    }

    #[tokio::test]
    async fn test_command_context() {
        let parsed = ParsedCommand::new(CommandType::Discover)
            .with_flag("verbose")
            .with_option("type", "desktop");

        let validated = ValidatedCommand::new(parsed);
        let context = CommandContext::new(validated);

        assert_eq!(context.command_type(), CommandType::Discover);
        assert!(context.has_flag("verbose"));
        assert_eq!(context.get_option("type"), Some(&"desktop".to_string()));
        assert!(context.verbose);
        assert!(!context.quiet);
    }

    #[tokio::test]
    async fn test_pipeline_execute() {
        let parsed = ParsedCommand::new(CommandType::Status);
        let validated = ValidatedCommand::new(parsed);

        let result = CommandPipeline::execute(validated).await;
        assert!(result.is_ok());

        let cmd_result = result.unwrap();
        assert!(cmd_result.success);
    }
}
