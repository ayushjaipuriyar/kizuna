// Command history management module

use crate::cli::error::{CLIError, CLIResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

/// Maximum number of history entries to keep
const MAX_HISTORY_ENTRIES: usize = 1000;

/// Command history entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: DateTime<Utc>,
    pub exit_code: Option<i32>,
    pub duration_ms: Option<u64>,
}

impl HistoryEntry {
    /// Create a new history entry
    pub fn new(command: String) -> Self {
        Self {
            command,
            timestamp: Utc::now(),
            exit_code: None,
            duration_ms: None,
        }
    }

    /// Create a history entry with execution details
    pub fn with_details(command: String, exit_code: i32, duration_ms: u64) -> Self {
        Self {
            command,
            timestamp: Utc::now(),
            exit_code: Some(exit_code),
            duration_ms: Some(duration_ms),
        }
    }

    /// Format the entry for display
    pub fn format(&self) -> String {
        let time = self.timestamp.format("%Y-%m-%d %H:%M:%S");
        if let Some(exit_code) = self.exit_code {
            format!("[{}] {} (exit: {})", time, self.command, exit_code)
        } else {
            format!("[{}] {}", time, self.command)
        }
    }
}

/// Command history manager
pub struct HistoryManager {
    history_file: PathBuf,
    entries: Vec<HistoryEntry>,
}

impl HistoryManager {
    /// Create a new history manager
    pub fn new() -> CLIResult<Self> {
        let history_file = Self::get_history_file_path()?;
        let entries = Self::load_history(&history_file)?;

        Ok(Self {
            history_file,
            entries,
        })
    }

    /// Get the history file path
    fn get_history_file_path() -> CLIResult<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| CLIError::other("Could not determine config directory"))?;

        let kizuna_dir = config_dir.join("kizuna");
        fs::create_dir_all(&kizuna_dir)
            .map_err(|e| CLIError::other(format!("Failed to create config directory: {}", e)))?;

        Ok(kizuna_dir.join("history"))
    }

    /// Load history from file
    fn load_history(path: &Path) -> CLIResult<Vec<HistoryEntry>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)
            .map_err(|e| CLIError::other(format!("Failed to open history file: {}", e)))?;

        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line.map_err(|e| CLIError::other(format!("Failed to read line: {}", e)))?;

            if let Ok(entry) = serde_json::from_str::<HistoryEntry>(&line) {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Save history to file
    fn save_history(&self) -> CLIResult<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.history_file)
            .map_err(|e| CLIError::other(format!("Failed to open history file: {}", e)))?;

        // Only save the most recent entries
        let start_index = if self.entries.len() > MAX_HISTORY_ENTRIES {
            self.entries.len() - MAX_HISTORY_ENTRIES
        } else {
            0
        };

        for entry in &self.entries[start_index..] {
            let json = serde_json::to_string(entry)
                .map_err(|e| CLIError::other(format!("Failed to serialize entry: {}", e)))?;

            writeln!(file, "{}", json)
                .map_err(|e| CLIError::other(format!("Failed to write entry: {}", e)))?;
        }

        Ok(())
    }

    /// Add a command to history
    pub fn add(&mut self, command: String) -> CLIResult<()> {
        let entry = HistoryEntry::new(command);
        self.entries.push(entry);
        self.save_history()
    }

    /// Add a command with execution details
    pub fn add_with_details(
        &mut self,
        command: String,
        exit_code: i32,
        duration_ms: u64,
    ) -> CLIResult<()> {
        let entry = HistoryEntry::with_details(command, exit_code, duration_ms);
        self.entries.push(entry);
        self.save_history()
    }

    /// Get all history entries
    pub fn get_all(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Get the most recent N entries
    pub fn get_recent(&self, count: usize) -> &[HistoryEntry] {
        let start = if self.entries.len() > count {
            self.entries.len() - count
        } else {
            0
        };
        &self.entries[start..]
    }

    /// Search history with fuzzy matching
    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        if query.is_empty() {
            return self.entries.iter().collect();
        }

        let query_lower = query.to_lowercase();
        let mut results: Vec<(&HistoryEntry, usize)> = self
            .entries
            .iter()
            .filter_map(|entry| {
                let command_lower = entry.command.to_lowercase();

                // Exact match gets highest score
                if command_lower == query_lower {
                    return Some((entry, 0));
                }

                // Contains match
                if command_lower.contains(&query_lower) {
                    return Some((entry, 1));
                }

                // Fuzzy match using Levenshtein distance
                let distance = levenshtein_distance(&command_lower, &query_lower);
                if distance <= 3 {
                    return Some((entry, distance + 2));
                }

                None
            })
            .collect();

        // Sort by score (lower is better)
        results.sort_by_key(|(_, score)| *score);

        results.into_iter().map(|(entry, _)| entry).collect()
    }

    /// Get command suggestions based on partial input
    pub fn suggest(&self, partial: &str) -> Vec<String> {
        if partial.is_empty() {
            return Vec::new();
        }

        let partial_lower = partial.to_lowercase();
        let mut suggestions: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| entry.command.to_lowercase().starts_with(&partial_lower))
            .map(|entry| entry.command.clone())
            .collect();

        // Remove duplicates while preserving order (most recent first)
        suggestions.reverse();
        suggestions.dedup();
        suggestions.reverse();

        suggestions
    }

    /// Clear all history
    pub fn clear(&mut self) -> CLIResult<()> {
        self.entries.clear();
        self.save_history()
    }

    /// Remove entries older than the specified number of days
    pub fn prune_old(&mut self, days: i64) -> CLIResult<()> {
        let cutoff = Utc::now() - chrono::Duration::days(days);
        self.entries.retain(|entry| entry.timestamp > cutoff);
        self.save_history()
    }

    /// Get statistics about command usage
    pub fn get_statistics(&self) -> HistoryStatistics {
        let mut command_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for entry in &self.entries {
            // Extract the base command (first word)
            let base_command = entry
                .command
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();

            *command_counts.entry(base_command).or_insert(0) += 1;
        }

        let mut most_used: Vec<(String, usize)> = command_counts.into_iter().collect();
        most_used.sort_by(|a, b| b.1.cmp(&a.1));

        HistoryStatistics {
            total_commands: self.entries.len(),
            most_used_commands: most_used.into_iter().take(10).collect(),
        }
    }
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            history_file: PathBuf::new(),
            entries: Vec::new(),
        })
    }
}

/// History statistics
#[derive(Debug, Clone)]
pub struct HistoryStatistics {
    pub total_commands: usize,
    pub most_used_commands: Vec<(String, usize)>,
}

/// Calculate Levenshtein distance between two strings
fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let len1 = s1.len();
    let len2 = s2.len();

    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }

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
    fn test_history_entry_creation() {
        let entry = HistoryEntry::new("kizuna discover".to_string());
        assert_eq!(entry.command, "kizuna discover");
        assert!(entry.exit_code.is_none());
        assert!(entry.duration_ms.is_none());

        let entry_with_details =
            HistoryEntry::with_details("kizuna send file.txt".to_string(), 0, 1500);
        assert_eq!(entry_with_details.command, "kizuna send file.txt");
        assert_eq!(entry_with_details.exit_code, Some(0));
        assert_eq!(entry_with_details.duration_ms, Some(1500));
    }

    #[test]
    fn test_history_entry_format() {
        let entry = HistoryEntry::new("kizuna discover".to_string());
        let formatted = entry.format();
        assert!(formatted.contains("kizuna discover"));

        let entry_with_details =
            HistoryEntry::with_details("kizuna send file.txt".to_string(), 0, 1500);
        let formatted = entry_with_details.format();
        assert!(formatted.contains("kizuna send file.txt"));
        assert!(formatted.contains("exit: 0"));
    }

    #[test]
    fn test_levenshtein_distance() {
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
        assert_eq!(levenshtein_distance("abc", "ab"), 1);
        assert_eq!(levenshtein_distance("abc", "def"), 3);
        assert_eq!(levenshtein_distance("discover", "discver"), 1);
    }

    #[test]
    fn test_history_search() {
        let manager = HistoryManager {
            history_file: PathBuf::new(),
            entries: vec![
                HistoryEntry::new("kizuna discover".to_string()),
                HistoryEntry::new("kizuna send file.txt".to_string()),
                HistoryEntry::new("kizuna receive".to_string()),
                HistoryEntry::new("kizuna discover --type desktop".to_string()),
            ],
        };

        let results = manager.search("discover");
        assert_eq!(results.len(), 2);

        let results = manager.search("send");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].command, "kizuna send file.txt");
    }

    #[test]
    fn test_history_suggestions() {
        let manager = HistoryManager {
            history_file: PathBuf::new(),
            entries: vec![
                HistoryEntry::new("kizuna discover".to_string()),
                HistoryEntry::new("kizuna send file.txt".to_string()),
                HistoryEntry::new("kizuna discover --watch".to_string()),
            ],
        };

        let suggestions = manager.suggest("kizuna d");
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions.iter().all(|s| s.starts_with("kizuna d")));

        let suggestions = manager.suggest("kizuna s");
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0], "kizuna send file.txt");
    }

    #[test]
    fn test_get_recent() {
        let manager = HistoryManager {
            history_file: PathBuf::new(),
            entries: vec![
                HistoryEntry::new("command1".to_string()),
                HistoryEntry::new("command2".to_string()),
                HistoryEntry::new("command3".to_string()),
                HistoryEntry::new("command4".to_string()),
            ],
        };

        let recent = manager.get_recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].command, "command3");
        assert_eq!(recent[1].command, "command4");
    }

    #[test]
    fn test_statistics() {
        let manager = HistoryManager {
            history_file: PathBuf::new(),
            entries: vec![
                HistoryEntry::new("kizuna discover".to_string()),
                HistoryEntry::new("kizuna send file1.txt".to_string()),
                HistoryEntry::new("kizuna discover --watch".to_string()),
                HistoryEntry::new("kizuna send file2.txt".to_string()),
                HistoryEntry::new("kizuna discover".to_string()),
            ],
        };

        let stats = manager.get_statistics();
        assert_eq!(stats.total_commands, 5);
        assert_eq!(stats.most_used_commands[0].0, "kizuna");
        assert_eq!(stats.most_used_commands[0].1, 5);
    }
}
