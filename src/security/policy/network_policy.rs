use std::sync::{Arc, RwLock};
use crate::security::error::{SecurityResult, PolicyError};
use super::ConnectionType;

/// Network policy mode
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NetworkMode {
    /// Allow all connection types
    Unrestricted,
    /// Only allow local network connections
    LocalOnly,
    /// Custom policy with specific allowed connection types
    Custom(Vec<ConnectionType>),
}

/// Network policy enforcer for connection type restrictions
pub struct NetworkPolicyEnforcer {
    /// Current network mode
    mode: Arc<RwLock<NetworkMode>>,
    /// Whether local-only mode is enabled
    local_only_enabled: Arc<RwLock<bool>>,
}

impl NetworkPolicyEnforcer {
    /// Create a new network policy enforcer
    pub fn new() -> Self {
        Self {
            mode: Arc::new(RwLock::new(NetworkMode::Unrestricted)),
            local_only_enabled: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Enable local-only mode
    pub fn enable_local_only(&self) -> SecurityResult<()> {
        let mut enabled = self.local_only_enabled.write().unwrap();
        *enabled = true;
        
        let mut mode = self.mode.write().unwrap();
        *mode = NetworkMode::LocalOnly;
        
        Ok(())
    }
    
    /// Disable local-only mode
    pub fn disable_local_only(&self) -> SecurityResult<()> {
        let mut enabled = self.local_only_enabled.write().unwrap();
        *enabled = false;
        
        let mut mode = self.mode.write().unwrap();
        *mode = NetworkMode::Unrestricted;
        
        Ok(())
    }
    
    /// Check if local-only mode is enabled
    pub fn is_local_only_enabled(&self) -> bool {
        let enabled = self.local_only_enabled.read().unwrap();
        *enabled
    }
    
    /// Set network mode
    pub fn set_mode(&self, mode: NetworkMode) -> SecurityResult<()> {
        let mut current_mode = self.mode.write().unwrap();
        *current_mode = mode.clone();
        
        // Update local-only flag
        let mut enabled = self.local_only_enabled.write().unwrap();
        *enabled = matches!(mode, NetworkMode::LocalOnly);
        
        Ok(())
    }
    
    /// Get current network mode
    pub fn get_mode(&self) -> NetworkMode {
        let mode = self.mode.read().unwrap();
        mode.clone()
    }
    
    /// Check if a connection type is allowed
    pub fn is_connection_type_allowed(&self, connection_type: &ConnectionType) -> SecurityResult<bool> {
        let mode = self.mode.read().unwrap();
        
        match &*mode {
            NetworkMode::Unrestricted => Ok(true),
            NetworkMode::LocalOnly => {
                match connection_type {
                    ConnectionType::LocalNetwork => Ok(true),
                    ConnectionType::Relay | ConnectionType::Direct => {
                        Err(PolicyError::LocalOnlyBlocked.into())
                    }
                }
            }
            NetworkMode::Custom(allowed_types) => {
                if allowed_types.contains(connection_type) {
                    Ok(true)
                } else {
                    Err(PolicyError::LocalOnlyBlocked.into())
                }
            }
        }
    }
    
    /// Check if relay connections are allowed
    pub fn are_relay_connections_allowed(&self) -> bool {
        let mode = self.mode.read().unwrap();
        
        match &*mode {
            NetworkMode::Unrestricted => true,
            NetworkMode::LocalOnly => false,
            NetworkMode::Custom(allowed_types) => {
                allowed_types.contains(&ConnectionType::Relay)
            }
        }
    }
    
    /// Check if global discovery is allowed
    pub fn is_global_discovery_allowed(&self) -> bool {
        // Global discovery is only allowed in unrestricted mode
        let mode = self.mode.read().unwrap();
        matches!(&*mode, NetworkMode::Unrestricted)
    }
    
    /// Get mode indicator string for UI display
    pub fn get_mode_indicator(&self) -> String {
        let mode = self.mode.read().unwrap();
        
        match &*mode {
            NetworkMode::Unrestricted => "Unrestricted".to_string(),
            NetworkMode::LocalOnly => "Local Only".to_string(),
            NetworkMode::Custom(_) => "Custom".to_string(),
        }
    }
    
    /// Get detailed mode description
    pub fn get_mode_description(&self) -> String {
        let mode = self.mode.read().unwrap();
        
        match &*mode {
            NetworkMode::Unrestricted => {
                "All connection types allowed (local, relay, direct)".to_string()
            }
            NetworkMode::LocalOnly => {
                "Only local network connections allowed. Relay and global discovery blocked.".to_string()
            }
            NetworkMode::Custom(allowed_types) => {
                let types: Vec<String> = allowed_types.iter()
                    .map(|t| format!("{:?}", t))
                    .collect();
                format!("Custom policy: allowed types = [{}]", types.join(", "))
            }
        }
    }
    
    /// Validate connection attempt against policy
    pub fn validate_connection(&self, connection_type: &ConnectionType) -> SecurityResult<()> {
        self.is_connection_type_allowed(connection_type)?;
        Ok(())
    }
}

impl Default for NetworkPolicyEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_local_only_mode() {
        let enforcer = NetworkPolicyEnforcer::new();
        
        assert!(!enforcer.is_local_only_enabled());
        
        enforcer.enable_local_only().unwrap();
        assert!(enforcer.is_local_only_enabled());
        
        enforcer.disable_local_only().unwrap();
        assert!(!enforcer.is_local_only_enabled());
    }
    
    #[test]
    fn test_connection_type_filtering() {
        let enforcer = NetworkPolicyEnforcer::new();
        
        // Unrestricted mode - all allowed
        assert!(enforcer.is_connection_type_allowed(&ConnectionType::LocalNetwork).is_ok());
        assert!(enforcer.is_connection_type_allowed(&ConnectionType::Relay).is_ok());
        assert!(enforcer.is_connection_type_allowed(&ConnectionType::Direct).is_ok());
        
        // Enable local-only mode
        enforcer.enable_local_only().unwrap();
        
        // Only local network allowed
        assert!(enforcer.is_connection_type_allowed(&ConnectionType::LocalNetwork).is_ok());
        assert!(enforcer.is_connection_type_allowed(&ConnectionType::Relay).is_err());
        assert!(enforcer.is_connection_type_allowed(&ConnectionType::Direct).is_err());
    }
    
    #[test]
    fn test_relay_connections() {
        let enforcer = NetworkPolicyEnforcer::new();
        
        assert!(enforcer.are_relay_connections_allowed());
        
        enforcer.enable_local_only().unwrap();
        assert!(!enforcer.are_relay_connections_allowed());
    }
    
    #[test]
    fn test_global_discovery() {
        let enforcer = NetworkPolicyEnforcer::new();
        
        assert!(enforcer.is_global_discovery_allowed());
        
        enforcer.enable_local_only().unwrap();
        assert!(!enforcer.is_global_discovery_allowed());
    }
    
    #[test]
    fn test_custom_mode() {
        let enforcer = NetworkPolicyEnforcer::new();
        
        let custom_mode = NetworkMode::Custom(vec![
            ConnectionType::LocalNetwork,
            ConnectionType::Direct,
        ]);
        
        enforcer.set_mode(custom_mode).unwrap();
        
        assert!(enforcer.is_connection_type_allowed(&ConnectionType::LocalNetwork).is_ok());
        assert!(enforcer.is_connection_type_allowed(&ConnectionType::Direct).is_ok());
        assert!(enforcer.is_connection_type_allowed(&ConnectionType::Relay).is_err());
    }
    
    #[test]
    fn test_mode_indicators() {
        let enforcer = NetworkPolicyEnforcer::new();
        
        assert_eq!(enforcer.get_mode_indicator(), "Unrestricted");
        
        enforcer.enable_local_only().unwrap();
        assert_eq!(enforcer.get_mode_indicator(), "Local Only");
        
        let description = enforcer.get_mode_description();
        assert!(description.contains("local network"));
    }
    
    #[test]
    fn test_validate_connection() {
        let enforcer = NetworkPolicyEnforcer::new();
        
        // Should succeed in unrestricted mode
        assert!(enforcer.validate_connection(&ConnectionType::Relay).is_ok());
        
        // Should fail in local-only mode
        enforcer.enable_local_only().unwrap();
        assert!(enforcer.validate_connection(&ConnectionType::Relay).is_err());
        assert!(enforcer.validate_connection(&ConnectionType::LocalNetwork).is_ok());
    }
}
