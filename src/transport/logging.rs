use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

use super::error::{TransportError, ErrorSeverity, ErrorCategory, ErrorContext, ContextualError};
use super::{PeerId, ConnectionInfo};

/// Comprehensive logging system for transport operations
#[derive(Debug)]
pub struct TransportLogger {
    /// Log entries organized by category
    logs: Arc<RwLock<HashMap<LogCategory, Vec<LogEntry>>>>,
    /// Configuration for logging behavior
    config: LoggingConfig,
    /// Performance metrics for logging operations
    metrics: Arc<RwLock<LoggingMetrics>>,
}

/// Configuration for transport logging
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Maximum number of log entries per category
    pub max_entries_per_category: usize,
    /// Minimum log level to record
    pub min_level: LogLevel,
    /// Enable structured logging with JSON format
    pub structured_logging: bool,
    /// Include stack traces for errors
    pub include_stack_traces: bool,
    /// Log rotation interval
    pub rotation_interval: Duration,
    /// Enable performance logging
    pub performance_logging: bool,
    /// Enable connection lifecycle logging
    pub connection_lifecycle_logging: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            max_entries_per_category: 1000,
            min_level: LogLevel::Info,
            structured_logging: false,
            include_stack_traces: true,
            rotation_interval: Duration::from_secs(3600), // 1 hour
            performance_logging: true,
            connection_lifecycle_logging: true,
        }
    }
}

/// Log levels for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Critical = 5,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Critical => write!(f, "CRITICAL"),
        }
    }
}

impl From<ErrorSeverity> for LogLevel {
    fn from(severity: ErrorSeverity) -> Self {
        match severity {
            ErrorSeverity::Info => LogLevel::Info,
            ErrorSeverity::Warning => LogLevel::Warn,
            ErrorSeverity::Error => LogLevel::Error,
            ErrorSeverity::Critical => LogLevel::Critical,
        }
    }
}

/// Categories for organizing log entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogCategory {
    Connection,
    Protocol,
    Network,
    Security,
    Performance,
    Error,
    Debug,
    Audit,
}

impl fmt::Display for LogCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogCategory::Connection => write!(f, "connection"),
            LogCategory::Protocol => write!(f, "protocol"),
            LogCategory::Network => write!(f, "network"),
            LogCategory::Security => write!(f, "security"),
            LogCategory::Performance => write!(f, "performance"),
            LogCategory::Error => write!(f, "error"),
            LogCategory::Debug => write!(f, "debug"),
            LogCategory::Audit => write!(f, "audit"),
        }
    }
}

/// Individual log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: SystemTime,
    pub level: LogLevel,
    pub category: LogCategory,
    pub message: String,
    pub peer_id: Option<PeerId>,
    pub protocol: Option<String>,
    pub operation: Option<String>,
    pub duration: Option<Duration>,
    pub metadata: HashMap<String, String>,
    pub error_context: Option<ErrorContext>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, category: LogCategory, message: String) -> Self {
        Self {
            timestamp: SystemTime::now(),
            level,
            category,
            message,
            peer_id: None,
            protocol: None,
            operation: None,
            duration: None,
            metadata: HashMap::new(),
            error_context: None,
        }
    }

    /// Add peer ID to the log entry
    pub fn with_peer_id(mut self, peer_id: PeerId) -> Self {
        self.peer_id = Some(peer_id);
        self
    }

    /// Add protocol information
    pub fn with_protocol(mut self, protocol: String) -> Self {
        self.protocol = Some(protocol);
        self
    }

    /// Add operation information
    pub fn with_operation(mut self, operation: String) -> Self {
        self.operation = Some(operation);
        self
    }

    /// Add duration information
    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }

    /// Add metadata
    pub fn with_metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Add error context
    pub fn with_error_context(mut self, context: ErrorContext) -> Self {
        self.error_context = Some(context);
        self
    }

    /// Format as human-readable string
    pub fn format_human_readable(&self) -> String {
        let timestamp = self.timestamp
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut parts = vec![
            format!("[{}]", self.level),
            format!("[{}]", self.category),
            format!("ts={}", timestamp),
        ];

        if let Some(peer_id) = &self.peer_id {
            parts.push(format!("peer={}", peer_id));
        }

        if let Some(protocol) = &self.protocol {
            parts.push(format!("proto={}", protocol));
        }

        if let Some(operation) = &self.operation {
            parts.push(format!("op={}", operation));
        }

        if let Some(duration) = &self.duration {
            parts.push(format!("dur={:?}", duration));
        }

        for (key, value) in &self.metadata {
            parts.push(format!("{}={}", key, value));
        }

        format!("{} {}", parts.join(" "), self.message)
    }

    /// Format as JSON string
    pub fn format_json(&self) -> String {
        let mut json_obj = serde_json::Map::new();
        
        json_obj.insert("timestamp".to_string(), 
            serde_json::Value::Number(serde_json::Number::from(
                self.timestamp.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
            ))
        );
        json_obj.insert("level".to_string(), serde_json::Value::String(self.level.to_string()));
        json_obj.insert("category".to_string(), serde_json::Value::String(self.category.to_string()));
        json_obj.insert("message".to_string(), serde_json::Value::String(self.message.clone()));

        if let Some(peer_id) = &self.peer_id {
            json_obj.insert("peer_id".to_string(), serde_json::Value::String(peer_id.clone()));
        }

        if let Some(protocol) = &self.protocol {
            json_obj.insert("protocol".to_string(), serde_json::Value::String(protocol.clone()));
        }

        if let Some(operation) = &self.operation {
            json_obj.insert("operation".to_string(), serde_json::Value::String(operation.clone()));
        }

        if let Some(duration) = &self.duration {
            json_obj.insert("duration_ms".to_string(), 
                serde_json::Value::Number(serde_json::Number::from(duration.as_millis() as u64))
            );
        }

        for (key, value) in &self.metadata {
            json_obj.insert(key.clone(), serde_json::Value::String(value.clone()));
        }

        serde_json::Value::Object(json_obj).to_string()
    }
}

/// Metrics for logging system performance
#[derive(Debug, Clone)]
pub struct LoggingMetrics {
    pub total_entries: u64,
    pub entries_by_level: HashMap<LogLevel, u64>,
    pub entries_by_category: HashMap<LogCategory, u64>,
    pub dropped_entries: u64,
    pub average_log_time: Duration,
    pub last_rotation: SystemTime,
}

impl Default for LoggingMetrics {
    fn default() -> Self {
        Self {
            total_entries: 0,
            entries_by_level: HashMap::new(),
            entries_by_category: HashMap::new(),
            dropped_entries: 0,
            average_log_time: Duration::ZERO,
            last_rotation: SystemTime::now(),
        }
    }
}

impl TransportLogger {
    /// Create a new transport logger
    pub fn new() -> Self {
        Self::with_config(LoggingConfig::default())
    }

    /// Create a new transport logger with custom configuration
    pub fn with_config(config: LoggingConfig) -> Self {
        Self {
            logs: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(RwLock::new(LoggingMetrics::default())),
        }
    }

    /// Log a message with specified level and category
    pub async fn log(&self, level: LogLevel, category: LogCategory, message: String) {
        if level < self.config.min_level {
            return;
        }

        let entry = LogEntry::new(level, category, message);
        self.add_log_entry(entry).await;
    }

    /// Log an error with context
    pub async fn log_error(&self, error: &ContextualError) {
        let level = LogLevel::from(error.error.severity());
        let category = match error.error.category() {
            ErrorCategory::Connection => LogCategory::Connection,
            ErrorCategory::Protocol => LogCategory::Protocol,
            ErrorCategory::Network => LogCategory::Network,
            ErrorCategory::Security => LogCategory::Security,
            _ => LogCategory::Error,
        };

        let entry = LogEntry::new(level, category, error.log_message())
            .with_error_context(error.context.clone());

        let mut entry = entry.with_error_context(error.context.clone());

        if let Some(peer_id) = &error.context.peer_id {
            entry = entry.with_peer_id(peer_id.clone());
        }

        if let Some(protocol) = &error.context.protocol {
            entry = entry.with_protocol(protocol.clone());
        }

        self.add_log_entry(entry).await;
    }

    /// Log connection lifecycle events
    pub async fn log_connection_event(&self, event: ConnectionEvent, info: &ConnectionInfo) {
        if !self.config.connection_lifecycle_logging {
            return;
        }

        let message = match event {
            ConnectionEvent::Established => format!("Connection established to {}", info.peer_id),
            ConnectionEvent::Closed => format!("Connection closed to {}", info.peer_id),
            ConnectionEvent::Failed(ref reason) => format!("Connection failed to {}: {}", info.peer_id, reason),
            ConnectionEvent::Upgraded(ref from, ref to) => {
                format!("Connection upgraded from {} to {} for peer {}", from, to, info.peer_id)
            },
        };

        let entry = LogEntry::new(LogLevel::Info, LogCategory::Connection, message)
            .with_peer_id(info.peer_id.clone())
            .with_protocol(info.protocol.clone())
            .with_metadata("local_addr", info.local_addr.to_string())
            .with_metadata("remote_addr", info.remote_addr.to_string())
            .with_metadata("bytes_sent", info.bytes_sent.to_string())
            .with_metadata("bytes_received", info.bytes_received.to_string());

        let mut entry = entry;

        if let Some(rtt) = info.rtt {
            entry = entry.with_metadata("rtt_ms", rtt.as_millis().to_string());
        }

        if let Some(bandwidth) = info.bandwidth {
            entry = entry.with_metadata("bandwidth_bps", bandwidth.to_string());
        }

        self.add_log_entry(entry).await;
    }

    /// Log performance metrics
    pub async fn log_performance(&self, operation: String, duration: Duration, metadata: HashMap<String, String>) {
        if !self.config.performance_logging {
            return;
        }

        let message = format!("Operation '{}' completed in {:?}", operation, duration);
        let mut entry = LogEntry::new(LogLevel::Debug, LogCategory::Performance, message)
            .with_operation(operation)
            .with_duration(duration);

        for (key, value) in metadata {
            entry = entry.with_metadata(key, value);
        }

        self.add_log_entry(entry).await;
    }

    /// Log protocol negotiation events
    pub async fn log_protocol_negotiation(&self, peer_id: &PeerId, offered: &[String], selected: Option<&str>) {
        let message = match selected {
            Some(protocol) => format!("Protocol negotiation successful: selected {}", protocol),
            None => "Protocol negotiation failed: no compatible protocol".to_string(),
        };

        let level = if selected.is_some() { LogLevel::Info } else { LogLevel::Warn };

        let entry = LogEntry::new(level, LogCategory::Protocol, message)
            .with_peer_id(peer_id.clone())
            .with_metadata("offered_protocols", offered.join(","))
            .with_metadata("selected_protocol", selected.unwrap_or("none").to_string());

        self.add_log_entry(entry).await;
    }

    /// Log security events
    pub async fn log_security_event(&self, event: SecurityEvent, peer_id: Option<&PeerId>) {
        let (level, message) = match event {
            SecurityEvent::AuthenticationSuccess => (LogLevel::Info, "Authentication successful".to_string()),
            SecurityEvent::AuthenticationFailure(ref reason) => {
                (LogLevel::Warn, format!("Authentication failed: {}", reason))
            },
            SecurityEvent::CertificateValidationFailure(ref reason) => {
                (LogLevel::Error, format!("Certificate validation failed: {}", reason))
            },
            SecurityEvent::SuspiciousActivity(ref description) => {
                (LogLevel::Warn, format!("Suspicious activity detected: {}", description))
            },
        };

        let mut entry = LogEntry::new(level, LogCategory::Security, message);

        if let Some(peer_id) = peer_id {
            entry = entry.with_peer_id(peer_id.clone());
        }

        self.add_log_entry(entry).await;
    }

    /// Get logs for a specific category
    pub async fn get_logs(&self, category: LogCategory) -> Vec<LogEntry> {
        let logs = self.logs.read().await;
        logs.get(&category).cloned().unwrap_or_default()
    }

    /// Get all logs
    pub async fn get_all_logs(&self) -> HashMap<LogCategory, Vec<LogEntry>> {
        self.logs.read().await.clone()
    }

    /// Get logging metrics
    pub async fn get_metrics(&self) -> LoggingMetrics {
        self.metrics.read().await.clone()
    }

    /// Clear logs for a specific category
    pub async fn clear_logs(&self, category: LogCategory) {
        let mut logs = self.logs.write().await;
        logs.remove(&category);
    }

    /// Clear all logs
    pub async fn clear_all_logs(&self) {
        let mut logs = self.logs.write().await;
        logs.clear();
    }

    /// Export logs as formatted strings
    pub async fn export_logs(&self, category: Option<LogCategory>) -> Vec<String> {
        let logs = self.logs.read().await;
        let mut result = Vec::new();

        let categories_to_export: Vec<LogCategory> = match category {
            Some(cat) => vec![cat],
            None => logs.keys().cloned().collect(),
        };

        for cat in categories_to_export {
            if let Some(entries) = logs.get(&cat) {
                for entry in entries {
                    let formatted = if self.config.structured_logging {
                        entry.format_json()
                    } else {
                        entry.format_human_readable()
                    };
                    result.push(formatted);
                }
            }
        }

        result.sort();
        result
    }

    /// Add a log entry to the appropriate category
    async fn add_log_entry(&self, entry: LogEntry) {
        let start_time = std::time::Instant::now();

        {
            let mut logs = self.logs.write().await;
            let category_logs = logs.entry(entry.category).or_insert_with(Vec::new);

            // Add the entry
            category_logs.push(entry.clone());

            // Trim if necessary
            if category_logs.len() > self.config.max_entries_per_category {
                category_logs.remove(0);
                
                // Update dropped entries metric
                let mut metrics = self.metrics.write().await;
                metrics.dropped_entries += 1;
            }
        }

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_entries += 1;
            *metrics.entries_by_level.entry(entry.level).or_insert(0) += 1;
            *metrics.entries_by_category.entry(entry.category).or_insert(0) += 1;

            // Update average log time
            let log_time = start_time.elapsed();
            if metrics.total_entries == 1 {
                metrics.average_log_time = log_time;
            } else {
                let total_time = metrics.average_log_time * (metrics.total_entries - 1) as u32 + log_time;
                metrics.average_log_time = total_time / metrics.total_entries as u32;
            }
        }

        // Print to console if appropriate
        if entry.level >= LogLevel::Info || (entry.level >= LogLevel::Debug && self.config.min_level <= LogLevel::Debug) {
            let formatted = if self.config.structured_logging {
                entry.format_json()
            } else {
                entry.format_human_readable()
            };
            println!("{}", formatted);
        }
    }
}

/// Connection lifecycle events
#[derive(Debug, Clone)]
pub enum ConnectionEvent {
    Established,
    Closed,
    Failed(String),
    Upgraded(String, String), // from protocol, to protocol
}

/// Security events for audit logging
#[derive(Debug, Clone)]
pub enum SecurityEvent {
    AuthenticationSuccess,
    AuthenticationFailure(String),
    CertificateValidationFailure(String),
    SuspiciousActivity(String),
}

impl Default for TransportLogger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    #[tokio::test]
    async fn test_basic_logging() {
        let logger = TransportLogger::new();
        
        logger.log(LogLevel::Info, LogCategory::Connection, "Test message".to_string()).await;
        
        let logs = logger.get_logs(LogCategory::Connection).await;
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "Test message");
        assert_eq!(logs[0].level, LogLevel::Info);
    }

    #[tokio::test]
    async fn test_log_filtering() {
        let mut config = LoggingConfig::default();
        config.min_level = LogLevel::Warn;
        let logger = TransportLogger::with_config(config);
        
        logger.log(LogLevel::Debug, LogCategory::Debug, "Debug message".to_string()).await;
        logger.log(LogLevel::Warn, LogCategory::Error, "Warning message".to_string()).await;
        
        let debug_logs = logger.get_logs(LogCategory::Debug).await;
        let error_logs = logger.get_logs(LogCategory::Error).await;
        
        assert_eq!(debug_logs.len(), 0); // Filtered out
        assert_eq!(error_logs.len(), 1); // Included
    }

    #[tokio::test]
    async fn test_connection_logging() {
        let logger = TransportLogger::new();
        
        let info = ConnectionInfo::new(
            "test-peer".to_string(),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080),
            SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080),
            "tcp".to_string(),
        );
        
        logger.log_connection_event(ConnectionEvent::Established, &info).await;
        
        let logs = logger.get_logs(LogCategory::Connection).await;
        assert_eq!(logs.len(), 1);
        assert!(logs[0].message.contains("Connection established"));
        assert_eq!(logs[0].peer_id, Some("test-peer".to_string()));
    }

    #[tokio::test]
    async fn test_log_rotation() {
        let mut config = LoggingConfig::default();
        config.max_entries_per_category = 2;
        let logger = TransportLogger::with_config(config);
        
        logger.log(LogLevel::Info, LogCategory::Debug, "Message 1".to_string()).await;
        logger.log(LogLevel::Info, LogCategory::Debug, "Message 2".to_string()).await;
        logger.log(LogLevel::Info, LogCategory::Debug, "Message 3".to_string()).await;
        
        let logs = logger.get_logs(LogCategory::Debug).await;
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].message, "Message 2"); // First message was dropped
        assert_eq!(logs[1].message, "Message 3");
        
        let metrics = logger.get_metrics().await;
        assert_eq!(metrics.dropped_entries, 1);
    }

    #[tokio::test]
    async fn test_structured_logging() {
        let mut config = LoggingConfig::default();
        config.structured_logging = true;
        let logger = TransportLogger::with_config(config);
        
        let entry = LogEntry::new(LogLevel::Info, LogCategory::Connection, "Test message".to_string())
            .with_peer_id("test-peer".to_string())
            .with_protocol("tcp".to_string());
        
        let json_output = entry.format_json();
        assert!(json_output.contains("\"level\":\"INFO\""));
        assert!(json_output.contains("\"peer_id\":\"test-peer\""));
        assert!(json_output.contains("\"protocol\":\"tcp\""));
    }
}