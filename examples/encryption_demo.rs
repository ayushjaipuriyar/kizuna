use kizuna::security::encryption::{EncryptionEngine, EncryptionEngineImpl};
use kizuna::security::identity::PeerId;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Kizuna Encryption Engine Demo ===\n");
    
    // Create encryption engine with default settings
    let engine = EncryptionEngineImpl::with_defaults();
    
    // Create a test peer ID
    let peer_id = PeerId::from_fingerprint([1u8; 32]);
    println!("Peer ID: {}", peer_id);
    
    // Establish a session
    println!("\n1. Establishing secure session...");
    let session_id = engine.establish_session(&peer_id).await?;
    println!("   Session established: {}", session_id);
    
    // Encrypt a message
    let message = b"Hello, secure world!";
    println!("\n2. Encrypting message: {:?}", String::from_utf8_lossy(message));
    let encrypted = engine.encrypt_message(&session_id, message).await?;
    println!("   Encrypted ({} bytes): {}", encrypted.len(), hex::encode(&encrypted[..20.min(encrypted.len())]));
    
    // Decrypt the message
    println!("\n3. Decrypting message...");
    let decrypted = engine.decrypt_message(&session_id, &encrypted).await?;
    println!("   Decrypted: {:?}", String::from_utf8_lossy(&decrypted));
    
    // Verify round-trip
    assert_eq!(message, decrypted.as_slice());
    println!("   ✓ Round-trip successful!");
    
    // Test key rotation
    println!("\n4. Testing key rotation...");
    engine.rotate_session_keys(&session_id).await?;
    println!("   ✓ Keys rotated successfully");
    
    // Encrypt another message after rotation
    let message2 = b"Message after key rotation";
    println!("\n5. Encrypting message after rotation: {:?}", String::from_utf8_lossy(message2));
    let encrypted2 = engine.encrypt_message(&session_id, message2).await?;
    let decrypted2 = engine.decrypt_message(&session_id, &encrypted2).await?;
    assert_eq!(message2, decrypted2.as_slice());
    println!("   ✓ Encryption still works after rotation!");
    
    // Show session count
    let count = engine.session_count().await;
    println!("\n6. Active sessions: {}", count);
    
    println!("\n=== Demo Complete ===");
    
    Ok(())
}
