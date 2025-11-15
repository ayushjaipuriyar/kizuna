use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use zeroize::{Zeroize, ZeroizeOnDrop};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer};
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;
use crate::security::error::{SecurityResult, IdentityError};

#[cfg(test)]
mod test_identity;

/// Device identity containing Ed25519 keypair
#[derive(Clone)]
pub struct DeviceIdentity {
    /// Ed25519 private signing key
    private_key: SigningKey,
    /// Ed25519 public verifying key
    public_key: VerifyingKey,
    /// Timestamp when identity was created
    created_at: u64,
    /// Optional backup phrase for recovery (not implemented in this task)
    backup_phrase: Option<String>,
}

impl DeviceIdentity {
    /// Generate a new device identity with a random Ed25519 keypair
    pub fn generate() -> SecurityResult<Self> {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| IdentityError::GenerationFailed(format!("System time error: {}", e)))?
            .as_secs();
        
        Ok(Self {
            private_key: signing_key,
            public_key: verifying_key,
            created_at,
            backup_phrase: None,
        })
    }
    
    /// Get the public key
    pub fn public_key(&self) -> &VerifyingKey {
        &self.public_key
    }
    
    /// Get the private key (use with caution)
    pub fn private_key(&self) -> &SigningKey {
        &self.private_key
    }
    
    /// Get the creation timestamp
    pub fn created_at(&self) -> u64 {
        self.created_at
    }
    
    /// Derive a PeerId from this identity
    pub fn derive_peer_id(&self) -> PeerId {
        PeerId::from_public_key(&self.public_key)
    }
    
    /// Sign data with the private key
    pub fn sign(&self, data: &[u8]) -> Signature {
        self.private_key.sign(data)
    }
    
    /// Serialize the identity to bytes for secure storage
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        
        // Add private key (32 bytes)
        bytes.extend_from_slice(&self.private_key.to_bytes());
        
        // Add public key (32 bytes)
        bytes.extend_from_slice(self.public_key.as_bytes());
        
        // Add created_at timestamp (8 bytes)
        bytes.extend_from_slice(&self.created_at.to_le_bytes());
        
        // Add backup phrase length and data if present
        if let Some(ref phrase) = self.backup_phrase {
            let phrase_bytes = phrase.as_bytes();
            bytes.extend_from_slice(&(phrase_bytes.len() as u32).to_le_bytes());
            bytes.extend_from_slice(phrase_bytes);
        } else {
            bytes.extend_from_slice(&0u32.to_le_bytes());
        }
        
        bytes
    }
    
    /// Deserialize an identity from bytes
    pub fn from_bytes(bytes: &[u8]) -> SecurityResult<Self> {
        if bytes.len() < 72 {
            return Err(IdentityError::Corrupted(
                "Insufficient data for identity deserialization".to_string()
            ).into());
        }
        
        // Extract private key (32 bytes)
        let private_key_bytes: [u8; 32] = bytes[0..32]
            .try_into()
            .map_err(|_| IdentityError::Corrupted("Invalid private key".to_string()))?;
        let private_key = SigningKey::from_bytes(&private_key_bytes);
        
        // Extract public key (32 bytes)
        let public_key_bytes: [u8; 32] = bytes[32..64]
            .try_into()
            .map_err(|_| IdentityError::Corrupted("Invalid public key".to_string()))?;
        let public_key = VerifyingKey::from_bytes(&public_key_bytes)
            .map_err(|e| IdentityError::Corrupted(format!("Invalid public key: {}", e)))?;
        
        // Verify that public key matches private key
        if public_key != private_key.verifying_key() {
            return Err(IdentityError::Corrupted(
                "Public key does not match private key".to_string()
            ).into());
        }
        
        // Extract created_at timestamp (8 bytes)
        let created_at_bytes: [u8; 8] = bytes[64..72]
            .try_into()
            .map_err(|_| IdentityError::Corrupted("Invalid timestamp".to_string()))?;
        let created_at = u64::from_le_bytes(created_at_bytes);
        
        // Extract backup phrase if present
        let backup_phrase = if bytes.len() > 72 {
            let phrase_len_bytes: [u8; 4] = bytes[72..76]
                .try_into()
                .map_err(|_| IdentityError::Corrupted("Invalid backup phrase length".to_string()))?;
            let phrase_len = u32::from_le_bytes(phrase_len_bytes) as usize;
            
            if phrase_len > 0 {
                if bytes.len() < 76 + phrase_len {
                    return Err(IdentityError::Corrupted(
                        "Insufficient data for backup phrase".to_string()
                    ).into());
                }
                
                let phrase_bytes = &bytes[76..76 + phrase_len];
                let phrase = String::from_utf8(phrase_bytes.to_vec())
                    .map_err(|e| IdentityError::Corrupted(format!("Invalid backup phrase UTF-8: {}", e)))?;
                Some(phrase)
            } else {
                None
            }
        } else {
            None
        };
        
        Ok(Self {
            private_key,
            public_key,
            created_at,
            backup_phrase,
        })
    }
}

/// Peer ID derived from public key fingerprint
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId {
    fingerprint: [u8; 32],
}

impl PeerId {
    /// Create a new PeerId from a fingerprint
    pub fn from_fingerprint(fingerprint: [u8; 32]) -> Self {
        Self { fingerprint }
    }
    
    /// Derive a PeerId from an Ed25519 public key using SHA-256
    pub fn from_public_key(public_key: &VerifyingKey) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(public_key.as_bytes());
        let fingerprint: [u8; 32] = hasher.finalize().into();
        
        Self { fingerprint }
    }
    
    /// Get the fingerprint bytes
    pub fn fingerprint(&self) -> &[u8; 32] {
        &self.fingerprint
    }
    
    /// Get a display-friendly representation (first 8 bytes as hex)
    pub fn display_name(&self) -> String {
        hex::encode(&self.fingerprint[..8])
    }
    
    /// Get the full fingerprint as hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.fingerprint)
    }
    
    /// Create a PeerId from a hex string
    pub fn from_hex(hex_str: &str) -> SecurityResult<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| IdentityError::InvalidPeerId(format!("Invalid hex: {}", e)))?;
        
        if bytes.len() != 32 {
            return Err(IdentityError::InvalidPeerId(
                format!("Expected 32 bytes, got {}", bytes.len())
            ).into());
        }
        
        let fingerprint: [u8; 32] = bytes.try_into()
            .map_err(|_| IdentityError::InvalidPeerId("Failed to convert to array".to_string()))?;
        
        Ok(Self { fingerprint })
    }
    
    /// Convert PeerId to string (hex representation)
    pub fn to_string(&self) -> String {
        self.to_hex()
    }
    
    /// Create PeerId from string (hex representation)
    pub fn from_string(s: &str) -> SecurityResult<Self> {
        Self::from_hex(s)
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Disposable identity for temporary use
#[derive(Clone)]
pub struct DisposableIdentity {
    /// Unique identifier for this disposable identity
    id: String,
    /// Ed25519 private signing key
    private_key: SigningKey,
    /// Ed25519 public verifying key
    public_key: VerifyingKey,
    /// Timestamp when identity was created
    created_at: u64,
    /// Timestamp when identity expires (optional)
    expires_at: Option<u64>,
    /// Whether this identity is currently active
    is_active: bool,
}

impl DisposableIdentity {
    /// Generate a new disposable identity
    pub fn generate(lifetime_seconds: Option<u64>) -> SecurityResult<Self> {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| IdentityError::GenerationFailed(format!("System time error: {}", e)))?
            .as_secs();
        
        let expires_at = lifetime_seconds.map(|lifetime| created_at + lifetime);
        
        // Generate a unique ID for this disposable identity
        let id = uuid::Uuid::new_v4().to_string();
        
        Ok(Self {
            id,
            private_key: signing_key,
            public_key: verifying_key,
            created_at,
            expires_at,
            is_active: false,
        })
    }
    
    /// Get the unique identifier
    pub fn id(&self) -> &str {
        &self.id
    }
    
    /// Get the public key
    pub fn public_key(&self) -> &VerifyingKey {
        &self.public_key
    }
    
    /// Get the private key (use with caution)
    pub fn private_key(&self) -> &SigningKey {
        &self.private_key
    }
    
    /// Get the creation timestamp
    pub fn created_at(&self) -> u64 {
        self.created_at
    }
    
    /// Get the expiration timestamp
    pub fn expires_at(&self) -> Option<u64> {
        self.expires_at
    }
    
    /// Check if this identity is active
    pub fn is_active(&self) -> bool {
        self.is_active
    }
    
    /// Check if this identity has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            now >= expires_at
        } else {
            false
        }
    }
    
    /// Activate this disposable identity
    pub fn activate(&mut self) {
        self.is_active = true;
    }
    
    /// Deactivate this disposable identity
    pub fn deactivate(&mut self) {
        self.is_active = false;
    }
    
    /// Derive a PeerId from this disposable identity
    pub fn derive_peer_id(&self) -> PeerId {
        PeerId::from_public_key(&self.public_key)
    }
    
    /// Sign data with the private key
    pub fn sign(&self, data: &[u8]) -> Signature {
        self.private_key.sign(data)
    }
}

/// Identity manager trait for managing device identities
#[async_trait]
pub trait IdentityManager: Send + Sync {
    /// Get the device identity
    async fn get_device_identity(&self) -> SecurityResult<DeviceIdentity>;
    
    /// Get the peer ID for this device
    async fn get_peer_id(&self) -> SecurityResult<PeerId>;
    
    /// Create a new disposable identity
    async fn create_disposable_identity(&self) -> SecurityResult<DisposableIdentity>;
    
    /// Activate a disposable identity
    async fn activate_disposable_identity(&self, id: DisposableIdentity) -> SecurityResult<()>;
    
    /// Clean up expired disposable identities
    async fn cleanup_expired_identities(&self) -> SecurityResult<()>;
}

/// Secure keystore for device identity storage
pub struct IdentityStore {
    service_name: String,
    username: String,
}

impl IdentityStore {
    /// Create a new identity store
    pub fn new(service_name: impl Into<String>, username: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
            username: username.into(),
        }
    }
    
    /// Default identity store for Kizuna
    pub fn default() -> Self {
        let username = whoami::username();
        Self::new("kizuna.device_identity", username)
    }
    
    /// Save device identity to secure storage
    pub fn save_identity(&self, identity: &DeviceIdentity) -> SecurityResult<()> {
        let entry = keyring::Entry::new(&self.service_name, &self.username)
            .map_err(|e| IdentityError::KeystoreError(format!("Failed to create keyring entry: {}", e)))?;
        
        let identity_bytes = identity.to_bytes();
        let identity_hex = hex::encode(&identity_bytes);
        
        entry.set_password(&identity_hex)
            .map_err(|e| IdentityError::SaveFailed(format!("Failed to save to keystore: {}", e)))?;
        
        Ok(())
    }
    
    /// Load device identity from secure storage
    pub fn load_identity(&self) -> SecurityResult<DeviceIdentity> {
        let entry = keyring::Entry::new(&self.service_name, &self.username)
            .map_err(|e| IdentityError::KeystoreError(format!("Failed to create keyring entry: {}", e)))?;
        
        let identity_hex = entry.get_password()
            .map_err(|e| IdentityError::LoadFailed(format!("Failed to load from keystore: {}", e)))?;
        
        let identity_bytes = hex::decode(&identity_hex)
            .map_err(|e| IdentityError::Corrupted(format!("Invalid hex data: {}", e)))?;
        
        DeviceIdentity::from_bytes(&identity_bytes)
    }
    
    /// Check if an identity exists in storage
    pub fn has_identity(&self) -> bool {
        let entry = match keyring::Entry::new(&self.service_name, &self.username) {
            Ok(e) => e,
            Err(_) => return false,
        };
        
        entry.get_password().is_ok()
    }
    
    /// Delete identity from secure storage
    pub fn delete_identity(&self) -> SecurityResult<()> {
        let entry = keyring::Entry::new(&self.service_name, &self.username)
            .map_err(|e| IdentityError::KeystoreError(format!("Failed to create keyring entry: {}", e)))?;
        
        entry.delete_password()
            .map_err(|e| IdentityError::KeystoreError(format!("Failed to delete from keystore: {}", e)))?;
        
        Ok(())
    }
    
    /// Get or create device identity
    /// 
    /// This is the primary method for obtaining a device identity.
    /// It will load an existing identity from storage, or generate and save a new one.
    pub fn get_or_create_identity(&self) -> SecurityResult<DeviceIdentity> {
        if self.has_identity() {
            self.load_identity()
        } else {
            let identity = DeviceIdentity::generate()?;
            self.save_identity(&identity)?;
            Ok(identity)
        }
    }
    
    /// Backup identity to a file (for migration/recovery)
    pub fn backup_to_file(&self, path: &std::path::Path) -> SecurityResult<()> {
        let identity = self.load_identity()?;
        let identity_bytes = identity.to_bytes();
        
        std::fs::write(path, &identity_bytes)
            .map_err(|e| IdentityError::SaveFailed(format!("Failed to write backup file: {}", e)))?;
        
        Ok(())
    }
    
    /// Restore identity from a backup file
    pub fn restore_from_file(&self, path: &std::path::Path) -> SecurityResult<()> {
        let identity_bytes = std::fs::read(path)
            .map_err(|e| IdentityError::LoadFailed(format!("Failed to read backup file: {}", e)))?;
        
        let identity = DeviceIdentity::from_bytes(&identity_bytes)?;
        self.save_identity(&identity)?;
        
        Ok(())
    }
    
    /// Migrate identity to a new version (for future compatibility)
    /// 
    /// Currently this is a no-op, but provides a hook for future key format migrations
    pub fn migrate_identity(&self, _from_version: u32, _to_version: u32) -> SecurityResult<()> {
        // Load existing identity
        let identity = self.load_identity()?;
        
        // In the future, this would perform version-specific transformations
        // For now, just re-save the identity
        self.save_identity(&identity)?;
        
        Ok(())
    }
}

/// Manager for disposable identities
pub struct DisposableIdentityManager {
    /// Pool of disposable identities
    identities: std::sync::Arc<tokio::sync::RwLock<Vec<DisposableIdentity>>>,
    /// Default lifetime for disposable identities (in seconds)
    default_lifetime: u64,
}

impl DisposableIdentityManager {
    /// Create a new disposable identity manager
    pub fn new(default_lifetime: u64) -> Self {
        Self {
            identities: std::sync::Arc::new(tokio::sync::RwLock::new(Vec::new())),
            default_lifetime,
        }
    }
    
    /// Create a new disposable identity with default lifetime
    pub async fn create_identity(&self) -> SecurityResult<DisposableIdentity> {
        self.create_identity_with_lifetime(Some(self.default_lifetime)).await
    }
    
    /// Create a new disposable identity with custom lifetime
    pub async fn create_identity_with_lifetime(&self, lifetime: Option<u64>) -> SecurityResult<DisposableIdentity> {
        let identity = DisposableIdentity::generate(lifetime)?;
        
        let mut identities = self.identities.write().await;
        identities.push(identity.clone());
        
        Ok(identity)
    }
    
    /// Activate a disposable identity
    pub async fn activate_identity(&self, id: &str) -> SecurityResult<()> {
        let mut identities = self.identities.write().await;
        
        // Deactivate all other identities
        for identity in identities.iter_mut() {
            identity.deactivate();
        }
        
        // Activate the requested identity
        if let Some(identity) = identities.iter_mut().find(|i| i.id() == id) {
            if identity.is_expired() {
                return Err(IdentityError::Corrupted(
                    "Cannot activate expired identity".to_string()
                ).into());
            }
            identity.activate();
            Ok(())
        } else {
            Err(IdentityError::LoadFailed(
                format!("Disposable identity not found: {}", id)
            ).into())
        }
    }
    
    /// Deactivate a disposable identity
    pub async fn deactivate_identity(&self, id: &str) -> SecurityResult<()> {
        let mut identities = self.identities.write().await;
        
        if let Some(identity) = identities.iter_mut().find(|i| i.id() == id) {
            identity.deactivate();
            Ok(())
        } else {
            Err(IdentityError::LoadFailed(
                format!("Disposable identity not found: {}", id)
            ).into())
        }
    }
    
    /// Get the currently active disposable identity
    pub async fn get_active_identity(&self) -> SecurityResult<Option<DisposableIdentity>> {
        let identities = self.identities.read().await;
        Ok(identities.iter().find(|i| i.is_active()).cloned())
    }
    
    /// Get a disposable identity by ID
    pub async fn get_identity(&self, id: &str) -> SecurityResult<Option<DisposableIdentity>> {
        let identities = self.identities.read().await;
        Ok(identities.iter().find(|i| i.id() == id).cloned())
    }
    
    /// List all disposable identities
    pub async fn list_identities(&self) -> Vec<DisposableIdentity> {
        let identities = self.identities.read().await;
        identities.clone()
    }
    
    /// Clean up expired disposable identities
    pub async fn cleanup_expired(&self) -> SecurityResult<usize> {
        let mut identities = self.identities.write().await;
        let initial_count = identities.len();
        
        identities.retain(|identity| !identity.is_expired());
        
        let removed_count = initial_count - identities.len();
        Ok(removed_count)
    }
    
    /// Delete a specific disposable identity
    pub async fn delete_identity(&self, id: &str) -> SecurityResult<()> {
        let mut identities = self.identities.write().await;
        
        let initial_len = identities.len();
        identities.retain(|identity| identity.id() != id);
        
        if identities.len() == initial_len {
            Err(IdentityError::LoadFailed(
                format!("Disposable identity not found: {}", id)
            ).into())
        } else {
            Ok(())
        }
    }
    
    /// Delete all disposable identities
    pub async fn delete_all(&self) -> SecurityResult<()> {
        let mut identities = self.identities.write().await;
        identities.clear();
        Ok(())
    }
}

impl Default for DisposableIdentityManager {
    fn default() -> Self {
        // Default lifetime of 24 hours
        Self::new(24 * 60 * 60)
    }
}
