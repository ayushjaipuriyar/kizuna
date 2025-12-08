/// Core API module providing the foundational Rust API
pub mod api;
pub mod config;
pub mod error;
pub mod events;
pub mod runtime;
pub mod versioning;
pub mod deprecation;
pub mod change_tracking;
pub mod version_manager;
pub mod logging;
pub mod error_recovery;
pub mod diagnostics;
pub mod integration;

#[cfg(test)]
mod integration_test;

// Re-export core types
pub use api::{KizunaAPI, KizunaInstance};
pub use config::KizunaConfig;
pub use error::KizunaError;
pub use events::KizunaEvent;
pub use runtime::AsyncRuntime;
pub use versioning::{ApiVersion, CompatibilityManager, CompatibilityCheck, CompatibilityLevel};
pub use deprecation::{DeprecationManager, DeprecationInfo, DeprecationStatus, MigrationGuide, MigrationStep};
pub use change_tracking::{ChangeTracker, Changelog, ApiChange, ChangeType, CompatibilityMatrixEntry};
pub use version_manager::IntegratedVersionManager;
pub use logging::{Logger, LogLevel, LogRecord, ConsoleLogger, StructuredLogger};
pub use error_recovery::{ErrorRecoveryManager, CircuitBreaker};
pub use diagnostics::{DiagnosticTools, HealthMonitor, PerformanceMonitor, HealthStatus, DiagnosticReport};
pub use integration::{IntegratedSystemManager, IntegratedOperations};

/// Result type for core API operations
pub type Result<T> = std::result::Result<T, KizunaError>;
