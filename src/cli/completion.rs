// Shell completion script generation module

use crate::cli::error::{CLIError, CLIResult};
use clap::{Command, CommandFactory};
use clap_complete::{generate, Generator, Shell};
use std::io;

/// Shell completion generator
pub struct CompletionGenerator;

impl CompletionGenerator {
    /// Generate completion script for the specified shell
    pub fn generate_for_shell(shell: Shell) -> CLIResult<String> {
        let mut app = build_cli_app();
        let mut buffer = Vec::new();
        
        generate(shell, &mut app, "kizuna", &mut buffer);
        
        String::from_utf8(buffer)
            .map_err(|e| CLIError::other(format!("Failed to generate completion script: {}", e)))
    }

    /// Generate bash completion script
    pub fn generate_bash() -> CLIResult<String> {
        Self::generate_for_shell(Shell::Bash)
    }

    /// Generate zsh completion script
    pub fn generate_zsh() -> CLIResult<String> {
        Self::generate_for_shell(Shell::Zsh)
    }

    /// Generate fish completion script
    pub fn generate_fish() -> CLIResult<String> {
        Self::generate_for_shell(Shell::Fish)
    }

    /// Generate PowerShell completion script
    pub fn generate_powershell() -> CLIResult<String> {
        Self::generate_for_shell(Shell::PowerShell)
    }

    /// Generate completion script and write to stdout
    pub fn generate_to_stdout(shell: Shell) -> CLIResult<()> {
        let mut app = build_cli_app();
        generate(shell, &mut app, "kizuna", &mut io::stdout());
        Ok(())
    }

    /// Get installation instructions for a shell
    pub fn get_install_instructions(shell: Shell) -> String {
        match shell {
            Shell::Bash => {
                r#"# Bash completion installation:
# 
# 1. Generate the completion script:
#    kizuna completion bash > ~/.local/share/bash-completion/completions/kizuna
#
# 2. Or add to your ~/.bashrc:
#    eval "$(kizuna completion bash)"
#
# 3. Reload your shell:
#    source ~/.bashrc
"#.to_string()
            }
            Shell::Zsh => {
                r#"# Zsh completion installation:
#
# 1. Generate the completion script:
#    kizuna completion zsh > "${fpath[1]}/_kizuna"
#
# 2. Or add to your ~/.zshrc:
#    eval "$(kizuna completion zsh)"
#
# 3. Reload your shell:
#    source ~/.zshrc
#
# Note: You may need to run 'compinit' after adding the completion
"#.to_string()
            }
            Shell::Fish => {
                r#"# Fish completion installation:
#
# 1. Generate the completion script:
#    kizuna completion fish > ~/.config/fish/completions/kizuna.fish
#
# 2. Or add to your config.fish:
#    kizuna completion fish | source
#
# 3. Reload your shell:
#    source ~/.config/fish/config.fish
"#.to_string()
            }
            Shell::PowerShell => {
                r#"# PowerShell completion installation:
#
# 1. Generate the completion script:
#    kizuna completion powershell | Out-String | Invoke-Expression
#
# 2. Or add to your PowerShell profile:
#    kizuna completion powershell >> $PROFILE
#
# 3. Reload your profile:
#    . $PROFILE
"#.to_string()
            }
            _ => "Completion installation instructions not available for this shell.".to_string(),
        }
    }

    /// Detect the current shell from environment
    pub fn detect_shell() -> Option<Shell> {
        if let Ok(shell) = std::env::var("SHELL") {
            if shell.contains("bash") {
                return Some(Shell::Bash);
            } else if shell.contains("zsh") {
                return Some(Shell::Zsh);
            } else if shell.contains("fish") {
                return Some(Shell::Fish);
            }
        }

        // Check for PowerShell on Windows
        #[cfg(windows)]
        {
            if std::env::var("PSModulePath").is_ok() {
                return Some(Shell::PowerShell);
            }
        }

        None
    }
}

/// Build the CLI application for completion generation
fn build_cli_app() -> Command {
    use clap::{Arg, ArgAction};

    Command::new("kizuna")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Kizuna Team")
        .about("Seamless device connectivity and file sharing")
        .arg_required_else_help(true)
        .subcommand_required(true)
        .subcommand(
            Command::new("discover")
                .about("Discover available peers on the network")
                .arg(
                    Arg::new("type")
                        .short('t')
                        .long("type")
                        .value_name("TYPE")
                        .help("Filter by device type")
                )
                .arg(
                    Arg::new("name")
                        .short('n')
                        .long("name")
                        .value_name("NAME")
                        .help("Filter by device name")
                )
                .arg(
                    Arg::new("timeout")
                        .long("timeout")
                        .value_name("SECONDS")
                        .help("Discovery timeout")
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
        )
        .subcommand(
            Command::new("send")
                .about("Send files to a peer")
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
        )
        .subcommand(
            Command::new("receive")
                .about("Receive incoming file transfers")
                .arg(
                    Arg::new("output")
                        .short('o')
                        .long("output")
                        .value_name("DIR")
                        .help("Output directory")
                )
                .arg(
                    Arg::new("auto-accept")
                        .short('a')
                        .long("auto-accept")
                        .action(ArgAction::SetTrue)
                        .help("Auto-accept from trusted peers")
                )
        )
        .subcommand(
            Command::new("stream")
                .about("Manage media streaming")
                .subcommand(
                    Command::new("camera")
                        .about("Stream camera feed")
                        .arg(
                            Arg::new("quality")
                                .short('q')
                                .long("quality")
                                .value_name("QUALITY")
                                .value_parser(["low", "medium", "high", "ultra"])
                                .help("Stream quality")
                        )
                )
        )
        .subcommand(
            Command::new("exec")
                .about("Execute command on remote peer")
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
                        .help("Target peer")
                )
        )
        .subcommand(
            Command::new("peers")
                .about("List connected peers")
                .arg(
                    Arg::new("watch")
                        .short('w')
                        .long("watch")
                        .action(ArgAction::SetTrue)
                        .help("Monitor peer status")
                )
        )
        .subcommand(
            Command::new("status")
                .about("Show system status")
                .arg(
                    Arg::new("detailed")
                        .short('d')
                        .long("detailed")
                        .action(ArgAction::SetTrue)
                        .help("Detailed status")
                )
        )
        .subcommand(
            Command::new("clipboard")
                .about("Manage clipboard sharing")
                .subcommand(
                    Command::new("share")
                        .about("Toggle clipboard sharing")
                        .arg(
                            Arg::new("enable")
                                .short('e')
                                .long("enable")
                                .action(ArgAction::SetTrue)
                                .help("Enable sharing")
                        )
                )
                .subcommand(Command::new("status").about("Show clipboard status"))
                .subcommand(Command::new("history").about("View clipboard history"))
        )
        .subcommand(Command::new("tui").about("Launch interactive TUI"))
        .subcommand(
            Command::new("config")
                .about("Manage configuration")
                .subcommand(
                    Command::new("get")
                        .about("Get configuration value")
                        .arg(Arg::new("key").required(true))
                )
                .subcommand(
                    Command::new("set")
                        .about("Set configuration value")
                        .arg(Arg::new("key").required(true))
                        .arg(Arg::new("value").required(true))
                )
                .subcommand(Command::new("list").about("List all configuration"))
        )
        .subcommand(
            Command::new("completion")
                .about("Generate shell completion scripts")
                .arg(
                    Arg::new("shell")
                        .value_name("SHELL")
                        .required(true)
                        .value_parser(["bash", "zsh", "fish", "powershell"])
                        .help("Shell type")
                )
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_bash_completion() {
        let result = CompletionGenerator::generate_bash();
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.contains("kizuna"));
        assert!(script.contains("discover"));
    }

    #[test]
    fn test_generate_zsh_completion() {
        let result = CompletionGenerator::generate_zsh();
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.contains("kizuna"));
    }

    #[test]
    fn test_generate_fish_completion() {
        let result = CompletionGenerator::generate_fish();
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.contains("kizuna"));
    }

    #[test]
    fn test_generate_powershell_completion() {
        let result = CompletionGenerator::generate_powershell();
        assert!(result.is_ok());
        let script = result.unwrap();
        assert!(script.contains("kizuna"));
    }

    #[test]
    fn test_get_install_instructions() {
        let bash_instructions = CompletionGenerator::get_install_instructions(Shell::Bash);
        assert!(bash_instructions.contains("bash"));
        assert!(bash_instructions.contains("bashrc"));

        let zsh_instructions = CompletionGenerator::get_install_instructions(Shell::Zsh);
        assert!(zsh_instructions.contains("zsh"));
        assert!(zsh_instructions.contains("zshrc"));

        let fish_instructions = CompletionGenerator::get_install_instructions(Shell::Fish);
        assert!(fish_instructions.contains("fish"));

        let ps_instructions = CompletionGenerator::get_install_instructions(Shell::PowerShell);
        assert!(ps_instructions.contains("PowerShell"));
    }
}
