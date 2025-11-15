use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use rand::Rng;
use crate::security::error::{SecurityResult, PolicyError};
use crate::security::identity::PeerId;

/// Invite code for private mode connections
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InviteCode {
    code: String,
    peer_id: PeerId,
    created_at: u64,
    expires_at: u64,
}

impl InviteCode {
    /// Create a new invite code
    pub fn new(peer_id: PeerId, validity_duration_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Generate a random 8-character alphanumeric code
        let code = Self::generate_code();
        
        Self {
            code,
            peer_id,
            created_at: now,
            expires_at: now + validity_duration_secs,
        }
    }
    
    /// Generate a random invite code
    fn generate_code() -> String {
        const CHARSET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789"; // Exclude ambiguous chars
        let mut rng = rand::thread_rng();
        
        (0..8)
            .map(|_| {
                let idx = rng.gen_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
    
    /// Get the code string
    pub fn code(&self) -> &str {
        &self.code
    }
    
    /// Get the peer ID this code is for
    pub fn peer_id(&self) -> &PeerId {
        &self.peer_id
    }
    
    /// Check if the code is expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        now > self.expires_at
    }
    
    /// Get time until expiration in seconds
    pub fn time_until_expiration(&self) -> Option<u64> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if now < self.expires_at {
            Some(self.expires_at - now)
        } else {
            None
        }
    }
}

/// Private mode controller for managing discovery visibility
pub struct PrivateModeController {
    /// Whether private mode is enabled
    enabled: Arc<RwLock<bool>>,
    /// Active invite codes
    invite_codes: Arc<RwLock<HashMap<String, InviteCode>>>,
    /// Peers allowed to connect in private mode (via invite or allowlist)
    allowed_peers: Arc<RwLock<Vec<PeerId>>>,
}

impl PrivateModeController {
    /// Create a new private mode controller
    pub fn new() -> Self {
        Self {
            enabled: Arc::new(RwLock::new(false)),
            invite_codes: Arc::new(RwLock::new(HashMap::new())),
            allowed_peers: Arc::new(RwLock::new(Vec::new())),
        }
    }
    
    /// Enable private mode
    pub fn enable(&self) -> SecurityResult<()> {
        let mut enabled = self.enabled.write().unwrap();
        *enabled = true;
        Ok(())
    }
    
    /// Disable private mode
    pub fn disable(&self) -> SecurityResult<()> {
        let mut enabled = self.enabled.write().unwrap();
        *enabled = false;
        Ok(())
    }
    
    /// Check if private mode is enabled
    pub fn is_enabled(&self) -> bool {
        let enabled = self.enabled.read().unwrap();
        *enabled
    }
    
    /// Generate an invite code for a specific peer
    pub fn generate_invite_code(&self, peer_id: PeerId, validity_duration_secs: u64) -> SecurityResult<InviteCode> {
        let invite = InviteCode::new(peer_id.clone(), validity_duration_secs);
        
        let mut codes = self.invite_codes.write().unwrap();
        codes.insert(invite.code().to_string(), invite.clone());
        
        // Also add peer to allowed list
        let mut allowed = self.allowed_peers.write().unwrap();
        if !allowed.contains(&peer_id) {
            allowed.push(peer_id);
        }
        
        Ok(invite)
    }
    
    /// Validate an invite code and return the associated peer ID
    pub fn validate_invite_code(&self, code: &str) -> SecurityResult<Option<PeerId>> {
        let mut codes = self.invite_codes.write().unwrap();
        
        if let Some(invite) = codes.get(code) {
            if invite.is_expired() {
                // Remove expired code
                codes.remove(code);
                return Ok(None);
            }
            
            return Ok(Some(invite.peer_id().clone()));
        }
        
        Ok(None)
    }
    
    /// Add a peer to the allowed list (for allowlist-based access)
    pub fn add_allowed_peer(&self, peer_id: PeerId) -> SecurityResult<()> {
        let mut allowed = self.allowed_peers.write().unwrap();
        if !allowed.contains(&peer_id) {
            allowed.push(peer_id);
        }
        Ok(())
    }
    
    /// Remove a peer from the allowed list
    pub fn remove_allowed_peer(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let mut allowed = self.allowed_peers.write().unwrap();
        allowed.retain(|p| p != peer_id);
        Ok(())
    }
    
    /// Check if a peer is allowed to discover/connect in private mode
    pub fn is_peer_allowed(&self, peer_id: &PeerId) -> bool {
        let allowed = self.allowed_peers.read().unwrap();
        allowed.contains(peer_id)
    }
    
    /// Get all allowed peers
    pub fn get_allowed_peers(&self) -> Vec<PeerId> {
        let allowed = self.allowed_peers.read().unwrap();
        allowed.clone()
    }
    
    /// Check if discovery should be allowed for a peer
    pub fn should_allow_discovery(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        if !self.is_enabled() {
            // Private mode disabled, allow all discovery
            return Ok(true);
        }
        
        // Private mode enabled, only allow if peer is in allowed list
        Ok(self.is_peer_allowed(peer_id))
    }
    
    /// Check if a connection should be allowed for a peer
    pub fn should_allow_connection(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        if !self.is_enabled() {
            // Private mode disabled, allow connection (subject to other policies)
            return Ok(true);
        }
        
        // Private mode enabled, only allow if peer is in allowed list
        if !self.is_peer_allowed(peer_id) {
            return Err(PolicyError::PrivateModeBlocked.into());
        }
        
        Ok(true)
    }
    
    /// Cleanup expired invite codes
    pub fn cleanup_expired_codes(&self) -> SecurityResult<()> {
        let mut codes = self.invite_codes.write().unwrap();
        codes.retain(|_, invite| !invite.is_expired());
        Ok(())
    }
    
    /// Get all active invite codes
    pub fn get_active_invite_codes(&self) -> Vec<InviteCode> {
        let codes = self.invite_codes.read().unwrap();
        codes.values()
            .filter(|invite| !invite.is_expired())
            .cloned()
            .collect()
    }
    
    /// Revoke an invite code
    pub fn revoke_invite_code(&self, code: &str) -> SecurityResult<()> {
        let mut codes = self.invite_codes.write().unwrap();
        codes.remove(code);
        Ok(())
    }
    
    /// Clear all invite codes
    pub fn clear_all_invite_codes(&self) -> SecurityResult<()> {
        let mut codes = self.invite_codes.write().unwrap();
        codes.clear();
        Ok(())
    }
    
    /// Clear all allowed peers
    pub fn clear_allowed_peers(&self) -> SecurityResult<()> {
        let mut allowed = self.allowed_peers.write().unwrap();
        allowed.clear();
        Ok(())
    }
}

impl Default for PrivateModeController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_invite_code_generation() {
        let peer_id = PeerId::from_string("test_peer").unwrap();
        let invite = InviteCode::new(peer_id.clone(), 3600);
        
        assert_eq!(invite.code().len(), 8);
        assert_eq!(invite.peer_id(), &peer_id);
        assert!(!invite.is_expired());
    }
    
    #[test]
    fn test_private_mode_enable_disable() {
        let controller = PrivateModeController::new();
        
        assert!(!controller.is_enabled());
        
        controller.enable().unwrap();
        assert!(controller.is_enabled());
        
        controller.disable().unwrap();
        assert!(!controller.is_enabled());
    }
    
    #[test]
    fn test_invite_code_validation() {
        let controller = PrivateModeController::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        let invite = controller.generate_invite_code(peer_id.clone(), 3600).unwrap();
        
        let validated = controller.validate_invite_code(invite.code()).unwrap();
        assert_eq!(validated, Some(peer_id));
        
        // Invalid code
        let invalid = controller.validate_invite_code("INVALID").unwrap();
        assert_eq!(invalid, None);
    }
    
    #[test]
    fn test_allowed_peers() {
        let controller = PrivateModeController::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        assert!(!controller.is_peer_allowed(&peer_id));
        
        controller.add_allowed_peer(peer_id.clone()).unwrap();
        assert!(controller.is_peer_allowed(&peer_id));
        
        controller.remove_allowed_peer(&peer_id).unwrap();
        assert!(!controller.is_peer_allowed(&peer_id));
    }
    
    #[test]
    fn test_discovery_filtering() {
        let controller = PrivateModeController::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Private mode disabled - allow all
        assert!(controller.should_allow_discovery(&peer_id).unwrap());
        
        // Enable private mode
        controller.enable().unwrap();
        
        // Not in allowed list - block
        assert!(!controller.should_allow_discovery(&peer_id).unwrap());
        
        // Add to allowed list
        controller.add_allowed_peer(peer_id.clone()).unwrap();
        
        // Now allowed
        assert!(controller.should_allow_discovery(&peer_id).unwrap());
    }
    
    #[test]
    fn test_connection_filtering() {
        let controller = PrivateModeController::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Private mode disabled - allow
        assert!(controller.should_allow_connection(&peer_id).unwrap());
        
        // Enable private mode
        controller.enable().unwrap();
        
        // Not in allowed list - block
        assert!(controller.should_allow_connection(&peer_id).is_err());
        
        // Add to allowed list
        controller.add_allowed_peer(peer_id.clone()).unwrap();
        
        // Now allowed
        assert!(controller.should_allow_connection(&peer_id).unwrap());
    }
}
