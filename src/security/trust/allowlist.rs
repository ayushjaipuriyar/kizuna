use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use crate::security::error::SecurityResult;
use crate::security::identity::PeerId;
use super::ServicePermissions;

/// Service types that can be controlled
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ServiceType {
    Clipboard,
    FileTransfer,
    Camera,
    Commands,
}

/// Allowlist manager for access control
pub struct AllowlistManager {
    /// Peers allowed to discover this device
    discovery_allowlist: Arc<RwLock<HashSet<PeerId>>>,
    /// Per-peer service permissions
    service_permissions: Arc<RwLock<HashMap<PeerId, ServicePermissions>>>,
}

impl AllowlistManager {
    /// Create a new allowlist manager
    pub fn new() -> Self {
        Self {
            discovery_allowlist: Arc::new(RwLock::new(HashSet::new())),
            service_permissions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Add a peer to the discovery allowlist
    pub fn add_to_discovery_allowlist(&self, peer_id: PeerId) -> SecurityResult<()> {
        let mut allowlist = self.discovery_allowlist.write().unwrap();
        allowlist.insert(peer_id);
        Ok(())
    }
    
    /// Remove a peer from the discovery allowlist
    pub fn remove_from_discovery_allowlist(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let mut allowlist = self.discovery_allowlist.write().unwrap();
        allowlist.remove(peer_id);
        Ok(())
    }
    
    /// Check if a peer is in the discovery allowlist
    pub fn is_in_discovery_allowlist(&self, peer_id: &PeerId) -> bool {
        let allowlist = self.discovery_allowlist.read().unwrap();
        allowlist.contains(peer_id)
    }
    
    /// Get all peers in the discovery allowlist
    pub fn get_discovery_allowlist(&self) -> Vec<PeerId> {
        let allowlist = self.discovery_allowlist.read().unwrap();
        allowlist.iter().cloned().collect()
    }
    
    /// Set service permissions for a peer
    pub fn set_permissions(&self, peer_id: PeerId, permissions: ServicePermissions) -> SecurityResult<()> {
        let mut perms = self.service_permissions.write().unwrap();
        perms.insert(peer_id, permissions);
        Ok(())
    }
    
    /// Get service permissions for a peer
    pub fn get_permissions(&self, peer_id: &PeerId) -> Option<ServicePermissions> {
        let perms = self.service_permissions.read().unwrap();
        perms.get(peer_id).cloned()
    }
    
    /// Check if a peer has permission for a specific service
    pub fn has_service_permission(&self, peer_id: &PeerId, service: ServiceType) -> bool {
        let perms = self.service_permissions.read().unwrap();
        
        if let Some(permissions) = perms.get(peer_id) {
            match service {
                ServiceType::Clipboard => permissions.clipboard,
                ServiceType::FileTransfer => permissions.file_transfer,
                ServiceType::Camera => permissions.camera,
                ServiceType::Commands => permissions.commands,
            }
        } else {
            // Default to deny if no permissions set
            false
        }
    }
    
    /// Grant permission for a specific service to a peer
    pub fn grant_service_permission(&self, peer_id: &PeerId, service: ServiceType) -> SecurityResult<()> {
        let mut perms = self.service_permissions.write().unwrap();
        
        let permissions = perms.entry(peer_id.clone()).or_insert_with(ServicePermissions::default);
        
        match service {
            ServiceType::Clipboard => permissions.clipboard = true,
            ServiceType::FileTransfer => permissions.file_transfer = true,
            ServiceType::Camera => permissions.camera = true,
            ServiceType::Commands => permissions.commands = true,
        }
        
        Ok(())
    }
    
    /// Revoke permission for a specific service from a peer
    pub fn revoke_service_permission(&self, peer_id: &PeerId, service: ServiceType) -> SecurityResult<()> {
        let mut perms = self.service_permissions.write().unwrap();
        
        if let Some(permissions) = perms.get_mut(peer_id) {
            match service {
                ServiceType::Clipboard => permissions.clipboard = false,
                ServiceType::FileTransfer => permissions.file_transfer = false,
                ServiceType::Camera => permissions.camera = false,
                ServiceType::Commands => permissions.commands = false,
            }
        }
        
        Ok(())
    }
    
    /// Remove all permissions for a peer
    pub fn remove_peer_permissions(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let mut perms = self.service_permissions.write().unwrap();
        perms.remove(peer_id);
        Ok(())
    }
    
    /// Get all peers with service permissions
    pub fn get_all_peers_with_permissions(&self) -> Vec<PeerId> {
        let perms = self.service_permissions.read().unwrap();
        perms.keys().cloned().collect()
    }
    
    /// Check if access should be allowed based on allowlist and permissions
    pub fn check_access(&self, peer_id: &PeerId, service: ServiceType) -> SecurityResult<bool> {
        // First check if peer is in discovery allowlist (basic access)
        if !self.is_in_discovery_allowlist(peer_id) {
            return Ok(false);
        }
        
        // Then check service-specific permission
        Ok(self.has_service_permission(peer_id, service))
    }
    
    /// Clear all allowlist entries
    pub fn clear_discovery_allowlist(&self) -> SecurityResult<()> {
        let mut allowlist = self.discovery_allowlist.write().unwrap();
        allowlist.clear();
        Ok(())
    }
    
    /// Clear all service permissions
    pub fn clear_all_permissions(&self) -> SecurityResult<()> {
        let mut perms = self.service_permissions.write().unwrap();
        perms.clear();
        Ok(())
    }
}

impl Default for AllowlistManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_discovery_allowlist() {
        let manager = AllowlistManager::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Initially not in allowlist
        assert!(!manager.is_in_discovery_allowlist(&peer_id));
        
        // Add to allowlist
        manager.add_to_discovery_allowlist(peer_id.clone()).unwrap();
        assert!(manager.is_in_discovery_allowlist(&peer_id));
        
        // Remove from allowlist
        manager.remove_from_discovery_allowlist(&peer_id).unwrap();
        assert!(!manager.is_in_discovery_allowlist(&peer_id));
    }
    
    #[test]
    fn test_service_permissions() {
        let manager = AllowlistManager::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Initially no permissions
        assert!(!manager.has_service_permission(&peer_id, ServiceType::Clipboard));
        
        // Grant clipboard permission
        manager.grant_service_permission(&peer_id, ServiceType::Clipboard).unwrap();
        assert!(manager.has_service_permission(&peer_id, ServiceType::Clipboard));
        
        // Revoke clipboard permission
        manager.revoke_service_permission(&peer_id, ServiceType::Clipboard).unwrap();
        assert!(!manager.has_service_permission(&peer_id, ServiceType::Clipboard));
    }
    
    #[test]
    fn test_set_permissions() {
        let manager = AllowlistManager::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        let permissions = ServicePermissions {
            clipboard: true,
            file_transfer: true,
            camera: false,
            commands: false,
        };
        
        manager.set_permissions(peer_id.clone(), permissions.clone()).unwrap();
        
        let retrieved = manager.get_permissions(&peer_id).unwrap();
        assert_eq!(retrieved.clipboard, permissions.clipboard);
        assert_eq!(retrieved.file_transfer, permissions.file_transfer);
        assert_eq!(retrieved.camera, permissions.camera);
        assert_eq!(retrieved.commands, permissions.commands);
    }
    
    #[test]
    fn test_check_access() {
        let manager = AllowlistManager::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // No access without allowlist entry
        assert!(!manager.check_access(&peer_id, ServiceType::Clipboard).unwrap());
        
        // Add to allowlist
        manager.add_to_discovery_allowlist(peer_id.clone()).unwrap();
        
        // Still no access without service permission
        assert!(!manager.check_access(&peer_id, ServiceType::Clipboard).unwrap());
        
        // Grant service permission
        manager.grant_service_permission(&peer_id, ServiceType::Clipboard).unwrap();
        
        // Now access should be granted
        assert!(manager.check_access(&peer_id, ServiceType::Clipboard).unwrap());
    }
}
