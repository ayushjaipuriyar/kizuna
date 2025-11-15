use rand::Rng;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use crate::security::error::{SecurityResult, AuthenticationError};
use crate::security::identity::PeerId;
use crate::security::constant_time::ConstantTime;
use super::PairingCode;

/// Pairing session information
struct PairingSession {
    code: PairingCode,
    peer_id: Option<PeerId>,
}

/// Service for managing pairing codes and verification
pub struct PairingService {
    sessions: Arc<Mutex<HashMap<String, PairingSession>>>,
    timeout_secs: u64,
}

impl PairingService {
    /// Create a new pairing service
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            timeout_secs: 60, // 60 second timeout as per requirements
        }
    }
    
    /// Create a new pairing service with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            timeout_secs,
        }
    }
    
    /// Generate a 6-digit pairing code
    pub fn generate_pairing_code(&self) -> SecurityResult<PairingCode> {
        let mut rng = rand::thread_rng();
        let code_num: u32 = rng.gen_range(0..1_000_000);
        let code = format!("{:06}", code_num);
        
        let pairing_code = PairingCode::new(code.clone());
        
        // Store the session
        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(code.clone(), PairingSession {
            code: pairing_code.clone(),
            peer_id: None,
        });
        
        Ok(pairing_code)
    }
    
    /// Verify a pairing code with a peer
    pub fn verify_pairing_code(&self, code: &PairingCode, peer_id: &PeerId) -> SecurityResult<bool> {
        // Check if code is expired
        if code.is_expired(self.timeout_secs) {
            return Ok(false);
        }
        
        let mut sessions = self.sessions.lock().unwrap();
        
        // Check if session exists
        if let Some(session) = sessions.get_mut(code.code()) {
            // Verify the code hasn't expired
            if session.code.is_expired(self.timeout_secs) {
                sessions.remove(code.code());
                return Ok(false);
            }
            
            // Use constant-time comparison for the pairing code to prevent timing attacks
            if !ConstantTime::compare(code.code().as_bytes(), session.code.code().as_bytes()) {
                return Ok(false);
            }
            
            // If peer_id is already set, verify it matches using constant-time comparison
            if let Some(existing_peer_id) = &session.peer_id {
                if !ConstantTime::compare(existing_peer_id.fingerprint(), peer_id.fingerprint()) {
                    return Err(AuthenticationError::Failed(
                        "Pairing code already used with different peer".to_string()
                    ).into());
                }
                return Ok(true);
            }
            
            // Set the peer_id for this session
            session.peer_id = Some(peer_id.clone());
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Complete a pairing session and remove it
    pub fn complete_pairing(&self, code: &str) -> SecurityResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.remove(code);
        Ok(())
    }
    
    /// Clean up expired pairing sessions
    pub fn cleanup_expired_sessions(&self) -> SecurityResult<()> {
        let mut sessions = self.sessions.lock().unwrap();
        
        // Collect expired session codes
        let expired: Vec<String> = sessions
            .iter()
            .filter(|(_, session)| session.code.is_expired(self.timeout_secs))
            .map(|(code, _)| code.clone())
            .collect();
        
        // Remove expired sessions
        for code in expired {
            sessions.remove(&code);
        }
        
        Ok(())
    }
    
    /// Get the number of active pairing sessions
    pub fn active_sessions_count(&self) -> usize {
        let sessions = self.sessions.lock().unwrap();
        sessions.len()
    }
    
    /// Check if a pairing code exists and is valid
    pub fn is_valid_code(&self, code: &str) -> bool {
        let sessions = self.sessions.lock().unwrap();
        
        if let Some(session) = sessions.get(code) {
            !session.code.is_expired(self.timeout_secs)
        } else {
            false
        }
    }
}

impl Default for PairingService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    
    #[test]
    fn test_generate_pairing_code() {
        let service = PairingService::new();
        let code = service.generate_pairing_code().unwrap();
        
        // Code should be 6 digits
        assert_eq!(code.code().len(), 6);
        assert!(code.code().chars().all(|c| c.is_ascii_digit()));
    }
    
    #[test]
    fn test_verify_pairing_code() {
        let service = PairingService::new();
        let code = service.generate_pairing_code().unwrap();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // First verification should succeed
        assert!(service.verify_pairing_code(&code, &peer_id).unwrap());
        
        // Second verification with same peer should succeed
        assert!(service.verify_pairing_code(&code, &peer_id).unwrap());
    }
    
    #[test]
    fn test_pairing_code_expiration() {
        let service = PairingService::with_timeout(1); // 1 second timeout
        let code = service.generate_pairing_code().unwrap();
        
        // Wait for expiration
        thread::sleep(Duration::from_secs(2));
        
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Verification should fail due to expiration
        assert!(!service.verify_pairing_code(&code, &peer_id).unwrap());
    }
    
    #[test]
    fn test_cleanup_expired_sessions() {
        let service = PairingService::with_timeout(1);
        
        // Generate some codes
        service.generate_pairing_code().unwrap();
        service.generate_pairing_code().unwrap();
        
        assert_eq!(service.active_sessions_count(), 2);
        
        // Wait for expiration
        thread::sleep(Duration::from_secs(2));
        
        // Cleanup
        service.cleanup_expired_sessions().unwrap();
        
        assert_eq!(service.active_sessions_count(), 0);
    }
}
