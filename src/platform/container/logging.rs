// Container logging and monitoring integration
//
// Provides structured logging, log aggregation, and monitoring integration
// for containerized environments.

use crate::platform::{PlatformResult, PlatformError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;

/// Container logger for structured logging
pub struct ContainerLogger {
    config: LogConfig,
    context: LogContext,
}

impl ContainerLogger {
    pub fn new(config: LogConfig) -> Self {
        Self {
            config,
            context: LogContext::default(),
        }
    }

    pub fn default() -> Self {
        Self::new(LogConfig::default())
    }

    /// Initialize logger
    pub fn initialize(&self) -> PlatformResult<()> {
        // Set up log level from environment or config
        let log_level = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| self.config.level.clone());

        std::env::set_var("RUST_LOG", log_level);

        Ok(())
    }

    /// Log a message with structured data
    pub fn log(&self, level: LogLevel, message: &str, fields: HashMap<String, String>) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());

        let entry = LogEntry {
            timestamp,
            level,
            message: message.to_string(),
            fields,
            context: self.context.clone(),
        };

        self.write_log_entry(&entry);
    }

    /// Log info message
    pub fn info(&self, message: &str) {
        self.log(LogLevel::Info, message, HashMap::new());
    }

    /// Log warning message
    pub fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message, HashMap::new());
    }

    /// Log error message
    pub fn error(&self, message: &str) {
        self.log(LogLevel::Error, message, HashMap::new());
    }

    /// Log debug message
    pub fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message, HashMap::new());
    }

    /// Write log entry based on format
    fn write_log_entry(&self, entry: &LogEntry) {
        match self.config.format {
            LogFormat::Json => {
                if let Ok(json) = serde_json::to_string(entry) {
                    self.write_output(&json);
                }
            }
            LogFormat::Text => {
                let text = format!(
                    "[{}] {} - {}",
                    entry.timestamp,
                    entry.level.as_str(),
                    entry.message
                );
                self.write_output(&text);
            }
            LogFormat::Structured => {
                let mut parts = vec![
                    format!("timestamp={}", entry.timestamp),
                    format!("level={}", entry.level.as_str()),
                    format!("message=\"{}\"", entry.message),
                ];

                for (key, value) in &entry.fields {
                    parts.push(format!("{}=\"{}\"", key, value));
                }

                self.write_output(&parts.join(" "));
            }
        }
    }

    /// Write to configured output
    fn write_output(&self, message: &str) {
        match &self.config.output {
            LogOutput::Stdout => {
                println!("{}", message);
            }
            LogOutput::Stderr => {
                eprintln!("{}", message);
            }
            LogOutput::File(path) => {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    let _ = writeln!(file, "{}", message);
                }
            }
            LogOutput::Syslog => {
                // Would integrate with syslog
                println!("{}", message);
            }
        }
    }

    /// Set context field
    pub fn set_context(&mut self, key: String, value: String) {
        self.context.fields.insert(key, value);
    }
}

/// Log configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub format: LogFormat,
    pub output: LogOutput,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Json,
            output: LogOutput::Stdout,
        }
    }
}

/// Log format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFormat {
    Json,
    Text,
    Structured,
}

/// Log output destination
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogOutput {
    Stdout,
    Stderr,
    File(std::path::PathBuf),
    Syslog,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn as_str(&self) -> &str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// Log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogEntry {
    timestamp: String,
    level: LogLevel,
    message: String,
    fields: HashMap<String, String>,
    context: LogContext,
}

/// Log context
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct LogContext {
    fields: HashMap<String, String>,
}

/// Monitoring metrics collector
pub struct MetricsCollector {
    metrics: HashMap<String, Metric>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    /// Record a counter metric
    pub fn counter(&mut self, name: String, value: f64, labels: HashMap<String, String>) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());

        self.metrics.insert(
            name.clone(),
            Metric {
                name,
                metric_type: MetricType::Counter,
                value,
                labels,
                timestamp,
            },
        );
    }

    /// Record a gauge metric
    pub fn gauge(&mut self, name: String, value: f64, labels: HashMap<String, String>) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());

        self.metrics.insert(
            name.clone(),
            Metric {
                name,
                metric_type: MetricType::Gauge,
                value,
                labels,
                timestamp,
            },
        );
    }

    /// Record a histogram metric
    pub fn histogram(&mut self, name: String, value: f64, labels: HashMap<String, String>) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string());

        self.metrics.insert(
            name.clone(),
            Metric {
                name,
                metric_type: MetricType::Histogram,
                value,
                labels,
                timestamp,
            },
        );
    }

    /// Get all metrics
    pub fn get_metrics(&self) -> &HashMap<String, Metric> {
        &self.metrics
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();

        for metric in self.metrics.values() {
            // Metric type comment
            output.push_str(&format!("# TYPE {} {}\n", metric.name, metric.metric_type.as_str()));

            // Metric value with labels
            output.push_str(&metric.name);
            if !metric.labels.is_empty() {
                output.push('{');
                let labels: Vec<String> = metric
                    .labels
                    .iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect();
                output.push_str(&labels.join(","));
                output.push('}');
            }
            output.push_str(&format!(" {}\n", metric.value));
        }

        output
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub metric_type: MetricType,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: String,
}

/// Metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
}

impl MetricType {
    fn as_str(&self) -> &str {
        match self {
            Self::Counter => "counter",
            Self::Gauge => "gauge",
            Self::Histogram => "histogram",
        }
    }
}

/// Log aggregation client for sending logs to external systems
pub struct LogAggregator {
    backend: AggregationBackend,
    buffer: Vec<LogEntry>,
    buffer_size: usize,
}

impl LogAggregator {
    pub fn new(backend: AggregationBackend, buffer_size: usize) -> Self {
        Self {
            backend,
            buffer: Vec::with_capacity(buffer_size),
            buffer_size,
        }
    }

    /// Add log entry to buffer
    pub fn add_entry(&mut self, entry: LogEntry) {
        self.buffer.push(entry);

        if self.buffer.len() >= self.buffer_size {
            let _ = self.flush();
        }
    }

    /// Flush buffered logs
    pub fn flush(&mut self) -> PlatformResult<()> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        match &self.backend {
            AggregationBackend::Elasticsearch { url } => {
                log::debug!("Flushing {} logs to Elasticsearch at {}", self.buffer.len(), url);
                // Would send to Elasticsearch
            }
            AggregationBackend::Loki { url } => {
                log::debug!("Flushing {} logs to Loki at {}", self.buffer.len(), url);
                // Would send to Loki
            }
            AggregationBackend::CloudWatch { region, log_group } => {
                log::debug!(
                    "Flushing {} logs to CloudWatch in {} / {}",
                    self.buffer.len(),
                    region,
                    log_group
                );
                // Would send to CloudWatch
            }
            AggregationBackend::None => {}
        }

        self.buffer.clear();
        Ok(())
    }
}

/// Log aggregation backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AggregationBackend {
    Elasticsearch { url: String },
    Loki { url: String },
    CloudWatch { region: String, log_group: String },
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_logger_creation() {
        let logger = ContainerLogger::default();
        assert!(logger.initialize().is_ok());
    }

    #[test]
    fn test_log_levels() {
        let logger = ContainerLogger::default();
        logger.info("Test info message");
        logger.warn("Test warning message");
        logger.error("Test error message");
        logger.debug("Test debug message");
    }

    #[test]
    fn test_metrics_collector() {
        let mut collector = MetricsCollector::new();
        
        collector.counter("requests_total".to_string(), 100.0, HashMap::new());
        collector.gauge("memory_usage".to_string(), 512.0, HashMap::new());

        assert_eq!(collector.get_metrics().len(), 2);
    }

    #[test]
    fn test_prometheus_export() {
        let mut collector = MetricsCollector::new();
        
        let mut labels = HashMap::new();
        labels.insert("method".to_string(), "GET".to_string());
        
        collector.counter("http_requests_total".to_string(), 42.0, labels);

        let prometheus = collector.export_prometheus();
        assert!(prometheus.contains("# TYPE http_requests_total counter"));
        assert!(prometheus.contains("http_requests_total"));
        assert!(prometheus.contains("method=\"GET\""));
        assert!(prometheus.contains("42"));
    }

    #[test]
    fn test_log_aggregator() {
        let mut aggregator = LogAggregator::new(AggregationBackend::None, 10);
        
        let entry = LogEntry {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            level: LogLevel::Info,
            message: "Test message".to_string(),
            fields: HashMap::new(),
            context: LogContext::default(),
        };

        aggregator.add_entry(entry);
        assert!(aggregator.flush().is_ok());
    }
}
