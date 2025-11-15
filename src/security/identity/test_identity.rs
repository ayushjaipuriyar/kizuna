#[cfg(test)]
mod tests {
    use super::super::{DeviceIdentity, PeerId, DisposableIdentity, DisposableIdentityManager};
    
    #[test]
    fn test_device_identity_generation() {
        let identity = DeviceIdentity::generate().expect("Failed to generate identity");
        let peer_id = identity.derive_peer_id();
        
        // Verify peer_id is derived correctly
        assert_eq!(peer_id.fingerprint().len(), 32);
        
        // Display name should be 16 hex characters (8 bytes)
        assert_eq!(peer_id.display_name().len(), 16);
    }
    
    #[test]
    fn test_identity_serialization() {
        let identity = DeviceIdentity::generate().expect("Failed to generate identity");
        let original_peer_id = identity.derive_peer_id();
        
        // Serialize and deserialize
        let bytes = identity.to_bytes();
        let restored = DeviceIdentity::from_bytes(&bytes).expect("Failed to deserialize");
        let restored_peer_id = restored.derive_peer_id();
        
        // Verify they match
        assert_eq!(original_peer_id, restored_peer_id);
    }
    
    #[test]
    fn test_peer_id_hex_conversion() {
        let identity = DeviceIdentity::generate().expect("Failed to generate identity");
        let peer_id = identity.derive_peer_id();
        
        // Convert to hex and back
        let hex = peer_id.to_hex();
        let restored = PeerId::from_hex(&hex).expect("Failed to parse hex");
        
        assert_eq!(peer_id, restored);
    }
    
    #[tokio::test]
    async fn test_disposable_identity_creation() {
        let identity = DisposableIdentity::generate(Some(3600)).expect("Failed to generate");
        
        assert!(!identity.is_active());
        assert!(!identity.is_expired());
        assert!(identity.expires_at().is_some());
    }
    
    #[tokio::test]
    async fn test_disposable_identity_manager() {
        let manager = DisposableIdentityManager::new(3600);
        
        // Create identities
        let id1 = manager.create_identity().await.expect("Failed to create");
        let id2 = manager.create_identity().await.expect("Failed to create");
        
        // Activate first identity
        manager.activate_identity(id1.id()).await.expect("Failed to activate");
        
        // Verify it's active
        let active = manager.get_active_identity().await.expect("Failed to get active");
        assert!(active.is_some());
        assert_eq!(active.unwrap().id(), id1.id());
        
        // List all
        let all = manager.list_identities().await;
        assert_eq!(all.len(), 2);
        
        // Delete one
        manager.delete_identity(id2.id()).await.expect("Failed to delete");
        let all = manager.list_identities().await;
        assert_eq!(all.len(), 1);
    }
}
