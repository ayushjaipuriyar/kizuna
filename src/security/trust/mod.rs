mod database;
mod pairing;
mod allowlist;

pub use database::TrustDatabase;
pub use pairing::PairingService;
pub use allowlist::AllowlistManager;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::security::error::SecurityResult;
use crate::security::identity::PeerId;

/// Trust level for a peer
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Pairing code verified
    Verified,
    /// Manually trusted by user
    Trusted,
    /// In allowlist but not verified
    Allowlisted,
}

/// Service permissions for a peer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServicePermissions {
    pub clipboard: bool,
    pub file_transfer: bool,
    pub camera: bool,
    pub commands: bool,
}

impl Default for ServicePermissions {
    fn default() -> Self {
        Self {
            clipboard: true,
            file_transfer: true,
            camera: false,
            commands: false,
        }
    }
}

/// Trust entry for a peer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustEntry {
    pub peer_id: PeerId,
    pub nickname: String,
    pub first_seen: u64,
    pub last_seen: u64,
    pub trust_level: TrustLevel,
    pub permissions: ServicePermissions,
}

impl TrustEntry {
    pub fn new(peer_id: PeerId, nickname: String, trust_level: TrustLevel) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self {
            peer_id,
            nickname,
            first_seen: now,
            last_seen: now,
            trust_level,
            permissions: ServicePermissions::default(),
        }
    }
}

/// Pairing code for verification
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairingCode {
    code: String,
    created_at: u64,
}

impl PairingCode {
    pub fn new(code: String) -> Self {
        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        Self { code, created_at }
    }
    
    pub fn code(&self) -> &str {
        &self.code
    }
    
    pub fn is_expired(&self, timeout_secs: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now - self.created_at > timeout_secs
    }
}

/// Trust manager trait for managing peer trust relationships
#[async_trait]
pub trait TrustManager: Send + Sync {
    /// Add a trusted peer
    async fn add_trusted_peer(&self, peer_id: PeerId, nickname: String) -> SecurityResult<()>;
    
    /// Remove a trusted peer
    async fn remove_trusted_peer(&self, peer_id: &PeerId) -> SecurityResult<()>;
    
    /// Check if a peer is trusted
    async fn is_trusted(&self, peer_id: &PeerId) -> SecurityResult<bool>;
    
    /// Generate a pairing code
    async fn generate_pairing_code(&self) -> SecurityResult<PairingCode>;
    
    /// Verify a pairing code
    async fn verify_pairing_code(&self, code: &PairingCode, peer_id: &PeerId) -> SecurityResult<bool>;
    
    /// Get the allowlist
    async fn get_allowlist(&self) -> SecurityResult<Vec<PeerId>>;
    
    /// Get a trust entry
    async fn get_trust_entry(&self, peer_id: &PeerId) -> SecurityResult<Option<TrustEntry>>;
    
    /// Get all trusted peers
    async fn get_all_trusted_peers(&self) -> SecurityResult<Vec<TrustEntry>>;
    
    /// Update permissions for a peer
    async fn update_permissions(&self, peer_id: &PeerId, permissions: ServicePermissions) -> SecurityResult<()>;
    
    /// Update trust level for a peer
    async fn update_trust_level(&self, peer_id: &PeerId, trust_level: TrustLevel) -> SecurityResult<()>;
}

/// Implementation of TrustManager
pub struct TrustManagerImpl {
    database: TrustDatabase,
    pairing_service: PairingService,
    allowlist_manager: AllowlistManager,
}

impl TrustManagerImpl {
    /// Create a new trust manager
    pub fn new(db_path: std::path::PathBuf) -> SecurityResult<Self> {
        Ok(Self {
            database: TrustDatabase::new(db_path)?,
            pairing_service: PairingService::new(),
            allowlist_manager: AllowlistManager::new(),
        })
    }
    
    /// Get reference to trust database
    pub fn trust_database(&self) -> &TrustDatabase {
        &self.database
    }
    
    /// Cleanup expired pairing sessions
    pub fn cleanup_expired_sessions(&self) -> SecurityResult<()> {
        self.pairing_service.cleanup_expired_sessions()
    }
}

#[async_trait]
impl TrustManager for TrustManagerImpl {
    async fn add_trusted_peer(&self, peer_id: PeerId, nickname: String) -> SecurityResult<()> {
        let entry = TrustEntry::new(peer_id.clone(), nickname, TrustLevel::Trusted);
        self.database.add_peer(entry)?;
        
        // Also add to allowlist
        self.allowlist_manager.add_to_discovery_allowlist(peer_id.clone())?;
        
        // Set default permissions
        let permissions = ServicePermissions::default();
        self.allowlist_manager.set_permissions(peer_id, permissions)?;
        
        Ok(())
    }
    
    async fn remove_trusted_peer(&self, peer_id: &PeerId) -> SecurityResult<()> {
        self.database.remove_peer(peer_id)?;
        self.allowlist_manager.remove_from_discovery_allowlist(peer_id)?;
        self.allowlist_manager.remove_peer_permissions(peer_id)?;
        Ok(())
    }
    
    async fn is_trusted(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        self.database.is_trusted(peer_id)
    }
    
    async fn generate_pairing_code(&self) -> SecurityResult<PairingCode> {
        self.pairing_service.generate_pairing_code()
    }
    
    async fn verify_pairing_code(&self, code: &PairingCode, peer_id: &PeerId) -> SecurityResult<bool> {
        self.pairing_service.verify_pairing_code(code, peer_id)
    }
    
    async fn get_allowlist(&self) -> SecurityResult<Vec<PeerId>> {
        Ok(self.allowlist_manager.get_discovery_allowlist())
    }
    
    async fn get_trust_entry(&self, peer_id: &PeerId) -> SecurityResult<Option<TrustEntry>> {
        self.database.get_peer(peer_id)
    }
    
    async fn get_all_trusted_peers(&self) -> SecurityResult<Vec<TrustEntry>> {
        self.database.get_all_peers()
    }
    
    async fn update_permissions(&self, peer_id: &PeerId, permissions: ServicePermissions) -> SecurityResult<()> {
        self.database.update_permissions(peer_id, permissions.clone())?;
        self.allowlist_manager.set_permissions(peer_id.clone(), permissions)?;
        Ok(())
    }
    
    async fn update_trust_level(&self, peer_id: &PeerId, trust_level: TrustLevel) -> SecurityResult<()> {
        self.database.update_trust_level(peer_id, trust_level)
    }
}
