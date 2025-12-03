// Command validation and suggestion system

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::types::{CommandType, ParsedCommand};
use std::path::Path;

/// Command validator with enhanced error messages and suggestions
pub struct CommandValidator;

impl CommandValidator {
    /// Validate a parsed command with detailed error messages
    pub fn validate(command: &ParsedCommand) -> CLIResult<Vec<ValidationWarning>> {
        let mut warnings = Vec::new();

        match command.command {
            CommandType::Discover => {
                Self::validate_discover(command, &mut warnings)?;
            }
            CommandType::Send => {
                Self::validate_send(command, &mut warnings)?;
            }
            CommandType::Receive => {
                Self::validate_receive(command, &mut warnings)?;
            }
            CommandType::Stream => {
                Self::validate_stream(command, &mut warnings)?;
            }
            CommandType::Exec => {
                Self::validate_exec(command, &mut warnings)?;
            }
            CommandType::Peers => {
                Self::validate_peers(command, &mut warnings)?;
            }
            CommandType::Status => {
                Self::validate_status(command, &mut warnings)?;
            }
            CommandType::Clipboard => {
                Self::validate_clipboard(command, &mut warnings)?;
            }
            CommandType::TUI => {
                Self::validate_tui(command, &mut warnings)?;
            }
            CommandType::Config => {
                Self::validate_config(command, &mut warnings)?;
            }
        }

        Ok(warnings)
    }

    fn validate_discover(
        command: &ParsedCommand,
        warnings: &mut Vec<ValidationWarning>,
    ) -> CLIResult<()> {
        // Validate timeout value
        if let Some(timeout) = command.get_option("timeout") {
            match timeout.parse::<u64>() {
                Ok(val) => {
                    if val == 0 {
                        return Err(CLIError::InvalidArgumentValue {
                            arg: "timeout".to_string(),
                            reason: "timeout must be greater than 0".to_string(),
                        });
                    }
                    if val > 300 {
                        warnings.push(ValidationWarning {
                            field: "timeout".to_string(),
                            message: "timeout is very long (>5 minutes), discovery may take a while"
                                .to_string(),
                            suggestion: Some("Consider using a shorter timeout like 10-30 seconds".to_string()),
                        });
                    }
                }
                Err(_) => {
                    return Err(CLIError::InvalidArgumentValue {
                        arg: "timeout".to_string(),
                        reason: "timeout must be a valid number".to_string(),
                    });
                }
            }
        }

        // Validate device type
        if let Some(device_type) = command.get_option("type") {
            let valid_types = ["desktop", "mobile", "tablet", "server"];
            if !valid_types.contains(&device_type.as_str()) {
                warnings.push(ValidationWarning {
                    field: "type".to_string(),
                    message: format!("'{}' is not a standard device type", device_type),
                    suggestion: Some(format!(
                        "Valid types are: {}. Custom types are allowed but may not match any devices.",
                        valid_types.join(", ")
                    )),
                });
            }
        }

        Ok(())
    }

    fn validate_send(
        command: &ParsedCommand,
        warnings: &mut Vec<ValidationWarning>,
    ) -> CLIResult<()> {
        // Ensure files are provided
        if command.arguments.is_empty() {
            return Err(CLIError::MissingArgument(
                "files - at least one file must be specified".to_string(),
            ));
        }

        // Validate file paths exist
        for file_path in &command.arguments {
            let path = Path::new(file_path);
            if !path.exists() {
                return Err(CLIError::InvalidArgumentValue {
                    arg: "files".to_string(),
                    reason: format!("file '{}' does not exist", file_path),
                });
            }

            // Warn about large files
            if let Ok(metadata) = path.metadata() {
                let size_mb = metadata.len() / (1024 * 1024);
                if size_mb > 1000 {
                    warnings.push(ValidationWarning {
                        field: "files".to_string(),
                        message: format!("'{}' is very large ({} MB)", file_path, size_mb),
                        suggestion: Some(
                            "Large file transfers may take significant time. Consider using compression."
                                .to_string(),
                        ),
                    });
                }
            }
        }

        // Warn if encryption is disabled
        if command.has_flag("no-encryption") {
            warnings.push(ValidationWarning {
                field: "no-encryption".to_string(),
                message: "Encryption is disabled - files will be sent without encryption".to_string(),
                suggestion: Some(
                    "Only disable encryption on trusted networks or for non-sensitive data"
                        .to_string(),
                ),
            });
        }

        // Check for conflicting flags
        if command.has_flag("no-compression") && command.has_flag("no-encryption") {
            warnings.push(ValidationWarning {
                field: "flags".to_string(),
                message: "Both compression and encryption are disabled".to_string(),
                suggestion: Some(
                    "This provides maximum speed but minimum security and efficiency".to_string(),
                ),
            });
        }

        Ok(())
    }

    fn validate_receive(
        command: &ParsedCommand,
        warnings: &mut Vec<ValidationWarning>,
    ) -> CLIResult<()> {
        // Validate output directory
        if let Some(output) = command.get_option("output") {
            let path = Path::new(output);
            if path.exists() && !path.is_dir() {
                return Err(CLIError::InvalidArgumentValue {
                    arg: "output".to_string(),
                    reason: format!("'{}' exists but is not a directory", output),
                });
            }

            if !path.exists() {
                warnings.push(ValidationWarning {
                    field: "output".to_string(),
                    message: format!("Directory '{}' does not exist", output),
                    suggestion: Some("The directory will be created automatically".to_string()),
                });
            }
        }

        // Warn about auto-accept
        if command.has_flag("auto-accept") {
            warnings.push(ValidationWarning {
                field: "auto-accept".to_string(),
                message: "Auto-accept is enabled - files will be received without confirmation"
                    .to_string(),
                suggestion: Some(
                    "Only use auto-accept with trusted peers to avoid unwanted transfers"
                        .to_string(),
                ),
            });
        }

        Ok(())
    }

    fn validate_stream(
        command: &ParsedCommand,
        warnings: &mut Vec<ValidationWarning>,
    ) -> CLIResult<()> {
        // Validate quality setting
        if let Some(quality) = command.get_option("quality") {
            let valid_qualities = ["low", "medium", "high", "ultra"];
            if !valid_qualities.contains(&quality.as_str()) {
                return Err(CLIError::InvalidArgumentValue {
                    arg: "quality".to_string(),
                    reason: format!(
                        "invalid quality '{}', must be one of: {}",
                        quality,
                        valid_qualities.join(", ")
                    ),
                });
            }

            if quality == "ultra" {
                warnings.push(ValidationWarning {
                    field: "quality".to_string(),
                    message: "Ultra quality requires significant bandwidth and processing power"
                        .to_string(),
                    suggestion: Some(
                        "Consider using 'high' quality for better compatibility".to_string(),
                    ),
                });
            }
        }

        // Validate recording output
        if command.has_flag("record") {
            if let Some(output) = command.get_option("output") {
                let path = Path::new(output);
                if path.exists() {
                    warnings.push(ValidationWarning {
                        field: "output".to_string(),
                        message: format!("File '{}' already exists", output),
                        suggestion: Some("The existing file will be overwritten".to_string()),
                    });
                }

                // Check file extension
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if !["mp4", "mkv", "avi", "webm"].contains(&ext_str.as_str()) {
                        warnings.push(ValidationWarning {
                            field: "output".to_string(),
                            message: format!("Unusual video file extension: .{}", ext_str),
                            suggestion: Some(
                                "Recommended extensions: .mp4, .mkv, .webm".to_string(),
                            ),
                        });
                    }
                }
            } else {
                warnings.push(ValidationWarning {
                    field: "record".to_string(),
                    message: "Recording enabled but no output file specified".to_string(),
                    suggestion: Some(
                        "A default filename will be generated in the current directory".to_string(),
                    ),
                });
            }
        }

        Ok(())
    }

    fn validate_exec(
        command: &ParsedCommand,
        warnings: &mut Vec<ValidationWarning>,
    ) -> CLIResult<()> {
        // Ensure command is provided
        if command.arguments.is_empty() {
            return Err(CLIError::MissingArgument(
                "command - a command to execute must be specified".to_string(),
            ));
        }

        // Ensure peer is specified
        if command.get_option("peer").is_none() {
            return Err(CLIError::MissingArgument(
                "peer - target peer must be specified with --peer".to_string(),
            ));
        }

        // Warn about potentially dangerous commands
        let cmd = &command.arguments[0];
        let dangerous_patterns = ["rm -rf", "del /f", "format", "mkfs", "dd if="];
        for pattern in &dangerous_patterns {
            if cmd.contains(pattern) {
                warnings.push(ValidationWarning {
                    field: "command".to_string(),
                    message: "Command contains potentially destructive operations".to_string(),
                    suggestion: Some(
                        "Ensure you have authorization and understand the consequences".to_string(),
                    ),
                });
                break;
            }
        }

        Ok(())
    }

    fn validate_peers(
        _command: &ParsedCommand,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> CLIResult<()> {
        // Peers command has minimal validation requirements
        Ok(())
    }

    fn validate_status(
        _command: &ParsedCommand,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> CLIResult<()> {
        // Status command has minimal validation requirements
        Ok(())
    }

    fn validate_clipboard(
        command: &ParsedCommand,
        warnings: &mut Vec<ValidationWarning>,
    ) -> CLIResult<()> {
        // Check for conflicting flags in share subcommand
        if command.subcommand.as_deref() == Some("share") {
            if command.has_flag("enable") && command.has_flag("disable") {
                return Err(CLIError::InvalidArgumentValue {
                    arg: "flags".to_string(),
                    reason: "cannot specify both --enable and --disable".to_string(),
                });
            }

            if command.has_flag("enable") {
                warnings.push(ValidationWarning {
                    field: "enable".to_string(),
                    message: "Clipboard sharing will be enabled".to_string(),
                    suggestion: Some(
                        "Clipboard content will be synchronized with connected peers".to_string(),
                    ),
                });
            }
        }

        Ok(())
    }

    fn validate_tui(
        _command: &ParsedCommand,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> CLIResult<()> {
        // TUI command has no validation requirements
        Ok(())
    }

    fn validate_config(
        command: &ParsedCommand,
        _warnings: &mut Vec<ValidationWarning>,
    ) -> CLIResult<()> {
        // Validate config subcommands
        match command.subcommand.as_deref() {
            Some("get") => {
                if command.arguments.is_empty() {
                    return Err(CLIError::MissingArgument(
                        "key - configuration key must be specified".to_string(),
                    ));
                }
            }
            Some("set") => {
                if command.arguments.len() < 2 {
                    return Err(CLIError::MissingArgument(
                        "key and value - both must be specified".to_string(),
                    ));
                }
            }
            Some("list") => {
                // No validation needed
            }
            Some(other) => {
                return Err(CLIError::InvalidCommand(format!(
                    "unknown config subcommand: {}",
                    other
                )));
            }
            None => {
                return Err(CLIError::MissingArgument(
                    "subcommand - specify 'get', 'set', or 'list'".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Suggest similar commands for typos
    pub fn suggest_similar_commands(invalid: &str) -> Vec<String> {
        let commands = vec![
            "discover", "send", "receive", "stream", "exec", "peers", "status", "clipboard",
            "tui", "config",
        ];

        let mut suggestions: Vec<(String, usize)> = commands
            .iter()
            .map(|&cmd| {
                let distance = levenshtein_distance(invalid, cmd);
                (cmd.to_string(), distance)
            })
            .filter(|(_, dist)| *dist <= 3)
            .collect();

        suggestions.sort_by_key(|(_, dist)| *dist);
        suggestions.into_iter().map(|(cmd, _)| cmd).collect()
    }

    /// Suggest similar option names for typos
    pub fn suggest_similar_options(invalid: &str, command_type: CommandType) -> Vec<String> {
        let options = match command_type {
            CommandType::Discover => vec!["type", "name", "timeout", "watch", "format", "json"],
            CommandType::Send => vec!["peer", "no-compression", "no-encryption", "verbose"],
            CommandType::Receive => vec!["output", "auto-accept", "from"],
            CommandType::Stream => vec!["camera", "quality", "record", "output"],
            CommandType::Exec => vec!["peer", "interactive"],
            CommandType::Peers => vec!["watch", "filter", "format"],
            CommandType::Status => vec!["detailed", "json"],
            CommandType::Clipboard => vec!["peer", "enable", "disable"],
            CommandType::TUI => vec![],
            CommandType::Config => vec!["key", "value"],
        };

        let mut suggestions: Vec<(String, usize)> = options
            .iter()
            .map(|&opt| {
                let distance = levenshtein_distance(invalid, opt);
                (opt.to_string(), distance)
            })
            .filter(|(_, dist)| *dist <= 2)
            .collect();

        suggestions.sort_by_key(|(_, dist)| *dist);
        suggestions.into_iter().map(|(opt, _)| opt).collect()
    }

    /// Generate context-aware help for a command
    pub fn generate_contextual_help(command: &ParsedCommand) -> String {
        match command.command {
            CommandType::Discover => {
                "Discover peers on the network. Use --type to filter by device type, \
                 --name to filter by name pattern, and --watch to continuously monitor for peers."
                    .to_string()
            }
            CommandType::Send => {
                "Send files to a peer. Specify one or more files and use --peer to select \
                 the target. Compression and encryption are enabled by default."
                    .to_string()
            }
            CommandType::Receive => {
                "Receive incoming file transfers. Use --output to specify download location \
                 and --auto-accept to skip confirmation for trusted peers."
                    .to_string()
            }
            CommandType::Stream => {
                "Manage media streaming. Use 'stream camera' to start camera streaming. \
                 Adjust quality with --quality and record with --record."
                    .to_string()
            }
            CommandType::Exec => {
                "Execute commands on remote peers. Requires --peer to specify target. \
                 The remote peer must authorize the command execution."
                    .to_string()
            }
            CommandType::Peers => {
                "List connected peers with their status and capabilities. Use --watch \
                 for continuous monitoring and --filter to narrow results."
                    .to_string()
            }
            CommandType::Status => {
                "Show Kizuna system status including network information and active operations. \
                 Use --detailed for comprehensive information."
                    .to_string()
            }
            CommandType::Clipboard => {
                "Manage clipboard sharing with peers. Use 'clipboard share' to toggle sharing, \
                 'clipboard status' to view current state, and 'clipboard history' to view past items."
                    .to_string()
            }
            CommandType::TUI => {
                "Launch the interactive Text User Interface for visual peer management \
                 and file operations."
                    .to_string()
            }
            CommandType::Config => {
                "Manage Kizuna configuration. Use 'config get <key>' to view settings, \
                 'config set <key> <value>' to change settings, and 'config list' to view all."
                    .to_string()
            }
        }
    }
}

/// Validation warning structure
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
    pub suggestion: Option<String>,
}

impl ValidationWarning {
    /// Format the warning as a user-friendly message
    pub fn format(&self) -> String {
        let mut msg = format!("Warning [{}]: {}", self.field, self.message);
        if let Some(suggestion) = &self.suggestion {
            msg.push_str(&format!("\n  Suggestion: {}", suggestion));
        }
        msg
    }
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];

    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }

    for (i, c1) in s1.chars().enumerate() {
        for (j, c2) in s2.chars().enumerate() {
            let cost = if c1 == c2 { 0 } else { 1 };
            matrix[i + 1][j + 1] = std::cmp::min(
                std::cmp::min(matrix[i][j + 1] + 1, matrix[i + 1][j] + 1),
                matrix[i][j] + cost,
            );
        }
    }

    matrix[len1][len2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_send_missing_files() {
        let command = ParsedCommand::new(CommandType::Send);
        let result = CommandValidator::validate(&command);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_exec_missing_peer() {
        let mut command = ParsedCommand::new(CommandType::Exec);
        command.arguments.push("ls".to_string());
        let result = CommandValidator::validate(&command);
        assert!(result.is_err());
    }

    #[test]
    fn test_suggest_similar_commands() {
        let suggestions = CommandValidator::suggest_similar_commands("discver");
        assert!(suggestions.contains(&"discover".to_string()));

        let suggestions = CommandValidator::suggest_similar_commands("snd");
        assert!(suggestions.contains(&"send".to_string()));
    }

    #[test]
    fn test_suggest_similar_options() {
        let suggestions =
            CommandValidator::suggest_similar_options("pear", CommandType::Send);
        assert!(suggestions.contains(&"peer".to_string()));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("discover", "discover"), 0);
        assert_eq!(levenshtein_distance("discover", "discver"), 1);
        assert_eq!(levenshtein_distance("send", "end"), 1);
    }
}
