// Command Execution Audit Module
//
// This module provides audit logging for authorization decisions and security events
// related to command execution, with log rotation and secure storage.

use async_trait::async_trait;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::command_execution::{
    error::{CommandError, CommandResult as CmdResult},
    types::*,
};

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    AuthorizationRequest,
    AuthorizationApproved,
    AuthorizationDenied,
    AuthorizationTimeout,
    AuthorizationModified,
    CommandExecutionStarted,
    CommandExecutionCompleted,
    CommandExecutionFailed,
    SecurityViolationAttempt,
    SandboxEscape,
    UnauthorizedAccess,
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub log_id: uuid::Uuid,
    pub event_type: AuditEventType,
    pub timestamp: DateTime<Utc>,
    pub peer_id: PeerId,
    pub request_id: Option<RequestId>,
    pub command_preview: Option<String>,
    pub risk_level: Option<RiskLevel>,
    pub decision: Option<String>,
    pub decided_by: Option<String>,
    pub details: String,
    pub severity: AuditSeverity,
}

/// Audit event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditSeverity {
    Info,
    Warning,
    Critical,
}

/// Audit logger trait
#[async_trait]
pub trait AuditLogger: Send + Sync {
    /// Log an audit event
    async fn log_event(&self, entry: AuditLogEntry) -> CmdResult<()>;

    /// Get audit logs with optional filtering
    async fn get_logs(&self, filter: Option<AuditFilter>) -> CmdResult<Vec<AuditLogEntry>>;

    /// Get authorization history for a specific peer
    async fn get_authorization_history(&self, peer_id: &PeerId) -> CmdResult<Vec<AuditLogEntry>>;

    /// Get security events (violations, unauthorized access, etc.)
    async fn get_security_events(&self) -> CmdResult<Vec<AuditLogEntry>>;

    /// Rotate logs (archive old logs and clean up)
    async fn rotate_logs(&self, retention_days: u32) -> CmdResult<usize>;

    /// Export audit logs to JSON
    async fn export_logs(&self, filter: Option<AuditFilter>) -> CmdResult<String>;
}

/// Filter for audit log queries
#[derive(Debug, Clone)]
pub struct AuditFilter {
    pub peer_id: Option<PeerId>,
    pub event_type: Option<AuditEventType>,
    pub severity: Option<AuditSeverity>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub request_id: Option<RequestId>,
}

impl Default for AuditFilter {
    fn default() -> Self {
        Self {
            peer_id: None,
            event_type: None,
            severity: None,
            start_date: None,
            end_date: None,
            request_id: None,
        }
    }
}

/// SQLite-based audit logger implementation
pub struct SqliteAuditLogger {
    db_path: PathBuf,
    connection: Arc<Mutex<Connection>>,
}

impl SqliteAuditLogger {
    /// Create new SQLite audit logger
    pub fn new(db_path: PathBuf) -> CmdResult<Self> {
        let connection = Connection::open(&db_path)
            .map_err(|e| CommandError::Internal(format!("Failed to open audit database: {}", e)))?;
        
        let logger = Self {
            db_path,
            connection: Arc::new(Mutex::new(connection)),
        };
        
        logger.initialize_database()?;
        Ok(logger)
    }

    /// Initialize the audit database schema
    fn initialize_database(&self) -> CmdResult<()> {
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        // Create audit log table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS audit_log (
                log_id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                peer_id TEXT NOT NULL,
                request_id TEXT,
                command_preview TEXT,
                risk_level TEXT,
                decision TEXT,
                decided_by TEXT,
                details TEXT NOT NULL,
                severity TEXT NOT NULL
            )",
            [],
        ).map_err(|e| CommandError::Internal(format!("Failed to create audit table: {}", e)))?;

        // Create indexes for better query performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp DESC)",
            [],
        ).map_err(|e| CommandError::Internal(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_peer_id ON audit_log(peer_id)",
            [],
        ).map_err(|e| CommandError::Internal(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_event_type ON audit_log(event_type)",
            [],
        ).map_err(|e| CommandError::Internal(format!("Failed to create index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_severity ON audit_log(severity)",
            [],
        ).map_err(|e| CommandError::Internal(format!("Failed to create index: {}", e)))?;

        Ok(())
    }

    /// Serialize an audit log entry for storage
    fn serialize_entry(&self, entry: &AuditLogEntry) -> SerializedAuditEntry {
        SerializedAuditEntry {
            log_id: entry.log_id.to_string(),
            event_type: format!("{:?}", entry.event_type),
            timestamp: entry.timestamp.timestamp(),
            peer_id: entry.peer_id.clone(),
            request_id: entry.request_id.map(|id| id.to_string()),
            command_preview: entry.command_preview.clone(),
            risk_level: entry.risk_level.as_ref().map(|r| format!("{:?}", r)),
            decision: entry.decision.clone(),
            decided_by: entry.decided_by.clone(),
            details: entry.details.clone(),
            severity: format!("{:?}", entry.severity),
        }
    }

    /// Deserialize an audit log entry from storage
    fn deserialize_entry(&self, row: &rusqlite::Row) -> CmdResult<AuditLogEntry> {
        let log_id: String = row.get(0)
            .map_err(|e| CommandError::Internal(format!("Failed to get log_id: {}", e)))?;
        let event_type_str: String = row.get(1)
            .map_err(|e| CommandError::Internal(format!("Failed to get event_type: {}", e)))?;
        let timestamp: i64 = row.get(2)
            .map_err(|e| CommandError::Internal(format!("Failed to get timestamp: {}", e)))?;
        let peer_id: String = row.get(3)
            .map_err(|e| CommandError::Internal(format!("Failed to get peer_id: {}", e)))?;
        let request_id: Option<String> = row.get(4)
            .map_err(|e| CommandError::Internal(format!("Failed to get request_id: {}", e)))?;
        let command_preview: Option<String> = row.get(5)
            .map_err(|e| CommandError::Internal(format!("Failed to get command_preview: {}", e)))?;
        let risk_level_str: Option<String> = row.get(6)
            .map_err(|e| CommandError::Internal(format!("Failed to get risk_level: {}", e)))?;
        let decision: Option<String> = row.get(7)
            .map_err(|e| CommandError::Internal(format!("Failed to get decision: {}", e)))?;
        let decided_by: Option<String> = row.get(8)
            .map_err(|e| CommandError::Internal(format!("Failed to get decided_by: {}", e)))?;
        let details: String = row.get(9)
            .map_err(|e| CommandError::Internal(format!("Failed to get details: {}", e)))?;
        let severity_str: String = row.get(10)
            .map_err(|e| CommandError::Internal(format!("Failed to get severity: {}", e)))?;

        // Parse event type
        let event_type = match event_type_str.as_str() {
            "AuthorizationRequest" => AuditEventType::AuthorizationRequest,
            "AuthorizationApproved" => AuditEventType::AuthorizationApproved,
            "AuthorizationDenied" => AuditEventType::AuthorizationDenied,
            "AuthorizationTimeout" => AuditEventType::AuthorizationTimeout,
            "AuthorizationModified" => AuditEventType::AuthorizationModified,
            "CommandExecutionStarted" => AuditEventType::CommandExecutionStarted,
            "CommandExecutionCompleted" => AuditEventType::CommandExecutionCompleted,
            "CommandExecutionFailed" => AuditEventType::CommandExecutionFailed,
            "SecurityViolationAttempt" => AuditEventType::SecurityViolationAttempt,
            "SandboxEscape" => AuditEventType::SandboxEscape,
            "UnauthorizedAccess" => AuditEventType::UnauthorizedAccess,
            _ => return Err(CommandError::Internal(format!("Unknown event type: {}", event_type_str))),
        };

        // Parse risk level
        let risk_level = risk_level_str.and_then(|s| match s.as_str() {
            "Low" => Some(RiskLevel::Low),
            "Medium" => Some(RiskLevel::Medium),
            "High" => Some(RiskLevel::High),
            "Critical" => Some(RiskLevel::Critical),
            _ => None,
        });

        // Parse severity
        let severity = match severity_str.as_str() {
            "Info" => AuditSeverity::Info,
            "Warning" => AuditSeverity::Warning,
            "Critical" => AuditSeverity::Critical,
            _ => return Err(CommandError::Internal(format!("Unknown severity: {}", severity_str))),
        };

        Ok(AuditLogEntry {
            log_id: uuid::Uuid::parse_str(&log_id)
                .map_err(|e| CommandError::Internal(format!("Invalid log_id UUID: {}", e)))?,
            event_type,
            timestamp: DateTime::from_timestamp(timestamp, 0)
                .ok_or_else(|| CommandError::Internal("Invalid timestamp".to_string()))?,
            peer_id,
            request_id: request_id.and_then(|s| uuid::Uuid::parse_str(&s).ok()),
            command_preview,
            risk_level,
            decision,
            decided_by,
            details,
            severity,
        })
    }
}

/// Helper struct for serialized audit entry data
struct SerializedAuditEntry {
    log_id: String,
    event_type: String,
    timestamp: i64,
    peer_id: String,
    request_id: Option<String>,
    command_preview: Option<String>,
    risk_level: Option<String>,
    decision: Option<String>,
    decided_by: Option<String>,
    details: String,
    severity: String,
}

#[async_trait]
impl AuditLogger for SqliteAuditLogger {
    async fn log_event(&self, entry: AuditLogEntry) -> CmdResult<()> {
        let serialized = self.serialize_entry(&entry);
        
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        conn.execute(
            "INSERT INTO audit_log (
                log_id, event_type, timestamp, peer_id, request_id,
                command_preview, risk_level, decision, decided_by, details, severity
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                serialized.log_id,
                serialized.event_type,
                serialized.timestamp,
                serialized.peer_id,
                serialized.request_id,
                serialized.command_preview,
                serialized.risk_level,
                serialized.decision,
                serialized.decided_by,
                serialized.details,
                serialized.severity,
            ],
        ).map_err(|e| CommandError::Internal(format!("Failed to insert audit log: {}", e)))?;

        Ok(())
    }

    async fn get_logs(&self, filter: Option<AuditFilter>) -> CmdResult<Vec<AuditLogEntry>> {
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        let (query, params_vec) = if let Some(f) = filter {
            let mut query = "SELECT * FROM audit_log WHERE 1=1".to_string();
            let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(peer_id) = &f.peer_id {
                query.push_str(" AND peer_id = ?");
                params_vec.push(Box::new(peer_id.clone()));
            }

            if let Some(event_type) = &f.event_type {
                query.push_str(" AND event_type = ?");
                let type_str = format!("{:?}", event_type);
                params_vec.push(Box::new(type_str));
            }

            if let Some(severity) = &f.severity {
                query.push_str(" AND severity = ?");
                let severity_str = format!("{:?}", severity);
                params_vec.push(Box::new(severity_str));
            }

            if let Some(start_date) = f.start_date {
                query.push_str(" AND timestamp >= ?");
                params_vec.push(Box::new(start_date.timestamp()));
            }

            if let Some(end_date) = f.end_date {
                query.push_str(" AND timestamp <= ?");
                params_vec.push(Box::new(end_date.timestamp()));
            }

            if let Some(request_id) = f.request_id {
                query.push_str(" AND request_id = ?");
                params_vec.push(Box::new(request_id.to_string()));
            }

            query.push_str(" ORDER BY timestamp DESC LIMIT 1000");
            (query, params_vec)
        } else {
            ("SELECT * FROM audit_log ORDER BY timestamp DESC LIMIT 1000".to_string(), Vec::new())
        };

        let mut stmt = conn.prepare(&query)
            .map_err(|e| CommandError::Internal(format!("Failed to prepare statement: {}", e)))?;

        let params_refs: Vec<&dyn rusqlite::ToSql> = params_vec.iter().map(|p| p.as_ref()).collect();
        
        let entries = stmt.query_map(params_refs.as_slice(), |row| {
            self.deserialize_entry(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        }).map_err(|e| CommandError::Internal(format!("Failed to query audit log: {}", e)))?;

        let mut result = Vec::new();
        for entry in entries {
            result.push(entry.map_err(|e| CommandError::Internal(format!("Failed to deserialize entry: {}", e)))?);
        }

        Ok(result)
    }

    async fn get_authorization_history(&self, peer_id: &PeerId) -> CmdResult<Vec<AuditLogEntry>> {
        let filter = AuditFilter {
            peer_id: Some(peer_id.clone()),
            event_type: None, // Get all authorization-related events
            severity: None,
            start_date: None,
            end_date: None,
            request_id: None,
        };

        let logs = self.get_logs(Some(filter)).await?;
        
        // Filter to only authorization-related events
        let auth_logs: Vec<AuditLogEntry> = logs.into_iter()
            .filter(|log| matches!(
                log.event_type,
                AuditEventType::AuthorizationRequest
                | AuditEventType::AuthorizationApproved
                | AuditEventType::AuthorizationDenied
                | AuditEventType::AuthorizationTimeout
                | AuditEventType::AuthorizationModified
            ))
            .collect();

        Ok(auth_logs)
    }

    async fn get_security_events(&self) -> CmdResult<Vec<AuditLogEntry>> {
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        let mut stmt = conn.prepare(
            "SELECT * FROM audit_log 
             WHERE event_type IN ('SecurityViolationAttempt', 'SandboxEscape', 'UnauthorizedAccess')
             OR severity = 'Critical'
             ORDER BY timestamp DESC LIMIT 1000"
        ).map_err(|e| CommandError::Internal(format!("Failed to prepare statement: {}", e)))?;

        let entries = stmt.query_map([], |row| {
            self.deserialize_entry(row).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
        }).map_err(|e| CommandError::Internal(format!("Failed to query security events: {}", e)))?;

        let mut result = Vec::new();
        for entry in entries {
            result.push(entry.map_err(|e| CommandError::Internal(format!("Failed to deserialize entry: {}", e)))?);
        }

        Ok(result)
    }

    async fn rotate_logs(&self, retention_days: u32) -> CmdResult<usize> {
        let conn = self.connection.lock()
            .map_err(|e| CommandError::Internal(format!("Failed to lock connection: {}", e)))?;

        let cutoff_timestamp = Utc::now().timestamp() - (retention_days as i64 * 86400);

        // Archive critical events before deletion (optional enhancement)
        // For now, just delete old entries
        let deleted = conn.execute(
            "DELETE FROM audit_log WHERE timestamp < ? AND severity != 'Critical'",
            params![cutoff_timestamp],
        ).map_err(|e| CommandError::Internal(format!("Failed to rotate logs: {}", e)))?;

        Ok(deleted)
    }

    async fn export_logs(&self, filter: Option<AuditFilter>) -> CmdResult<String> {
        let logs = self.get_logs(filter).await?;

        serde_json::to_string_pretty(&logs)
            .map_err(|e| CommandError::Internal(format!("Failed to serialize audit logs: {}", e)))
    }
}

/// Helper function to create an authorization audit log entry
pub fn create_authorization_log(
    event_type: AuditEventType,
    peer_id: PeerId,
    request_id: RequestId,
    command_preview: String,
    risk_level: RiskLevel,
    decision: Option<String>,
    decided_by: Option<String>,
    details: String,
) -> AuditLogEntry {
    let severity = match event_type {
        AuditEventType::AuthorizationDenied => AuditSeverity::Warning,
        AuditEventType::AuthorizationTimeout => AuditSeverity::Warning,
        AuditEventType::SecurityViolationAttempt => AuditSeverity::Critical,
        AuditEventType::SandboxEscape => AuditSeverity::Critical,
        AuditEventType::UnauthorizedAccess => AuditSeverity::Critical,
        _ => AuditSeverity::Info,
    };

    AuditLogEntry {
        log_id: uuid::Uuid::new_v4(),
        event_type,
        timestamp: Utc::now(),
        peer_id,
        request_id: Some(request_id),
        command_preview: Some(command_preview),
        risk_level: Some(risk_level),
        decision,
        decided_by,
        details,
        severity,
    }
}

/// Helper function to create a security event audit log entry
pub fn create_security_event_log(
    event_type: AuditEventType,
    peer_id: PeerId,
    details: String,
) -> AuditLogEntry {
    AuditLogEntry {
        log_id: uuid::Uuid::new_v4(),
        event_type,
        timestamp: Utc::now(),
        peer_id,
        request_id: None,
        command_preview: None,
        risk_level: None,
        decision: None,
        decided_by: None,
        details,
        severity: AuditSeverity::Critical,
    }
}
