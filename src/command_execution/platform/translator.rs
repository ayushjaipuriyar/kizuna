// Cross-platform command translation
//
// This module provides automatic translation of common commands between platforms
// and handles platform-specific path and environment variable differences.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::command_execution::error::CommandResult;

/// Platform types for command translation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Windows,
    MacOS,
    Linux,
    Unix, // Generic Unix
}

impl Platform {
    /// Detect the current platform
    pub fn current() -> Self {
        #[cfg(target_os = "windows")]
        return Platform::Windows;
        
        #[cfg(target_os = "macos")]
        return Platform::MacOS;
        
        #[cfg(target_os = "linux")]
        return Platform::Linux;
        
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        return Platform::Unix;
    }

    /// Check if this is a Unix-like platform
    pub fn is_unix(&self) -> bool {
        matches!(self, Platform::MacOS | Platform::Linux | Platform::Unix)
    }

    /// Check if this is Windows
    pub fn is_windows(&self) -> bool {
        matches!(self, Platform::Windows)
    }

    /// Get the path separator for this platform
    pub fn path_separator(&self) -> char {
        match self {
            Platform::Windows => '\\',
            _ => '/',
        }
    }

    /// Get the environment variable path separator
    pub fn env_path_separator(&self) -> char {
        match self {
            Platform::Windows => ';',
            _ => ':',
        }
    }
}

/// Command translator for cross-platform compatibility
pub struct CommandTranslator {
    source_platform: Platform,
    target_platform: Platform,
    command_map: HashMap<String, CommandMapping>,
}

/// Mapping between platform-specific commands
#[derive(Debug, Clone)]
struct CommandMapping {
    windows: String,
    unix: String,
    description: String,
}

impl CommandTranslator {
    /// Create a new command translator
    pub fn new(source_platform: Platform, target_platform: Platform) -> Self {
        let mut translator = Self {
            source_platform,
            target_platform,
            command_map: HashMap::new(),
        };
        translator.initialize_mappings();
        translator
    }

    /// Create a translator for the current platform
    pub fn for_current_platform() -> Self {
        let current = Platform::current();
        Self::new(current, current)
    }

    /// Initialize common command mappings
    fn initialize_mappings(&mut self) {
        let mappings = vec![
            ("ls", "dir", "List directory contents"),
            ("cat", "type", "Display file contents"),
            ("cp", "copy", "Copy files"),
            ("mv", "move", "Move/rename files"),
            ("rm", "del", "Delete files"),
            ("mkdir", "mkdir", "Create directory"),
            ("rmdir", "rmdir", "Remove directory"),
            ("pwd", "cd", "Print working directory"),
            ("clear", "cls", "Clear screen"),
            ("grep", "findstr", "Search text"),
            ("ps", "tasklist", "List processes"),
            ("kill", "taskkill", "Terminate process"),
            ("which", "where", "Locate command"),
            ("touch", "type nul >", "Create empty file"),
            ("chmod", "icacls", "Change permissions"),
            ("env", "set", "Display environment variables"),
            ("export", "set", "Set environment variable"),
            ("echo", "echo", "Print text"),
            ("date", "date", "Display date"),
            ("hostname", "hostname", "Display hostname"),
            ("whoami", "whoami", "Display current user"),
            ("curl", "curl", "Transfer data from URLs"),
            ("wget", "wget", "Download files"),
            ("tar", "tar", "Archive files"),
            ("zip", "zip", "Compress files"),
            ("unzip", "unzip", "Extract compressed files"),
            ("find", "dir /s", "Find files"),
            ("head", "more", "Display first lines"),
            ("tail", "more", "Display last lines"),
            ("diff", "fc", "Compare files"),
        ];

        for (unix_cmd, win_cmd, desc) in mappings {
            self.command_map.insert(
                unix_cmd.to_string(),
                CommandMapping {
                    windows: win_cmd.to_string(),
                    unix: unix_cmd.to_string(),
                    description: desc.to_string(),
                },
            );
            // Also add reverse mapping
            self.command_map.insert(
                win_cmd.to_string(),
                CommandMapping {
                    windows: win_cmd.to_string(),
                    unix: unix_cmd.to_string(),
                    description: desc.to_string(),
                },
            );
        }
    }

    /// Translate a command from source to target platform
    pub fn translate_command(&self, command: &str) -> CommandResult<String> {
        // If source and target are the same, no translation needed
        if self.source_platform == self.target_platform {
            return Ok(command.to_string());
        }

        // Look up the command in our mapping
        if let Some(mapping) = self.command_map.get(command) {
            let translated = if self.target_platform.is_windows() {
                &mapping.windows
            } else {
                &mapping.unix
            };
            return Ok(translated.clone());
        }

        // If no mapping found, return original command
        // (it might be a platform-agnostic command or a custom script)
        Ok(command.to_string())
    }

    /// Translate command arguments (e.g., path separators)
    pub fn translate_arguments(&self, args: &[String]) -> Vec<String> {
        args.iter()
            .map(|arg| self.translate_path_in_string(arg))
            .collect()
    }

    /// Translate path separators in a string
    fn translate_path_in_string(&self, s: &str) -> String {
        if self.source_platform == self.target_platform {
            return s.to_string();
        }

        let source_sep = self.source_platform.path_separator();
        let target_sep = self.target_platform.path_separator();

        s.replace(source_sep, &target_sep.to_string())
    }

    /// Normalize a path for the target platform
    pub fn normalize_path(&self, path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        let normalized = self.translate_path_in_string(&path_str);
        PathBuf::from(normalized)
    }

    /// Translate environment variable syntax
    pub fn translate_env_var(&self, var_name: &str) -> String {
        match (self.source_platform.is_windows(), self.target_platform.is_windows()) {
            (true, false) => {
                // Windows to Unix: %VAR% -> $VAR
                if var_name.starts_with('%') && var_name.ends_with('%') {
                    format!("${}", &var_name[1..var_name.len() - 1])
                } else {
                    format!("${}", var_name)
                }
            }
            (false, true) => {
                // Unix to Windows: $VAR -> %VAR%
                if var_name.starts_with('$') {
                    format!("%{}%", &var_name[1..])
                } else {
                    format!("%{}%", var_name)
                }
            }
            _ => var_name.to_string(),
        }
    }

    /// Check if a command needs translation
    pub fn needs_translation(&self, command: &str) -> bool {
        if self.source_platform == self.target_platform {
            return false;
        }
        self.command_map.contains_key(command)
    }

    /// Get available command mappings
    pub fn get_mappings(&self) -> Vec<(String, String, String)> {
        self.command_map
            .values()
            .map(|m| {
                if self.target_platform.is_windows() {
                    (m.unix.clone(), m.windows.clone(), m.description.clone())
                } else {
                    (m.windows.clone(), m.unix.clone(), m.description.clone())
                }
            })
            .collect()
    }

    /// Detect platform from command syntax
    pub fn detect_platform_from_command(command: &str) -> Option<Platform> {
        // Check for Windows-specific indicators
        if command.contains("\\") && !command.contains("/") {
            return Some(Platform::Windows);
        }
        if command.contains("%") && command.matches('%').count() >= 2 {
            return Some(Platform::Windows);
        }
        if command.starts_with("dir ") || command.starts_with("type ") || command.starts_with("del ") {
            return Some(Platform::Windows);
        }

        // Check for Unix-specific indicators
        if command.contains("$") && !command.contains("%") {
            return Some(Platform::Unix);
        }
        if command.starts_with("ls ") || command.starts_with("cat ") || command.starts_with("rm ") {
            return Some(Platform::Unix);
        }

        None
    }

    /// Translate a full command line (command + arguments)
    pub fn translate_command_line(&self, command: &str, args: &[String]) -> CommandResult<(String, Vec<String>)> {
        let translated_cmd = self.translate_command(command)?;
        let translated_args = self.translate_arguments(args);
        Ok((translated_cmd, translated_args))
    }
}

impl Default for CommandTranslator {
    fn default() -> Self {
        Self::for_current_platform()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let current = Platform::current();
        #[cfg(target_os = "windows")]
        assert_eq!(current, Platform::Windows);
        
        #[cfg(target_os = "macos")]
        assert_eq!(current, Platform::MacOS);
        
        #[cfg(target_os = "linux")]
        assert_eq!(current, Platform::Linux);
    }

    #[test]
    fn test_command_translation_unix_to_windows() {
        let translator = CommandTranslator::new(Platform::Linux, Platform::Windows);
        
        assert_eq!(translator.translate_command("ls").unwrap(), "dir");
        assert_eq!(translator.translate_command("cat").unwrap(), "type");
        assert_eq!(translator.translate_command("rm").unwrap(), "del");
        assert_eq!(translator.translate_command("clear").unwrap(), "cls");
    }

    #[test]
    fn test_command_translation_windows_to_unix() {
        let translator = CommandTranslator::new(Platform::Windows, Platform::Linux);
        
        assert_eq!(translator.translate_command("dir").unwrap(), "ls");
        assert_eq!(translator.translate_command("type").unwrap(), "cat");
        assert_eq!(translator.translate_command("del").unwrap(), "rm");
        assert_eq!(translator.translate_command("cls").unwrap(), "clear");
    }

    #[test]
    fn test_no_translation_same_platform() {
        let translator = CommandTranslator::new(Platform::Linux, Platform::Linux);
        
        assert_eq!(translator.translate_command("ls").unwrap(), "ls");
        assert_eq!(translator.translate_command("cat").unwrap(), "cat");
    }

    #[test]
    fn test_path_separator() {
        assert_eq!(Platform::Windows.path_separator(), '\\');
        assert_eq!(Platform::Linux.path_separator(), '/');
        assert_eq!(Platform::MacOS.path_separator(), '/');
    }

    #[test]
    fn test_env_path_separator() {
        assert_eq!(Platform::Windows.env_path_separator(), ';');
        assert_eq!(Platform::Linux.env_path_separator(), ':');
    }

    #[test]
    fn test_path_translation() {
        let translator = CommandTranslator::new(Platform::Windows, Platform::Linux);
        let path = "C:\\Users\\test\\file.txt";
        let translated = translator.translate_path_in_string(path);
        assert_eq!(translated, "C:/Users/test/file.txt");
    }

    #[test]
    fn test_env_var_translation_unix_to_windows() {
        let translator = CommandTranslator::new(Platform::Linux, Platform::Windows);
        assert_eq!(translator.translate_env_var("$HOME"), "%HOME%");
        assert_eq!(translator.translate_env_var("$PATH"), "%PATH%");
    }

    #[test]
    fn test_env_var_translation_windows_to_unix() {
        let translator = CommandTranslator::new(Platform::Windows, Platform::Linux);
        assert_eq!(translator.translate_env_var("%USERPROFILE%"), "$USERPROFILE");
        assert_eq!(translator.translate_env_var("%PATH%"), "$PATH");
    }

    #[test]
    fn test_needs_translation() {
        let translator = CommandTranslator::new(Platform::Linux, Platform::Windows);
        assert!(translator.needs_translation("ls"));
        assert!(translator.needs_translation("cat"));
        assert!(!translator.needs_translation("unknown_command"));
    }

    #[test]
    fn test_detect_platform_from_command() {
        assert_eq!(
            CommandTranslator::detect_platform_from_command("dir /s"),
            Some(Platform::Windows)
        );
        assert_eq!(
            CommandTranslator::detect_platform_from_command("ls -la"),
            Some(Platform::Unix)
        );
        assert_eq!(
            CommandTranslator::detect_platform_from_command("C:\\Users\\test"),
            Some(Platform::Windows)
        );
        assert_eq!(
            CommandTranslator::detect_platform_from_command("echo $HOME"),
            Some(Platform::Unix)
        );
    }

    #[test]
    fn test_translate_command_line() {
        let translator = CommandTranslator::new(Platform::Linux, Platform::Windows);
        let (cmd, args) = translator.translate_command_line(
            "ls",
            &vec!["-la".to_string(), "/home/user".to_string()]
        ).unwrap();
        
        assert_eq!(cmd, "dir");
        // Arguments should have path separators translated
        assert!(args[1].contains("\\") || args[1] == "/home/user");
    }

    #[test]
    fn test_argument_translation() {
        let translator = CommandTranslator::new(Platform::Windows, Platform::Linux);
        let args = vec![
            "C:\\Users\\test".to_string(),
            "file.txt".to_string(),
        ];
        let translated = translator.translate_arguments(&args);
        assert_eq!(translated[0], "C:/Users/test");
        assert_eq!(translated[1], "file.txt");
    }
}
