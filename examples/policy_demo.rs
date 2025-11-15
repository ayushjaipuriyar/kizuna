use kizuna::security::policy::{PolicyEngineImpl, PolicyEngine, ConnectionType, SecurityEvent, SecurityEventType};
use kizuna::security::identity::PeerId;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Security Policy Engine Demo ===\n");
    
    // Create a new policy engine
    let engine = PolicyEngineImpl::new();
    let peer_id = PeerId::from_string("demo_peer")?;
    
    // Test 1: Basic connection allowed
    println!("Test 1: Basic connection (should succeed)");
    match engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await {
        Ok(_) => println!("✓ Connection allowed\n"),
        Err(e) => println!("✗ Connection denied: {}\n", e),
    }
    
    // Test 2: Enable private mode
    println!("Test 2: Enable private mode");
    engine.enable_private_mode().await?;
    println!("✓ Private mode enabled");
    
    match engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await {
        Ok(_) => println!("✗ Connection should have been blocked\n"),
        Err(e) => println!("✓ Connection blocked: {}\n", e),
    }
    
    // Test 3: Generate invite code
    println!("Test 3: Generate invite code for peer");
    let invite = engine.generate_invite_code(peer_id.clone()).await?;
    println!("✓ Invite code generated: {}", invite.code());
    println!("  Valid for: {} seconds\n", invite.time_until_expiration().unwrap_or(0));
    
    // Test 4: Connection should now be allowed
    println!("Test 4: Connection with invite code (should succeed)");
    match engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await {
        Ok(_) => println!("✓ Connection allowed with invite\n"),
        Err(e) => println!("✗ Connection denied: {}\n", e),
    }
    
    // Test 5: Enable local-only mode
    println!("Test 5: Enable local-only mode");
    engine.disable_private_mode().await?;
    engine.enable_local_only_mode().await?;
    println!("✓ Local-only mode enabled");
    
    // Local connection should work
    match engine.is_connection_allowed(&peer_id, ConnectionType::LocalNetwork).await {
        Ok(_) => println!("✓ Local connection allowed"),
        Err(e) => println!("✗ Local connection denied: {}", e),
    }
    
    // Relay connection should be blocked
    match engine.is_connection_allowed(&peer_id, ConnectionType::Relay).await {
        Ok(_) => println!("✗ Relay connection should have been blocked"),
        Err(e) => println!("✓ Relay connection blocked: {}\n", e),
    }
    
    // Test 6: Rate limiting
    println!("Test 6: Rate limiting (making rapid connection attempts)");
    let test_peer = PeerId::from_string("rate_limit_test")?;
    
    for i in 1..=7 {
        match engine.is_connection_allowed(&test_peer, ConnectionType::LocalNetwork).await {
            Ok(_) => println!("  Attempt {}: allowed", i),
            Err(e) => {
                println!("  Attempt {}: blocked - {}", i, e);
                break;
            }
        }
    }
    println!();
    
    // Test 7: Audit log
    println!("Test 7: View audit log");
    let log = engine.get_audit_log(5).await?;
    println!("✓ Recent security events:");
    for (i, event) in log.iter().enumerate() {
        println!("  {}. {:?} - {}", i + 1, event.event_type, event.details);
    }
    println!();
    
    // Test 8: Policy status
    println!("Test 8: Current policy status");
    let policy = engine.get_policy().await?;
    println!("  Private mode: {}", policy.private_mode);
    println!("  Local-only mode: {}", policy.local_only_mode);
    println!("  Require pairing: {}", policy.require_pairing);
    println!("  Auto-accept trusted: {}", policy.auto_accept_trusted);
    println!("  Session timeout: {:?}", policy.session_timeout);
    println!("  Key rotation interval: {:?}", policy.key_rotation_interval);
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}
