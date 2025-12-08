//! Audit Logging for Browser Operations
//!
//! This module provides comprehensive audit logging for browser client operations
//! and security events.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::security::PeerId;
use crate::browser_support::error::{BrowserResult, BrowserSupportError};
use crate::browser_support::security_integration::BrowserOperation;

/// Audit log entry for browser operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserAuditEntry {
    /// Unique entry ID
    pub entry_id: Uuid,
    /// Timestamp of the event
    pub timestamp: u64,
    /// Browser session ID
    pub session_id: Uuid,
    /// Peer ID of the browser client
    pub peer_id: String,
    /// Event type
    pub event_type: BrowserAuditEventType,
    /// Operation that was performed
    pub operation: Option<String>,
    /// Whether the operation was allowed
    pub allowed: bool,
    /// Result of the operation
    pub result: BrowserAuditResult,
    /// Additional metadata
    pub metadata: serde_json::Value,
    /// IP address or origin
    pub origin: Option<String>,
}

/// Browser audit event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserAuditEventType {
    /// Authentication attempt
    Authentication,
    /// Session creation
    SessionCreated,
    /// Session refresh
    SessionRefreshed,
    /// Session revoked
    SessionRevoked,
    /// Permission check
    PermissionCheck,
    /// File transfer operation
    FileTransfer,
    /// Clipboard sync operation
    ClipboardSync,
    /// Command execution
    CommandExecution,
    /// Camera streaming
    CameraStreaming,
    /// System info access
    SystemInfoAccess,
    /// Security violation
    SecurityViolation,
    /// Configuration change
    ConfigurationChange,
}

/// Result of an audited operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserAuditResult {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failure(String),
    /// Operation was denied
    Denied(String),
}

impl BrowserAuditEntry {
    /// Create a new audit entry
    pub fn new(
        session_id: Uuid,
        peer_id: String,
        event_type: BrowserAuditEventType,
        allowed: bool,
        result: BrowserAuditResult,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            entry_id: Uuid::new_v4(),
            timestamp,
            session_id,
            peer_id,
            event_type,
            operation: None,
            allowed,
            result,
            metadata: serde_json::Value::Null,
            origin: None,
        }
    }
    
    /// Set operation name
    pub fn with_operation(mut self, operation: String) -> Self {
        self.operation = Some(operation);
        self
    }
    
    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
    
    /// Set origin
    pub fn with_origin(mut self, origin: String) -> Self {
        self.origin = Some(origin);
        self
    }
}

/// Browser audit logger
pub struct BrowserAuditLogger {
    /// Audit log entries (in-memory circular buffer)
    entries: Arc<RwLock<VecDeque<BrowserAuditEntry>>>,
    /// Maximum number of entries to keep
    max_entries: usize,
    /// Whether to log to console
    console_logging: bool,
}

impl BrowserAuditLogger {
    /// Create a new audit logger
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(max_entries))),
            max_entries,
            console_logging: false,
        }
    }
    
    /// Create with default settings (10000 entries)
    pub fn with_defaults() -> Self {
        Self::new(10000)
    }
    
    /// Enable console logging
    pub fn with_console_logging(mut self, enabled: bool) -> Self {
        self.console_logging = enabled;
        self
    }
    
    /// Log an audit entry
    pub async fn log(&self, entry: BrowserAuditEntry) -> BrowserResult<()> {
        // Log to console if enabled
        if self.console_logging {
            println!("[AUDIT] {:?} - {:?} - {:?}", entry.event_type, entry.peer_id, entry.result);
        }
        
        // Add to in-memory log
        let mut entries = self.entries.write().await;
        
        // Remove oldest entry if at capacity
        if entries.len() >= self.max_entries {
            entries.pop_front();
        }
        
        entries.push_back(entry);
        
        Ok(())
    }
    
    /// Log an authentication event
    pub async fn log_authentication(
        &self,
        session_id: Uuid,
        peer_id: &PeerId,
        success: bool,
        origin: Option<String>,
    ) -> BrowserResult<()> {
        let result = if success {
            BrowserAuditResult::Success
        } else {
            BrowserAuditResult::Failure("Authentication failed".to_string())
        };
        
        let entry = BrowserAuditEntry::new(
            session_id,
            peer_id.to_string(),
            BrowserAuditEventType::Authentication,
            success,
            result,
        );
        
        let entry = if let Some(origin) = origin {
            entry.with_origin(origin)
        } else {
            entry
        };
        
        self.log(entry).await
    }
    
    /// Log a session event
    pub async fn log_session_event(
        &self,
        session_id: Uuid,
        peer_id: &PeerId,
        event_type: BrowserAuditEventType,
    ) -> BrowserResult<()> {
        let entry = BrowserAuditEntry::new(
            session_id,
            peer_id.to_string(),
            event_type,
            true,
            BrowserAuditResult::Success,
        );
        
        self.log(entry).await
    }
    
    /// Log a permission check
    pub async fn log_permission_check(
        &self,
        session_id: Uuid,
        peer_id: &PeerId,
        operation: BrowserOperation,
        allowed: bool,
    ) -> BrowserResult<()> {
        let operation_name = format!("{:?}", operation);
        
        let result = if allowed {
            BrowserAuditResult::Success
        } else {
            BrowserAuditResult::Denied(format!("Permission denied for {:?}", operation))
        };
        
        let entry = BrowserAuditEntry::new(
            session_id,
            peer_id.to_string(),
            BrowserAuditEventType::PermissionCheck,
            allowed,
            result,
        ).with_operation(operation_name);
        
        self.log(entry).await
    }
    
    /// Log an operation
    pub async fn log_operation(
        &self,
        session_id: Uuid,
        peer_id: &PeerId,
        event_type: BrowserAuditEventType,
        operation: String,
        success: bool,
        error: Option<String>,
    ) -> BrowserResult<()> {
        let result = if success {
            BrowserAuditResult::Success
        } else {
            BrowserAuditResult::Failure(error.unwrap_or_else(|| "Operation failed".to_string()))
        };
        
        let entry = BrowserAuditEntry::new(
            session_id,
            peer_id.to_string(),
            event_type,
            success,
            result,
        ).with_operation(operation);
        
        self.log(entry).await
    }
    
    /// Log a security violation
    pub async fn log_security_violation(
        &self,
        session_id: Uuid,
        peer_id: &PeerId,
        violation: String,
        metadata: serde_json::Value,
    ) -> BrowserResult<()> {
        let entry = BrowserAuditEntry::new(
            session_id,
            peer_id.to_string(),
            BrowserAuditEventType::SecurityViolation,
            false,
            BrowserAuditResult::Denied(violation.clone()),
        )
        .with_operation(violation)
        .with_metadata(metadata);
        
        self.log(entry).await
    }
    
    /// Get recent audit entries
    pub async fn get_recent_entries(&self, limit: usize) -> Vec<BrowserAuditEntry> {
        let entries = self.entries.read().await;
        
        entries.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get entries for a specific session
    pub async fn get_session_entries(&self, session_id: &Uuid) -> Vec<BrowserAuditEntry> {
        let entries = self.entries.read().await;
        
        entries.iter()
            .filter(|e| &e.session_id == session_id)
            .cloned()
            .collect()
    }
    
    /// Get entries for a specific peer
    pub async fn get_peer_entries(&self, peer_id: &PeerId) -> Vec<BrowserAuditEntry> {
        let entries = self.entries.read().await;
        let peer_id_str = peer_id.to_string();
        
        entries.iter()
            .filter(|e| e.peer_id == peer_id_str)
            .cloned()
            .collect()
    }
    
    /// Get entries by event type
    pub async fn get_entries_by_type(&self, event_type: BrowserAuditEventType) -> Vec<BrowserAuditEntry> {
        let entries = self.entries.read().await;
        
        entries.iter()
            .filter(|e| std::mem::discriminant(&e.event_type) == std::mem::discriminant(&event_type))
            .cloned()
            .collect()
    }
    
    /// Get failed operations
    pub async fn get_failed_operations(&self, limit: usize) -> Vec<BrowserAuditEntry> {
        let entries = self.entries.read().await;
        
        entries.iter()
            .rev()
            .filter(|e| matches!(e.result, BrowserAuditResult::Failure(_) | BrowserAuditResult::Denied(_)))
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get security violations
    pub async fn get_security_violations(&self, limit: usize) -> Vec<BrowserAuditEntry> {
        let entries = self.entries.read().await;
        
        entries.iter()
            .rev()
            .filter(|e| matches!(e.event_type, BrowserAuditEventType::SecurityViolation))
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Clear all audit entries
    pub async fn clear(&self) -> BrowserResult<()> {
        let mut entries = self.entries.write().await;
        entries.clear();
        Ok(())
    }
    
    /// Get total entry count
    pub async fn entry_count(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }
    
    /// Export audit log to JSON
    pub async fn export_to_json(&self) -> BrowserResult<String> {
        let entries = self.entries.read().await;
        
        serde_json::to_string_pretty(&*entries)
            .map_err(|e| BrowserSupportError::SecurityError {
                message: format!("Failed to export audit log: {}", e)
            })
    }
}

impl Default for BrowserAuditLogger {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::identity::DeviceIdentity;
    
    #[tokio::test]
    async fn test_audit_logger() {
        let logger = BrowserAuditLogger::with_defaults();
        
        let session_id = Uuid::new_v4();
        let identity = DeviceIdentity::generate().unwrap();
        let peer_id = identity.derive_peer_id();
        
        // Log authentication
        logger.log_authentication(session_id, &peer_id, true, Some("https://example.com".to_string()))
            .await
            .unwrap();
        
        // Log permission check
        logger.log_permission_check(session_id, &peer_id, BrowserOperation::FileTransfer, true)
            .await
            .unwrap();
        
        // Check entries
        assert_eq!(logger.entry_count().await, 2);
        
        let recent = logger.get_recent_entries(10).await;
        assert_eq!(recent.len(), 2);
    }
    
    #[tokio::test]
    async fn test_session_filtering() {
        let logger = BrowserAuditLogger::with_defaults();
        
        let session1 = Uuid::new_v4();
        let session2 = Uuid::new_v4();
        let identity = DeviceIdentity::generate().unwrap();
        let peer_id = identity.derive_peer_id();
        
        // Log events for different sessions
        logger.log_authentication(session1, &peer_id, true, None).await.unwrap();
        logger.log_authentication(session2, &peer_id, true, None).await.unwrap();
        logger.log_permission_check(session1, &peer_id, BrowserOperation::ClipboardSync, true).await.unwrap();
        
        // Get entries for session1
        let session1_entries = logger.get_session_entries(&session1).await;
        assert_eq!(session1_entries.len(), 2);
        
        // Get entries for session2
        let session2_entries = logger.get_session_entries(&session2).await;
        assert_eq!(session2_entries.len(), 1);
    }
    
    #[tokio::test]
    async fn test_security_violations() {
        let logger = BrowserAuditLogger::with_defaults();
        
        let session_id = Uuid::new_v4();
        let identity = DeviceIdentity::generate().unwrap();
        let peer_id = identity.derive_peer_id();
        
        // Log security violation
        logger.log_security_violation(
            session_id,
            &peer_id,
            "Unauthorized access attempt".to_string(),
            serde_json::json!({"ip": "192.168.1.1"}),
        ).await.unwrap();
        
        // Get security violations
        let violations = logger.get_security_violations(10).await;
        assert_eq!(violations.len(), 1);
        assert!(matches!(violations[0].event_type, BrowserAuditEventType::SecurityViolation));
    }
    
    #[tokio::test]
    async fn test_failed_operations() {
        let logger = BrowserAuditLogger::with_defaults();
        
        let session_id = Uuid::new_v4();
        let identity = DeviceIdentity::generate().unwrap();
        let peer_id = identity.derive_peer_id();
        
        // Log failed operation
        logger.log_operation(
            session_id,
            &peer_id,
            BrowserAuditEventType::FileTransfer,
            "upload_file".to_string(),
            false,
            Some("File too large".to_string()),
        ).await.unwrap();
        
        // Get failed operations
        let failed = logger.get_failed_operations(10).await;
        assert_eq!(failed.len(), 1);
    }
    
    #[tokio::test]
    async fn test_export_to_json() {
        let logger = BrowserAuditLogger::with_defaults();
        
        let session_id = Uuid::new_v4();
        let identity = DeviceIdentity::generate().unwrap();
        let peer_id = identity.derive_peer_id();
        
        logger.log_authentication(session_id, &peer_id, true, None).await.unwrap();
        
        let json = logger.export_to_json().await.unwrap();
        assert!(!json.is_empty());
        assert!(json.contains("Authentication"));
    }
}
