use kizuna::security::{SecuritySystem, SecuritySystemBuilder};
use kizuna::security::identity::DeviceIdentity;
use kizuna::security::policy::{SecurityPolicy, ConnectionType};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Kizuna Security Integration Demo ===\n");
    
    // Create security system with custom configuration
    println!("1. Creating security system...");
    let security = SecuritySystemBuilder::new()
        .session_timeout(Duration::from_secs(1800)) // 30 minutes
        .key_rotation_interval(Duration::from_secs(600)) // 10 minutes
        .build()?;
    
    println!("   ✓ Security system initialized\n");
    
    // Get device identity
    println!("2. Getting device identity...");
    let identity = security.get_device_identity().await?;
    let peer_id = identity.derive_peer_id();
    println!("   ✓ Device Peer ID: {}", peer_id);
    println!("   ✓ Full fingerprint: {}\n", peer_id.to_hex());
    
    // Create a test peer
    println!("3. Creating test peer...");
    let test_identity = DeviceIdentity::generate()?;
    let test_peer_id = test_identity.derive_peer_id();
    println!("   ✓ Test Peer ID: {}\n", test_peer_id);
    
    // Test trust management
    println!("4. Testing trust management...");
    println!("   - Checking if test peer is trusted: {}", 
        security.is_trusted(&test_peer_id).await?);
    
    println!("   - Adding test peer to trust list...");
    security.add_trusted_peer(test_peer_id.clone(), "Test Device".to_string()).await?;
    
    println!("   - Checking if test peer is trusted: {}", 
        security.is_trusted(&test_peer_id).await?);
    println!("   ✓ Trust management working\n");
    
    // Test encryption session
    println!("5. Testing encryption session...");
    let session_id = security.establish_session(&test_peer_id).await?;
    println!("   ✓ Session established: {}", session_id);
    
    let plaintext = b"Hello, secure world!";
    println!("   - Encrypting message: {:?}", std::str::from_utf8(plaintext)?);
    
    let ciphertext = security.encrypt_message(&session_id, plaintext).await?;
    println!("   ✓ Encrypted ({} bytes)", ciphertext.len());
    
    let decrypted = security.decrypt_message(&session_id, &ciphertext).await?;
    println!("   - Decrypted message: {:?}", std::str::from_utf8(&decrypted)?);
    println!("   ✓ Encryption/decryption working\n");
    
    // Test policy management
    println!("6. Testing policy management...");
    let policy = security.get_policy().await?;
    println!("   - Private mode: {}", policy.private_mode);
    println!("   - Local-only mode: {}", policy.local_only_mode);
    
    println!("   - Enabling private mode...");
    security.enable_private_mode().await?;
    let policy = security.get_policy().await?;
    println!("   ✓ Private mode: {}", policy.private_mode);
    
    println!("   - Disabling private mode...");
    security.disable_private_mode().await?;
    let policy = security.get_policy().await?;
    println!("   ✓ Private mode: {}\n", policy.private_mode);
    
    // Test connection policy enforcement
    println!("7. Testing connection policy enforcement...");
    let allowed = security.is_connection_allowed(
        &test_peer_id,
        ConnectionType::LocalNetwork,
    ).await?;
    println!("   - Local network connection allowed: {}", allowed);
    
    let allowed = security.is_connection_allowed(
        &test_peer_id,
        ConnectionType::Relay,
    ).await?;
    println!("   - Relay connection allowed: {}\n", allowed);
    
    // Test pairing code generation
    println!("8. Testing pairing code generation...");
    let pairing_code = security.generate_pairing_code().await?;
    println!("   ✓ Pairing code generated: {}\n", pairing_code.code());
    
    // Test disposable identity
    println!("9. Testing disposable identity...");
    let disposable = security.create_disposable_identity().await?;
    println!("   ✓ Disposable identity created: {}", disposable.id());
    println!("   - Active: {}", disposable.is_active());
    
    security.activate_disposable_identity(disposable.id()).await?;
    let active = security.get_active_disposable_identity().await?;
    println!("   ✓ Disposable identity activated: {}\n", 
        active.map(|d| d.is_active()).unwrap_or(false));
    
    // Get trusted peers
    println!("10. Listing trusted peers...");
    let trusted_peers = security.get_trusted_peers().await?;
    println!("   ✓ Total trusted peers: {}", trusted_peers.len());
    for peer in trusted_peers {
        println!("     - {} ({})", peer.nickname, peer.peer_id);
    }
    println!();
    
    // Get audit log
    println!("11. Getting audit log...");
    let audit_log = security.get_audit_log(10).await?;
    println!("   ✓ Recent security events: {}", audit_log.len());
    for event in audit_log.iter().take(5) {
        println!("     - {:?}: {}", event.event_type, event.details);
    }
    println!();
    
    // Cleanup
    println!("12. Cleanup...");
    let removed_sessions = security.cleanup_expired_sessions().await?;
    println!("   ✓ Removed {} expired sessions", removed_sessions);
    
    let removed_identities = security.cleanup_expired_identities().await?;
    println!("   ✓ Removed {} expired disposable identities", removed_identities);
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}
