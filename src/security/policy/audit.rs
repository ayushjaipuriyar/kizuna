use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::security::error::SecurityResult;
use crate::security::identity::PeerId;
use super::{SecurityEvent, SecurityEventType};

/// Audit log entry with additional metadata
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub event: SecurityEvent,
    pub severity: Severity,
}

/// Severity level for audit events
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl AuditLogEntry {
    pub fn new(event: SecurityEvent, severity: Severity) -> Self {
        Self { event, severity }
    }
}

/// Audit log configuration
#[derive(Clone, Debug)]
pub struct AuditConfig {
    /// Maximum number of entries to keep in memory
    pub max_memory_entries: usize,
    /// Whether to persist logs to disk
    pub persist_to_disk: bool,
    /// Path to log file
    pub log_file_path: Option<PathBuf>,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            max_memory_entries: 1000,
            persist_to_disk: false,
            log_file_path: None,
        }
    }
}

/// Security auditor for logging and monitoring security events
pub struct SecurityAuditor {
    /// Configuration
    config: Arc<RwLock<AuditConfig>>,
    /// In-memory audit log (circular buffer)
    log_entries: Arc<RwLock<VecDeque<AuditLogEntry>>>,
}

impl SecurityAuditor {
    /// Create a new security auditor with default configuration
    pub fn new() -> Self {
        Self::with_config(AuditConfig::default())
    }
    
    /// Create a new security auditor with custom configuration
    pub fn with_config(config: AuditConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            log_entries: Arc::new(RwLock::new(VecDeque::new())),
        }
    }
    
    /// Log a security event
    pub fn log_event(&self, event: SecurityEvent) -> SecurityResult<()> {
        let severity = Self::determine_severity(&event.event_type);
        let entry = AuditLogEntry::new(event, severity);
        
        self.add_entry(entry)
    }
    
    /// Add an audit log entry
    fn add_entry(&self, entry: AuditLogEntry) -> SecurityResult<()> {
        let config = self.config.read().unwrap();
        let mut log_entries = self.log_entries.write().unwrap();
        
        // Add to in-memory log
        log_entries.push_back(entry.clone());
        
        // Maintain max size (circular buffer)
        while log_entries.len() > config.max_memory_entries {
            log_entries.pop_front();
        }
        
        // Persist to disk if configured
        if config.persist_to_disk {
            if let Some(ref log_path) = config.log_file_path {
                self.persist_entry(&entry, log_path)?;
            }
        }
        
        Ok(())
    }
    
    /// Persist an entry to disk
    fn persist_entry(&self, entry: &AuditLogEntry, log_path: &PathBuf) -> SecurityResult<()> {
        use std::fs::OpenOptions;
        use std::io::Write;
        
        let json = serde_json::to_string(entry)?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        
        writeln!(file, "{}", json)?;
        
        Ok(())
    }
    
    /// Determine severity based on event type
    fn determine_severity(event_type: &SecurityEventType) -> Severity {
        match event_type {
            SecurityEventType::ConnectionAttempt => Severity::Info,
            SecurityEventType::ConnectionAccepted => Severity::Info,
            SecurityEventType::ConnectionRejected => Severity::Warning,
            SecurityEventType::PairingAttempt => Severity::Info,
            SecurityEventType::PairingSuccess => Severity::Info,
            SecurityEventType::PairingFailure => Severity::Warning,
            SecurityEventType::RateLimitExceeded => Severity::Critical,
            SecurityEventType::SuspiciousActivity => Severity::Critical,
            SecurityEventType::PolicyViolation => Severity::Warning,
        }
    }
    
    /// Get recent audit log entries
    pub fn get_recent_entries(&self, limit: usize) -> Vec<AuditLogEntry> {
        let log_entries = self.log_entries.read().unwrap();
        
        log_entries.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get all audit log entries
    pub fn get_all_entries(&self) -> Vec<AuditLogEntry> {
        let log_entries = self.log_entries.read().unwrap();
        log_entries.iter().cloned().collect()
    }
    
    /// Get entries for a specific peer
    pub fn get_entries_for_peer(&self, peer_id: &PeerId, limit: usize) -> Vec<AuditLogEntry> {
        let log_entries = self.log_entries.read().unwrap();
        
        log_entries.iter()
            .rev()
            .filter(|entry| {
                if let Some(ref event_peer_id) = entry.event.peer_id {
                    event_peer_id == peer_id
                } else {
                    false
                }
            })
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get entries by event type
    pub fn get_entries_by_type(&self, event_type: SecurityEventType, limit: usize) -> Vec<AuditLogEntry> {
        let log_entries = self.log_entries.read().unwrap();
        
        log_entries.iter()
            .rev()
            .filter(|entry| {
                std::mem::discriminant(&entry.event.event_type) == std::mem::discriminant(&event_type)
            })
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get entries by severity
    pub fn get_entries_by_severity(&self, severity: Severity, limit: usize) -> Vec<AuditLogEntry> {
        let log_entries = self.log_entries.read().unwrap();
        
        log_entries.iter()
            .rev()
            .filter(|entry| entry.severity == severity)
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get critical events
    pub fn get_critical_events(&self, limit: usize) -> Vec<AuditLogEntry> {
        self.get_entries_by_severity(Severity::Critical, limit)
    }
    
    /// Clear all audit log entries
    pub fn clear(&self) -> SecurityResult<()> {
        let mut log_entries = self.log_entries.write().unwrap();
        log_entries.clear();
        Ok(())
    }
    
    /// Get count of entries
    pub fn entry_count(&self) -> usize {
        let log_entries = self.log_entries.read().unwrap();
        log_entries.len()
    }
    
    /// Update configuration
    pub fn update_config(&self, config: AuditConfig) -> SecurityResult<()> {
        let mut current_config = self.config.write().unwrap();
        *current_config = config;
        Ok(())
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> AuditConfig {
        let config = self.config.read().unwrap();
        config.clone()
    }
}

impl Default for SecurityAuditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Audit log wrapper for easier access
pub struct AuditLog {
    auditor: Arc<SecurityAuditor>,
}

impl AuditLog {
    pub fn new(auditor: Arc<SecurityAuditor>) -> Self {
        Self { auditor }
    }
    
    pub fn log(&self, event: SecurityEvent) -> SecurityResult<()> {
        self.auditor.log_event(event)
    }
    
    pub fn get_recent(&self, limit: usize) -> Vec<AuditLogEntry> {
        self.auditor.get_recent_entries(limit)
    }
    
    pub fn get_for_peer(&self, peer_id: &PeerId, limit: usize) -> Vec<AuditLogEntry> {
        self.auditor.get_entries_for_peer(peer_id, limit)
    }
    
    pub fn get_critical(&self, limit: usize) -> Vec<AuditLogEntry> {
        self.auditor.get_critical_events(limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_log_event() {
        let auditor = SecurityAuditor::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        let event = SecurityEvent::new(
            SecurityEventType::ConnectionAttempt,
            Some(peer_id),
            "Test connection".to_string(),
        );
        
        auditor.log_event(event).unwrap();
        assert_eq!(auditor.entry_count(), 1);
    }
    
    #[test]
    fn test_get_recent_entries() {
        let auditor = SecurityAuditor::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Log multiple events
        for i in 0..5 {
            let event = SecurityEvent::new(
                SecurityEventType::ConnectionAttempt,
                Some(peer_id.clone()),
                format!("Connection {}", i),
            );
            auditor.log_event(event).unwrap();
        }
        
        let recent = auditor.get_recent_entries(3);
        assert_eq!(recent.len(), 3);
    }
    
    #[test]
    fn test_get_entries_for_peer() {
        let auditor = SecurityAuditor::new();
        let peer1 = PeerId::from_string("peer1").unwrap();
        let peer2 = PeerId::from_string("peer2").unwrap();
        
        // Log events for different peers
        for _ in 0..3 {
            let event = SecurityEvent::new(
                SecurityEventType::ConnectionAttempt,
                Some(peer1.clone()),
                "Peer 1 connection".to_string(),
            );
            auditor.log_event(event).unwrap();
        }
        
        for _ in 0..2 {
            let event = SecurityEvent::new(
                SecurityEventType::ConnectionAttempt,
                Some(peer2.clone()),
                "Peer 2 connection".to_string(),
            );
            auditor.log_event(event).unwrap();
        }
        
        let peer1_entries = auditor.get_entries_for_peer(&peer1, 10);
        assert_eq!(peer1_entries.len(), 3);
        
        let peer2_entries = auditor.get_entries_for_peer(&peer2, 10);
        assert_eq!(peer2_entries.len(), 2);
    }
    
    #[test]
    fn test_severity_classification() {
        let auditor = SecurityAuditor::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Log critical event
        let critical_event = SecurityEvent::new(
            SecurityEventType::RateLimitExceeded,
            Some(peer_id.clone()),
            "Rate limit exceeded".to_string(),
        );
        auditor.log_event(critical_event).unwrap();
        
        // Log info event
        let info_event = SecurityEvent::new(
            SecurityEventType::ConnectionAccepted,
            Some(peer_id),
            "Connection accepted".to_string(),
        );
        auditor.log_event(info_event).unwrap();
        
        let critical = auditor.get_critical_events(10);
        assert_eq!(critical.len(), 1);
    }
    
    #[test]
    fn test_circular_buffer() {
        let config = AuditConfig {
            max_memory_entries: 5,
            persist_to_disk: false,
            log_file_path: None,
        };
        
        let auditor = SecurityAuditor::with_config(config);
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Log more than max entries
        for i in 0..10 {
            let event = SecurityEvent::new(
                SecurityEventType::ConnectionAttempt,
                Some(peer_id.clone()),
                format!("Connection {}", i),
            );
            auditor.log_event(event).unwrap();
        }
        
        // Should only keep last 5
        assert_eq!(auditor.entry_count(), 5);
    }
    
    #[test]
    fn test_clear() {
        let auditor = SecurityAuditor::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        let event = SecurityEvent::new(
            SecurityEventType::ConnectionAttempt,
            Some(peer_id),
            "Test".to_string(),
        );
        auditor.log_event(event).unwrap();
        
        assert_eq!(auditor.entry_count(), 1);
        
        auditor.clear().unwrap();
        assert_eq!(auditor.entry_count(), 0);
    }
}
