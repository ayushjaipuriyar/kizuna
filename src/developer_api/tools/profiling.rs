/// Performance profiling utilities
use std::time::{Duration, Instant};
use std::collections::HashMap;

/// Performance profiler for API operations
pub struct PerformanceProfiler {
    measurements: HashMap<String, Vec<Duration>>,
    active_operations: HashMap<String, Instant>,
}

impl PerformanceProfiler {
    /// Creates a new performance profiler
    pub fn new() -> Self {
        Self {
            measurements: HashMap::new(),
            active_operations: HashMap::new(),
        }
    }
    
    /// Starts profiling an operation
    pub fn start_operation(&mut self, operation: String) {
        self.active_operations.insert(operation, Instant::now());
    }
    
    /// Ends profiling an operation
    pub fn end_operation(&mut self, operation: &str) {
        if let Some(start) = self.active_operations.remove(operation) {
            let duration = start.elapsed();
            self.measurements
                .entry(operation.to_string())
                .or_insert_with(Vec::new)
                .push(duration);
        }
    }
    
    /// Gets the average duration for an operation
    pub fn average_duration(&self, operation: &str) -> Option<Duration> {
        self.measurements.get(operation).and_then(|durations| {
            if durations.is_empty() {
                None
            } else {
                let total: Duration = durations.iter().sum();
                Some(total / durations.len() as u32)
            }
        })
    }
    
    /// Gets all measurements
    pub fn get_measurements(&self) -> &HashMap<String, Vec<Duration>> {
        &self.measurements
    }
    
    /// Generates a performance report
    pub fn generate_report(&self) -> PerformanceReport {
        let mut operations = Vec::new();
        
        for (operation, durations) in &self.measurements {
            if !durations.is_empty() {
                let total: Duration = durations.iter().sum();
                let avg = total / durations.len() as u32;
                let min = *durations.iter().min().unwrap();
                let max = *durations.iter().max().unwrap();
                
                operations.push(OperationStats {
                    operation: operation.clone(),
                    count: durations.len(),
                    average: avg,
                    min,
                    max,
                });
            }
        }
        
        PerformanceReport { operations }
    }
    
    /// Clears all measurements
    pub fn clear(&mut self) {
        self.measurements.clear();
        self.active_operations.clear();
    }
}

impl Default for PerformanceProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance report
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    /// Statistics for each operation
    pub operations: Vec<OperationStats>,
}

/// Statistics for a single operation
#[derive(Debug, Clone)]
pub struct OperationStats {
    /// Operation name
    pub operation: String,
    
    /// Number of times executed
    pub count: usize,
    
    /// Average duration
    pub average: Duration,
    
    /// Minimum duration
    pub min: Duration,
    
    /// Maximum duration
    pub max: Duration,
}
