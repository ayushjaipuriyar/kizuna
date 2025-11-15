/// Plugin system module for extensibility
pub mod registry;
pub mod hooks;
pub mod sandbox;
pub mod loader;

// Re-export plugin types
pub use registry::PluginRegistry;
pub use hooks::{PluginHook, HookType};
pub use sandbox::{PluginSandbox, ResourceLimits, PluginPermissions};
pub use loader::PluginLoader;

use crate::developer_api::core::KizunaError;
use std::collections::HashMap;
use std::path::PathBuf;

/// Plugin trait that all plugins must implement
#[async_trait::async_trait]
pub trait Plugin: Send + Sync {
    /// Returns the plugin name
    fn name(&self) -> &str;
    
    /// Returns the plugin version
    fn version(&self) -> &str;
    
    /// Initializes the plugin with the given context
    async fn initialize(&mut self, context: PluginContext) -> Result<(), KizunaError>;
    
    /// Shuts down the plugin and cleans up resources
    async fn shutdown(&mut self) -> Result<(), KizunaError>;
}

/// Context provided to plugins during initialization
pub struct PluginContext {
    /// Configuration parameters for the plugin
    pub config: HashMap<String, serde_json::Value>,
    
    /// Data directory for plugin storage
    pub data_dir: PathBuf,
}

/// Plugin manager for managing plugin lifecycle
pub struct PluginManager {
    registry: PluginRegistry,
    sandbox: PluginSandbox,
}

impl PluginManager {
    /// Creates a new plugin manager
    pub fn new() -> Self {
        Self {
            registry: PluginRegistry::new(),
            sandbox: PluginSandbox::new(),
        }
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
