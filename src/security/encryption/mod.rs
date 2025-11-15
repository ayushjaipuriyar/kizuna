use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop};

use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng as AeadOsRng},
    ChaCha20Poly1305, Nonce,
};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey};
use sha2::{Sha256, Digest};
use hmac::{Hmac, Mac};

use crate::security::error::{SecurityResult, EncryptionError};
use crate::security::identity::PeerId;
use crate::security::secure_memory::{SecureKey, SecureBuffer, SecureMemory};
use crate::security::constant_time::ConstantTime;

type HmacSha256 = Hmac<Sha256>;

#[cfg(test)]
mod test_encryption;

/// Session ID for encrypted communications
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId {
    id: Uuid,
}

impl SessionId {
    pub fn new() -> Self {
        Self { id: Uuid::new_v4() }
    }
    
    pub fn from_uuid(id: Uuid) -> Self {
        Self { id }
    }
    
    pub fn as_uuid(&self) -> &Uuid {
        &self.id
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id)
    }
}

/// Security session containing encryption keys
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct SecuritySession {
    /// Unique session identifier
    #[zeroize(skip)]
    session_id: SessionId,
    /// Peer ID for this session
    #[zeroize(skip)]
    peer_id: PeerId,
    /// Shared secret from key exchange
    shared_secret: SecureKey<32>,
    /// Encryption key for sending messages
    send_key: SecureKey<32>,
    /// Encryption key for receiving messages
    recv_key: SecureKey<32>,
    /// Nonce counter for sending (prevents reuse)
    send_nonce_counter: u64,
    /// Nonce counter for receiving (prevents replay)
    recv_nonce_counter: u64,
    /// Timestamp when session was created
    created_at: u64,
    /// Timestamp of last key rotation
    last_rotation: u64,
}

impl SecuritySession {
    /// Create a new security session from a shared secret
    fn new(peer_id: PeerId, shared_secret: [u8; 32]) -> SecurityResult<Self> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| EncryptionError::KeyExchangeFailed(format!("System time error: {}", e)))?
            .as_secs();
        
        // Derive separate send and receive keys using HKDF
        let (send_key, recv_key) = Self::derive_session_keys(&shared_secret)?;
        
        Ok(Self {
            session_id: SessionId::new(),
            peer_id,
            shared_secret: SecureKey::new(shared_secret),
            send_key: SecureKey::new(send_key),
            recv_key: SecureKey::new(recv_key),
            send_nonce_counter: 0,
            recv_nonce_counter: 0,
            created_at: now,
            last_rotation: now,
        })
    }
    
    /// Derive session keys from shared secret using HKDF-like construction
    fn derive_session_keys(shared_secret: &[u8; 32]) -> SecurityResult<([u8; 32], [u8; 32])> {
        // Use HMAC-SHA256 as a KDF (simplified HKDF)
        use hmac::Mac;
        let mut send_mac = <HmacSha256 as Mac>::new_from_slice(shared_secret)
            .map_err(|e| EncryptionError::KeyExchangeFailed(format!("HMAC init failed: {}", e)))?;
        send_mac.update(b"kizuna-send-key-v1");
        let send_key: [u8; 32] = send_mac.finalize().into_bytes().into();
        
        let mut recv_mac = <HmacSha256 as Mac>::new_from_slice(shared_secret)
            .map_err(|e| EncryptionError::KeyExchangeFailed(format!("HMAC init failed: {}", e)))?;
        recv_mac.update(b"kizuna-recv-key-v1");
        let recv_key: [u8; 32] = recv_mac.finalize().into_bytes().into();
        
        Ok((send_key, recv_key))
    }
    
    /// Get the session ID
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }
    
    /// Get the peer ID
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
    
    /// Get the next send nonce
    fn next_send_nonce(&mut self) -> [u8; 12] {
        let counter = self.send_nonce_counter;
        self.send_nonce_counter = self.send_nonce_counter.wrapping_add(1);
        
        let mut nonce = [0u8; 12];
        nonce[4..12].copy_from_slice(&counter.to_le_bytes());
        nonce
    }
    
    /// Validate and get the next receive nonce
    fn validate_recv_nonce(&mut self, nonce: &[u8; 12]) -> SecurityResult<()> {
        // Extract counter from nonce
        let mut counter_bytes = [0u8; 8];
        counter_bytes.copy_from_slice(&nonce[4..12]);
        let counter = u64::from_le_bytes(counter_bytes);
        
        // Prevent replay attacks - nonce must be greater than last received
        // Use constant-time comparison to avoid timing side-channels
        let is_valid = !ConstantTime::less_than_u64(counter, self.recv_nonce_counter + 1);
        
        if !is_valid {
            return Err(EncryptionError::AuthenticationFailed.into());
        }
        
        self.recv_nonce_counter = counter;
        Ok(())
    }
    
    /// Check if session has expired
    pub fn is_expired(&self, timeout: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        now - self.created_at > timeout.as_secs()
    }
    
    /// Check if keys need rotation
    pub fn needs_rotation(&self, rotation_interval: Duration) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        now - self.last_rotation > rotation_interval.as_secs()
    }
    
    /// Rotate session keys for forward secrecy
    pub fn rotate_keys(&mut self) -> SecurityResult<()> {
        // Derive new keys from current shared secret + timestamp
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| EncryptionError::KeyRotationFailed(format!("System time error: {}", e)))?
            .as_secs();
        
        // Create new shared secret by hashing old secret + timestamp
        let mut hasher = Sha256::new();
        hasher.update(self.shared_secret.as_bytes());
        hasher.update(&now.to_le_bytes());
        let new_shared_secret: [u8; 32] = hasher.finalize().into();
        
        // Zeroize old keys before replacing (SecureKey handles this automatically on drop)
        self.send_key.zeroize_key();
        self.recv_key.zeroize_key();
        self.shared_secret.zeroize_key();
        
        // Derive new session keys
        let (send_key, recv_key) = Self::derive_session_keys(&new_shared_secret)?;
        
        self.shared_secret = SecureKey::new(new_shared_secret);
        self.send_key = SecureKey::new(send_key);
        self.recv_key = SecureKey::new(recv_key);
        self.last_rotation = now;
        
        // Reset nonce counters after rotation
        self.send_nonce_counter = 0;
        self.recv_nonce_counter = 0;
        
        Ok(())
    }
}

/// Key exchange handler for X25519 ECDH
pub struct KeyExchange {
    /// Our ephemeral secret key
    secret: EphemeralSecret,
    /// Our ephemeral public key
    public_key: X25519PublicKey,
}

impl KeyExchange {
    /// Create a new key exchange with a random ephemeral key
    pub fn new() -> Self {
        let secret = EphemeralSecret::random_from_rng(AeadOsRng);
        let public_key = X25519PublicKey::from(&secret);
        
        Self { secret, public_key }
    }
    
    /// Get our public key to send to peer
    pub fn public_key(&self) -> &X25519PublicKey {
        &self.public_key
    }
    
    /// Perform key exchange with peer's public key
    pub fn exchange(self, peer_public_key: &X25519PublicKey) -> [u8; 32] {
        let shared_secret = self.secret.diffie_hellman(peer_public_key);
        shared_secret.to_bytes()
    }
}

impl Default for KeyExchange {
    fn default() -> Self {
        Self::new()
    }
}

/// Encryption engine implementation for end-to-end encryption
pub struct EncryptionEngineImpl {
    /// Active sessions indexed by session ID
    sessions: Arc<RwLock<HashMap<SessionId, SecuritySession>>>,
    /// Session timeout duration
    session_timeout: Duration,
    /// Key rotation interval
    key_rotation_interval: Duration,
}

impl EncryptionEngineImpl {
    /// Create a new encryption engine
    pub fn new(session_timeout: Duration, key_rotation_interval: Duration) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            session_timeout,
            key_rotation_interval,
        }
    }
    
    /// Create with default settings (1 hour timeout, 15 minute rotation)
    pub fn with_defaults() -> Self {
        Self::new(
            Duration::from_secs(3600),      // 1 hour session timeout
            Duration::from_secs(900),       // 15 minute key rotation
        )
    }
    
    /// Establish a session with a peer using key exchange
    /// 
    /// This performs X25519 ECDH key exchange and derives session keys
    pub async fn establish_session_with_exchange(
        &self,
        peer_id: PeerId,
        our_public_key: &X25519PublicKey,
        peer_public_key: &X25519PublicKey,
        our_secret: EphemeralSecret,
    ) -> SecurityResult<SessionId> {
        // Perform ECDH key exchange
        let shared_secret_obj = our_secret.diffie_hellman(peer_public_key);
        let shared_secret = shared_secret_obj.to_bytes();
        
        // Create session from shared secret
        let session = SecuritySession::new(peer_id, shared_secret)?;
        let session_id = session.session_id().clone();
        
        // Store session
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);
        
        Ok(session_id)
    }
    
    /// Encrypt a message for a session using ChaCha20-Poly1305
    async fn encrypt_with_session(
        &self,
        session: &mut SecuritySession,
        data: &[u8],
    ) -> SecurityResult<Vec<u8>> {
        // Check if key rotation is needed
        if session.needs_rotation(self.key_rotation_interval) {
            session.rotate_keys()?;
        }
        
        // Create cipher from send key
        let cipher = ChaCha20Poly1305::new_from_slice(session.send_key.as_bytes())
            .map_err(|e| EncryptionError::EncryptionFailed(format!("Cipher init failed: {}", e)))?;
        
        // Get next nonce
        let nonce_bytes = session.next_send_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt data with authenticated encryption
        let ciphertext = cipher
            .encrypt(nonce, data)
            .map_err(|e| EncryptionError::EncryptionFailed(format!("Encryption failed: {}", e)))?;
        
        // Prepend nonce to ciphertext for transmission
        let mut result = Vec::with_capacity(12 + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);
        
        Ok(result)
    }
    
    /// Decrypt a message from a session using ChaCha20-Poly1305
    async fn decrypt_with_session(
        &self,
        session: &mut SecuritySession,
        data: &[u8],
    ) -> SecurityResult<Vec<u8>> {
        // Extract nonce and ciphertext
        if data.len() < 12 {
            return Err(EncryptionError::DecryptionFailed(
                "Data too short to contain nonce".to_string()
            ).into());
        }
        
        let mut nonce_bytes = [0u8; 12];
        nonce_bytes.copy_from_slice(&data[0..12]);
        let ciphertext = &data[12..];
        
        // Validate nonce to prevent replay attacks
        session.validate_recv_nonce(&nonce_bytes)?;
        
        // Create cipher from receive key
        let cipher = ChaCha20Poly1305::new_from_slice(session.recv_key.as_bytes())
            .map_err(|e| EncryptionError::DecryptionFailed(format!("Cipher init failed: {}", e)))?;
        
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Decrypt and verify authentication tag
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| EncryptionError::AuthenticationFailed)?;
        
        Ok(plaintext)
    }
    
    /// Clean up expired sessions
    pub async fn cleanup_expired_sessions(&self) -> SecurityResult<usize> {
        let mut sessions = self.sessions.write().await;
        let initial_count = sessions.len();
        
        sessions.retain(|_, session| !session.is_expired(self.session_timeout));
        
        let removed_count = initial_count - sessions.len();
        Ok(removed_count)
    }
    
    /// Get session count
    pub async fn session_count(&self) -> usize {
        let sessions = self.sessions.read().await;
        sessions.len()
    }
    
    /// Remove a specific session
    pub async fn remove_session(&self, session_id: &SessionId) -> SecurityResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        Ok(())
    }
}

/// Encryption engine trait for end-to-end encryption
#[async_trait]
pub trait EncryptionEngine: Send + Sync {
    /// Establish a secure session with a peer
    async fn establish_session(&self, peer_id: &PeerId) -> SecurityResult<SessionId>;
    
    /// Encrypt a message for a session
    async fn encrypt_message(&self, session_id: &SessionId, data: &[u8]) -> SecurityResult<Vec<u8>>;
    
    /// Decrypt a message from a session
    async fn decrypt_message(&self, session_id: &SessionId, data: &[u8]) -> SecurityResult<Vec<u8>>;
    
    /// Rotate session keys for forward secrecy
    async fn rotate_session_keys(&self, session_id: &SessionId) -> SecurityResult<()>;
}

#[async_trait]
impl EncryptionEngine for EncryptionEngineImpl {
    async fn establish_session(&self, peer_id: &PeerId) -> SecurityResult<SessionId> {
        // For the trait implementation, we create a simplified session
        // In practice, this would involve actual key exchange with the peer
        let key_exchange = KeyExchange::new();
        
        // In a real implementation, we would:
        // 1. Send our public key to the peer
        // 2. Receive peer's public key
        // 3. Perform the exchange
        // For now, we create a dummy shared secret for testing
        let shared_secret = [0u8; 32]; // This would be the result of key_exchange.exchange()
        
        let session = SecuritySession::new(peer_id.clone(), shared_secret)?;
        let session_id = session.session_id().clone();
        
        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session);
        
        Ok(session_id)
    }
    
    async fn encrypt_message(&self, session_id: &SessionId, data: &[u8]) -> SecurityResult<Vec<u8>> {
        let mut sessions = self.sessions.write().await;
        
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| EncryptionError::SessionNotFound(session_id.to_string()))?;
        
        // Check if session has expired
        if session.is_expired(self.session_timeout) {
            return Err(EncryptionError::SessionExpired(session_id.to_string()).into());
        }
        
        self.encrypt_with_session(session, data).await
    }
    
    async fn decrypt_message(&self, session_id: &SessionId, data: &[u8]) -> SecurityResult<Vec<u8>> {
        let mut sessions = self.sessions.write().await;
        
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| EncryptionError::SessionNotFound(session_id.to_string()))?;
        
        // Check if session has expired
        if session.is_expired(self.session_timeout) {
            return Err(EncryptionError::SessionExpired(session_id.to_string()).into());
        }
        
        self.decrypt_with_session(session, data).await
    }
    
    async fn rotate_session_keys(&self, session_id: &SessionId) -> SecurityResult<()> {
        let mut sessions = self.sessions.write().await;
        
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| EncryptionError::SessionNotFound(session_id.to_string()))?;
        
        session.rotate_keys()
    }
}
