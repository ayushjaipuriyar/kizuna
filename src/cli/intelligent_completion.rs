// Intelligent completion system with context-aware suggestions

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::history::HistoryManager;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Completion context information
#[derive(Debug, Clone)]
pub struct CompletionContext {
    pub command: String,
    pub subcommand: Option<String>,
    pub current_arg: String,
    pub previous_args: Vec<String>,
    pub cursor_position: usize,
}

impl CompletionContext {
    /// Create a new completion context
    pub fn new(input: &str, cursor_position: usize) -> Self {
        let parts: Vec<String> = input.split_whitespace().map(|s| s.to_string()).collect();

        let command = parts.first().cloned().unwrap_or_default();
        let subcommand = if parts.len() > 1 {
            Some(parts[1].clone())
        } else {
            None
        };

        let current_arg = parts.last().cloned().unwrap_or_default();
        let previous_args = if parts.len() > 1 {
            parts[..parts.len() - 1].to_vec()
        } else {
            Vec::new()
        };

        Self {
            command,
            subcommand,
            current_arg,
            previous_args,
            cursor_position,
        }
    }

    /// Check if we're completing a command
    pub fn is_completing_command(&self) -> bool {
        self.previous_args.is_empty() || self.command.is_empty()
    }

    /// Check if we're completing a subcommand
    pub fn is_completing_subcommand(&self) -> bool {
        !self.command.is_empty() && self.previous_args.len() == 1 && self.subcommand.is_none()
    }

    /// Check if we're completing an option
    pub fn is_completing_option(&self) -> bool {
        self.current_arg.starts_with('-')
    }

    /// Check if we're completing a file path
    pub fn is_completing_path(&self) -> bool {
        self.current_arg.contains('/') || self.current_arg.contains('\\')
    }
}

/// Completion suggestion
#[derive(Debug, Clone)]
pub struct Completion {
    pub value: String,
    pub description: Option<String>,
    pub score: usize,
}

impl Completion {
    /// Create a new completion
    pub fn new(value: String) -> Self {
        Self {
            value,
            description: None,
            score: 0,
        }
    }

    /// Create a completion with description
    pub fn with_description(value: String, description: String) -> Self {
        Self {
            value,
            description: Some(description),
            score: 0,
        }
    }

    /// Create a completion with score
    pub fn with_score(value: String, score: usize) -> Self {
        Self {
            value,
            description: None,
            score,
        }
    }
}

/// Completion cache entry
struct CacheEntry {
    completions: Vec<Completion>,
    timestamp: Instant,
}

/// Intelligent completion system
pub struct IntelligentCompletion {
    history_manager: Arc<RwLock<HistoryManager>>,
    peer_cache: Arc<RwLock<Vec<String>>>,
    completion_cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    cache_ttl: Duration,
}

impl IntelligentCompletion {
    /// Create a new intelligent completion system
    pub fn new(history_manager: HistoryManager) -> Self {
        Self {
            history_manager: Arc::new(RwLock::new(history_manager)),
            peer_cache: Arc::new(RwLock::new(Vec::new())),
            completion_cache: Arc::new(RwLock::new(HashMap::new())),
            cache_ttl: Duration::from_secs(60),
        }
    }

    /// Update the peer cache
    pub fn update_peer_cache(&self, peers: Vec<String>) {
        if let Ok(mut cache) = self.peer_cache.write() {
            *cache = peers;
        }
    }

    /// Get completions for the given context
    pub fn complete(&self, context: &CompletionContext) -> CLIResult<Vec<Completion>> {
        // Check cache first
        let cache_key = format!("{:?}", context);
        if let Some(cached) = self.get_cached_completions(&cache_key) {
            return Ok(cached);
        }

        let completions = if context.is_completing_command() {
            self.complete_command(&context.current_arg)
        } else if context.is_completing_subcommand() {
            self.complete_subcommand(&context.command, &context.current_arg)
        } else if context.is_completing_option() {
            self.complete_option(&context.command, &context.current_arg)
        } else if context.is_completing_path() {
            self.complete_path(&context.current_arg)
        } else if self.is_peer_argument(context) {
            self.complete_peer(&context.current_arg)
        } else {
            self.complete_from_history(context)
        }?;

        // Cache the results
        self.cache_completions(cache_key, completions.clone());

        Ok(completions)
    }

    /// Complete command names
    fn complete_command(&self, partial: &str) -> CLIResult<Vec<Completion>> {
        let commands = vec![
            ("discover", "Discover available peers"),
            ("send", "Send files to a peer"),
            ("receive", "Receive incoming file transfers"),
            ("stream", "Manage media streaming"),
            ("exec", "Execute command on remote peer"),
            ("peers", "List connected peers"),
            ("status", "Show system status"),
            ("clipboard", "Manage clipboard sharing"),
            ("tui", "Launch interactive TUI"),
            ("config", "Manage configuration"),
            ("completion", "Generate shell completion scripts"),
        ];

        let mut completions: Vec<Completion> = commands
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(partial))
            .map(|(cmd, desc)| Completion::with_description(cmd.to_string(), desc.to_string()))
            .collect();

        // Add fuzzy matches
        if completions.is_empty() {
            completions = commands
                .iter()
                .filter_map(|(cmd, desc)| {
                    let distance = fuzzy_match(cmd, partial);
                    if distance <= 2 {
                        Some(Completion {
                            value: cmd.to_string(),
                            description: Some(desc.to_string()),
                            score: distance,
                        })
                    } else {
                        None
                    }
                })
                .collect();
        }

        completions.sort_by_key(|c| c.score);
        Ok(completions)
    }

    /// Complete subcommand names
    fn complete_subcommand(&self, command: &str, partial: &str) -> CLIResult<Vec<Completion>> {
        let subcommands = match command {
            "stream" => vec![("camera", "Stream camera feed")],
            "clipboard" => vec![
                ("share", "Toggle clipboard sharing"),
                ("status", "Show clipboard status"),
                ("history", "View clipboard history"),
            ],
            "config" => vec![
                ("get", "Get configuration value"),
                ("set", "Set configuration value"),
                ("list", "List all configuration"),
            ],
            _ => vec![],
        };

        let completions: Vec<Completion> = subcommands
            .iter()
            .filter(|(cmd, _)| cmd.starts_with(partial))
            .map(|(cmd, desc)| Completion::with_description(cmd.to_string(), desc.to_string()))
            .collect();

        Ok(completions)
    }

    /// Complete option names
    fn complete_option(&self, command: &str, partial: &str) -> CLIResult<Vec<Completion>> {
        let options = match command {
            "discover" => vec![
                ("--type", "Filter by device type"),
                ("--name", "Filter by device name"),
                ("--timeout", "Discovery timeout"),
                ("--watch", "Continuously watch for peers"),
                ("--format", "Output format"),
            ],
            "send" => vec![
                ("--peer", "Target peer name or ID"),
                ("--no-compression", "Disable compression"),
                ("--no-encryption", "Disable encryption"),
                ("--verbose", "Show detailed progress"),
            ],
            "receive" => vec![
                ("--output", "Output directory"),
                ("--auto-accept", "Auto-accept from trusted peers"),
                ("--from", "Only accept from specific peer"),
            ],
            _ => vec![],
        };

        let completions: Vec<Completion> = options
            .iter()
            .filter(|(opt, _)| opt.starts_with(partial))
            .map(|(opt, desc)| Completion::with_description(opt.to_string(), desc.to_string()))
            .collect();

        Ok(completions)
    }

    /// Complete file paths
    fn complete_path(&self, partial: &str) -> CLIResult<Vec<Completion>> {
        let path = Path::new(partial);
        let (dir, prefix) = if partial.ends_with('/') || partial.ends_with('\\') {
            (path, "")
        } else {
            (
                path.parent().unwrap_or_else(|| Path::new(".")),
                path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(""),
            )
        };

        let mut completions = Vec::new();

        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(file_name) = entry.file_name().into_string() {
                    if file_name.starts_with(prefix) {
                        let mut path_str = if dir == Path::new(".") {
                            file_name.clone()
                        } else {
                            dir.join(&file_name)
                                .to_string_lossy()
                                .to_string()
                        };

                        // Add trailing slash for directories
                        if entry.path().is_dir() {
                            path_str.push('/');
                        }

                        completions.push(Completion::new(path_str));
                    }
                }
            }
        }

        Ok(completions)
    }

    /// Complete peer names
    fn complete_peer(&self, partial: &str) -> CLIResult<Vec<Completion>> {
        let peers = self.peer_cache.read().map_err(|_| {
            CLIError::other("Failed to read peer cache")
        })?;

        let completions: Vec<Completion> = peers
            .iter()
            .filter(|peer| peer.to_lowercase().starts_with(&partial.to_lowercase()))
            .map(|peer| Completion::new(peer.clone()))
            .collect();

        Ok(completions)
    }

    /// Complete from command history
    fn complete_from_history(&self, context: &CompletionContext) -> CLIResult<Vec<Completion>> {
        let history = self.history_manager.read().map_err(|_| {
            CLIError::other("Failed to read history")
        })?;

        let suggestions = history.suggest(&context.current_arg);

        let completions: Vec<Completion> = suggestions
            .into_iter()
            .map(|s| Completion::new(s))
            .collect();

        Ok(completions)
    }

    /// Check if the current argument position expects a peer name
    fn is_peer_argument(&self, context: &CompletionContext) -> bool {
        // Check if previous argument was --peer or -p
        if let Some(last_arg) = context.previous_args.last() {
            if last_arg == "--peer" || last_arg == "-p" {
                return true;
            }
        }

        // Check if command typically requires peer selection
        matches!(context.command.as_str(), "send" | "exec")
            && !context.current_arg.starts_with('-')
            && context.previous_args.len() > 1
    }

    /// Get cached completions if available and not expired
    fn get_cached_completions(&self, key: &str) -> Option<Vec<Completion>> {
        if let Ok(cache) = self.completion_cache.read() {
            if let Some(entry) = cache.get(key) {
                if entry.timestamp.elapsed() < self.cache_ttl {
                    return Some(entry.completions.clone());
                }
            }
        }
        None
    }

    /// Cache completions
    fn cache_completions(&self, key: String, completions: Vec<Completion>) {
        if let Ok(mut cache) = self.completion_cache.write() {
            cache.insert(
                key,
                CacheEntry {
                    completions,
                    timestamp: Instant::now(),
                },
            );

            // Clean up old entries
            cache.retain(|_, entry| entry.timestamp.elapsed() < self.cache_ttl * 2);
        }
    }

    /// Clear the completion cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.completion_cache.write() {
            cache.clear();
        }
    }
}

/// Calculate fuzzy match score (lower is better)
fn fuzzy_match(target: &str, query: &str) -> usize {
    if target == query {
        return 0;
    }

    if target.starts_with(query) {
        return 1;
    }

    // Simple Levenshtein distance
    let len1 = target.len();
    let len2 = query.len();

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

    for (i, c1) in target.chars().enumerate() {
        for (j, c2) in query.chars().enumerate() {
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
    fn test_completion_context() {
        let ctx = CompletionContext::new("kizuna discover", 16);
        assert_eq!(ctx.command, "kizuna");
        assert_eq!(ctx.subcommand, Some("discover".to_string()));
        assert!(ctx.is_completing_subcommand());

        let ctx = CompletionContext::new("kizuna", 6);
        assert!(ctx.is_completing_command());

        let ctx = CompletionContext::new("kizuna send --peer", 18);
        assert!(ctx.is_completing_option());
    }

    #[test]
    fn test_fuzzy_match() {
        assert_eq!(fuzzy_match("discover", "discover"), 0);
        assert_eq!(fuzzy_match("discover", "disc"), 1);
        assert_eq!(fuzzy_match("discover", "discver"), 1);
        assert!(fuzzy_match("discover", "xyz") > 3);
    }

    #[test]
    fn test_complete_command() {
        let history = HistoryManager::default();
        let completion = IntelligentCompletion::new(history);

        let completions = completion.complete_command("dis").unwrap();
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.value == "discover"));
    }

    #[test]
    fn test_complete_subcommand() {
        let history = HistoryManager::default();
        let completion = IntelligentCompletion::new(history);

        let completions = completion.complete_subcommand("clipboard", "sh").unwrap();
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.value == "share"));
    }

    #[test]
    fn test_complete_option() {
        let history = HistoryManager::default();
        let completion = IntelligentCompletion::new(history);

        let completions = completion.complete_option("discover", "--t").unwrap();
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.value == "--type"));
    }

    #[test]
    fn test_complete_peer() {
        let history = HistoryManager::default();
        let completion = IntelligentCompletion::new(history);

        completion.update_peer_cache(vec![
            "laptop".to_string(),
            "desktop".to_string(),
            "phone".to_string(),
        ]);

        let completions = completion.complete_peer("lap").unwrap();
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].value, "laptop");
    }
}
