pub mod identity;
pub mod trust;
pub mod encryption;
pub mod policy;
pub mod error;
pub mod api;
pub mod secure_memory;
pub mod constant_time;

pub use error::{SecurityError, SecurityResult};
pub use api::{SecuritySystem, SecuritySystemConfig, SecuritySystemBuilder};
pub use identity::{DeviceIdentity, PeerId, DisposableIdentity};
pub use encryption::SessionId;
pub use trust::TrustManager;
pub use policy::{PolicyEngine, SecurityEvent, SecurityEventType};

use async_trait::async_trait;

/// Core security trait providing unified interface for cryptographic operations
#[async_trait]
pub trait Security: Send + Sync {
    /// Get the current device identity
    async fn get_device_identity(&self) -> SecurityResult<DeviceIdentity>;
    
    /// Get the peer ID for this device
    async fn get_peer_id(&self) -> SecurityResult<PeerId>;
    
    /// Establish a secure session with a peer
    async fn establish_session(&self, peer_id: &PeerId) -> SecurityResult<SessionId>;
    
    /// Encrypt data for a session
    async fn encrypt_message(&self, session_id: &SessionId, data: &[u8]) -> SecurityResult<Vec<u8>>;
    
    /// Decrypt data from a session
    async fn decrypt_message(&self, session_id: &SessionId, data: &[u8]) -> SecurityResult<Vec<u8>>;
    
    /// Check if a peer is trusted
    async fn is_trusted(&self, peer_id: &PeerId) -> SecurityResult<bool>;
    
    /// Add a trusted peer
    async fn add_trusted_peer(&self, peer_id: PeerId, nickname: String) -> SecurityResult<()>;
}
