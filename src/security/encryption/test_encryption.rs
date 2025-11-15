#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::security::identity::PeerId;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_session_establishment() {
        let engine = EncryptionEngineImpl::with_defaults();
        let peer_id = PeerId::from_fingerprint([1u8; 32]);
        
        let session_id = engine.establish_session(&peer_id).await.unwrap();
        assert_eq!(engine.session_count().await, 1);
    }
    
    #[tokio::test]
    async fn test_encrypt_decrypt_roundtrip() {
        let engine = EncryptionEngineImpl::with_defaults();
        let peer_id = PeerId::from_fingerprint([2u8; 32]);
        
        let session_id = engine.establish_session(&peer_id).await.unwrap();
        
        let message = b"Hello, secure world!";
        let encrypted = engine.encrypt_message(&session_id, message).await.unwrap();
        let decrypted = engine.decrypt_message(&session_id, &encrypted).await.unwrap();
        
        assert_eq!(message, decrypted.as_slice());
    }
    
    #[tokio::test]
    async fn test_key_rotation() {
        let engine = EncryptionEngineImpl::with_defaults();
        let peer_id = PeerId::from_fingerprint([3u8; 32]);
        
        let session_id = engine.establish_session(&peer_id).await.unwrap();
        
        // Encrypt before rotation
        let message1 = b"Before rotation";
        let encrypted1 = engine.encrypt_message(&session_id, message1).await.unwrap();
        let decrypted1 = engine.decrypt_message(&session_id, &encrypted1).await.unwrap();
        assert_eq!(message1, decrypted1.as_slice());
        
        // Rotate keys
        engine.rotate_session_keys(&session_id).await.unwrap();
        
        // Encrypt after rotation
        let message2 = b"After rotation";
        let encrypted2 = engine.encrypt_message(&session_id, message2).await.unwrap();
        let decrypted2 = engine.decrypt_message(&session_id, &encrypted2).await.unwrap();
        assert_eq!(message2, decrypted2.as_slice());
    }
    
    #[tokio::test]
    async fn test_multiple_messages() {
        let engine = EncryptionEngineImpl::with_defaults();
        let peer_id = PeerId::from_fingerprint([4u8; 32]);
        
        let session_id = engine.establish_session(&peer_id).await.unwrap();
        
        // Send multiple messages
        for i in 0..10 {
            let message = format!("Message {}", i);
            let encrypted = engine.encrypt_message(&session_id, message.as_bytes()).await.unwrap();
            let decrypted = engine.decrypt_message(&session_id, &encrypted).await.unwrap();
            assert_eq!(message.as_bytes(), decrypted.as_slice());
        }
    }
    
    #[tokio::test]
    async fn test_session_not_found() {
        let engine = EncryptionEngineImpl::with_defaults();
        let fake_session_id = SessionId::new();
        
        let result = engine.encrypt_message(&fake_session_id, b"test").await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_key_exchange() {
        let kx1 = KeyExchange::new();
        let kx2 = KeyExchange::new();
        
        let pk1 = kx1.public_key().clone();
        let pk2 = kx2.public_key().clone();
        
        let shared1 = kx1.exchange(&pk2);
        let shared2 = kx2.exchange(&pk1);
        
        // Both sides should derive the same shared secret
        assert_eq!(shared1, shared2);
    }
    
    #[tokio::test]
    async fn test_session_cleanup() {
        let engine = EncryptionEngineImpl::new(
            Duration::from_secs(1),  // 1 second timeout
            Duration::from_secs(60),
        );
        
        let peer_id = PeerId::from_fingerprint([5u8; 32]);
        let _session_id = engine.establish_session(&peer_id).await.unwrap();
        
        assert_eq!(engine.session_count().await, 1);
        
        // Wait for session to expire
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Clean up expired sessions
        let removed = engine.cleanup_expired_sessions().await.unwrap();
        assert_eq!(removed, 1);
        assert_eq!(engine.session_count().await, 0);
    }
}
