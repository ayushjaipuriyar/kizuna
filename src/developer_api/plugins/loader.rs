/// Plugin loader for dynamic plugin loading
use crate::developer_api::core::KizunaError;
use std::path::Path;

/// Plugin loader for loading plugins from disk
pub struct PluginLoader {
    plugin_dir: std::path::PathBuf,
}

impl PluginLoader {
    /// Creates a new plugin loader
    pub fn new<P: AsRef<Path>>(plugin_dir: P) -> Self {
        Self {
            plugin_dir: plugin_dir.as_ref().to_path_buf(),
        }
    }
    
    /// Discovers plugins in the plugin directory
    pub fn discover_plugins(&self) -> Result<Vec<std::path::PathBuf>, KizunaError> {
        let mut plugins = Vec::new();
        
        if !self.plugin_dir.exists() {
            return Ok(plugins);
        }
        
        let entries = std::fs::read_dir(&self.plugin_dir)
            .map_err(|e| KizunaError::plugin("loader", &format!("Failed to read plugin directory: {}", e)))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| KizunaError::plugin("loader", &format!("Failed to read entry: {}", e)))?;
            let path = entry.path();
            
            // Check for dynamic library extensions
            if let Some(ext) = path.extension() {
                if ext == "so" || ext == "dll" || ext == "dylib" {
                    plugins.push(path);
                }
            }
        }
        
        Ok(plugins)
    }
    
    /// Loads a plugin from the given path
    pub fn load_plugin(&self, _path: &Path) -> Result<(), KizunaError> {
        // TODO: Implement dynamic plugin loading using libloading
        Err(KizunaError::plugin("loader", "Dynamic plugin loading not yet implemented"))
    }
}
