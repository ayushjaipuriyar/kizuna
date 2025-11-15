//! Clipboard history management and storage

use async_trait::async_trait;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use crate::clipboard::{
    ClipboardContent, ClipboardResult, ClipboardError,
    HistoryId, ContentSource, Timestamp
};

/// Clipboard history entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub entry_id: HistoryId,
    pub content: ClipboardContent,
    pub source: ContentSource,
    pub created_at: Timestamp,
    pub access_count: u32,
    pub last_accessed: Timestamp,
    pub tags: Vec<String>,
}

impl HistoryEntry {
    /// Check if this entry is from a local source
    pub fn is_local(&self) -> bool {
        matches!(self.source, ContentSource::Local)
    }
    
    /// Check if this entry is from a remote source
    pub fn is_remote(&self) -> bool {
        matches!(self.source, ContentSource::Remote(_))
    }
    
    /// Get the source device ID if this is a remote entry
    pub fn source_device_id(&self) -> Option<&str> {
        match &self.source {
            ContentSource::Remote(peer_id) => Some(peer_id.as_str()),
            _ => None,
        }
    }
    
    /// Get a visual indicator for the source type
    pub fn source_indicator(&self) -> &'static str {
        match &self.source {
            ContentSource::Local => "📋",
            ContentSource::Remote(_) => "🔗",
            ContentSource::History(_) => "📜",
        }
    }
    
    /// Get a human-readable source description
    pub fn source_description(&self) -> String {
        match &self.source {
            ContentSource::Local => "Local".to_string(),
            ContentSource::Remote(peer_id) => format!("Remote ({})", peer_id),
            ContentSource::History(history_id) => format!("History ({})", history_id),
        }
    }
    
    /// Get content preview (first 100 characters for text)
    pub fn content_preview(&self) -> String {
        match &self.content {
            ClipboardContent::Text(text) => {
                let preview = text.text.chars().take(100).collect::<String>();
                if text.text.len() > 100 {
                    format!("{}...", preview)
                } else {
                    preview
                }
            }
            ClipboardContent::Image(image) => {
                format!("Image ({}x{}, {})", image.width, image.height, 
                    match image.format {
                        crate::clipboard::ImageFormat::Png => "PNG",
                        crate::clipboard::ImageFormat::Jpeg => "JPEG",
                        crate::clipboard::ImageFormat::Bmp => "BMP",
                        crate::clipboard::ImageFormat::Gif => "GIF",
                        crate::clipboard::ImageFormat::Tiff => "TIFF",
                    })
            }
            ClipboardContent::Files(files) => {
                format!("{} file(s)", files.len())
            }
            ClipboardContent::Custom { mime_type, .. } => {
                format!("Custom ({})", mime_type)
            }
        }
    }
    
    /// Get the age of this entry as a human-readable string
    pub fn age_description(&self) -> String {
        let now = std::time::SystemTime::now();
        if let Ok(duration) = now.duration_since(self.created_at) {
            let seconds = duration.as_secs();
            if seconds < 60 {
                format!("{} seconds ago", seconds)
            } else if seconds < 3600 {
                format!("{} minutes ago", seconds / 60)
            } else if seconds < 86400 {
                format!("{} hours ago", seconds / 3600)
            } else {
                format!("{} days ago", seconds / 86400)
            }
        } else {
            "Unknown".to_string()
        }
    }
}

/// History manager trait
#[async_trait]
pub trait HistoryManager: Send + Sync {
    /// Add content to history
    async fn add_to_history(&self, content: ClipboardContent, source: ContentSource) -> ClipboardResult<()>;
    
    /// Get recent history entries
    async fn get_history(&self, limit: usize) -> ClipboardResult<Vec<HistoryEntry>>;
    
    /// Search history by text content
    async fn search_history(&self, query: &str) -> ClipboardResult<Vec<HistoryEntry>>;
    
    /// Restore content from history to clipboard
    async fn restore_content(&self, entry_id: HistoryId) -> ClipboardResult<()>;
    
    /// Clear all history
    async fn clear_history(&self) -> ClipboardResult<()>;
    
    /// Get history statistics
    async fn get_history_stats(&self) -> ClipboardResult<HistoryStats>;
    
    /// Get a specific history entry by ID
    async fn get_entry(&self, entry_id: HistoryId) -> ClipboardResult<Option<HistoryEntry>>;
    
    /// Get history entries from a specific source
    async fn get_history_by_source(&self, source_type: &str, limit: usize) -> ClipboardResult<Vec<HistoryEntry>>;
    
    /// Add tags to a history entry
    async fn add_tags(&self, entry_id: HistoryId, tags: Vec<String>) -> ClipboardResult<()>;
    
    /// Remove tags from a history entry
    async fn remove_tags(&self, entry_id: HistoryId, tags: Vec<String>) -> ClipboardResult<()>;
    
    /// Get statistics by source
    async fn get_source_stats(&self) -> ClipboardResult<Vec<SourceStats>>;
    
    /// Get count of entries by source type
    async fn get_source_count(&self, source_type: &str) -> ClipboardResult<u64>;
}

/// History statistics
#[derive(Debug, Clone)]
pub struct HistoryStats {
    pub total_entries: u64,
    pub total_size_bytes: u64,
    pub oldest_entry: Option<Timestamp>,
    pub newest_entry: Option<Timestamp>,
}

impl HistoryStats {
    /// Get average entry size
    pub fn average_entry_size(&self) -> u64 {
        if self.total_entries > 0 {
            self.total_size_bytes / self.total_entries
        } else {
            0
        }
    }
}

/// Source statistics for tracking clipboard sources
#[derive(Debug, Clone)]
pub struct SourceStats {
    pub source_type: String,
    pub source_id: Option<String>,
    pub entry_count: u64,
    pub total_size: u64,
    pub last_activity: Option<Timestamp>,
}

impl SourceStats {
    /// Get a display name for this source
    pub fn display_name(&self) -> String {
        match self.source_type.as_str() {
            "local" => "Local Device".to_string(),
            "remote" => {
                if let Some(id) = &self.source_id {
                    format!("Remote Device ({})", id)
                } else {
                    "Remote Device".to_string()
                }
            }
            "history" => "History".to_string(),
            _ => format!("Unknown ({})", self.source_type),
        }
    }
}

/// History browser for navigating and filtering history entries
pub struct HistoryBrowser {
    manager: Box<dyn HistoryManager>,
}

impl HistoryBrowser {
    /// Create a new history browser
    pub fn new(manager: Box<dyn HistoryManager>) -> Self {
        Self { manager }
    }
    
    /// Browse history with pagination
    pub async fn browse(&self, page: usize, page_size: usize) -> ClipboardResult<Vec<HistoryEntry>> {
        let all_entries = self.manager.get_history(page * page_size + page_size).await?;
        let start = page * page_size;
        let end = std::cmp::min(start + page_size, all_entries.len());
        Ok(all_entries[start..end].to_vec())
    }
    
    /// Browse history in chronological order (oldest first)
    pub async fn browse_chronological(&self, limit: usize) -> ClipboardResult<Vec<HistoryEntry>> {
        let mut entries = self.manager.get_history(limit).await?;
        entries.reverse();
        Ok(entries)
    }
    
    /// Browse only local entries
    pub async fn browse_local(&self, limit: usize) -> ClipboardResult<Vec<HistoryEntry>> {
        self.manager.get_history_by_source("local", limit).await
    }
    
    /// Browse only remote entries
    pub async fn browse_remote(&self, limit: usize) -> ClipboardResult<Vec<HistoryEntry>> {
        self.manager.get_history_by_source("remote", limit).await
    }
    
    /// Search history with query
    pub async fn search(&self, query: &str) -> ClipboardResult<Vec<HistoryEntry>> {
        self.manager.search_history(query).await
    }
    
    /// Get entries with specific tags
    pub async fn browse_by_tags(&self, tags: Vec<String>) -> ClipboardResult<Vec<HistoryEntry>> {
        let all_entries = self.manager.get_history(100).await?;
        let filtered: Vec<HistoryEntry> = all_entries
            .into_iter()
            .filter(|entry| {
                tags.iter().any(|tag| entry.tags.contains(tag))
            })
            .collect();
        Ok(filtered)
    }
    
    /// Get most accessed entries
    pub async fn browse_most_accessed(&self, limit: usize) -> ClipboardResult<Vec<HistoryEntry>> {
        let mut entries = self.manager.get_history(100).await?;
        entries.sort_by(|a, b| b.access_count.cmp(&a.access_count));
        entries.truncate(limit);
        Ok(entries)
    }
    
    /// Get recently accessed entries
    pub async fn browse_recently_accessed(&self, limit: usize) -> ClipboardResult<Vec<HistoryEntry>> {
        let mut entries = self.manager.get_history(100).await?;
        entries.sort_by(|a, b| b.last_accessed.cmp(&a.last_accessed));
        entries.truncate(limit);
        Ok(entries)
    }
    
    /// Restore an entry to the clipboard
    pub async fn restore(&self, entry_id: HistoryId) -> ClipboardResult<ClipboardContent> {
        let entry = self.manager.get_entry(entry_id).await?
            .ok_or_else(|| ClipboardError::content("Entry not found"))?;
        self.manager.restore_content(entry_id).await?;
        Ok(entry.content)
    }
}

/// SQLite-based history manager implementation
pub struct SqliteHistoryManager {
    db_path: PathBuf,
}

impl SqliteHistoryManager {
    /// Create new SQLite history manager
    pub fn new(db_path: PathBuf) -> ClipboardResult<Self> {
        let manager = Self { db_path };
        manager.initialize_database()?;
        Ok(manager)
    }
    
    /// Initialize the database schema
    fn initialize_database(&self) -> ClipboardResult<()> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        conn.execute(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id TEXT PRIMARY KEY,
                content_type TEXT NOT NULL,
                content_data BLOB NOT NULL,
                source_type TEXT NOT NULL,
                source_data TEXT,
                created_at INTEGER NOT NULL,
                access_count INTEGER DEFAULT 0,
                last_accessed INTEGER NOT NULL,
                tags TEXT DEFAULT ''
            )",
            [],
        ).map_err(|e| ClipboardError::database("create table", e))?;
        
        // Create indexes for better performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_created_at ON clipboard_history(created_at DESC)",
            [],
        ).map_err(|e| ClipboardError::database("create index", e))?;
        
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_content_type ON clipboard_history(content_type)",
            [],
        ).map_err(|e| ClipboardError::database("create index", e))?;
        
        Ok(())
    }
    
    /// Serialize clipboard content for storage
    fn serialize_content(&self, content: &ClipboardContent) -> ClipboardResult<(String, Vec<u8>)> {
        let content_json = serde_json::to_string(content)
            .map_err(|e| ClipboardError::serialization("serialize content", e))?;
            
        let content_type = match content {
            ClipboardContent::Text(_) => "text",
            ClipboardContent::Image(_) => "image", 
            ClipboardContent::Files(_) => "files",
            ClipboardContent::Custom { .. } => "custom",
        };
        
        Ok((content_type.to_string(), content_json.into_bytes()))
    }
    
    /// Deserialize clipboard content from storage
    fn deserialize_content(&self, content_data: &[u8]) -> ClipboardResult<ClipboardContent> {
        let content_json = String::from_utf8(content_data.to_vec())
            .map_err(|_| ClipboardError::content("Invalid UTF-8 in stored content"))?;
            
        serde_json::from_str(&content_json)
            .map_err(|e| ClipboardError::serialization("deserialize content", e))
    }
    
    /// Serialize content source for storage
    fn serialize_source(&self, source: &ContentSource) -> ClipboardResult<(String, String)> {
        match source {
            ContentSource::Local => Ok(("local".to_string(), "".to_string())),
            ContentSource::Remote(peer_id) => Ok(("remote".to_string(), peer_id.clone())),
            ContentSource::History(history_id) => Ok(("history".to_string(), history_id.to_string())),
        }
    }
    
    /// Deserialize content source from storage
    fn deserialize_source(&self, source_type: &str, source_data: &str) -> ClipboardResult<ContentSource> {
        match source_type {
            "local" => Ok(ContentSource::Local),
            "remote" => Ok(ContentSource::Remote(source_data.to_string())),
            "history" => {
                let history_id = source_data.parse()
                    .map_err(|_| ClipboardError::content("Invalid history ID"))?;
                Ok(ContentSource::History(history_id))
            }
            _ => Err(ClipboardError::content(format!("Unknown source type: {}", source_type))),
        }
    }
    
    /// Clean up old entries to maintain size limit
    async fn cleanup_old_entries(&self, max_entries: usize) -> ClipboardResult<()> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        // Keep only the most recent entries
        conn.execute(
            "DELETE FROM clipboard_history WHERE id NOT IN (
                SELECT id FROM clipboard_history 
                ORDER BY created_at DESC 
                LIMIT ?
            )",
            params![max_entries],
        ).map_err(|e| ClipboardError::database("cleanup old entries", e))?;
        
        Ok(())
    }
}

#[async_trait]
impl HistoryManager for SqliteHistoryManager {
    async fn add_to_history(&self, content: ClipboardContent, source: ContentSource) -> ClipboardResult<()> {
        let entry_id = uuid::Uuid::new_v4();
        let now = std::time::SystemTime::now();
        let timestamp = now.duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ClipboardError::internal("Invalid system time"))?
            .as_secs() as i64;
            
        let (content_type, content_data) = self.serialize_content(&content)?;
        let (source_type, source_data) = self.serialize_source(&source)?;
        
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        conn.execute(
            "INSERT INTO clipboard_history 
             (id, content_type, content_data, source_type, source_data, created_at, last_accessed)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                entry_id.to_string(),
                content_type,
                content_data,
                source_type,
                source_data,
                timestamp,
                timestamp
            ],
        ).map_err(|e| ClipboardError::database("insert history entry", e))?;
        
        // Cleanup old entries (keep last 50)
        self.cleanup_old_entries(50).await?;
        
        Ok(())
    }
    
    async fn get_history(&self, limit: usize) -> ClipboardResult<Vec<HistoryEntry>> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        let mut stmt = conn.prepare(
            "SELECT id, content_data, source_type, source_data, created_at, access_count, last_accessed, tags
             FROM clipboard_history 
             ORDER BY created_at DESC 
             LIMIT ?"
        ).map_err(|e| ClipboardError::database("prepare statement", e))?;
        
        let rows = stmt.query_map(params![limit], |row| {
            let id_str: String = row.get(0)?;
            let content_data: Vec<u8> = row.get(1)?;
            let source_type: String = row.get(2)?;
            let source_data: String = row.get(3)?;
            let created_at: i64 = row.get(4)?;
            let access_count: u32 = row.get(5)?;
            let last_accessed: i64 = row.get(6)?;
            let tags_str: String = row.get(7)?;
            
            Ok((id_str, content_data, source_type, source_data, created_at, access_count, last_accessed, tags_str))
        }).map_err(|e| ClipboardError::database("query history", e))?;
        
        let mut entries = Vec::new();
        for row in rows {
            let (id_str, content_data, source_type, source_data, created_at, access_count, last_accessed, tags_str) = 
                row.map_err(|e| ClipboardError::database("read row", e))?;
                
            let entry_id = id_str.parse()
                .map_err(|_| ClipboardError::content("Invalid entry ID"))?;
            let content = self.deserialize_content(&content_data)?;
            let source = self.deserialize_source(&source_type, &source_data)?;
            
            let created_timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(created_at as u64);
            let last_accessed_timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(last_accessed as u64);
            
            let tags: Vec<String> = if tags_str.is_empty() {
                vec![]
            } else {
                tags_str.split(',').map(|s| s.trim().to_string()).collect()
            };
            
            entries.push(HistoryEntry {
                entry_id,
                content,
                source,
                created_at: created_timestamp,
                access_count,
                last_accessed: last_accessed_timestamp,
                tags,
            });
        }
        
        Ok(entries)
    }
    
    async fn search_history(&self, query: &str) -> ClipboardResult<Vec<HistoryEntry>> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        let mut stmt = conn.prepare(
            "SELECT id, content_data, source_type, source_data, created_at, access_count, last_accessed, tags
             FROM clipboard_history 
             WHERE content_type = 'text' AND content_data LIKE ?
             ORDER BY created_at DESC 
             LIMIT 50"
        ).map_err(|e| ClipboardError::database("prepare search statement", e))?;
        
        let search_pattern = format!("%{}%", query);
        let rows = stmt.query_map(params![search_pattern], |row| {
            let id_str: String = row.get(0)?;
            let content_data: Vec<u8> = row.get(1)?;
            let source_type: String = row.get(2)?;
            let source_data: String = row.get(3)?;
            let created_at: i64 = row.get(4)?;
            let access_count: u32 = row.get(5)?;
            let last_accessed: i64 = row.get(6)?;
            let tags_str: String = row.get(7)?;
            
            Ok((id_str, content_data, source_type, source_data, created_at, access_count, last_accessed, tags_str))
        }).map_err(|e| ClipboardError::database("search history", e))?;
        
        let mut entries = Vec::new();
        for row in rows {
            let (id_str, content_data, source_type, source_data, created_at, access_count, last_accessed, tags_str) = 
                row.map_err(|e| ClipboardError::database("read search row", e))?;
                
            let entry_id = id_str.parse()
                .map_err(|_| ClipboardError::content("Invalid entry ID"))?;
            let content = self.deserialize_content(&content_data)?;
            let source = self.deserialize_source(&source_type, &source_data)?;
            
            let created_timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(created_at as u64);
            let last_accessed_timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(last_accessed as u64);
            
            let tags: Vec<String> = if tags_str.is_empty() {
                vec![]
            } else {
                tags_str.split(',').map(|s| s.trim().to_string()).collect()
            };
            
            entries.push(HistoryEntry {
                entry_id,
                content,
                source,
                created_at: created_timestamp,
                access_count,
                last_accessed: last_accessed_timestamp,
                tags,
            });
        }
        
        Ok(entries)
    }
    
    async fn restore_content(&self, entry_id: HistoryId) -> ClipboardResult<()> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        // Update access count and last accessed time
        let now = std::time::SystemTime::now();
        let timestamp = now.duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ClipboardError::internal("Invalid system time"))?
            .as_secs() as i64;
            
        conn.execute(
            "UPDATE clipboard_history 
             SET access_count = access_count + 1, last_accessed = ?
             WHERE id = ?",
            params![timestamp, entry_id.to_string()],
        ).map_err(|e| ClipboardError::database("update access count", e))?;
        
        Ok(())
    }
    
    async fn clear_history(&self) -> ClipboardResult<()> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        conn.execute("DELETE FROM clipboard_history", [])
            .map_err(|e| ClipboardError::database("clear history", e))?;
            
        Ok(())
    }
    
    async fn get_history_stats(&self) -> ClipboardResult<HistoryStats> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        let mut stmt = conn.prepare(
            "SELECT COUNT(*), SUM(LENGTH(content_data)), MIN(created_at), MAX(created_at) 
             FROM clipboard_history"
        ).map_err(|e| ClipboardError::database("prepare stats query", e))?;
        
        let (total_entries, total_size, oldest, newest) = stmt.query_row([], |row| {
            let count: i64 = row.get(0)?;
            let size: Option<i64> = row.get(1)?;
            let oldest: Option<i64> = row.get(2)?;
            let newest: Option<i64> = row.get(3)?;
            Ok((count, size, oldest, newest))
        }).map_err(|e| ClipboardError::database("query stats", e))?;
        
        let oldest_entry = oldest.map(|ts| std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts as u64));
        let newest_entry = newest.map(|ts| std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts as u64));
        
        Ok(HistoryStats {
            total_entries: total_entries as u64,
            total_size_bytes: total_size.unwrap_or(0) as u64,
            oldest_entry,
            newest_entry,
        })
    }
    
    async fn get_entry(&self, entry_id: HistoryId) -> ClipboardResult<Option<HistoryEntry>> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        let mut stmt = conn.prepare(
            "SELECT id, content_data, source_type, source_data, created_at, access_count, last_accessed, tags
             FROM clipboard_history 
             WHERE id = ?"
        ).map_err(|e| ClipboardError::database("prepare get entry statement", e))?;
        
        let result = stmt.query_row(params![entry_id.to_string()], |row| {
            let id_str: String = row.get(0)?;
            let content_data: Vec<u8> = row.get(1)?;
            let source_type: String = row.get(2)?;
            let source_data: String = row.get(3)?;
            let created_at: i64 = row.get(4)?;
            let access_count: u32 = row.get(5)?;
            let last_accessed: i64 = row.get(6)?;
            let tags_str: String = row.get(7)?;
            
            Ok((id_str, content_data, source_type, source_data, created_at, access_count, last_accessed, tags_str))
        });
        
        match result {
            Ok((id_str, content_data, source_type, source_data, created_at, access_count, last_accessed, tags_str)) => {
                let entry_id = id_str.parse()
                    .map_err(|_| ClipboardError::content("Invalid entry ID"))?;
                let content = self.deserialize_content(&content_data)?;
                let source = self.deserialize_source(&source_type, &source_data)?;
                
                let created_timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(created_at as u64);
                let last_accessed_timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(last_accessed as u64);
                
                let tags: Vec<String> = if tags_str.is_empty() {
                    vec![]
                } else {
                    tags_str.split(',').map(|s| s.trim().to_string()).collect()
                };
                
                Ok(Some(HistoryEntry {
                    entry_id,
                    content,
                    source,
                    created_at: created_timestamp,
                    access_count,
                    last_accessed: last_accessed_timestamp,
                    tags,
                }))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(ClipboardError::database("get entry", e)),
        }
    }
    
    async fn get_history_by_source(&self, source_type: &str, limit: usize) -> ClipboardResult<Vec<HistoryEntry>> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        let mut stmt = conn.prepare(
            "SELECT id, content_data, source_type, source_data, created_at, access_count, last_accessed, tags
             FROM clipboard_history 
             WHERE source_type = ?
             ORDER BY created_at DESC 
             LIMIT ?"
        ).map_err(|e| ClipboardError::database("prepare source query statement", e))?;
        
        let rows = stmt.query_map(params![source_type, limit], |row| {
            let id_str: String = row.get(0)?;
            let content_data: Vec<u8> = row.get(1)?;
            let source_type: String = row.get(2)?;
            let source_data: String = row.get(3)?;
            let created_at: i64 = row.get(4)?;
            let access_count: u32 = row.get(5)?;
            let last_accessed: i64 = row.get(6)?;
            let tags_str: String = row.get(7)?;
            
            Ok((id_str, content_data, source_type, source_data, created_at, access_count, last_accessed, tags_str))
        }).map_err(|e| ClipboardError::database("query history by source", e))?;
        
        let mut entries = Vec::new();
        for row in rows {
            let (id_str, content_data, source_type, source_data, created_at, access_count, last_accessed, tags_str) = 
                row.map_err(|e| ClipboardError::database("read source row", e))?;
                
            let entry_id = id_str.parse()
                .map_err(|_| ClipboardError::content("Invalid entry ID"))?;
            let content = self.deserialize_content(&content_data)?;
            let source = self.deserialize_source(&source_type, &source_data)?;
            
            let created_timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(created_at as u64);
            let last_accessed_timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(last_accessed as u64);
            
            let tags: Vec<String> = if tags_str.is_empty() {
                vec![]
            } else {
                tags_str.split(',').map(|s| s.trim().to_string()).collect()
            };
            
            entries.push(HistoryEntry {
                entry_id,
                content,
                source,
                created_at: created_timestamp,
                access_count,
                last_accessed: last_accessed_timestamp,
                tags,
            });
        }
        
        Ok(entries)
    }
    
    async fn add_tags(&self, entry_id: HistoryId, new_tags: Vec<String>) -> ClipboardResult<()> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        // Get current tags
        let current_tags: String = conn.query_row(
            "SELECT tags FROM clipboard_history WHERE id = ?",
            params![entry_id.to_string()],
            |row| row.get(0),
        ).map_err(|e| ClipboardError::database("get current tags", e))?;
        
        // Parse and merge tags
        let mut tags_set: std::collections::HashSet<String> = if current_tags.is_empty() {
            std::collections::HashSet::new()
        } else {
            current_tags.split(',').map(|s| s.trim().to_string()).collect()
        };
        
        for tag in new_tags {
            tags_set.insert(tag);
        }
        
        let updated_tags = tags_set.into_iter().collect::<Vec<_>>().join(",");
        
        // Update tags
        conn.execute(
            "UPDATE clipboard_history SET tags = ? WHERE id = ?",
            params![updated_tags, entry_id.to_string()],
        ).map_err(|e| ClipboardError::database("update tags", e))?;
        
        Ok(())
    }
    
    async fn remove_tags(&self, entry_id: HistoryId, tags_to_remove: Vec<String>) -> ClipboardResult<()> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        // Get current tags
        let current_tags: String = conn.query_row(
            "SELECT tags FROM clipboard_history WHERE id = ?",
            params![entry_id.to_string()],
            |row| row.get(0),
        ).map_err(|e| ClipboardError::database("get current tags", e))?;
        
        if current_tags.is_empty() {
            return Ok(());
        }
        
        // Parse and filter tags
        let mut tags_set: std::collections::HashSet<String> = 
            current_tags.split(',').map(|s| s.trim().to_string()).collect();
        
        for tag in tags_to_remove {
            tags_set.remove(&tag);
        }
        
        let updated_tags = tags_set.into_iter().collect::<Vec<_>>().join(",");
        
        // Update tags
        conn.execute(
            "UPDATE clipboard_history SET tags = ? WHERE id = ?",
            params![updated_tags, entry_id.to_string()],
        ).map_err(|e| ClipboardError::database("update tags", e))?;
        
        Ok(())
    }
    
    async fn get_source_stats(&self) -> ClipboardResult<Vec<SourceStats>> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        let mut stmt = conn.prepare(
            "SELECT source_type, source_data, COUNT(*), SUM(LENGTH(content_data)), MAX(created_at)
             FROM clipboard_history
             GROUP BY source_type, source_data
             ORDER BY COUNT(*) DESC"
        ).map_err(|e| ClipboardError::database("prepare source stats query", e))?;
        
        let rows = stmt.query_map([], |row| {
            let source_type: String = row.get(0)?;
            let source_data: String = row.get(1)?;
            let count: i64 = row.get(2)?;
            let size: Option<i64> = row.get(3)?;
            let last_activity: Option<i64> = row.get(4)?;
            Ok((source_type, source_data, count, size, last_activity))
        }).map_err(|e| ClipboardError::database("query source stats", e))?;
        
        let mut stats = Vec::new();
        for row in rows {
            let (source_type, source_data, count, size, last_activity) = 
                row.map_err(|e| ClipboardError::database("read source stats row", e))?;
                
            let source_id = if source_data.is_empty() {
                None
            } else {
                Some(source_data)
            };
            
            let last_activity_timestamp = last_activity.map(|ts| 
                std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts as u64)
            );
            
            stats.push(SourceStats {
                source_type,
                source_id,
                entry_count: count as u64,
                total_size: size.unwrap_or(0) as u64,
                last_activity: last_activity_timestamp,
            });
        }
        
        Ok(stats)
    }
    
    async fn get_source_count(&self, source_type: &str) -> ClipboardResult<u64> {
        let conn = Connection::open(&self.db_path)
            .map_err(|e| ClipboardError::database("open database", e))?;
            
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM clipboard_history WHERE source_type = ?",
            params![source_type],
            |row| row.get(0),
        ).map_err(|e| ClipboardError::database("query source count", e))?;
        
        Ok(count as u64)
    }
}