// Advanced filtering and search for CLI
//
// Implements advanced peer filtering with multiple criteria,
// file search and filtering within TUI file browser, and
// operation history search and filtering.
//
// Requirements: 1.3

use crate::cli::error::{CLIError, CLIResult};
use crate::cli::types::{ConnectionStatus, OperationStatus, OperationType, PeerInfo, TrustStatus};
use chrono::{DateTime, Utc};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Peer filter criteria
#[derive(Debug, Clone, Default)]
pub struct PeerFilter {
    pub name_pattern: Option<String>,
    pub device_type: Option<String>,
    pub connection_status: Option<ConnectionStatus>,
    pub trust_status: Option<TrustStatus>,
    pub capabilities: Vec<String>,
    pub last_seen_after: Option<DateTime<Utc>>,
    pub last_seen_before: Option<DateTime<Utc>>,
}

impl PeerFilter {
    /// Create a new empty peer filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Set name pattern filter
    pub fn with_name_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.name_pattern = Some(pattern.into());
        self
    }

    /// Set device type filter
    pub fn with_device_type(mut self, device_type: impl Into<String>) -> Self {
        self.device_type = Some(device_type.into());
        self
    }

    /// Set connection status filter
    pub fn with_connection_status(mut self, status: ConnectionStatus) -> Self {
        self.connection_status = Some(status);
        self
    }

    /// Set trust status filter
    pub fn with_trust_status(mut self, status: TrustStatus) -> Self {
        self.trust_status = Some(status);
        self
    }

    /// Add capability filter
    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.capabilities.push(capability.into());
        self
    }

    /// Set last seen after filter
    pub fn with_last_seen_after(mut self, time: DateTime<Utc>) -> Self {
        self.last_seen_after = Some(time);
        self
    }

    /// Set last seen before filter
    pub fn with_last_seen_before(mut self, time: DateTime<Utc>) -> Self {
        self.last_seen_before = Some(time);
        self
    }

    /// Apply filter to a peer
    pub fn matches(&self, peer: &PeerInfo) -> bool {
        // Check name pattern
        if let Some(pattern) = &self.name_pattern {
            if !peer.name.to_lowercase().contains(&pattern.to_lowercase()) {
                return false;
            }
        }

        // Check device type
        if let Some(device_type) = &self.device_type {
            if !peer
                .device_type
                .to_lowercase()
                .contains(&device_type.to_lowercase())
            {
                return false;
            }
        }

        // Check connection status
        if let Some(status) = &self.connection_status {
            if peer.connection_status != *status {
                return false;
            }
        }

        // Check trust status
        if let Some(status) = &self.trust_status {
            if peer.trust_status != *status {
                return false;
            }
        }

        // Check capabilities
        if !self.capabilities.is_empty() {
            let has_all_capabilities = self
                .capabilities
                .iter()
                .all(|cap| peer.capabilities.contains(cap));
            if !has_all_capabilities {
                return false;
            }
        }

        // Check last seen time range
        if let Some(last_seen) = peer.last_seen {
            if let Some(after) = self.last_seen_after {
                if last_seen < after {
                    return false;
                }
            }
            if let Some(before) = self.last_seen_before {
                if last_seen > before {
                    return false;
                }
            }
        }

        true
    }

    /// Filter a list of peers
    pub fn filter_peers(&self, peers: &[PeerInfo]) -> Vec<PeerInfo> {
        peers
            .iter()
            .filter(|peer| self.matches(peer))
            .cloned()
            .collect()
    }
}

/// File filter criteria for TUI file browser
#[derive(Debug, Clone, Default)]
pub struct FileFilter {
    pub name_pattern: Option<String>,
    pub extension: Option<String>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub modified_after: Option<DateTime<Utc>>,
    pub modified_before: Option<DateTime<Utc>>,
    pub is_directory: Option<bool>,
}

impl FileFilter {
    /// Create a new empty file filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Set name pattern filter
    pub fn with_name_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.name_pattern = Some(pattern.into());
        self
    }

    /// Set extension filter
    pub fn with_extension(mut self, extension: impl Into<String>) -> Self {
        self.extension = Some(extension.into());
        self
    }

    /// Set minimum size filter
    pub fn with_min_size(mut self, size: u64) -> Self {
        self.min_size = Some(size);
        self
    }

    /// Set maximum size filter
    pub fn with_max_size(mut self, size: u64) -> Self {
        self.max_size = Some(size);
        self
    }

    /// Set modified after filter
    pub fn with_modified_after(mut self, time: DateTime<Utc>) -> Self {
        self.modified_after = Some(time);
        self
    }

    /// Set modified before filter
    pub fn with_modified_before(mut self, time: DateTime<Utc>) -> Self {
        self.modified_before = Some(time);
        self
    }

    /// Set directory filter
    pub fn with_is_directory(mut self, is_dir: bool) -> Self {
        self.is_directory = Some(is_dir);
        self
    }

    /// Apply filter to a file entry
    pub fn matches(&self, entry: &FileEntry) -> bool {
        // Check name pattern
        if let Some(pattern) = &self.name_pattern {
            if !entry
                .name
                .to_lowercase()
                .contains(&pattern.to_lowercase())
            {
                return false;
            }
        }

        // Check extension
        if let Some(ext) = &self.extension {
            if let Some(file_ext) = entry.path.extension() {
                if file_ext.to_string_lossy().to_lowercase() != ext.to_lowercase() {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Check size range
        if let Some(min_size) = self.min_size {
            if entry.size < min_size {
                return false;
            }
        }
        if let Some(max_size) = self.max_size {
            if entry.size > max_size {
                return false;
            }
        }

        // Check modified time range
        if let Some(after) = self.modified_after {
            if entry.modified < after {
                return false;
            }
        }
        if let Some(before) = self.modified_before {
            if entry.modified > before {
                return false;
            }
        }

        // Check directory flag
        if let Some(is_dir) = self.is_directory {
            if entry.is_directory != is_dir {
                return false;
            }
        }

        true
    }

    /// Filter a list of file entries
    pub fn filter_files(&self, entries: &[FileEntry]) -> Vec<FileEntry> {
        entries
            .iter()
            .filter(|entry| self.matches(entry))
            .cloned()
            .collect()
    }
}

/// File entry structure for filtering
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub modified: DateTime<Utc>,
    pub is_directory: bool,
}

impl FileEntry {
    /// Create a file entry from a path
    pub fn from_path(path: impl AsRef<Path>) -> CLIResult<Self> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path)
            .map_err(|e| CLIError::IOError(e))?;

        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        let modified = metadata
            .modified()
            .map_err(|e| CLIError::IOError(e))?;
        let modified = DateTime::<Utc>::from(modified);

        Ok(Self {
            path: path.to_path_buf(),
            name,
            size: metadata.len(),
            modified,
            is_directory: metadata.is_dir(),
        })
    }
}

/// Operation history filter criteria
#[derive(Debug, Clone, Default)]
pub struct OperationFilter {
    pub operation_type: Option<OperationType>,
    pub peer_id: Option<Uuid>,
    pub started_after: Option<DateTime<Utc>>,
    pub started_before: Option<DateTime<Utc>>,
    pub status_pattern: Option<String>,
}

impl OperationFilter {
    /// Create a new empty operation filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Set operation type filter
    pub fn with_operation_type(mut self, op_type: OperationType) -> Self {
        self.operation_type = Some(op_type);
        self
    }

    /// Set peer ID filter
    pub fn with_peer_id(mut self, peer_id: Uuid) -> Self {
        self.peer_id = Some(peer_id);
        self
    }

    /// Set started after filter
    pub fn with_started_after(mut self, time: DateTime<Utc>) -> Self {
        self.started_after = Some(time);
        self
    }

    /// Set started before filter
    pub fn with_started_before(mut self, time: DateTime<Utc>) -> Self {
        self.started_before = Some(time);
        self
    }

    /// Set status pattern filter
    pub fn with_status_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.status_pattern = Some(pattern.into());
        self
    }

    /// Apply filter to an operation
    pub fn matches(&self, operation: &OperationStatus) -> bool {
        // Check operation type
        if let Some(op_type) = &self.operation_type {
            if operation.operation_type != *op_type {
                return false;
            }
        }

        // Check peer ID
        if let Some(peer_id) = &self.peer_id {
            if operation.peer_id != *peer_id {
                return false;
            }
        }

        // Check started time range
        if let Some(after) = self.started_after {
            if operation.started_at < after {
                return false;
            }
        }
        if let Some(before) = self.started_before {
            if operation.started_at > before {
                return false;
            }
        }

        // Check status pattern
        if let Some(pattern) = &self.status_pattern {
            let status_str = format!("{:?}", operation.status);
            if !status_str.to_lowercase().contains(&pattern.to_lowercase()) {
                return false;
            }
        }

        true
    }

    /// Filter a list of operations
    pub fn filter_operations(&self, operations: &[OperationStatus]) -> Vec<OperationStatus> {
        operations
            .iter()
            .filter(|op| self.matches(op))
            .cloned()
            .collect()
    }
}

/// Search engine for fuzzy matching
pub struct SearchEngine {
    case_sensitive: bool,
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new(case_sensitive: bool) -> Self {
        Self { case_sensitive }
    }

    /// Perform fuzzy search on a list of strings
    pub fn fuzzy_search(&self, query: &str, items: &[String]) -> Vec<SearchMatch> {
        let query = if self.case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        let mut matches = Vec::new();

        for (idx, item) in items.iter().enumerate() {
            let item_str = if self.case_sensitive {
                item.clone()
            } else {
                item.to_lowercase()
            };

            if let Some(score) = self.calculate_match_score(&query, &item_str) {
                matches.push(SearchMatch {
                    index: idx,
                    item: item.clone(),
                    score,
                });
            }
        }

        // Sort by score (higher is better)
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        matches
    }

    /// Calculate match score for fuzzy matching
    fn calculate_match_score(&self, query: &str, item: &str) -> Option<f64> {
        // Exact match gets highest score
        if item == query {
            return Some(100.0);
        }

        // Contains match gets high score
        if item.contains(query) {
            let ratio = query.len() as f64 / item.len() as f64;
            return Some(80.0 * ratio);
        }

        // Subsequence match gets medium score
        if self.is_subsequence(query, item) {
            let ratio = query.len() as f64 / item.len() as f64;
            return Some(60.0 * ratio);
        }

        // Levenshtein distance for fuzzy matching
        let distance = self.levenshtein_distance(query, item);
        let max_len = query.len().max(item.len());

        if distance as f64 / max_len as f64 <= 0.3 {
            // Allow up to 30% difference
            let similarity = 1.0 - (distance as f64 / max_len as f64);
            return Some(40.0 * similarity);
        }

        None
    }

    /// Check if query is a subsequence of item
    fn is_subsequence(&self, query: &str, item: &str) -> bool {
        let mut query_chars = query.chars();
        let mut current_char = query_chars.next();

        for item_char in item.chars() {
            if let Some(qc) = current_char {
                if qc == item_char {
                    current_char = query_chars.next();
                }
            } else {
                return true;
            }
        }

        current_char.is_none()
    }

    /// Calculate Levenshtein distance between two strings
    fn levenshtein_distance(&self, s1: &str, s2: &str) -> usize {
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

        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1_chars[i - 1] == s2_chars[j - 1] {
                    0
                } else {
                    1
                };

                matrix[i][j] = (matrix[i - 1][j] + 1)
                    .min(matrix[i][j - 1] + 1)
                    .min(matrix[i - 1][j - 1] + cost);
            }
        }

        matrix[len1][len2]
    }
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new(false)
    }
}

/// Search match result
#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub index: usize,
    pub item: String,
    pub score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_filter_name_pattern() {
        let filter = PeerFilter::new().with_name_pattern("laptop");

        let peer1 = PeerInfo {
            id: Uuid::new_v4(),
            name: "my-laptop".to_string(),
            device_type: "laptop".to_string(),
            connection_status: ConnectionStatus::Connected,
            capabilities: vec![],
            trust_status: TrustStatus::Trusted,
            last_seen: None,
        };

        let peer2 = PeerInfo {
            id: Uuid::new_v4(),
            name: "desktop-pc".to_string(),
            device_type: "desktop".to_string(),
            connection_status: ConnectionStatus::Connected,
            capabilities: vec![],
            trust_status: TrustStatus::Trusted,
            last_seen: None,
        };

        assert!(filter.matches(&peer1));
        assert!(!filter.matches(&peer2));
    }

    #[test]
    fn test_peer_filter_multiple_criteria() {
        let filter = PeerFilter::new()
            .with_device_type("laptop")
            .with_connection_status(ConnectionStatus::Connected)
            .with_trust_status(TrustStatus::Trusted);

        let peer = PeerInfo {
            id: Uuid::new_v4(),
            name: "test-laptop".to_string(),
            device_type: "laptop".to_string(),
            connection_status: ConnectionStatus::Connected,
            capabilities: vec![],
            trust_status: TrustStatus::Trusted,
            last_seen: None,
        };

        assert!(filter.matches(&peer));
    }

    #[test]
    fn test_file_filter_extension() {
        let filter = FileFilter::new().with_extension("txt");

        let entry = FileEntry {
            path: PathBuf::from("/path/to/file.txt"),
            name: "file.txt".to_string(),
            size: 1024,
            modified: Utc::now(),
            is_directory: false,
        };

        assert!(filter.matches(&entry));
    }

    #[test]
    fn test_file_filter_size_range() {
        let filter = FileFilter::new().with_min_size(100).with_max_size(2000);

        let entry1 = FileEntry {
            path: PathBuf::from("/path/to/file1.txt"),
            name: "file1.txt".to_string(),
            size: 500,
            modified: Utc::now(),
            is_directory: false,
        };

        let entry2 = FileEntry {
            path: PathBuf::from("/path/to/file2.txt"),
            name: "file2.txt".to_string(),
            size: 50,
            modified: Utc::now(),
            is_directory: false,
        };

        assert!(filter.matches(&entry1));
        assert!(!filter.matches(&entry2));
    }

    #[test]
    fn test_search_engine_exact_match() {
        let engine = SearchEngine::new(false);
        let items = vec!["apple".to_string(), "banana".to_string(), "cherry".to_string()];

        let matches = engine.fuzzy_search("apple", &items);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].item, "apple");
        assert_eq!(matches[0].score, 100.0);
    }

    #[test]
    fn test_search_engine_contains_match() {
        let engine = SearchEngine::new(false);
        let items = vec![
            "my-laptop".to_string(),
            "desktop-pc".to_string(),
            "laptop-work".to_string(),
        ];

        let matches = engine.fuzzy_search("laptop", &items);
        assert!(matches.len() >= 2);
        assert!(matches.iter().any(|m| m.item == "my-laptop"));
        assert!(matches.iter().any(|m| m.item == "laptop-work"));
    }

    #[test]
    fn test_search_engine_fuzzy_match() {
        let engine = SearchEngine::new(false);
        let items = vec!["apple".to_string(), "aple".to_string(), "banana".to_string()];

        let matches = engine.fuzzy_search("apple", &items);
        assert!(matches.len() >= 2);
        assert_eq!(matches[0].item, "apple"); // Exact match should be first
    }

    #[test]
    fn test_levenshtein_distance() {
        let engine = SearchEngine::new(false);
        assert_eq!(engine.levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(engine.levenshtein_distance("saturday", "sunday"), 3);
        assert_eq!(engine.levenshtein_distance("", "abc"), 3);
        assert_eq!(engine.levenshtein_distance("abc", ""), 3);
    }
}
