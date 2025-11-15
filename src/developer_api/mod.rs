/// Developer API module providing comprehensive programming interfaces
/// and extensibility mechanisms for Kizuna
pub mod core;
pub mod bindings;
pub mod plugins;
pub mod tools;

// Re-export core types for convenience
pub use core::{KizunaAPI, KizunaInstance, KizunaConfig, KizunaError, KizunaEvent};
pub use plugins::{Plugin, PluginContext, PluginManager};

/// Result type for developer API operations
pub type Result<T> = std::result::Result<T, KizunaError>;
