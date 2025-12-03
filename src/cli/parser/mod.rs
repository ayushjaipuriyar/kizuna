// Command-line argument parsing module

mod clap_parser;
mod integration;
mod router;
mod validator;

pub use clap_parser::ClapCommandParser;
pub use integration::CommandExecutor;
pub use router::{CommandContext, CommandPipeline, CommandRouter};
pub use validator::{CommandValidator, ValidationWarning};

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::types::{CommandType, ParsedCommand};
use async_trait::async_trait;
use std::collections::{HashMap, HashSet};

/// Command parser trait
#[async_trait]
pub trait CommandParser {
    /// Parse command-line arguments into a structured command
    async fn parse_args(&self, args: Vec<String>) -> CLIResult<ParsedCommand>;

    /// Validate a parsed command
    async fn validate_command(&self, command: ParsedCommand) -> CLIResult<ValidatedCommand>;

    /// Generate help text for a command
    async fn generate_help(&self, command: Option<String>) -> CLIResult<HelpText>;

    /// Suggest corrections for invalid commands
    async fn suggest_corrections(&self, invalid_command: String) -> CLIResult<Vec<String>>;
}

/// Validated command structure
#[derive(Debug, Clone)]
pub struct ValidatedCommand {
    pub command: ParsedCommand,
    pub validation_warnings: Vec<String>,
}

/// Help text structure
#[derive(Debug, Clone)]
pub struct HelpText {
    pub command: Option<String>,
    pub description: String,
    pub usage: String,
    pub examples: Vec<String>,
    pub options: Vec<HelpOption>,
}

/// Help option structure
#[derive(Debug, Clone)]
pub struct HelpOption {
    pub short: Option<String>,
    pub long: String,
    pub description: String,
    pub required: bool,
}

impl ParsedCommand {
    /// Create a new parsed command
    pub fn new(command: CommandType) -> Self {
        Self {
            command,
            subcommand: None,
            arguments: Vec::new(),
            options: HashMap::new(),
            flags: HashSet::new(),
        }
    }

    /// Add a subcommand
    pub fn with_subcommand(mut self, subcommand: impl Into<String>) -> Self {
        self.subcommand = Some(subcommand.into());
        self
    }

    /// Add an argument
    pub fn with_argument(mut self, arg: impl Into<String>) -> Self {
        self.arguments.push(arg.into());
        self
    }

    /// Add an option
    pub fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }

    /// Add a flag
    pub fn with_flag(mut self, flag: impl Into<String>) -> Self {
        self.flags.insert(flag.into());
        self
    }

    /// Get an option value
    pub fn get_option(&self, key: &str) -> Option<&String> {
        self.options.get(key)
    }

    /// Check if a flag is set
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }
}

impl ValidatedCommand {
    /// Create a new validated command
    pub fn new(command: ParsedCommand) -> Self {
        Self {
            command,
            validation_warnings: Vec::new(),
        }
    }

    /// Add a validation warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.validation_warnings.push(warning.into());
        self
    }
}
