# Command Execution Module

This module provides secure remote command execution capabilities with comprehensive history tracking and audit logging.

## Features

### Command History
- SQLite-based storage for command execution history
- Stores command requests, results, authorization decisions, and execution status
- Configurable retention periods for automatic cleanup
- Full-text search within command history
- Advanced filtering by peer, command type, execution status, date range, and risk level
- JSON export for analysis and reporting

### Audit Logging
- Comprehensive audit trail for all authorization decisions
- Security event logging (violations, unauthorized access attempts, sandbox escapes)
- Severity-based event classification (Info, Warning, Critical)
- Log rotation with configurable retention
- Secure storage with indexed queries for performance
- Export capabilities for compliance and analysis

## Usage Example

```rust
use kizuna::command_execution::{
    SqliteHistoryManager, SqliteAuditLogger,
    CommandHistoryEntry, AuditLogEntry, AuditEventType,
    create_authorization_log, RiskLevel,
};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize history manager
    let history_db = PathBuf::from("command_history.db");
    let history_manager = SqliteHistoryManager::new(history_db)?;

    // Initialize audit logger
    let audit_db = PathBuf::from("audit_log.db");
    let audit_logger = SqliteAuditLogger::new(audit_db)?;

    // Log an authorization event
    let auth_log = create_authorization_log(
        AuditEventType::AuthorizationApproved,
        "peer-123".to_string(),
        uuid::Uuid::new_v4(),
        "ls -la".to_string(),
        RiskLevel::Low,
        Some("Approved".to_string()),
        Some("user@example.com".to_string()),
        "User approved directory listing command".to_string(),
    );
    audit_logger.log_event(auth_log).await?;

    // Get command history
    let history = history_manager.get_history(Some(10)).await?;
    println!("Recent commands: {}", history.len());

    // Search command history
    let search_results = history_manager.search_history("ls").await?;
    println!("Commands matching 'ls': {}", search_results.len());

    // Get authorization history for a peer
    let auth_history = audit_logger
        .get_authorization_history(&"peer-123".to_string())
        .await?;
    println!("Authorization events for peer-123: {}", auth_history.len());

    // Get security events
    let security_events = audit_logger.get_security_events().await?;
    println!("Security events: {}", security_events.len());

    // Cleanup old entries (older than 30 days)
    let deleted = history_manager.cleanup_old_entries(30).await?;
    println!("Deleted {} old history entries", deleted);

    // Rotate audit logs (keep 90 days)
    let rotated = audit_logger.rotate_logs(90).await?;
    println!("Rotated {} audit log entries", rotated);

    Ok(())
}
```

## Database Schema

### Command History Table
- `entry_id`: Unique identifier for the history entry
- `request_id`: ID of the command request
- `command`: The command that was executed
- `arguments`: Command arguments (JSON)
- `working_directory`: Working directory for execution
- `environment`: Environment variables (JSON)
- `timeout_secs`: Execution timeout in seconds
- `sandbox_config`: Sandbox configuration (JSON)
- `requester`: Peer ID of the requester
- `created_at`: Timestamp when command was created
- `completed_at`: Timestamp when command completed
- `exit_code`: Command exit code
- `stdout`: Standard output
- `stderr`: Standard error
- `execution_time_ms`: Execution time in milliseconds
- `resource_usage`: Resource usage statistics (JSON)
- `authorization_decision`: Authorization decision (JSON)
- `authorization_decided_at`: Authorization timestamp
- `authorization_decided_by`: Who made the authorization decision
- `execution_status`: Current execution status (JSON)
- `command_type`: Type of command
- `risk_level`: Risk level assessment

### Audit Log Table
- `log_id`: Unique identifier for the audit entry
- `event_type`: Type of audit event
- `timestamp`: When the event occurred
- `peer_id`: Peer ID associated with the event
- `request_id`: Related command request ID (optional)
- `command_preview`: Preview of the command (optional)
- `risk_level`: Risk level (optional)
- `decision`: Authorization decision (optional)
- `decided_by`: Who made the decision (optional)
- `details`: Detailed description of the event
- `severity`: Event severity (Info, Warning, Critical)

## Performance Considerations

- Indexes are created on frequently queried columns (timestamp, peer_id, event_type, etc.)
- Query results are limited to prevent memory issues
- Configurable retention periods help manage database size
- Log rotation preserves critical events while cleaning up old data
