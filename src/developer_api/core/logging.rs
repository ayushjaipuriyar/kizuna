/// Logging and tracing integration for the Developer API
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Log level enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Trace level - very detailed information
    Trace,
    /// Debug level - detailed information for debugging
    Debug,
    /// Info level - general informational messages
    Info,
    /// Warning level - warning messages
    Warn,
    /// Error level - error messages
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "TRACE"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Log record containing all information about a log event
#[derive(Debug, Clone)]
pub struct LogRecord {
    /// Log level
    pub level: LogLevel,
    
    /// Log message
    pub message: String,
    
    /// Module path where the log originated
    pub module: String,
    
    /// File name where the log originated
    pub file: Option<String>,
    
    /// Line number where the log originated
    pub line: Option<u32>,
    
    /// Timestamp of the log event
    pub timestamp: SystemTime,
    
    /// Additional structured fields
    pub fields: HashMap<String, String>,
    
    /// Trace ID for distributed tracing
    pub trace_id: Option<String>,
    
    /// Span ID for distributed tracing
    pub span_id: Option<String>,
}

impl LogRecord {
    /// Creates a new log record
    pub fn new<S: Into<String>>(level: LogLevel, message: S, module: S) -> Self {
        Self {
            level,
            message: message.into(),
            module: module.into(),
            file: None,
            line: None,
            timestamp: SystemTime::now(),
            fields: HashMap::new(),
            trace_id: None,
            span_id: None,
        }
    }
    
    /// Adds a field to the log record
    pub fn with_field<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }
    
    /// Sets the trace ID
    pub fn with_trace_id<S: Into<String>>(mut self, trace_id: S) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }
    
    /// Sets the span ID
    pub fn with_span_id<S: Into<String>>(mut self, span_id: S) -> Self {
        self.span_id = Some(span_id.into());
        self
    }
    
    /// Sets the file and line information
    pub fn with_location<S: Into<String>>(mut self, file: S, line: u32) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self
    }
}

/// Logger trait for custom logging implementations
pub trait Logger: Send + Sync {
    /// Logs a record
    fn log(&self, record: &LogRecord);
    
    /// Returns whether the given log level is enabled
    fn enabled(&self, level: LogLevel) -> bool;
    
    /// Flushes any buffered log records
    fn flush(&self);
}

/// Console logger that writes to stderr
pub struct ConsoleLogger {
    min_level: LogLevel,
}

impl ConsoleLogger {
    /// Creates a new console logger
    pub fn new(min_level: LogLevel) -> Self {
        Self { min_level }
    }
}

impl Logger for ConsoleLogger {
    fn log(&self, record: &LogRecord) {
        if record.level < self.min_level {
            return;
        }
        
        let timestamp = record.timestamp
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default();
        
        let mut output = format!(
            "[{}.{:03}] {} [{}] {}",
            timestamp.as_secs(),
            timestamp.subsec_millis(),
            record.level,
            record.module,
            record.message
        );
        
        if !record.fields.is_empty() {
            output.push_str(" {");
            for (i, (key, value)) in record.fields.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format!("{}: {}", key, value));
            }
            output.push('}');
        }
        
        if let Some(trace_id) = &record.trace_id {
            output.push_str(&format!(" [trace: {}]", trace_id));
        }
        
        eprintln!("{}", output);
    }
    
    fn enabled(&self, level: LogLevel) -> bool {
        level >= self.min_level
    }
    
    fn flush(&self) {
        // Console output is not buffered
    }
}

/// Structured logger that collects logs in memory
pub struct StructuredLogger {
    min_level: LogLevel,
    records: Arc<Mutex<Vec<LogRecord>>>,
}

impl StructuredLogger {
    /// Creates a new structured logger
    pub fn new(min_level: LogLevel) -> Self {
        Self {
            min_level,
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Gets all collected log records
    pub fn get_records(&self) -> Vec<LogRecord> {
        self.records.lock().unwrap().clone()
    }
    
    /// Clears all collected log records
    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
    }
}

impl Logger for StructuredLogger {
    fn log(&self, record: &LogRecord) {
        if record.level < self.min_level {
            return;
        }
        
        self.records.lock().unwrap().push(record.clone());
    }
    
    fn enabled(&self, level: LogLevel) -> bool {
        level >= self.min_level
    }
    
    fn flush(&self) {
        // Records are already in memory
    }
}

/// Global logger instance
static GLOBAL_LOGGER: Mutex<Option<Arc<dyn Logger>>> = Mutex::new(None);

/// Sets the global logger
pub fn set_logger(logger: Arc<dyn Logger>) {
    *GLOBAL_LOGGER.lock().unwrap() = Some(logger);
}

/// Gets the global logger
pub fn get_logger() -> Option<Arc<dyn Logger>> {
    GLOBAL_LOGGER.lock().unwrap().clone()
}

/// Logs a message at the given level
pub fn log(level: LogLevel, message: String, module: String) {
    if let Some(logger) = get_logger() {
        if logger.enabled(level) {
            let record = LogRecord::new(level, message, module);
            logger.log(&record);
        }
    }
}

/// Macro for logging at trace level
#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        $crate::developer_api::core::logging::log(
            $crate::developer_api::core::logging::LogLevel::Trace,
            format!($($arg)*),
            module_path!().to_string()
        )
    };
}

/// Macro for logging at debug level
#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        $crate::developer_api::core::logging::log(
            $crate::developer_api::core::logging::LogLevel::Debug,
            format!($($arg)*),
            module_path!().to_string()
        )
    };
}

/// Macro for logging at info level
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::developer_api::core::logging::log(
            $crate::developer_api::core::logging::LogLevel::Info,
            format!($($arg)*),
            module_path!().to_string()
        )
    };
}

/// Macro for logging at warn level
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        $crate::developer_api::core::logging::log(
            $crate::developer_api::core::logging::LogLevel::Warn,
            format!($($arg)*),
            module_path!().to_string()
        )
    };
}

/// Macro for logging at error level
#[macro_export]
macro_rules! error_log {
    ($($arg:tt)*) => {
        $crate::developer_api::core::logging::log(
            $crate::developer_api::core::logging::LogLevel::Error,
            format!($($arg)*),
            module_path!().to_string()
        )
    };
}

/// Distributed tracing support
pub mod tracing {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    /// Trace context for distributed tracing
    #[derive(Debug, Clone)]
    pub struct TraceContext {
        /// Trace ID - unique identifier for the entire trace
        pub trace_id: String,
        
        /// Span ID - unique identifier for this span
        pub span_id: String,
        
        /// Parent span ID if this is a child span
        pub parent_span_id: Option<String>,
        
        /// Span name
        pub name: String,
        
        /// Start time
        pub start_time: SystemTime,
        
        /// End time (None if span is still active)
        pub end_time: Option<SystemTime>,
        
        /// Span attributes
        pub attributes: HashMap<String, String>,
        
        /// Span events
        pub events: Vec<SpanEvent>,
    }
    
    /// Span event
    #[derive(Debug, Clone)]
    pub struct SpanEvent {
        /// Event name
        pub name: String,
        
        /// Event timestamp
        pub timestamp: SystemTime,
        
        /// Event attributes
        pub attributes: HashMap<String, String>,
    }
    
    impl TraceContext {
        /// Creates a new trace context
        pub fn new<S: Into<String>>(name: S) -> Self {
            Self {
                trace_id: generate_trace_id(),
                span_id: generate_span_id(),
                parent_span_id: None,
                name: name.into(),
                start_time: SystemTime::now(),
                end_time: None,
                attributes: HashMap::new(),
                events: Vec::new(),
            }
        }
        
        /// Creates a child span
        pub fn child<S: Into<String>>(&self, name: S) -> Self {
            Self {
                trace_id: self.trace_id.clone(),
                span_id: generate_span_id(),
                parent_span_id: Some(self.span_id.clone()),
                name: name.into(),
                start_time: SystemTime::now(),
                end_time: None,
                attributes: HashMap::new(),
                events: Vec::new(),
            }
        }
        
        /// Adds an attribute to the span
        pub fn add_attribute<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
            self.attributes.insert(key.into(), value.into());
        }
        
        /// Adds an event to the span
        pub fn add_event<S: Into<String>>(&mut self, name: S) {
            self.events.push(SpanEvent {
                name: name.into(),
                timestamp: SystemTime::now(),
                attributes: HashMap::new(),
            });
        }
        
        /// Ends the span
        pub fn end(&mut self) {
            self.end_time = Some(SystemTime::now());
        }
        
        /// Returns the duration of the span
        pub fn duration(&self) -> Option<std::time::Duration> {
            self.end_time.and_then(|end| end.duration_since(self.start_time).ok())
        }
    }
    
    /// Generates a unique trace ID
    fn generate_trace_id() -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        format!("{:016x}{:016x}", timestamp, count)
    }
    
    /// Generates a unique span ID
    fn generate_span_id() -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let count = COUNTER.fetch_add(1, Ordering::SeqCst);
        format!("{:016x}", count)
    }
    
    /// Tracer for managing trace contexts
    pub struct Tracer {
        active_spans: Arc<Mutex<Vec<TraceContext>>>,
    }
    
    impl Tracer {
        /// Creates a new tracer
        pub fn new() -> Self {
            Self {
                active_spans: Arc::new(Mutex::new(Vec::new())),
            }
        }
        
        /// Starts a new span
        pub fn start_span<S: Into<String>>(&self, name: S) -> TraceContext {
            let span = TraceContext::new(name);
            self.active_spans.lock().unwrap().push(span.clone());
            span
        }
        
        /// Ends a span
        pub fn end_span(&self, mut span: TraceContext) {
            span.end();
            let mut spans = self.active_spans.lock().unwrap();
            spans.retain(|s| s.span_id != span.span_id);
        }
        
        /// Gets all active spans
        pub fn active_spans(&self) -> Vec<TraceContext> {
            self.active_spans.lock().unwrap().clone()
        }
    }
    
    impl Default for Tracer {
        fn default() -> Self {
            Self::new()
        }
    }
}

/// Diagnostic information collection
pub mod diagnostics {
    use super::*;
    
    /// Diagnostic information about the system
    #[derive(Debug, Clone)]
    pub struct DiagnosticInfo {
        /// System information
        pub system: HashMap<String, String>,
        
        /// API state information
        pub api_state: HashMap<String, String>,
        
        /// Recent errors
        pub recent_errors: Vec<String>,
        
        /// Performance metrics
        pub metrics: HashMap<String, f64>,
        
        /// Active operations
        pub active_operations: Vec<String>,
    }
    
    impl DiagnosticInfo {
        /// Creates a new diagnostic info
        pub fn new() -> Self {
            Self {
                system: HashMap::new(),
                api_state: HashMap::new(),
                recent_errors: Vec::new(),
                metrics: HashMap::new(),
                active_operations: Vec::new(),
            }
        }
        
        /// Adds system information
        pub fn add_system_info<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
            self.system.insert(key.into(), value.into());
        }
        
        /// Adds API state information
        pub fn add_api_state<K: Into<String>, V: Into<String>>(&mut self, key: K, value: V) {
            self.api_state.insert(key.into(), value.into());
        }
        
        /// Adds an error
        pub fn add_error<S: Into<String>>(&mut self, error: S) {
            self.recent_errors.push(error.into());
        }
        
        /// Adds a metric
        pub fn add_metric<K: Into<String>>(&mut self, key: K, value: f64) {
            self.metrics.insert(key.into(), value);
        }
        
        /// Adds an active operation
        pub fn add_operation<S: Into<String>>(&mut self, operation: S) {
            self.active_operations.push(operation.into());
        }
    }
    
    impl Default for DiagnosticInfo {
        fn default() -> Self {
            Self::new()
        }
    }
    
    /// Diagnostic collector for gathering system information
    pub struct DiagnosticCollector {
        info: Arc<Mutex<DiagnosticInfo>>,
    }
    
    impl DiagnosticCollector {
        /// Creates a new diagnostic collector
        pub fn new() -> Self {
            Self {
                info: Arc::new(Mutex::new(DiagnosticInfo::new())),
            }
        }
        
        /// Collects diagnostic information
        pub fn collect(&self) -> DiagnosticInfo {
            self.info.lock().unwrap().clone()
        }
        
        /// Updates diagnostic information
        pub fn update<F>(&self, f: F)
        where
            F: FnOnce(&mut DiagnosticInfo),
        {
            let mut info = self.info.lock().unwrap();
            f(&mut info);
        }
    }
    
    impl Default for DiagnosticCollector {
        fn default() -> Self {
            Self::new()
        }
    }
}
