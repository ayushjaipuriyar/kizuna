use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::security::{Security, SecurityResult};
use crate::security::identity::{PeerId, DeviceIdentity};
use crate::security::trust::TrustManager;
use crate::security::policy::{PolicyEngine, SecurityEvent, SecurityEventType};
use crate::discovery::{ServiceRecord, DiscoveryError};

/// Identity proof for secure peer announcement
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct IdentityProof {
    /// Peer ID of the announcing device
    pub peer_id: PeerId,
    /// Timestamp of the announcement
    pub timestamp: u64,
    /// Signature of (peer_id + timestamp) using device private key
    pub signature: Vec<u8>,
    /// Public key for verification
    pub public_key: Vec<u8>,
}

impl IdentityProof {
    /// Create a new identity proof
    pub fn new(identity: &DeviceIdentity) -> SecurityResult<Self> {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let peer_id = identity.derive_peer_id();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| crate::security::error::IdentityError::GenerationFailed(
                format!("System time error: {}", e)
            ))?
            .as_secs();
        
        // Create message to sign: peer_id + timestamp
        let mut message = Vec::new();
        message.extend_from_slice(peer_id.fingerprint());
        message.extend_from_slice(&timestamp.to_le_bytes());
        
        // Sign the message
        let signature = identity.sign(&message);
        let public_key = identity.public_key().as_bytes().to_vec();
        
        Ok(Self {
            peer_id,
            timestamp,
            signature: signature.to_vec(),
            public_key,
        })
    }
    
    /// Verify the identity proof
    pub fn verify(&self) -> SecurityResult<bool> {
        use ed25519_dalek::{Signature, VerifyingKey, Verifier};
        
        // Reconstruct the message
        let mut message = Vec::new();
        message.extend_from_slice(self.peer_id.fingerprint());
        message.extend_from_slice(&self.timestamp.to_le_bytes());
        
        // Parse public key
        let public_key_bytes: [u8; 32] = self.public_key
            .as_slice()
            .try_into()
            .map_err(|_| crate::security::error::IdentityError::InvalidPeerId(
                "Invalid public key length".to_string()
            ))?;
        
        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
            .map_err(|e| crate::security::error::IdentityError::InvalidPeerId(
                format!("Invalid public key: {}", e)
            ))?;
        
        // Parse signature
        let signature_bytes: [u8; 64] = self.signature
            .as_slice()
            .try_into()
            .map_err(|_| crate::security::error::IdentityError::InvalidPeerId(
                "Invalid signature length".to_string()
            ))?;
        
        let signature = Signature::from_bytes(&signature_bytes);
        
        // Verify signature
        match verifying_key.verify(&message, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Check if the proof has expired (older than 5 minutes)
    pub fn is_expired(&self) -> bool {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        
        // Proof expires after 5 minutes
        now - self.timestamp > 300
    }
    
    /// Verify that the peer ID matches the public key
    pub fn verify_peer_id(&self) -> SecurityResult<bool> {
        use ed25519_dalek::VerifyingKey;
        
        let public_key_bytes: [u8; 32] = self.public_key
            .as_slice()
            .try_into()
            .map_err(|_| crate::security::error::IdentityError::InvalidPeerId(
                "Invalid public key length".to_string()
            ))?;
        
        let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
            .map_err(|e| crate::security::error::IdentityError::InvalidPeerId(
                format!("Invalid public key: {}", e)
            ))?;
        
        let derived_peer_id = PeerId::from_public_key(&verifying_key);
        Ok(derived_peer_id == self.peer_id)
    }
}

/// Enhanced service record with identity proof
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SecureServiceRecord {
    /// Base service record
    pub service_record: ServiceRecord,
    /// Identity proof for verification
    pub identity_proof: IdentityProof,
}

impl SecureServiceRecord {
    /// Create a new secure service record
    pub fn new(service_record: ServiceRecord, identity_proof: IdentityProof) -> Self {
        Self {
            service_record,
            identity_proof,
        }
    }
    
    /// Verify the identity proof
    pub fn verify(&self) -> SecurityResult<bool> {
        // Verify the signature
        if !self.identity_proof.verify()? {
            return Ok(false);
        }
        
        // Verify peer ID matches public key
        if !self.identity_proof.verify_peer_id()? {
            return Ok(false);
        }
        
        // Verify peer ID matches service record
        if self.service_record.peer_id != self.identity_proof.peer_id.to_string() {
            return Ok(false);
        }
        
        // Check if proof has expired
        if self.identity_proof.is_expired() {
            return Ok(false);
        }
        
        Ok(true)
    }
}

/// Discovery security hooks for identity verification
pub struct DiscoverySecurityHooks {
    /// Security system reference
    security: Arc<dyn Security>,
    /// Trust manager reference
    trust_manager: Arc<dyn TrustManager>,
    /// Policy engine reference
    policy_engine: Arc<dyn PolicyEngine>,
    /// Verified peers cache
    verified_peers: Arc<RwLock<std::collections::HashMap<String, IdentityProof>>>,
}

impl DiscoverySecurityHooks {
    /// Create new discovery security hooks
    pub fn new(
        security: Arc<dyn Security>,
        trust_manager: Arc<dyn TrustManager>,
        policy_engine: Arc<dyn PolicyEngine>,
    ) -> Self {
        Self {
            security,
            trust_manager,
            policy_engine,
            verified_peers: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    /// Create identity proof for announcement
    pub async fn create_identity_proof(&self) -> SecurityResult<IdentityProof> {
        let identity = self.security.get_device_identity().await?;
        IdentityProof::new(&identity)
    }
    
    /// Verify peer identity during discovery
    pub async fn verify_peer_identity(
        &self,
        peer_id: &str,
        identity_proof: &IdentityProof,
    ) -> SecurityResult<bool> {
        // Verify the proof signature and structure
        if !identity_proof.verify()? {
            let event = SecurityEvent::new(
                SecurityEventType::SuspiciousActivity,
                Some(identity_proof.peer_id.clone()),
                format!("Invalid identity proof signature from peer: {}", peer_id),
            );
            self.policy_engine.log_event(event).await?;
            return Ok(false);
        }
        
        // Verify peer ID matches
        if peer_id != identity_proof.peer_id.to_string() {
            let event = SecurityEvent::new(
                SecurityEventType::SuspiciousActivity,
                Some(identity_proof.peer_id.clone()),
                format!("Peer ID mismatch: expected {}, got {}", peer_id, identity_proof.peer_id),
            );
            self.policy_engine.log_event(event).await?;
            return Ok(false);
        }
        
        // Check if proof has expired
        if identity_proof.is_expired() {
            return Ok(false);
        }
        
        // Cache verified peer
        let mut verified = self.verified_peers.write().await;
        verified.insert(peer_id.to_string(), identity_proof.clone());
        
        Ok(true)
    }
    
    /// Filter discovered peers based on trust
    pub async fn filter_by_trust(
        &self,
        peers: Vec<ServiceRecord>,
    ) -> SecurityResult<Vec<ServiceRecord>> {
        let policy = self.policy_engine.get_policy().await?;
        
        // If private mode is disabled, return all peers
        if !policy.private_mode {
            return Ok(peers);
        }
        
        // Filter to only trusted peers in private mode
        let mut filtered = Vec::new();
        for peer in peers {
            // Parse peer ID
            let peer_id = match PeerId::from_string(&peer.peer_id) {
                Ok(id) => id,
                Err(_) => continue, // Skip invalid peer IDs
            };
            
            // Check if peer is trusted
            if self.trust_manager.is_trusted(&peer_id).await? {
                filtered.push(peer);
            }
        }
        
        Ok(filtered)
    }
    
    /// Check if peer is allowed to discover this device
    pub async fn is_discovery_allowed(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        let policy = self.policy_engine.get_policy().await?;
        
        // If private mode is enabled, only allow trusted peers
        if policy.private_mode {
            return self.trust_manager.is_trusted(peer_id).await;
        }
        
        // Otherwise, allow all peers
        Ok(true)
    }
    
    /// Create secure announcement with identity proof
    pub async fn create_secure_announcement(
        &self,
        service_record: ServiceRecord,
    ) -> SecurityResult<SecureServiceRecord> {
        let identity_proof = self.create_identity_proof().await?;
        Ok(SecureServiceRecord::new(service_record, identity_proof))
    }
    
    /// Verify secure announcement from peer
    pub async fn verify_secure_announcement(
        &self,
        secure_record: &SecureServiceRecord,
    ) -> SecurityResult<bool> {
        // Verify the record structure
        if !secure_record.verify()? {
            return Ok(false);
        }
        
        // Verify peer identity
        self.verify_peer_identity(
            &secure_record.service_record.peer_id,
            &secure_record.identity_proof,
        ).await
    }
    
    /// Get verified peers from cache
    pub async fn get_verified_peers(&self) -> Vec<(String, IdentityProof)> {
        let verified = self.verified_peers.read().await;
        verified.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
    
    /// Clear expired proofs from cache
    pub async fn cleanup_expired_proofs(&self) -> usize {
        let mut verified = self.verified_peers.write().await;
        let initial_count = verified.len();
        
        verified.retain(|_, proof| !proof.is_expired());
        
        initial_count - verified.len()
    }
}

/// Convert DiscoveryError to SecurityResult
impl From<DiscoveryError> for crate::security::error::SecurityError {
    fn from(err: DiscoveryError) -> Self {
        crate::security::error::SecurityError::Other(format!("Discovery error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::identity::DeviceIdentity;
    
    #[test]
    fn test_identity_proof_creation_and_verification() {
        // Generate a test identity
        let identity = DeviceIdentity::generate().unwrap();
        
        // Create identity proof
        let proof = IdentityProof::new(&identity).unwrap();
        
        // Verify the proof
        assert!(proof.verify().unwrap());
        
        // Verify peer ID matches
        assert!(proof.verify_peer_id().unwrap());
        
        // Check not expired
        assert!(!proof.is_expired());
    }
    
    #[test]
    fn test_identity_proof_tamper_detection() {
        // Generate a test identity
        let identity = DeviceIdentity::generate().unwrap();
        
        // Create identity proof
        let mut proof = IdentityProof::new(&identity).unwrap();
        
        // Tamper with the signature
        proof.signature[0] ^= 0xFF;
        
        // Verification should fail
        assert!(!proof.verify().unwrap());
    }
    
    #[test]
    fn test_secure_service_record_verification() {
        use std::net::{IpAddr, Ipv4Addr, SocketAddr};
        
        // Generate a test identity
        let identity = DeviceIdentity::generate().unwrap();
        let peer_id = identity.derive_peer_id();
        
        // Create service record
        let mut service_record = ServiceRecord::new(
            peer_id.to_string(),
            "Test Device".to_string(),
            8080
        );
        service_record.add_address(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080));
        service_record.add_capability("clipboard".to_string(), "enabled".to_string());
        
        // Create identity proof
        let identity_proof = IdentityProof::new(&identity).unwrap();
        
        // Create secure service record
        let secure_record = SecureServiceRecord::new(service_record, identity_proof);
        
        // Verify the secure record
        assert!(secure_record.verify().unwrap());
    }
}
