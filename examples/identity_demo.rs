use kizuna::security::identity::{DeviceIdentity, IdentityStore, DisposableIdentityManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Device Identity Demo ===\n");
    
    // 1. Generate a new device identity
    println!("1. Generating new device identity...");
    let identity = DeviceIdentity::generate()?;
    let peer_id = identity.derive_peer_id();
    println!("   Generated PeerId: {}", peer_id);
    println!("   Full fingerprint: {}", peer_id.to_hex());
    
    // 2. Test serialization
    println!("\n2. Testing serialization...");
    let identity_bytes = identity.to_bytes();
    println!("   Serialized to {} bytes", identity_bytes.len());
    
    let restored_identity = DeviceIdentity::from_bytes(&identity_bytes)?;
    let restored_peer_id = restored_identity.derive_peer_id();
    println!("   Restored PeerId: {}", restored_peer_id);
    assert_eq!(peer_id, restored_peer_id, "PeerIds should match after serialization");
    println!("   ✓ Serialization works correctly");
    
    // 3. Test disposable identities
    println!("\n3. Testing disposable identities...");
    let manager = DisposableIdentityManager::new(3600); // 1 hour lifetime
    
    let disposable1 = manager.create_identity().await?;
    println!("   Created disposable identity: {}", disposable1.id());
    println!("   Disposable PeerId: {}", disposable1.derive_peer_id());
    
    let disposable2 = manager.create_identity().await?;
    println!("   Created second disposable identity: {}", disposable2.id());
    
    // Activate first identity
    manager.activate_identity(disposable1.id()).await?;
    println!("   Activated first disposable identity");
    
    if let Some(active) = manager.get_active_identity().await? {
        println!("   Active identity: {}", active.id());
        assert_eq!(active.id(), disposable1.id());
    }
    
    // List all identities
    let all_identities = manager.list_identities().await;
    println!("   Total disposable identities: {}", all_identities.len());
    
    // Cleanup (none should be expired yet)
    let removed = manager.cleanup_expired().await?;
    println!("   Cleaned up {} expired identities", removed);
    
    println!("\n✓ All identity management features working correctly!");
    
    Ok(())
}
