// Comprehensive help and documentation system for CLI
//
// Implements detailed help text with examples and usage patterns,
// man page generation for Unix systems, and interactive help system
// with search and navigation.
//
// Requirements: 10.5

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::types::CommandType;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

/// Help system manager
pub struct HelpSystem {
    commands: HashMap<String, CommandHelp>,
}

impl HelpSystem {
    /// Create a new help system with all command documentation
    pub fn new() -> Self {
        let mut commands = HashMap::new();

        // Add help for all commands
        commands.insert("discover".to_string(), Self::discover_help());
        commands.insert("send".to_string(), Self::send_help());
        commands.insert("receive".to_string(), Self::receive_help());
        commands.insert("stream".to_string(), Self::stream_help());
        commands.insert("exec".to_string(), Self::exec_help());
        commands.insert("peers".to_string(), Self::peers_help());
        commands.insert("status".to_string(), Self::status_help());
        commands.insert("clipboard".to_string(), Self::clipboard_help());
        commands.insert("tui".to_string(), Self::tui_help());
        commands.insert("config".to_string(), Self::config_help());

        Self { commands }
    }

    /// Get help for a specific command
    pub fn get_command_help(&self, command: &str) -> CLIResult<&CommandHelp> {
        self.commands
            .get(command)
            .ok_or_else(|| CLIError::parse(format!("No help available for command: {}", command)))
    }

    /// Get general help text
    pub fn get_general_help(&self) -> String {
        let mut help = String::new();
        writeln!(
            &mut help,
            "Kizuna - Secure peer-to-peer file transfer and collaboration"
        )
        .unwrap();
        writeln!(&mut help).unwrap();
        writeln!(&mut help, "USAGE:").unwrap();
        writeln!(&mut help, "    kizuna <COMMAND> [OPTIONS]").unwrap();
        writeln!(&mut help).unwrap();
        writeln!(&mut help, "COMMANDS:").unwrap();
        writeln!(&mut help, "    discover    Discover available peers on the network").unwrap();
        writeln!(&mut help, "    send        Send files to a peer").unwrap();
        writeln!(&mut help, "    receive     Receive files from peers").unwrap();
        writeln!(&mut help, "    stream      Stream camera to peers").unwrap();
        writeln!(&mut help, "    exec        Execute commands on remote peers").unwrap();
        writeln!(&mut help, "    peers       List connected peers").unwrap();
        writeln!(&mut help, "    status      Show system status").unwrap();
        writeln!(&mut help, "    clipboard   Manage clipboard sharing").unwrap();
        writeln!(&mut help, "    tui         Launch interactive TUI mode").unwrap();
        writeln!(&mut help, "    config      Manage configuration").unwrap();
        writeln!(&mut help).unwrap();
        writeln!(&mut help, "OPTIONS:").unwrap();
        writeln!(&mut help, "    -h, --help       Print help information").unwrap();
        writeln!(&mut help, "    -V, --version    Print version information").unwrap();
        writeln!(&mut help).unwrap();
        writeln!(
            &mut help,
            "Use 'kizuna <COMMAND> --help' for more information on a specific command."
        )
        .unwrap();

        help
    }

    /// Search help content for a query
    pub fn search(&self, query: &str) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for (command_name, help) in &self.commands {
            // Search in command name
            if command_name.contains(&query_lower) {
                results.push(SearchResult {
                    command: command_name.clone(),
                    match_type: MatchType::CommandName,
                    context: help.short_description.clone(),
                });
            }

            // Search in description
            if help.short_description.to_lowercase().contains(&query_lower)
                || help.long_description.to_lowercase().contains(&query_lower)
            {
                results.push(SearchResult {
                    command: command_name.clone(),
                    match_type: MatchType::Description,
                    context: help.short_description.clone(),
                });
            }

            // Search in examples
            for example in &help.examples {
                if example.description.to_lowercase().contains(&query_lower)
                    || example.command.to_lowercase().contains(&query_lower)
                {
                    results.push(SearchResult {
                        command: command_name.clone(),
                        match_type: MatchType::Example,
                        context: example.description.clone(),
                    });
                }
            }

            // Search in options
            for option in &help.options {
                if option.name.to_lowercase().contains(&query_lower)
                    || option.description.to_lowercase().contains(&query_lower)
                {
                    results.push(SearchResult {
                        command: command_name.clone(),
                        match_type: MatchType::Option,
                        context: format!("{}: {}", option.name, option.description),
                    });
                }
            }
        }

        results
    }

    /// Generate man page for a command
    pub fn generate_man_page(&self, command: &str) -> CLIResult<String> {
        let help = self.get_command_help(command)?;
        let mut man = String::new();

        // Man page header
        writeln!(&mut man, ".TH KIZUNA-{} 1 \"{}\" \"Kizuna\" \"User Commands\"",
                 command.to_uppercase(),
                 chrono::Utc::now().format("%B %Y")).unwrap();
        writeln!(&mut man).unwrap();

        // Name section
        writeln!(&mut man, ".SH NAME").unwrap();
        writeln!(&mut man, "kizuna-{} \\- {}", command, help.short_description).unwrap();
        writeln!(&mut man).unwrap();

        // Synopsis section
        writeln!(&mut man, ".SH SYNOPSIS").unwrap();
        writeln!(&mut man, ".B kizuna {}", command).unwrap();
        for option in &help.options {
            if option.required {
                writeln!(&mut man, ".I {}", option.name).unwrap();
            } else {
                writeln!(&mut man, ".RI [ {} ]", option.name).unwrap();
            }
        }
        writeln!(&mut man).unwrap();

        // Description section
        writeln!(&mut man, ".SH DESCRIPTION").unwrap();
        writeln!(&mut man, "{}", help.long_description).unwrap();
        writeln!(&mut man).unwrap();

        // Options section
        if !help.options.is_empty() {
            writeln!(&mut man, ".SH OPTIONS").unwrap();
            for option in &help.options {
                writeln!(&mut man, ".TP").unwrap();
                if let Some(short) = &option.short {
                    writeln!(&mut man, ".BR {} \", \" {}", short, option.name).unwrap();
                } else {
                    writeln!(&mut man, ".B {}", option.name).unwrap();
                }
                writeln!(&mut man, "{}", option.description).unwrap();
            }
            writeln!(&mut man).unwrap();
        }

        // Examples section
        if !help.examples.is_empty() {
            writeln!(&mut man, ".SH EXAMPLES").unwrap();
            for example in &help.examples {
                writeln!(&mut man, ".TP").unwrap();
                writeln!(&mut man, "{}", example.description).unwrap();
                writeln!(&mut man, ".PP").unwrap();
                writeln!(&mut man, ".nf").unwrap();
                writeln!(&mut man, ".RS").unwrap();
                writeln!(&mut man, "{}", example.command).unwrap();
                writeln!(&mut man, ".RE").unwrap();
                writeln!(&mut man, ".fi").unwrap();
            }
            writeln!(&mut man).unwrap();
        }

        // See also section
        writeln!(&mut man, ".SH SEE ALSO").unwrap();
        writeln!(&mut man, ".BR kizuna (1)").unwrap();

        Ok(man)
    }

    /// Format help text for terminal display
    pub fn format_help(&self, command: &str) -> CLIResult<String> {
        let help = self.get_command_help(command)?;
        let mut output = String::new();

        // Command name and description
        writeln!(&mut output, "{}", help.short_description).unwrap();
        writeln!(&mut output).unwrap();

        // Usage
        writeln!(&mut output, "USAGE:").unwrap();
        writeln!(&mut output, "    {}", help.usage).unwrap();
        writeln!(&mut output).unwrap();

        // Long description
        if !help.long_description.is_empty() {
            writeln!(&mut output, "DESCRIPTION:").unwrap();
            writeln!(&mut output, "    {}", help.long_description).unwrap();
            writeln!(&mut output).unwrap();
        }

        // Options
        if !help.options.is_empty() {
            writeln!(&mut output, "OPTIONS:").unwrap();
            for option in &help.options {
                if let Some(short) = &option.short {
                    writeln!(
                        &mut output,
                        "    {}, {}",
                        short, option.name
                    )
                    .unwrap();
                } else {
                    writeln!(&mut output, "    {}", option.name).unwrap();
                }
                writeln!(&mut output, "        {}", option.description).unwrap();
                writeln!(&mut output).unwrap();
            }
        }

        // Examples
        if !help.examples.is_empty() {
            writeln!(&mut output, "EXAMPLES:").unwrap();
            for example in &help.examples {
                writeln!(&mut output, "    {}", example.description).unwrap();
                writeln!(&mut output, "    $ {}", example.command).unwrap();
                writeln!(&mut output).unwrap();
            }
        }

        Ok(output)
    }

    // Command-specific help definitions

    fn discover_help() -> CommandHelp {
        CommandHelp {
            short_description: "Discover available peers on the network".to_string(),
            long_description: "Scan the local network for available Kizuna peers. Supports filtering by device type, name, and connection status.".to_string(),
            usage: "kizuna discover [OPTIONS]".to_string(),
            options: vec![
                HelpOption {
                    short: Some("-t".to_string()),
                    name: "--type <TYPE>".to_string(),
                    description: "Filter peers by device type (laptop, desktop, mobile)".to_string(),
                    required: false,
                },
                HelpOption {
                    short: Some("-n".to_string()),
                    name: "--name <NAME>".to_string(),
                    description: "Filter peers by name pattern".to_string(),
                    required: false,
                },
                HelpOption {
                    short: Some("-T".to_string()),
                    name: "--timeout <SECONDS>".to_string(),
                    description: "Discovery timeout in seconds (default: 10)".to_string(),
                    required: false,
                },
                HelpOption {
                    short: Some("-c".to_string()),
                    name: "--continuous".to_string(),
                    description: "Enable continuous discovery mode".to_string(),
                    required: false,
                },
                HelpOption {
                    short: Some("-f".to_string()),
                    name: "--format <FORMAT>".to_string(),
                    description: "Output format: table, json, csv, minimal (default: table)".to_string(),
                    required: false,
                },
            ],
            examples: vec![
                HelpExample {
                    description: "Discover all peers".to_string(),
                    command: "kizuna discover".to_string(),
                },
                HelpExample {
                    description: "Discover only laptop devices".to_string(),
                    command: "kizuna discover --type laptop".to_string(),
                },
                HelpExample {
                    description: "Discover peers with JSON output".to_string(),
                    command: "kizuna discover --format json".to_string(),
                },
            ],
        }
    }

    fn send_help() -> CommandHelp {
        CommandHelp {
            short_description: "Send files to a peer".to_string(),
            long_description: "Transfer one or more files to a specified peer. Supports compression, encryption, and batch transfers.".to_string(),
            usage: "kizuna send <FILES>... --peer <PEER> [OPTIONS]".to_string(),
            options: vec![
                HelpOption {
                    short: Some("-p".to_string()),
                    name: "--peer <PEER>".to_string(),
                    description: "Target peer name or ID".to_string(),
                    required: true,
                },
                HelpOption {
                    short: Some("-c".to_string()),
                    name: "--compression".to_string(),
                    description: "Enable compression (default: true)".to_string(),
                    required: false,
                },
                HelpOption {
                    short: Some("-e".to_string()),
                    name: "--encryption".to_string(),
                    description: "Enable encryption (default: true)".to_string(),
                    required: false,
                },
                HelpOption {
                    short: Some("-P".to_string()),
                    name: "--parallel".to_string(),
                    description: "Enable parallel transfer for multiple files".to_string(),
                    required: false,
                },
            ],
            examples: vec![
                HelpExample {
                    description: "Send a single file".to_string(),
                    command: "kizuna send document.pdf --peer laptop-1".to_string(),
                },
                HelpExample {
                    description: "Send multiple files".to_string(),
                    command: "kizuna send file1.txt file2.txt file3.txt --peer desktop-2".to_string(),
                },
                HelpExample {
                    description: "Send files with parallel transfer".to_string(),
                    command: "kizuna send *.jpg --peer phone-1 --parallel".to_string(),
                },
            ],
        }
    }

    fn receive_help() -> CommandHelp {
        CommandHelp {
            short_description: "Receive files from peers".to_string(),
            long_description: "Accept incoming file transfers from peers. Supports auto-accept for trusted peers and custom download locations.".to_string(),
            usage: "kizuna receive [OPTIONS]".to_string(),
            options: vec![
                HelpOption {
                    short: Some("-d".to_string()),
                    name: "--download-path <PATH>".to_string(),
                    description: "Download directory (default: ~/Downloads)".to_string(),
                    required: false,
                },
                HelpOption {
                    short: Some("-a".to_string()),
                    name: "--auto-accept".to_string(),
                    description: "Automatically accept transfers from trusted peers".to_string(),
                    required: false,
                },
            ],
            examples: vec![
                HelpExample {
                    description: "Start receiving files".to_string(),
                    command: "kizuna receive".to_string(),
                },
                HelpExample {
                    description: "Receive with custom download path".to_string(),
                    command: "kizuna receive --download-path /tmp/transfers".to_string(),
                },
            ],
        }
    }

    fn stream_help() -> CommandHelp {
        CommandHelp {
            short_description: "Stream camera to peers".to_string(),
            long_description: "Start camera streaming to connected peers. Supports quality settings, recording, and viewer management.".to_string(),
            usage: "kizuna stream [OPTIONS]".to_string(),
            options: vec![
                HelpOption {
                    short: Some("-c".to_string()),
                    name: "--camera <ID>".to_string(),
                    description: "Camera device ID (default: 0)".to_string(),
                    required: false,
                },
                HelpOption {
                    short: Some("-q".to_string()),
                    name: "--quality <QUALITY>".to_string(),
                    description: "Stream quality: low, medium, high (default: medium)".to_string(),
                    required: false,
                },
                HelpOption {
                    short: Some("-r".to_string()),
                    name: "--record".to_string(),
                    description: "Record stream to file".to_string(),
                    required: false,
                },
                HelpOption {
                    short: Some("-o".to_string()),
                    name: "--output <FILE>".to_string(),
                    description: "Recording output file".to_string(),
                    required: false,
                },
            ],
            examples: vec![
                HelpExample {
                    description: "Start camera streaming".to_string(),
                    command: "kizuna stream".to_string(),
                },
                HelpExample {
                    description: "Stream with high quality and recording".to_string(),
                    command: "kizuna stream --quality high --record --output stream.mp4".to_string(),
                },
            ],
        }
    }

    fn exec_help() -> CommandHelp {
        CommandHelp {
            short_description: "Execute commands on remote peers".to_string(),
            long_description: "Run commands on connected peers with real-time output. Requires authorization from the remote peer.".to_string(),
            usage: "kizuna exec <COMMAND> --peer <PEER> [OPTIONS]".to_string(),
            options: vec![
                HelpOption {
                    short: Some("-p".to_string()),
                    name: "--peer <PEER>".to_string(),
                    description: "Target peer name or ID".to_string(),
                    required: true,
                },
                HelpOption {
                    short: Some("-t".to_string()),
                    name: "--timeout <SECONDS>".to_string(),
                    description: "Command timeout in seconds".to_string(),
                    required: false,
                },
            ],
            examples: vec![
                HelpExample {
                    description: "Execute a command on a peer".to_string(),
                    command: "kizuna exec 'ls -la' --peer laptop-1".to_string(),
                },
                HelpExample {
                    description: "Execute with timeout".to_string(),
                    command: "kizuna exec 'long-running-task' --peer server-1 --timeout 300".to_string(),
                },
            ],
        }
    }

    fn peers_help() -> CommandHelp {
        CommandHelp {
            short_description: "List connected peers".to_string(),
            long_description: "Display detailed information about connected peers including capabilities, trust status, and last activity.".to_string(),
            usage: "kizuna peers [OPTIONS]".to_string(),
            options: vec![
                HelpOption {
                    short: Some("-f".to_string()),
                    name: "--format <FORMAT>".to_string(),
                    description: "Output format: table, json, csv, minimal (default: table)".to_string(),
                    required: false,
                },
            ],
            examples: vec![
                HelpExample {
                    description: "List all connected peers".to_string(),
                    command: "kizuna peers".to_string(),
                },
                HelpExample {
                    description: "List peers in JSON format".to_string(),
                    command: "kizuna peers --format json".to_string(),
                },
            ],
        }
    }

    fn status_help() -> CommandHelp {
        CommandHelp {
            short_description: "Show system status".to_string(),
            long_description: "Display system and connection information including network health, active operations, and diagnostics.".to_string(),
            usage: "kizuna status [OPTIONS]".to_string(),
            options: vec![
                HelpOption {
                    short: Some("-c".to_string()),
                    name: "--continuous".to_string(),
                    description: "Enable continuous monitoring mode".to_string(),
                    required: false,
                },
            ],
            examples: vec![
                HelpExample {
                    description: "Show current status".to_string(),
                    command: "kizuna status".to_string(),
                },
                HelpExample {
                    description: "Monitor status continuously".to_string(),
                    command: "kizuna status --continuous".to_string(),
                },
            ],
        }
    }

    fn clipboard_help() -> CommandHelp {
        CommandHelp {
            short_description: "Manage clipboard sharing".to_string(),
            long_description: "Control clipboard synchronization with connected peers. View clipboard history and manage content.".to_string(),
            usage: "kizuna clipboard <SUBCOMMAND> [OPTIONS]".to_string(),
            options: vec![
                HelpOption {
                    short: None,
                    name: "share".to_string(),
                    description: "Toggle clipboard sharing".to_string(),
                    required: false,
                },
                HelpOption {
                    short: None,
                    name: "status".to_string(),
                    description: "Show clipboard sync status".to_string(),
                    required: false,
                },
                HelpOption {
                    short: None,
                    name: "history".to_string(),
                    description: "View clipboard history".to_string(),
                    required: false,
                },
            ],
            examples: vec![
                HelpExample {
                    description: "Enable clipboard sharing".to_string(),
                    command: "kizuna clipboard share --enable".to_string(),
                },
                HelpExample {
                    description: "View clipboard status".to_string(),
                    command: "kizuna clipboard status".to_string(),
                },
            ],
        }
    }

    fn tui_help() -> CommandHelp {
        CommandHelp {
            short_description: "Launch interactive TUI mode".to_string(),
            long_description: "Start the interactive text user interface for visual management of peers, transfers, and operations.".to_string(),
            usage: "kizuna tui".to_string(),
            options: vec![],
            examples: vec![
                HelpExample {
                    description: "Launch TUI".to_string(),
                    command: "kizuna tui".to_string(),
                },
            ],
        }
    }

    fn config_help() -> CommandHelp {
        CommandHelp {
            short_description: "Manage configuration".to_string(),
            long_description: "View and modify Kizuna configuration settings. Supports profiles and validation.".to_string(),
            usage: "kizuna config <SUBCOMMAND> [OPTIONS]".to_string(),
            options: vec![
                HelpOption {
                    short: None,
                    name: "show".to_string(),
                    description: "Show current configuration".to_string(),
                    required: false,
                },
                HelpOption {
                    short: None,
                    name: "set".to_string(),
                    description: "Set a configuration value".to_string(),
                    required: false,
                },
                HelpOption {
                    short: None,
                    name: "profile".to_string(),
                    description: "Manage configuration profiles".to_string(),
                    required: false,
                },
            ],
            examples: vec![
                HelpExample {
                    description: "Show configuration".to_string(),
                    command: "kizuna config show".to_string(),
                },
                HelpExample {
                    description: "Set default peer".to_string(),
                    command: "kizuna config set default_peer laptop-1".to_string(),
                },
            ],
        }
    }
}

impl Default for HelpSystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Command help structure
#[derive(Debug, Clone)]
pub struct CommandHelp {
    pub short_description: String,
    pub long_description: String,
    pub usage: String,
    pub options: Vec<HelpOption>,
    pub examples: Vec<HelpExample>,
}

/// Help option structure
#[derive(Debug, Clone)]
pub struct HelpOption {
    pub short: Option<String>,
    pub name: String,
    pub description: String,
    pub required: bool,
}

/// Help example structure
#[derive(Debug, Clone)]
pub struct HelpExample {
    pub description: String,
    pub command: String,
}

/// Search result structure
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub command: String,
    pub match_type: MatchType,
    pub context: String,
}

/// Match type for search results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    CommandName,
    Description,
    Example,
    Option,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_system_creation() {
        let help = HelpSystem::new();
        assert!(help.commands.len() > 0);
    }

    #[test]
    fn test_get_command_help() {
        let help = HelpSystem::new();
        let discover_help = help.get_command_help("discover");
        assert!(discover_help.is_ok());
        assert!(discover_help.unwrap().short_description.contains("Discover"));
    }

    #[test]
    fn test_get_general_help() {
        let help = HelpSystem::new();
        let general = help.get_general_help();
        assert!(general.contains("Kizuna"));
        assert!(general.contains("USAGE"));
        assert!(general.contains("COMMANDS"));
    }

    #[test]
    fn test_search_help() {
        let help = HelpSystem::new();
        let results = help.search("transfer");
        assert!(results.len() > 0);
    }

    #[test]
    fn test_format_help() {
        let help = HelpSystem::new();
        let formatted = help.format_help("discover");
        assert!(formatted.is_ok());
        let text = formatted.unwrap();
        assert!(text.contains("USAGE"));
        assert!(text.contains("OPTIONS"));
        assert!(text.contains("EXAMPLES"));
    }

    #[test]
    fn test_generate_man_page() {
        let help = HelpSystem::new();
        let man = help.generate_man_page("discover");
        assert!(man.is_ok());
        let page = man.unwrap();
        assert!(page.contains(".TH KIZUNA-DISCOVER"));
        assert!(page.contains(".SH NAME"));
        assert!(page.contains(".SH SYNOPSIS"));
    }

    #[test]
    fn test_search_by_command_name() {
        let help = HelpSystem::new();
        let results = help.search("discover");
        assert!(results.iter().any(|r| r.match_type == MatchType::CommandName));
    }

    #[test]
    fn test_search_by_description() {
        let help = HelpSystem::new();
        let results = help.search("peers");
        assert!(results.len() > 0);
    }
}
