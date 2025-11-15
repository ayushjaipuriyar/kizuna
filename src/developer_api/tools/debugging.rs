/// Debugging and tracing utilities
use std::time::SystemTime;

/// Debug tracer for API calls
pub struct DebugTracer {
    enabled: bool,
    traces: Vec<TraceEntry>,
}

impl DebugTracer {
    /// Creates a new debug tracer
    pub fn new() -> Self {
        Self {
            enabled: false,
            traces: Vec::new(),
        }
    }
    
    /// Enables tracing
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    /// Disables tracing
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    /// Records a trace entry
    pub fn trace(&mut self, operation: String, details: String) {
        if self.enabled {
            self.traces.push(TraceEntry {
                timestamp: SystemTime::now(),
                operation,
                details,
            });
        }
    }
    
    /// Gets all trace entries
    pub fn get_traces(&self) -> &[TraceEntry] {
        &self.traces
    }
    
    /// Clears all traces
    pub fn clear(&mut self) {
        self.traces.clear();
    }
}

impl Default for DebugTracer {
    fn default() -> Self {
        Self::new()
    }
}

/// A single trace entry
#[derive(Debug, Clone)]
pub struct TraceEntry {
    /// Timestamp of the trace
    pub timestamp: SystemTime,
    
    /// Operation being traced
    pub operation: String,
    
    /// Additional details
    pub details: String,
}
