// Command Execution History Module
//
// This module provides SQLite-based storage for command execution history,
// including command requests, results, authorization decisions, and audit logs.

use async_trait::async_trait;
use rusqlite::{Connection, params, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

use crate::command_execution::{
    error::{CommandError, CommandResult as CmdResult},
    types::*,
};

/// History manager trait for command execution history
#[async_trait]
pub trait HistoryManager: Send + Sync {
    /// Add a command execution to history
    async fn add_command_execution(
        &self,
        entry: CommandHistoryEntry,
    ) -> CmdResult<()>;

    /// Get command history with optional limit
    async fn get_history(&self, limit: Option<usize>) -> CmdResult<Vec<CommandHistoryEntry>>;

    /// Search command history by text
    async fn search_history(&self, query: &str) -> CmdResult<Vec<CommandHistoryEntry>>;

    /// Filter history by criteria
    async fn filter_history(&self, filter: HistoryFilter) -> CmdResult<Vec<CommandHistoryEntry>>;

    /// Get a specific history entry by ID
    async fn get_entry(&self, entry_id: EntryId) -> CmdResult<Option<CommandHistoryEntry>>;

    /// Delete old entries based on retention policy
    async fn cleanup_old_entries(&self, retention_days: u32) -> CmdResult<usize>;

    /// Export history to JSON
    async fn export_history(&self, filter: Option<HistoryFilter>) -> CmdResult<String>;
}

/// Filter criteria for history queries
#[derive(Debug, Clone)]
pub struct HistoryFilter {
    pub peer_id: Option<PeerId>,
    pub command_type: Option<CommandType>,
    pub execution_status: Option<ExecutionStatus>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub risk_level: Option<RiskLevel>,
}

impl Default for HistoryFilter {
    fn default() -> Self {
        Self {
            peer_id: None,
            command_type: None,
            execution_status: None,
            start_date: None,
            end_date: None,
            risk_level: None,
        }
    }
}

/// SQLite-based history manager implementation
pub struct SqliteHistoryManager {
    db_path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}

impl SqliteHistoryManager {
    /// Create new SQLite history manager
    pub fn new(db_path: PathBuf) -> CmdResult<Self> {
        let connection = Connection::open(&db_path)
            .map_err(|e| CommandError::Internal(format!("Failed to open database: {}", e)))?;
        
        let manager = Self {
            db_path,
            connection: Arc::new(Mutex::new(connection)),
        };
        
        manager.initialize_database()?;
        Ok(manager)
    }

    /// Initialize the database schema
    fn initialize_database(&self) -> CmdResult<()> {
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        // Create command history table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS command_history (
                entry_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                command TEXT NOT NULL,
                arguments TEXT NOT NULL,
                working_directory TEXT,
                environment TEXT NOT NULL,
                timeout_secs INTEGER NOT NULL,
                sandbox_config TEXT NOT NULL,
                requester TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                completed_at INTEGER,
                exit_code INTEGER,
                stdout TEXT,
                stderr TEXT,
                execution_time_ms INTEGER,
                resource_usage TEXT,
                authorization_decision TEXT NOT NULL,
                authorization_decided_at INTEGER NOT NULL,
                authorization_decided_by TEXT NOT NULL,
                execution_status TEXT NOT NULL,
                command_type TEXT NOT NULL,
                risk_level TEXT NOT NULL
            )",
            [],
        ).map_err(|e| CommandError::Internal(format!("Failed to create table: {}", e)))?;

        // Create indexes for better query performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_created_at ON command_history(created_at DESC)",
            [],
        ).map_err(|e| CommandError::Internal(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_requester ON command_history(requester)",
            [],
        ).map_err(|e| CommandError::Internal(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_execution_status ON command_history(execution_status)",
            [],
        ).map_err(|e| CommandError::Internal(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_command_type ON command_history(command_type)",
            [],
        ).map_err(|e| CommandError::Internal(format!("Failed to create index: {}", e)))?;

        Ok(())
    }

    /// Serialize a command history entry for storage
    fn serialize_entry(&self, entry: &CommandHistoryEntry) -> CmdResult<SerializedEntry> {
        let arguments_json = serde_json::to_string(&entry.command_request.arguments)
            .map_err(|e| CommandError::Internal(format!("Failed to serialize arguments: {}", e)))?;
        
        let environment_json = serde_json::to_string(&entry.command_request.environment)
            .map_err(|e| CommandError::Internal(format!("Failed to serialize environment: {}", e)))?;
        
        let sandbox_config_json = serde_json::to_string(&entry.command_request.sandbox_config)
            .map_err(|e| CommandError::Internal(format!("Failed to serialize sandbox config: {}", e)))?;
        
        let resource_usage_json = entry.result.as_ref().map(|r| {
            serde_json::to_string(&r.resource_usage)
                .map_err(|e| CommandError::Internal(format!("Failed to serialize resource usage: {}", e)))
        }).transpose()?;
        
        let authorization_decision_json = serde_json::to_string(&entry.authorization.decision)
            .map_err(|e| CommandError::Internal(format!("Failed to serialize authorization decision: {}", e)))?;
        
        let execution_status_json = serde_json::to_string(&entry.execution_status)
            .map_err(|e| CommandError::Internal(format!("Failed to serialize execution status: {}", e)))?;
        
        // Determine command type and risk level from the request
        let command_type = "SimpleCommand"; // Default, could be enhanced
        let risk_level = "Medium"; // Default, could be enhanced
        
        Ok(SerializedEntry {
            entry_id: entry.entry_id.to_string(),
            request_id: entry.command_request.request_id.to_string(),
            command: entry.command_request.command.clone(),
            arguments: arguments_json,
            working_directory: entry.command_request.working_directory.as_ref().map(|p| p.to_string_lossy().to_string()),
            environment: environment_json,
            timeout_secs: entry.command_request.timeout.as_secs() as i64,
            sandbox_config: sandbox_config_json,
            requester: entry.command_request.requester.clone(),
            created_at: entry.created_at.timestamp(),
            completed_at: entry.completed_at.map(|t| t.timestamp()),
            exit_code: entry.result.as_ref().map(|r| r.exit_code),
            stdout: entry.result.as_ref().map(|r| r.stdout.clone()),
            stderr: entry.result.as_ref().map(|r| r.stderr.clone()),
            execution_time_ms: entry.result.as_ref().map(|r| r.execution_time.as_millis() as i64),
            resource_usage: resource_usage_json,
            authorization_decision: authorization_decision_json,
            authorization_decided_at: entry.authorization.decided_at.timestamp(),
            authorization_decided_by: entry.authorization.decided_by.clone(),
            execution_status: execution_status_json,
            command_type: command_type.to_string(),
            risk_level: risk_level.to_string(),
        })
    }

    /// Deserialize a command history entry from storage
    fn deserialize_entry(&self, row: &rusqlite::Row) -> CmdResult<CommandHistoryEntry> {
        let entry_id: String = row.get(0)
            .map_err(|e| CommandError::Internal(format!("Failed to get entry_id: {}", e)))?;
        let request_id: String = row.get(1)
            .map_err(|e| CommandError::Internal(format!("Failed to get request_id: {}", e)))?;
        let command: String = row.get(2)
            .map_err(|e| CommandError::Internal(format!("Failed to get command: {}", e)))?;
        let arguments_json: String = row.get(3)
            .map_err(|e| CommandError::Internal(format!("Failed to get arguments: {}", e)))?;
        let working_directory: Option<String> = row.get(4)
            .map_err(|e| CommandError::Internal(format!("Failed to get working_directory: {}", e)))?;
        let environment_json: String = row.get(5)
            .map_err(|e| CommandError::Internal(format!("Failed to get environment: {}", e)))?;
        let timeout_secs: i64 = row.get(6)
            .map_err(|e| CommandError::Internal(format!("Failed to get timeout: {}", e)))?;
        let sandbox_config_json: String = row.get(7)
            .map_err(|e| CommandError::Internal(format!("Failed to get sandbox_config: {}", e)))?;
        let requester: String = row.get(8)
            .map_err(|e| CommandError::Internal(format!("Failed to get requester: {}", e)))?;
        let created_at: i64 = row.get(9)
            .map_err(|e| CommandError::Internal(format!("Failed to get created_at: {}", e)))?;
        let completed_at: Option<i64> = row.get(10)
            .map_err(|e| CommandError::Internal(format!("Failed to get completed_at: {}", e)))?;
        let exit_code: Option<i32> = row.get(11)
            .map_err(|e| CommandError::Internal(format!("Failed to get exit_code: {}", e)))?;
        let stdout: Option<String> = row.get(12)
            .map_err(|e| CommandError::Internal(format!("Failed to get stdout: {}", e)))?;
        let stderr: Option<String> = row.get(13)
            .map_err(|e| CommandError::Internal(format!("Failed to get stderr: {}", e)))?;
        let execution_time_ms: Option<i64> = row.get(14)
            .map_err(|e| CommandError::Internal(format!("Failed to get execution_time: {}", e)))?;
        let resource_usage_json: Option<String> = row.get(15)
            .map_err(|e| CommandError::Internal(format!("Failed to get resource_usage: {}", e)))?;
        let authorization_decision_json: String = row.get(16)
            .map_err(|e| CommandError::Internal(format!("Failed to get authorization_decision: {}", e)))?;
        let authorization_decided_at: i64 = row.get(17)
            .map_err(|e| CommandError::Internal(format!("Failed to get authorization_decided_at: {}", e)))?;
        let authorization_decided_by: String = row.get(18)
            .map_err(|e| CommandError::Internal(format!("Failed to get authorization_decided_by: {}", e)))?;
        let execution_status_json: String = row.get(19)
            .map_err(|e| CommandError::Internal(format!("Failed to get execution_status: {}", e)))?;

        // Deserialize JSON fields
        let arguments: Vec<String> = serde_json::from_str(&arguments_json)
            .map_err(|e| CommandError::Internal(format!("Failed to deserialize arguments: {}", e)))?;
        let environment: std::collections::HashMap<String, String> = serde_json::from_str(&environment_json)
            .map_err(|e| CommandError::Internal(format!("Failed to deserialize environment: {}", e)))?;
        let sandbox_config: SandboxConfig = serde_json::from_str(&sandbox_config_json)
            .map_err(|e| CommandError::Internal(format!("Failed to deserialize sandbox_config: {}", e)))?;
        let authorization_decision: AuthorizationDecision = serde_json::from_str(&authorization_decision_json)
            .map_err(|e| CommandError::Internal(format!("Failed to deserialize authorization_decision: {}", e)))?;
        let execution_status: ExecutionStatus = serde_json::from_str(&execution_status_json)
            .map_err(|e| CommandError::Internal(format!("Failed to deserialize execution_status: {}", e)))?;

        // Build result if available
        let result = if let (Some(exit_code), Some(stdout), Some(stderr), Some(execution_time_ms)) = 
            (exit_code, stdout, stderr, execution_time_ms) {
            let resource_usage = if let Some(ru_json) = resource_usage_json {
                serde_json::from_str(&ru_json)
                    .map_err(|e| CommandError::Internal(format!("Failed to deserialize resource_usage: {}", e)))?
            } else {
                ResourceUsage::default()
            };
            
            Some(CommandResult {
                request_id: uuid::Uuid::parse_str(&request_id)
                    .map_err(|e| CommandError::Internal(format!("Invalid request_id UUID: {}", e)))?,
                exit_code,
                stdout,
                stderr,
                execution_time: std::time::Duration::from_millis(execution_time_ms as u64),
                resource_usage,
                completed_at: DateTime::from_timestamp(completed_at.unwrap_or(created_at), 0)
                    .ok_or_else(|| CommandError::Internal("Invalid completed_at timestamp".to_string()))?,
            })
        } else {
            None
        };

        Ok(CommandHistoryEntry {
            entry_id: uuid::Uuid::parse_str(&entry_id)
                .map_err(|e| CommandError::Internal(format!("Invalid entry_id UUID: {}", e)))?,
            command_request: CommandRequest {
                request_id: uuid::Uuid::parse_str(&request_id)
                    .map_err(|e| CommandError::Internal(format!("Invalid request_id UUID: {}", e)))?,
                command,
                arguments,
                working_directory: working_directory.map(PathBuf::from),
                environment,
                timeout: std::time::Duration::from_secs(timeout_secs as u64),
                sandbox_config,
                requester,
                created_at: DateTime::from_timestamp(created_at, 0)
                    .ok_or_else(|| CommandError::Internal("Invalid created_at timestamp".to_string()))?,
            },
            result,
            authorization: AuthorizationRecord {
                request_id: uuid::Uuid::parse_str(&request_id)
                    .map_err(|e| CommandError::Internal(format!("Invalid request_id UUID: {}", e)))?,
                decision: authorization_decision,
                decided_at: DateTime::from_timestamp(authorization_decided_at, 0)
                    .ok_or_else(|| CommandError::Internal("Invalid authorization_decided_at timestamp".to_string()))?,
                decided_by: authorization_decided_by,
            },
            execution_status,
            created_at: DateTime::from_timestamp(created_at, 0)
                .ok_or_else(|| CommandError::Internal("Invalid created_at timestamp".to_string()))?,
            completed_at: completed_at.and_then(|ts| DateTime::from_timestamp(ts, 0)),
        })
    }
}

/// Helper struct for serialized entry data
struct SerializedEntry {
    entry_id: String,
    request_id: String,
    command: String,
    arguments: String,
    working_directory: Option<String>,
    environment: String,
    timeout_secs: i64,
    sandbox_config: String,
    requester: String,
    created_at: i64,
    completed_at: Option<i64>,
    exit_code: Option<i32>,
    stdout: Option<String>,
    stderr: Option<String>,
    execution_time_ms: Option<i64>,
    resource_usage: Option<String>,
    authorization_decision: String,
    authorization_decided_at: i64,
    authorization_decided_by: String,
    execution_status: String,
    command_type: String,
    risk_level: String,
}

#[async_trait]
impl HistoryManager for SqliteHistoryManager {
    async fn add_command_execution(&self, entry: CommandHistoryEntry) -> CmdResult<()> {
        let serialized = self.serialize_entry(&entry)?;
        
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        conn.execute(
            "INSERT INTO command_history (
                entry_id, request_id, command, arguments, working_directory,
                environment, timeout_secs, sandbox_config, requester, created_at,
                completed_at, exit_code, stdout, stderr, execution_time_ms,
                resource_usage, authorization_decision, authorization_decided_at,
                authorization_decided_by, execution_status, command_type, risk_level
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                serialized.entry_id,
                serialized.request_id,
                serialized.command,
                serialized.arguments,
                serialized.working_directory,
                serialized.environment,
                serialized.timeout_secs,
                serialized.sandbox_config,
                serialized.requester,
                serialized.created_at,
                serialized.completed_at,
                serialized.exit_code,
                serialized.stdout,
                serialized.stderr,
                serialized.execution_time_ms,
                serialized.resource_usage,
                serialized.authorization_decision,
                serialized.authorization_decided_at,
                serialized.authorization_decided_by,
                serialized.execution_status,
                serialized.command_type,
                serialized.risk_level,
            ],
        ).map_err(|e| CommandError::Internal(format!("Failed to insert history entry: {}", e)))?;

        Ok(())
    }

    async fn get_history(&self, limit: Option<usize>) -> CmdResult<Vec<CommandHistoryEntry>> {
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        let limit_value = limit.unwrap_or(100);
        let mut stmt = conn.prepare(
            "SELECT * FROM command_history ORDER BY created_at DESC LIMIT ?"
        ).map_err(|e| CommandError::Internal(format!("Failed to prepare statement: {}", e)))?;

        let entries = stmt.query_map(params![limit_value], |row| {
            self.deserialize_entry(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        }).map_err(|e| CommandError::Internal(format!("Failed to query history: {}", e)))?;

        let mut result = Vec::new();
        for entry in entries {
            result.push(entry.map_err(|e| CommandError::Internal(format!("Failed to deserialize entry: {}", e)))?);
        }

        Ok(result)
    }

    async fn search_history(&self, query: &str) -> CmdResult<Vec<CommandHistoryEntry>> {
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        let search_pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT * FROM command_history 
             WHERE command LIKE ? OR stdout LIKE ? OR stderr LIKE ?
             ORDER BY created_at DESC LIMIT 100"
        ).map_err(|e| CommandError::Internal(format!("Failed to prepare statement: {}", e)))?;

        let entries = stmt.query_map(params![search_pattern, search_pattern, search_pattern], |row| {
            self.deserialize_entry(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        }).map_err(|e| CommandError::Internal(format!("Failed to query history: {}", e)))?;

        let mut result = Vec::new();
        for entry in entries {
            result.push(entry.map_err(|e| CommandError::Internal(format!("Failed to deserialize entry: {}", e)))?);
        }

        Ok(result)
    }

    async fn filter_history(&self, filter: HistoryFilter) -> CmdResult<Vec<CommandHistoryEntry>> {
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        let mut query = "SELECT * FROM command_history WHERE 1=1".to_string();
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(peer_id) = &filter.peer_id {
            query.push_str(" AND requester = ?");
            params_vec.push(Box::new(peer_id.clone()));
        }

        if let Some(cmd_type) = &filter.command_type {
            query.push_str(" AND command_type = ?");
            let type_str = format!("{:?}", cmd_type);
            params_vec.push(Box::new(type_str));
        }

        if let Some(status) = &filter.execution_status {
            query.push_str(" AND execution_status LIKE ?");
            let status_pattern = format!("%{}%", format!("{:?}", status));
            params_vec.push(Box::new(status_pattern));
        }

        if let Some(start_date) = filter.start_date {
            query.push_str(" AND created_at >= ?");
            params_vec.push(Box::new(start_date.timestamp()));
        }

        if let Some(end_date) = filter.end_date {
            query.push_str(" AND created_at <= ?");
            params_vec.push(Box::new(end_date.timestamp()));
        }

        if let Some(risk) = &filter.risk_level {
            query.push_str(" AND risk_level = ?");
            let risk_str = format!("{:?}", risk);
            params_vec.push(Box::new(risk_str));
        }

        query.push_str(" ORDER BY created_at DESC LIMIT 100");

        let mut stmt = conn.prepare(&query)
            .map_err(|e| CommandError::Internal(format!("Failed to prepare statement: {}", e)))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        
        let entries = stmt.query_map(params_refs.as_slice(), |row| {
            self.deserialize_entry(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        }).map_err(|e| CommandError::Internal(format!("Failed to query history: {}", e)))?;

        let mut result = Vec::new();
        for entry in entries {
            result.push(entry.map_err(|e| CommandError::Internal(format!("Failed to deserialize entry: {}", e)))?);
        }

        Ok(result)
    }

    async fn get_entry(&self, entry_id: EntryId) -> CmdResult<Option<CommandHistoryEntry>> {
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        let mut stmt = conn.prepare(
            "SELECT * FROM command_history WHERE entry_id = ?"
        ).map_err(|e| CommandError::Internal(format!("Failed to prepare statement: {}", e)))?;

        let entry = stmt.query_row(params![entry_id.to_string()], |row| {
            self.deserialize_entry(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        }).optional()
        .map_err(|e| CommandError::Internal(format!("Failed to query entry: {}", e)))?;

        Ok(entry)
    }

    async fn cleanup_old_entries(&self, retention_days: u32) -> CmdResult<usize> {
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        let cutoff_timestamp = Utc::now().timestamp() - (retention_days as i64 * 86400);

        let deleted = conn.execute(
            "DELETE FROM command_history WHERE created_at < ?",
            params![cutoff_timestamp],
        ).map_err(|e| CommandError::Internal(format!("Failed to delete old entries: {}", e)))?;

        Ok(deleted)
    }

    async fn export_history(&self, filter: Option<HistoryFilter>) -> CmdResult<String> {
        let entries = if let Some(f) = filter {
            self.filter_history(f).await?
        } else {
            self.get_history(None).await?
        };

        serde_json::to_string_pretty(&entries)
            .map_err(|e| CommandError::Internal(format!("Failed to serialize history: {}", e)))
    }
}
