/// Plugin registry for managing loaded plugins
use super::{Plugin, PluginContext};
use crate::developer_api::core::KizunaError;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing plugins
pub struct PluginRegistry {
    plugins: Arc<RwLock<HashMap<String, Box<dyn Plugin>>>>,
}

impl PluginRegistry {
    /// Creates a new plugin registry
    pub fn new() -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Registers a plugin
    pub async fn register(&self, mut plugin: Box<dyn Plugin>, context: PluginContext) -> Result<(), KizunaError> {
        let name = plugin.name().to_string();
        
        // Initialize the plugin
        plugin.initialize(context).await?;
        
        // Add to registry
        let mut plugins = self.plugins.write().await;
        plugins.insert(name.clone(), plugin);
        
        Ok(())
    }
    
    /// Unregisters a plugin
    pub async fn unregister(&self, name: &str) -> Result<(), KizunaError> {
        let mut plugins = self.plugins.write().await;
        
        if let Some(mut plugin) = plugins.remove(name) {
            plugin.shutdown().await?;
        }
        
        Ok(())
    }
    
    /// Gets a plugin by name
    pub async fn get(&self, name: &str) -> Option<String> {
        let plugins = self.plugins.read().await;
        plugins.get(name).map(|p| p.name().to_string())
    }
    
    /// Lists all registered plugins
    pub async fn list(&self) -> Vec<String> {
        let plugins = self.plugins.read().await;
        plugins.keys().cloned().collect()
    }
    
    /// Shuts down all plugins
    pub async fn shutdown_all(&self) -> Result<(), KizunaError> {
        let mut plugins = self.plugins.write().await;
        
        for (_, mut plugin) in plugins.drain() {
            if let Err(e) = plugin.shutdown().await {
                eprintln!("Error shutting down plugin {}: {}", plugin.name(), e);
            }
        }
        
        Ok(())
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
