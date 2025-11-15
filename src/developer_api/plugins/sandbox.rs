/// Plugin sandboxing and security
use std::time::Duration;

/// Resource limits for plugin execution
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory: usize,
    
    /// Maximum CPU time
    pub max_cpu_time: Duration,
    
    /// Maximum number of file handles
    pub max_file_handles: usize,
    
    /// Whether network access is allowed
    pub allowed_network_access: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 100 * 1024 * 1024, // 100 MB
            max_cpu_time: Duration::from_secs(30),
            max_file_handles: 100,
            allowed_network_access: false,
        }
    }
}

/// Plugin permissions
#[derive(Debug, Clone)]
pub struct PluginPermissions {
    /// Can access file system
    pub can_access_filesystem: bool,
    
    /// Can access network
    pub can_access_network: bool,
    
    /// Can execute commands
    pub can_execute_commands: bool,
    
    /// Can access sensitive data
    pub can_access_sensitive_data: bool,
}

impl Default for PluginPermissions {
    fn default() -> Self {
        Self {
            can_access_filesystem: false,
            can_access_network: false,
            can_execute_commands: false,
            can_access_sensitive_data: false,
        }
    }
}

/// Plugin sandbox for isolating plugin execution
pub struct PluginSandbox {
    limits: ResourceLimits,
    permissions: PluginPermissions,
}

impl PluginSandbox {
    /// Creates a new plugin sandbox with default limits
    pub fn new() -> Self {
        Self {
            limits: ResourceLimits::default(),
            permissions: PluginPermissions::default(),
        }
    }
    
    /// Creates a sandbox with custom limits
    pub fn with_limits(limits: ResourceLimits) -> Self {
        Self {
            limits,
            permissions: PluginPermissions::default(),
        }
    }
    
    /// Creates a sandbox with custom permissions
    pub fn with_permissions(permissions: PluginPermissions) -> Self {
        Self {
            limits: ResourceLimits::default(),
            permissions,
        }
    }
    
    /// Gets the resource limits
    pub fn limits(&self) -> &ResourceLimits {
        &self.limits
    }
    
    /// Gets the permissions
    pub fn permissions(&self) -> &PluginPermissions {
        &self.permissions
    }
}

impl Default for PluginSandbox {
    fn default() -> Self {
        Self::new()
    }
}
