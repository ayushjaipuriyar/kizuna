// Clap-based command parser implementation

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::parser::{CommandParser, CommandValidator, HelpOption, HelpText, ValidatedCommand};
use crate::cli::types::{CommandType, ParsedCommand};
use async_trait::async_trait;
use clap::{Arg, ArgAction, ArgMatches, Command};
use std::collections::{HashMap, HashSet};

/// Clap-based command parser implementation
pub struct ClapCommandParser {
    app: Command,
}

impl ClapCommandParser {
    /// Create a new clap command parser
    pub fn new() -> Self {
        let app = build_cli();
        Self { app }
    }

    /// Parse clap matches into a ParsedCommand
    fn matches_to_parsed_command(&self, matches: &ArgMatches) -> CLIResult<ParsedCommand> {
        let (command_type, subcommand_matches) = match matches.subcommand() {
            Some(("discover", sub_m)) => (CommandType::Discover, sub_m),
            Some(("send", sub_m)) => (CommandType::Send, sub_m),
            Some(("receive", sub_m)) => (CommandType::Receive, sub_m),
            Some(("stream", sub_m)) => (CommandType::Stream, sub_m),
            Some(("exec", sub_m)) => (CommandType::Exec, sub_m),
            Some(("peers", sub_m)) => (CommandType::Peers, sub_m),
            Some(("status", sub_m)) => (CommandType::Status, sub_m),
            Some(("clipboard", sub_m)) => (CommandType::Clipboard, sub_m),
            Some(("tui", sub_m)) => (CommandType::TUI, sub_m),
            Some(("config", sub_m)) => (CommandType::Config, sub_m),
            _ => {
                return Err(CLIError::InvalidCommand(
                    "No valid command provided".to_string(),
                ))
            }
        };

        let mut parsed = ParsedCommand::new(command_type);

        // Extract subcommand if present
        if let Some((sub_name, _)) = subcommand_matches.subcommand() {
            parsed.subcommand = Some(sub_name.to_string());
        }

        // Extract arguments and options based on command type
        self.extract_command_data(&mut parsed, subcommand_matches)?;

        Ok(parsed)
    }

    /// Extract command-specific data from matches
    fn extract_command_data(
        &self,
        parsed: &mut ParsedCommand,
        matches: &ArgMatches,
    ) -> CLIResult<()> {
        // Extract common options
        if let Some(format) = matches.get_one::<String>("format") {
            parsed.options.insert("format".to_string(), format.clone());
        }

        if matches.get_flag("json") {
            parsed.flags.insert("json".to_string());
        }

        if matches.get_flag("verbose") {
            parsed.flags.insert("verbose".to_string());
        }

        if matches.get_flag("quiet") {
            parsed.flags.insert("quiet".to_string());
        }

        // Extract command-specific data
        match parsed.command {
            CommandType::Discover => self.extract_discover_data(parsed, matches)?,
            CommandType::Send => self.extract_send_data(parsed, matches)?,
            CommandType::Receive => self.extract_receive_data(parsed, matches)?,
            CommandType::Stream => self.extract_stream_data(parsed, matches)?,
            CommandType::Exec => self.extract_exec_data(parsed, matches)?,
            CommandType::Peers => self.extract_peers_data(parsed, matches)?,
            CommandType::Status => self.extract_status_data(parsed, matches)?,
            CommandType::Clipboard => self.extract_clipboard_data(parsed, matches)?,
            CommandType::TUI => self.extract_tui_data(parsed, matches)?,
            CommandType::Config => self.extract_config_data(parsed, matches)?,
        }

        Ok(())
    }

    fn extract_discover_data(
        &self,
        parsed: &mut ParsedCommand,
        matches: &ArgMatches,
    ) -> CLIResult<()> {
        if let Some(device_type) = matches.get_one::<String>("type") {
            parsed.options.insert("type".to_string(), device_type.clone());
        }

        if let Some(name) = matches.get_one::<String>("name") {
            parsed.options.insert("name".to_string(), name.clone());
        }

        if let Some(timeout) = matches.get_one::<String>("timeout") {
            parsed.options.insert("timeout".to_string(), timeout.clone());
        }

        if matches.get_flag("watch") {
            parsed.flags.insert("watch".to_string());
        }

        Ok(())
    }

    fn extract_send_data(
        &self,
        parsed: &mut ParsedCommand,
        matches: &ArgMatches,
    ) -> CLIResult<()> {
        // Get file paths
        if let Some(files) = matches.get_many::<String>("files") {
            parsed.arguments = files.map(|s| s.clone()).collect();
        }

        if let Some(peer) = matches.get_one::<String>("peer") {
            parsed.options.insert("peer".to_string(), peer.clone());
        }

        if matches.get_flag("no-compression") {
            parsed.flags.insert("no-compression".to_string());
        }

        if matches.get_flag("no-encryption") {
            parsed.flags.insert("no-encryption".to_string());
        }

        Ok(())
    }

    fn extract_receive_data(
        &self,
        parsed: &mut ParsedCommand,
        matches: &ArgMatches,
    ) -> CLIResult<()> {
        if let Some(output) = matches.get_one::<String>("output") {
            parsed.options.insert("output".to_string(), output.clone());
        }

        if matches.get_flag("auto-accept") {
            parsed.flags.insert("auto-accept".to_string());
        }

        if let Some(from) = matches.get_one::<String>("from") {
            parsed.options.insert("from".to_string(), from.clone());
        }

        Ok(())
    }

    fn extract_stream_data(
        &self,
        parsed: &mut ParsedCommand,
        matches: &ArgMatches,
    ) -> CLIResult<()> {
        // Check for camera subcommand
        if let Some((sub_name, sub_matches)) = matches.subcommand() {
            parsed.subcommand = Some(sub_name.to_string());

            if let Some(quality) = sub_matches.get_one::<String>("quality") {
                parsed.options.insert("quality".to_string(), quality.clone());
            }

            if let Some(camera) = sub_matches.get_one::<String>("camera") {
                parsed.options.insert("camera".to_string(), camera.clone());
            }

            if sub_matches.get_flag("record") {
                parsed.flags.insert("record".to_string());
            }

            if let Some(output) = sub_matches.get_one::<String>("output") {
                parsed.options.insert("output".to_string(), output.clone());
            }
        }

        Ok(())
    }

    fn extract_exec_data(
        &self,
        parsed: &mut ParsedCommand,
        matches: &ArgMatches,
    ) -> CLIResult<()> {
        if let Some(command) = matches.get_one::<String>("command") {
            parsed.arguments.push(command.clone());
        }

        if let Some(peer) = matches.get_one::<String>("peer") {
            parsed.options.insert("peer".to_string(), peer.clone());
        }

        if matches.get_flag("interactive") {
            parsed.flags.insert("interactive".to_string());
        }

        Ok(())
    }

    fn extract_peers_data(
        &self,
        parsed: &mut ParsedCommand,
        matches: &ArgMatches,
    ) -> CLIResult<()> {
        if matches.get_flag("watch") {
            parsed.flags.insert("watch".to_string());
        }

        if let Some(filter) = matches.get_one::<String>("filter") {
            parsed.options.insert("filter".to_string(), filter.clone());
        }

        Ok(())
    }

    fn extract_status_data(
        &self,
        parsed: &mut ParsedCommand,
        matches: &ArgMatches,
    ) -> CLIResult<()> {
        if matches.get_flag("detailed") {
            parsed.flags.insert("detailed".to_string());
        }

        Ok(())
    }

    fn extract_clipboard_data(
        &self,
        parsed: &mut ParsedCommand,
        matches: &ArgMatches,
    ) -> CLIResult<()> {
        // Check for clipboard subcommand
        if let Some((sub_name, sub_matches)) = matches.subcommand() {
            parsed.subcommand = Some(sub_name.to_string());

            if let Some(peer) = sub_matches.get_one::<String>("peer") {
                parsed.options.insert("peer".to_string(), peer.clone());
            }

            if sub_matches.get_flag("enable") {
                parsed.flags.insert("enable".to_string());
            }

            if sub_matches.get_flag("disable") {
                parsed.flags.insert("disable".to_string());
            }
        }

        Ok(())
    }

    fn extract_tui_data(
        &self,
        _parsed: &mut ParsedCommand,
        _matches: &ArgMatches,
    ) -> CLIResult<()> {
        // TUI has no specific arguments
        Ok(())
    }

    fn extract_config_data(
        &self,
        parsed: &mut ParsedCommand,
        matches: &ArgMatches,
    ) -> CLIResult<()> {
        // Check for config subcommand
        if let Some((sub_name, sub_matches)) = matches.subcommand() {
            parsed.subcommand = Some(sub_name.to_string());

            if let Some(key) = sub_matches.get_one::<String>("key") {
                parsed.arguments.push(key.clone());
            }

            if let Some(value) = sub_matches.get_one::<String>("value") {
                parsed.arguments.push(value.clone());
            }
        }

        Ok(())
    }
}

impl Default for ClapCommandParser {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CommandParser for ClapCommandParser {
    async fn parse_args(&self, args: Vec<String>) -> CLIResult<ParsedCommand> {
        // Clone the app for parsing
        let app = build_cli();

        match app.try_get_matches_from(args) {
            Ok(matches) => self.matches_to_parsed_command(&matches),
            Err(e) => Err(CLIError::ParseError(e.to_string())),
        }
    }

    async fn validate_command(&self, command: ParsedCommand) -> CLIResult<ValidatedCommand> {
        // Use the CommandValidator for comprehensive validation
        let warnings = CommandValidator::validate(&command)?;

        let mut validated = ValidatedCommand::new(command);
        for warning in warnings {
            validated = validated.with_warning(warning.format());
        }

        Ok(validated)
    }

    async fn generate_help(&self, command: Option<String>) -> CLIResult<HelpText> {
        let app = build_cli();

        let help_text = if let Some(cmd_name) = command {
            // Generate help for specific command
            if let Some(subcommand) = app.find_subcommand(&cmd_name) {
                let mut buffer = Vec::new();
                subcommand
                    .clone()
                    .write_long_help(&mut buffer)
                    .map_err(|e| CLIError::other(format!("Failed to generate help: {}", e)))?;

                HelpText {
                    command: Some(cmd_name.clone()),
                    description: subcommand
                        .get_about()
                        .map(|s| s.to_string())
                        .unwrap_or_default(),
                    usage: String::from_utf8_lossy(&buffer).to_string(),
                    examples: get_command_examples(&cmd_name),
                    options: Vec::new(),
                }
            } else {
                return Err(CLIError::InvalidCommand(format!(
                    "Unknown command: {}",
                    cmd_name
                )));
            }
        } else {
            // Generate general help
            let mut buffer = Vec::new();
            app.clone()
                .write_long_help(&mut buffer)
                .map_err(|e| CLIError::other(format!("Failed to generate help: {}", e)))?;

            HelpText {
                command: None,
                description: "Kizuna - Seamless device connectivity and file sharing".to_string(),
                usage: String::from_utf8_lossy(&buffer).to_string(),
                examples: vec![
                    "kizuna discover".to_string(),
                    "kizuna send file.txt --peer laptop".to_string(),
                    "kizuna tui".to_string(),
                ],
                options: Vec::new(),
            }
        };

        Ok(help_text)
    }

    async fn suggest_corrections(&self, invalid_command: String) -> CLIResult<Vec<String>> {
        // Use the CommandValidator for suggestions
        Ok(CommandValidator::suggest_similar_commands(&invalid_command))
    }
}

/// Build the CLI application structure
fn build_cli() -> Command {
    Command::new("kizuna")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Kizuna Team")
        .about("Seamless device connectivity and file sharing")
        .arg_required_else_help(true)
        .subcommand_required(true)
        .subcommand(build_discover_command())
        .subcommand(build_send_command())
        .subcommand(build_receive_command())
        .subcommand(build_stream_command())
        .subcommand(build_exec_command())
        .subcommand(build_peers_command())
        .subcommand(build_status_command())
        .subcommand(build_clipboard_command())
        .subcommand(build_tui_command())
        .subcommand(build_config_command())
}

fn build_discover_command() -> Command {
    Command::new("discover")
        .about("Discover available peers on the network")
        .long_about("Discover and list available Kizuna peers on the local network. \
                     Results are displayed incrementally as peers are found.")
        .arg(
            Arg::new("type")
                .short('t')
                .long("type")
                .value_name("TYPE")
                .help("Filter by device type (desktop, mobile, tablet)")
        )
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .value_name("NAME")
                .help("Filter by device name (supports wildcards)")
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .value_name("SECONDS")
                .default_value("10")
                .help("Discovery timeout in seconds")
        )
        .arg(
            Arg::new("watch")
                .short('w')
                .long("watch")
                .action(ArgAction::SetTrue)
                .help("Continuously watch for peers")
        )
        .arg(
            Arg::new("format")
                .short('f')
                .long("format")
                .value_name("FORMAT")
                .value_parser(["table", "json", "csv", "minimal"])
                .help("Output format")
        )
        .arg(
            Arg::new("json")
                .short('j')
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Output in JSON format (shorthand for --format json)")
        )
}

fn build_send_command() -> Command {
    Command::new("send")
        .about("Send files to a peer")
        .long_about("Transfer one or more files to a connected peer. \
                     Supports compression and encryption for secure transfers.")
        .arg(
            Arg::new("files")
                .value_name("FILES")
                .required(true)
                .num_args(1..)
                .help("Files to send")
        )
        .arg(
            Arg::new("peer")
                .short('p')
                .long("peer")
                .value_name("PEER")
                .help("Target peer name or ID")
        )
        .arg(
            Arg::new("no-compression")
                .long("no-compression")
                .action(ArgAction::SetTrue)
                .help("Disable compression")
        )
        .arg(
            Arg::new("no-encryption")
                .long("no-encryption")
                .action(ArgAction::SetTrue)
                .help("Disable encryption (not recommended)")
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::SetTrue)
                .help("Show detailed progress")
        )
}

fn build_receive_command() -> Command {
    Command::new("receive")
        .about("Receive incoming file transfers")
        .long_about("Accept and receive incoming file transfers from peers. \
                     Specify download location and acceptance options.")
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .value_name("DIR")
                .help("Output directory for received files")
        )
        .arg(
            Arg::new("auto-accept")
                .short('a')
                .long("auto-accept")
                .action(ArgAction::SetTrue)
                .help("Automatically accept transfers from trusted peers")
        )
        .arg(
            Arg::new("from")
                .short('f')
                .long("from")
                .value_name("PEER")
                .help("Only accept from specific peer")
        )
}

fn build_stream_command() -> Command {
    Command::new("stream")
        .about("Manage media streaming")
        .long_about("Start and manage camera or screen streaming to connected peers.")
        .subcommand(
            Command::new("camera")
                .about("Stream camera feed")
                .arg(
                    Arg::new("camera")
                        .short('c')
                        .long("camera")
                        .value_name("ID")
                        .help("Camera device ID or index")
                )
                .arg(
                    Arg::new("quality")
                        .short('q')
                        .long("quality")
                        .value_name("QUALITY")
                        .value_parser(["low", "medium", "high", "ultra"])
                        .default_value("medium")
                        .help("Stream quality")
                )
                .arg(
                    Arg::new("record")
                        .short('r')
                        .long("record")
                        .action(ArgAction::SetTrue)
                        .help("Record stream to file")
                )
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("FILE")
                        .help("Recording output file")
                )
        )
}

fn build_exec_command() -> Command {
    Command::new("exec")
        .about("Execute command on remote peer")
        .long_about("Execute a command on a remote peer with real-time output. \
                     Requires authorization from the remote peer.")
        .arg(
            Arg::new("command")
                .value_name("COMMAND")
                .required(true)
                .help("Command to execute")
        )
        .arg(
            Arg::new("peer")
                .short('p')
                .long("peer")
                .value_name("PEER")
                .required(true)
                .help("Target peer name or ID")
        )
        .arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .action(ArgAction::SetTrue)
                .help("Interactive mode with stdin support")
        )
}

fn build_peers_command() -> Command {
    Command::new("peers")
        .about("List connected peers")
        .long_about("Display information about connected peers including status, \
                     capabilities, and trust information.")
        .arg(
            Arg::new("watch")
                .short('w')
                .long("watch")
                .action(ArgAction::SetTrue)
                .help("Continuously monitor peer status")
        )
        .arg(
            Arg::new("filter")
                .short('f')
                .long("filter")
                .value_name("FILTER")
                .help("Filter peers by criteria")
        )
        .arg(
            Arg::new("format")
                .long("format")
                .value_name("FORMAT")
                .value_parser(["table", "json", "csv"])
                .help("Output format")
        )
}

fn build_status_command() -> Command {
    Command::new("status")
        .about("Show system status")
        .long_about("Display Kizuna system status including network information, \
                     active operations, and connection health.")
        .arg(
            Arg::new("detailed")
                .short('d')
                .long("detailed")
                .action(ArgAction::SetTrue)
                .help("Show detailed status information")
        )
        .arg(
            Arg::new("json")
                .short('j')
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Output in JSON format")
        )
}

fn build_clipboard_command() -> Command {
    Command::new("clipboard")
        .about("Manage clipboard sharing")
        .long_about("Control clipboard synchronization with connected peers.")
        .subcommand(
            Command::new("share")
                .about("Toggle clipboard sharing")
                .arg(
                    Arg::new("peer")
                        .short('p')
                        .long("peer")
                        .value_name("PEER")
                        .help("Specific peer to share with")
                )
                .arg(
                    Arg::new("enable")
                        .short('e')
                        .long("enable")
                        .action(ArgAction::SetTrue)
                        .help("Enable clipboard sharing")
                )
                .arg(
                    Arg::new("disable")
                        .short('d')
                        .long("disable")
                        .action(ArgAction::SetTrue)
                        .help("Disable clipboard sharing")
                )
        )
        .subcommand(
            Command::new("status")
                .about("Show clipboard sharing status")
        )
        .subcommand(
            Command::new("history")
                .about("View clipboard history")
        )
}

fn build_tui_command() -> Command {
    Command::new("tui")
        .about("Launch interactive TUI")
        .long_about("Launch the interactive Text User Interface for visual peer \
                     management and file operations.")
}

fn build_config_command() -> Command {
    Command::new("config")
        .about("Manage configuration")
        .long_about("View and modify Kizuna configuration settings.")
        .subcommand(
            Command::new("get")
                .about("Get configuration value")
                .arg(
                    Arg::new("key")
                        .value_name("KEY")
                        .required(true)
                        .help("Configuration key")
                )
        )
        .subcommand(
            Command::new("set")
                .about("Set configuration value")
                .arg(
                    Arg::new("key")
                        .value_name("KEY")
                        .required(true)
                        .help("Configuration key")
                )
                .arg(
                    Arg::new("value")
                        .value_name("VALUE")
                        .required(true)
                        .help("Configuration value")
                )
        )
        .subcommand(
            Command::new("list")
                .about("List all configuration")
        )
}

/// Get command-specific examples
fn get_command_examples(command: &str) -> Vec<String> {
    match command {
        "discover" => vec![
            "kizuna discover".to_string(),
            "kizuna discover --type desktop".to_string(),
            "kizuna discover --name 'laptop*' --json".to_string(),
            "kizuna discover --watch".to_string(),
        ],
        "send" => vec![
            "kizuna send file.txt --peer laptop".to_string(),
            "kizuna send *.jpg --peer phone".to_string(),
            "kizuna send document.pdf --no-compression".to_string(),
        ],
        "receive" => vec![
            "kizuna receive".to_string(),
            "kizuna receive --output ~/Downloads".to_string(),
            "kizuna receive --auto-accept --from laptop".to_string(),
        ],
        "stream" => vec![
            "kizuna stream camera".to_string(),
            "kizuna stream camera --quality high".to_string(),
            "kizuna stream camera --record --output recording.mp4".to_string(),
        ],
        "exec" => vec![
            "kizuna exec 'ls -la' --peer server".to_string(),
            "kizuna exec 'uptime' --peer laptop".to_string(),
        ],
        "peers" => vec![
            "kizuna peers".to_string(),
            "kizuna peers --watch".to_string(),
            "kizuna peers --format json".to_string(),
        ],
        "status" => vec![
            "kizuna status".to_string(),
            "kizuna status --detailed".to_string(),
        ],
        "clipboard" => vec![
            "kizuna clipboard share --enable".to_string(),
            "kizuna clipboard status".to_string(),
            "kizuna clipboard history".to_string(),
        ],
        _ => vec![],
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

    #[tokio::test]
    async fn test_parse_discover_command() {
        let parser = ClapCommandParser::new();
        let args = vec![
            "kizuna".to_string(),
            "discover".to_string(),
            "--type".to_string(),
            "desktop".to_string(),
        ];

        let result = parser.parse_args(args).await;
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.command, CommandType::Discover);
        assert_eq!(parsed.get_option("type"), Some(&"desktop".to_string()));
    }

    #[tokio::test]
    async fn test_parse_send_command() {
        let parser = ClapCommandParser::new();
        let args = vec![
            "kizuna".to_string(),
            "send".to_string(),
            "file.txt".to_string(),
            "--peer".to_string(),
            "laptop".to_string(),
        ];

        let result = parser.parse_args(args).await;
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.command, CommandType::Send);
        assert_eq!(parsed.arguments.len(), 1);
        assert_eq!(parsed.arguments[0], "file.txt");
        assert_eq!(parsed.get_option("peer"), Some(&"laptop".to_string()));
    }

    #[tokio::test]
    async fn test_suggest_corrections() {
        let parser = ClapCommandParser::new();
        let suggestions = parser.suggest_corrections("discver".to_string()).await.unwrap();

        assert!(!suggestions.is_empty());
        assert!(suggestions.contains(&"discover".to_string()));
    }

    #[tokio::test]
    async fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("discover", "discover"), 0);
        assert_eq!(levenshtein_distance("discover", "discver"), 1);
        assert_eq!(levenshtein_distance("send", "end"), 1);
        assert_eq!(levenshtein_distance("abc", "xyz"), 3);
    }
}
