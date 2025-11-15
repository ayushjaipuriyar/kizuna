use kizuna::security::trust::{TrustManager, TrustManagerImpl, TrustLevel, ServicePermissions};
use kizuna::security::identity::PeerId;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Trust Management Demo ===\n");
    
    // Create a temporary database for demo
    let db_path = PathBuf::from("/tmp/kizuna_trust_demo.db");
    
    // Initialize trust manager
    let trust_manager = TrustManagerImpl::new(db_path)?;
    
    // Demo 1: Add a trusted peer
    println!("1. Adding a trusted peer...");
    let peer_id = PeerId::from_string("abcd1234567890abcd1234567890abcd1234567890abcd1234567890abcd1234")?;
    trust_manager.add_trusted_peer(peer_id.clone(), "Alice's Device".to_string()).await?;
    println!("   ✓ Added peer: {}", peer_id.display_name());
    
    // Demo 2: Check if peer is trusted
    println!("\n2. Checking if peer is trusted...");
    let is_trusted = trust_manager.is_trusted(&peer_id).await?;
    println!("   ✓ Peer is trusted: {}", is_trusted);
    
    // Demo 3: Generate pairing code
    println!("\n3. Generating pairing code...");
    let pairing_code = trust_manager.generate_pairing_code().await?;
    println!("   ✓ Pairing code: {}", pairing_code.code());
    
    // Demo 4: Verify pairing code
    println!("\n4. Verifying pairing code...");
    let new_peer_id = PeerId::from_string("1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd")?;
    let verified = trust_manager.verify_pairing_code(&pairing_code, &new_peer_id).await?;
    println!("   ✓ Pairing code verified: {}", verified);
    
    // Demo 5: Update permissions
    println!("\n5. Updating peer permissions...");
    let mut permissions = ServicePermissions::default();
    permissions.camera = true;
    permissions.commands = true;
    trust_manager.update_permissions(&peer_id, permissions).await?;
    println!("   ✓ Permissions updated");
    
    // Demo 6: Get trust entry
    println!("\n6. Retrieving trust entry...");
    if let Some(entry) = trust_manager.get_trust_entry(&peer_id).await? {
        println!("   ✓ Peer: {}", entry.nickname);
        println!("     Trust Level: {:?}", entry.trust_level);
        println!("     Permissions:");
        println!("       - Clipboard: {}", entry.permissions.clipboard);
        println!("       - File Transfer: {}", entry.permissions.file_transfer);
        println!("       - Camera: {}", entry.permissions.camera);
        println!("       - Commands: {}", entry.permissions.commands);
    }
    
    // Demo 7: List all trusted peers
    println!("\n7. Listing all trusted peers...");
    let all_peers = trust_manager.get_all_trusted_peers().await?;
    println!("   ✓ Total trusted peers: {}", all_peers.len());
    for entry in all_peers {
        println!("     - {} ({})", entry.nickname, entry.peer_id.display_name());
    }
    
    // Demo 8: Update trust level
    println!("\n8. Updating trust level...");
    trust_manager.update_trust_level(&peer_id, TrustLevel::Verified).await?;
    println!("   ✓ Trust level updated to Verified");
    
    // Demo 9: Get allowlist
    println!("\n9. Getting allowlist...");
    let allowlist = trust_manager.get_allowlist().await?;
    println!("   ✓ Allowlist size: {}", allowlist.len());
    
    // Demo 10: Remove trusted peer
    println!("\n10. Removing trusted peer...");
    trust_manager.remove_trusted_peer(&peer_id).await?;
    println!("   ✓ Peer removed");
    
    // Verify removal
    let is_still_trusted = trust_manager.is_trusted(&peer_id).await?;
    println!("   ✓ Peer is still trusted: {}", is_still_trusted);
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}
