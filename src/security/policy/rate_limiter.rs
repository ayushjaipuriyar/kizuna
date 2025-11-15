use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::security::error::{SecurityResult, PolicyError};
use crate::security::identity::PeerId;

/// Connection attempt record
#[derive(Clone, Debug)]
struct ConnectionAttempt {
    timestamp: u64,
    count: u32,
}

/// Rate limiter configuration
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum connection attempts per time window
    pub max_attempts: u32,
    /// Time window in seconds
    pub window_secs: u64,
    /// Backoff multiplier for repeated violations
    pub backoff_multiplier: u32,
    /// Maximum backoff duration in seconds
    pub max_backoff_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            window_secs: 60,
            backoff_multiplier: 2,
            max_backoff_secs: 3600, // 1 hour
        }
    }
}

/// Rate limiter for connection attempts
pub struct RateLimiter {
    /// Configuration
    config: Arc<RwLock<RateLimitConfig>>,
    /// Connection attempts per peer
    attempts: Arc<RwLock<HashMap<PeerId, Vec<u64>>>>,
    /// Blocked peers with unblock timestamp
    blocked_peers: Arc<RwLock<HashMap<PeerId, u64>>>,
    /// Violation counts for exponential backoff
    violation_counts: Arc<RwLock<HashMap<PeerId, u32>>>,
}

impl RateLimiter {
    /// Create a new rate limiter with default configuration
    pub fn new() -> Self {
        Self::with_config(RateLimitConfig::default())
    }
    
    /// Create a new rate limiter with custom configuration
    pub fn with_config(config: RateLimitConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            attempts: Arc::new(RwLock::new(HashMap::new())),
            blocked_peers: Arc::new(RwLock::new(HashMap::new())),
            violation_counts: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get current timestamp
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
    
    /// Check if a peer is currently blocked
    pub fn is_blocked(&self, peer_id: &PeerId) -> bool {
        let blocked = self.blocked_peers.read().unwrap();
        
        if let Some(&unblock_time) = blocked.get(peer_id) {
            let now = Self::now();
            if now < unblock_time {
                return true;
            }
        }
        
        false
    }
    
    /// Record a connection attempt and check if it should be allowed
    pub fn check_rate_limit(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        // First check if peer is blocked
        if self.is_blocked(peer_id) {
            return Err(PolicyError::RateLimitExceeded.into());
        }
        
        let now = Self::now();
        let config = self.config.read().unwrap();
        let window_start = now - config.window_secs;
        
        let mut attempts = self.attempts.write().unwrap();
        let peer_attempts = attempts.entry(peer_id.clone()).or_insert_with(Vec::new);
        
        // Remove attempts outside the time window
        peer_attempts.retain(|&timestamp| timestamp > window_start);
        
        // Add current attempt
        peer_attempts.push(now);
        
        // Check if limit exceeded
        if peer_attempts.len() > config.max_attempts as usize {
            // Rate limit exceeded - block the peer
            self.block_peer(peer_id)?;
            return Err(PolicyError::RateLimitExceeded.into());
        }
        
        Ok(true)
    }
    
    /// Block a peer with exponential backoff
    fn block_peer(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let config = self.config.read().unwrap();
        let mut violation_counts = self.violation_counts.write().unwrap();
        let mut blocked_peers = self.blocked_peers.write().unwrap();
        
        // Increment violation count
        let violations = violation_counts.entry(peer_id.clone()).or_insert(0);
        *violations += 1;
        
        // Calculate backoff duration with exponential increase
        let backoff_duration = std::cmp::min(
            config.window_secs * (config.backoff_multiplier.pow(*violations - 1) as u64),
            config.max_backoff_secs,
        );
        
        let now = Self::now();
        let unblock_time = now + backoff_duration;
        
        blocked_peers.insert(peer_id.clone(), unblock_time);
        
        Ok(())
    }
    
    /// Manually unblock a peer
    pub fn unblock_peer(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let mut blocked_peers = self.blocked_peers.write().unwrap();
        blocked_peers.remove(peer_id);
        
        // Reset violation count
        let mut violation_counts = self.violation_counts.write().unwrap();
        violation_counts.remove(peer_id);
        
        Ok(())
    }
    
    /// Get time until a peer is unblocked (in seconds)
    pub fn time_until_unblock(&self, peer_id: &PeerId) -> Option<u64> {
        let blocked = self.blocked_peers.read().unwrap();
        
        if let Some(&unblock_time) = blocked.get(peer_id) {
            let now = Self::now();
            if now < unblock_time {
                return Some(unblock_time - now);
            }
        }
        
        None
    }
    
    /// Get the number of recent attempts for a peer
    pub fn get_attempt_count(&self, peer_id: &PeerId) -> u32 {
        let config = self.config.read().unwrap();
        let now = Self::now();
        let window_start = now - config.window_secs;
        
        let attempts = self.attempts.read().unwrap();
        
        if let Some(peer_attempts) = attempts.get(peer_id) {
            peer_attempts.iter()
                .filter(|&&timestamp| timestamp > window_start)
                .count() as u32
        } else {
            0
        }
    }
    
    /// Cleanup old attempt records and expired blocks
    pub fn cleanup(&self) -> SecurityResult<()> {
        let config = self.config.read().unwrap();
        let now = Self::now();
        let window_start = now - config.window_secs;
        
        // Cleanup old attempts
        let mut attempts = self.attempts.write().unwrap();
        for peer_attempts in attempts.values_mut() {
            peer_attempts.retain(|&timestamp| timestamp > window_start);
        }
        attempts.retain(|_, v| !v.is_empty());
        
        // Cleanup expired blocks
        let mut blocked_peers = self.blocked_peers.write().unwrap();
        blocked_peers.retain(|_, &mut unblock_time| now < unblock_time);
        
        Ok(())
    }
    
    /// Reset all rate limiting data for a peer
    pub fn reset_peer(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let mut attempts = self.attempts.write().unwrap();
        attempts.remove(peer_id);
        
        let mut blocked_peers = self.blocked_peers.write().unwrap();
        blocked_peers.remove(peer_id);
        
        let mut violation_counts = self.violation_counts.write().unwrap();
        violation_counts.remove(peer_id);
        
        Ok(())
    }
    
    /// Get all currently blocked peers
    pub fn get_blocked_peers(&self) -> Vec<PeerId> {
        let blocked = self.blocked_peers.read().unwrap();
        let now = Self::now();
        
        blocked.iter()
            .filter(|&(_, &unblock_time)| now < unblock_time)
            .map(|(peer_id, _)| peer_id.clone())
            .collect()
    }
    
    /// Update rate limit configuration
    pub fn update_config(&self, config: RateLimitConfig) -> SecurityResult<()> {
        let mut current_config = self.config.write().unwrap();
        *current_config = config;
        Ok(())
    }
    
    /// Get current configuration
    pub fn get_config(&self) -> RateLimitConfig {
        let config = self.config.read().unwrap();
        config.clone()
    }
}

impl Default for RateLimiter {
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
    fn test_rate_limit_basic() {
        let limiter = RateLimiter::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // First few attempts should succeed
        for _ in 0..5 {
            assert!(limiter.check_rate_limit(&peer_id).is_ok());
        }
        
        // Next attempt should fail (exceeds limit of 5)
        assert!(limiter.check_rate_limit(&peer_id).is_err());
        assert!(limiter.is_blocked(&peer_id));
    }
    
    #[test]
    fn test_rate_limit_window() {
        let config = RateLimitConfig {
            max_attempts: 3,
            window_secs: 1, // 1 second window
            backoff_multiplier: 2,
            max_backoff_secs: 60,
        };
        
        let limiter = RateLimiter::with_config(config);
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Make 3 attempts
        for _ in 0..3 {
            assert!(limiter.check_rate_limit(&peer_id).is_ok());
        }
        
        // 4th attempt should fail
        assert!(limiter.check_rate_limit(&peer_id).is_err());
        
        // Wait for window to expire
        thread::sleep(Duration::from_secs(2));
        
        // Cleanup old attempts
        limiter.cleanup().unwrap();
        
        // Should be able to make attempts again (but peer is still blocked)
        // Need to unblock first
        limiter.unblock_peer(&peer_id).unwrap();
        assert!(limiter.check_rate_limit(&peer_id).is_ok());
    }
    
    #[test]
    fn test_manual_unblock() {
        let limiter = RateLimiter::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Exceed rate limit
        for _ in 0..6 {
            let _ = limiter.check_rate_limit(&peer_id);
        }
        
        assert!(limiter.is_blocked(&peer_id));
        
        // Manually unblock
        limiter.unblock_peer(&peer_id).unwrap();
        assert!(!limiter.is_blocked(&peer_id));
    }
    
    #[test]
    fn test_attempt_count() {
        let limiter = RateLimiter::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        assert_eq!(limiter.get_attempt_count(&peer_id), 0);
        
        limiter.check_rate_limit(&peer_id).unwrap();
        assert_eq!(limiter.get_attempt_count(&peer_id), 1);
        
        limiter.check_rate_limit(&peer_id).unwrap();
        assert_eq!(limiter.get_attempt_count(&peer_id), 2);
    }
    
    #[test]
    fn test_reset_peer() {
        let limiter = RateLimiter::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Make some attempts
        for _ in 0..3 {
            limiter.check_rate_limit(&peer_id).unwrap();
        }
        
        assert_eq!(limiter.get_attempt_count(&peer_id), 3);
        
        // Reset peer
        limiter.reset_peer(&peer_id).unwrap();
        assert_eq!(limiter.get_attempt_count(&peer_id), 0);
    }
}
