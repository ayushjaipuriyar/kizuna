use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use crate::security::error::{SecurityResult, PolicyError};
use crate::security::identity::PeerId;

/// Pattern of suspicious activity
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SuspiciousPattern {
    /// Rapid connection attempts
    RapidConnections,
    /// Multiple failed pairing attempts
    FailedPairings,
    /// Connection attempts from blocked peer
    BlockedPeerAttempt,
    /// Unusual connection timing pattern
    UnusualTiming,
    /// Multiple connections from same peer
    MultipleConnections,
}

/// Activity record for a peer
#[derive(Clone, Debug)]
struct ActivityRecord {
    connection_attempts: Vec<u64>,
    failed_pairings: u32,
    last_blocked_attempt: Option<u64>,
    active_connections: u32,
}

impl ActivityRecord {
    fn new() -> Self {
        Self {
            connection_attempts: Vec::new(),
            failed_pairings: 0,
            last_blocked_attempt: None,
            active_connections: 0,
        }
    }
}

/// Configuration for attack detection
#[derive(Clone, Debug)]
pub struct AttackDetectorConfig {
    /// Threshold for rapid connections (attempts per minute)
    pub rapid_connection_threshold: u32,
    /// Threshold for failed pairings
    pub failed_pairing_threshold: u32,
    /// Time window for pattern detection (seconds)
    pub detection_window_secs: u64,
    /// Maximum simultaneous connections per peer
    pub max_simultaneous_connections: u32,
}

impl Default for AttackDetectorConfig {
    fn default() -> Self {
        Self {
            rapid_connection_threshold: 10,
            failed_pairing_threshold: 3,
            detection_window_secs: 60,
            max_simultaneous_connections: 3,
        }
    }
}

/// Attack detector for identifying suspicious patterns
pub struct AttackDetector {
    /// Configuration
    config: Arc<RwLock<AttackDetectorConfig>>,
    /// Activity records per peer
    activity: Arc<RwLock<HashMap<PeerId, ActivityRecord>>>,
    /// Blocked peers
    blocked_peers: Arc<RwLock<HashMap<PeerId, u64>>>,
}

impl AttackDetector {
    /// Create a new attack detector with default configuration
    pub fn new() -> Self {
        Self::with_config(AttackDetectorConfig::default())
    }
    
    /// Create a new attack detector with custom configuration
    pub fn with_config(config: AttackDetectorConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            activity: Arc::new(RwLock::new(HashMap::new())),
            blocked_peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get current timestamp
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
    
    /// Record a connection attempt
    pub fn record_connection_attempt(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let now = Self::now();
        let mut activity = self.activity.write().unwrap();
        
        let record = activity.entry(peer_id.clone()).or_insert_with(ActivityRecord::new);
        record.connection_attempts.push(now);
        
        Ok(())
    }
    
    /// Record a failed pairing attempt
    pub fn record_failed_pairing(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let mut activity = self.activity.write().unwrap();
        
        let record = activity.entry(peer_id.clone()).or_insert_with(ActivityRecord::new);
        record.failed_pairings += 1;
        
        Ok(())
    }
    
    /// Record a successful connection
    pub fn record_connection_established(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let mut activity = self.activity.write().unwrap();
        
        let record = activity.entry(peer_id.clone()).or_insert_with(ActivityRecord::new);
        record.active_connections += 1;
        
        Ok(())
    }
    
    /// Record a connection closed
    pub fn record_connection_closed(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let mut activity = self.activity.write().unwrap();
        
        if let Some(record) = activity.get_mut(peer_id) {
            if record.active_connections > 0 {
                record.active_connections -= 1;
            }
        }
        
        Ok(())
    }
    
    /// Detect suspicious patterns for a peer
    pub fn detect_suspicious_patterns(&self, peer_id: &PeerId) -> SecurityResult<Vec<SuspiciousPattern>> {
        let config = self.config.read().unwrap();
        let activity = self.activity.read().unwrap();
        let blocked_peers = self.blocked_peers.read().unwrap();
        
        let mut patterns = Vec::new();
        
        // Check if peer is blocked and still attempting
        if blocked_peers.contains_key(peer_id) {
            patterns.push(SuspiciousPattern::BlockedPeerAttempt);
        }
        
        if let Some(record) = activity.get(peer_id) {
            let now = Self::now();
            let window_start = now - config.detection_window_secs;
            
            // Check for rapid connections
            let recent_attempts = record.connection_attempts.iter()
                .filter(|&&timestamp| timestamp > window_start)
                .count() as u32;
            
            if recent_attempts > config.rapid_connection_threshold {
                patterns.push(SuspiciousPattern::RapidConnections);
            }
            
            // Check for failed pairings
            if record.failed_pairings >= config.failed_pairing_threshold {
                patterns.push(SuspiciousPattern::FailedPairings);
            }
            
            // Check for multiple simultaneous connections
            if record.active_connections > config.max_simultaneous_connections {
                patterns.push(SuspiciousPattern::MultipleConnections);
            }
            
            // Check for unusual timing patterns (connections at very regular intervals)
            if record.connection_attempts.len() >= 5 {
                let recent: Vec<u64> = record.connection_attempts.iter()
                    .filter(|&&timestamp| timestamp > window_start)
                    .copied()
                    .collect();
                
                if Self::has_regular_interval_pattern(&recent) {
                    patterns.push(SuspiciousPattern::UnusualTiming);
                }
            }
        }
        
        Ok(patterns)
    }
    
    /// Check if connection attempts follow a suspiciously regular pattern
    fn has_regular_interval_pattern(timestamps: &[u64]) -> bool {
        if timestamps.len() < 5 {
            return false;
        }
        
        // Calculate intervals between consecutive attempts
        let mut intervals = Vec::new();
        for i in 1..timestamps.len() {
            intervals.push(timestamps[i] - timestamps[i - 1]);
        }
        
        // Check if intervals are suspiciously similar (within 2 seconds)
        if intervals.len() < 4 {
            return false;
        }
        
        let avg_interval = intervals.iter().sum::<u64>() / intervals.len() as u64;
        let similar_count = intervals.iter()
            .filter(|&&interval| {
                let diff = if interval > avg_interval {
                    interval - avg_interval
                } else {
                    avg_interval - interval
                };
                diff <= 2
            })
            .count();
        
        // If more than 75% of intervals are similar, it's suspicious
        similar_count as f64 / intervals.len() as f64 > 0.75
    }
    
    /// Check if activity should be blocked
    pub fn should_block(&self, peer_id: &PeerId) -> SecurityResult<bool> {
        let patterns = self.detect_suspicious_patterns(peer_id)?;
        
        if patterns.is_empty() {
            return Ok(false);
        }
        
        // Block if any critical patterns detected
        for pattern in &patterns {
            match pattern {
                SuspiciousPattern::RapidConnections |
                SuspiciousPattern::FailedPairings |
                SuspiciousPattern::BlockedPeerAttempt => {
                    return Ok(true);
                }
                _ => {}
            }
        }
        
        Ok(false)
    }
    
    /// Block a peer
    pub fn block_peer(&self, peer_id: &PeerId, duration_secs: u64) -> SecurityResult<()> {
        let now = Self::now();
        let unblock_time = now + duration_secs;
        
        let mut blocked_peers = self.blocked_peers.write().unwrap();
        blocked_peers.insert(peer_id.clone(), unblock_time);
        
        // Record the blocked attempt
        let mut activity = self.activity.write().unwrap();
        if let Some(record) = activity.get_mut(peer_id) {
            record.last_blocked_attempt = Some(now);
        }
        
        Ok(())
    }
    
    /// Unblock a peer
    pub fn unblock_peer(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let mut blocked_peers = self.blocked_peers.write().unwrap();
        blocked_peers.remove(peer_id);
        Ok(())
    }
    
    /// Check if a peer is blocked
    pub fn is_blocked(&self, peer_id: &PeerId) -> bool {
        let blocked_peers = self.blocked_peers.read().unwrap();
        
        if let Some(&unblock_time) = blocked_peers.get(peer_id) {
            let now = Self::now();
            return now < unblock_time;
        }
        
        false
    }
    
    /// Reset activity for a peer
    pub fn reset_peer_activity(&self, peer_id: &PeerId) -> SecurityResult<()> {
        let mut activity = self.activity.write().unwrap();
        activity.remove(peer_id);
        
        let mut blocked_peers = self.blocked_peers.write().unwrap();
        blocked_peers.remove(peer_id);
        
        Ok(())
    }
    
    /// Cleanup old activity records
    pub fn cleanup(&self) -> SecurityResult<()> {
        let config = self.config.read().unwrap();
        let now = Self::now();
        let window_start = now - config.detection_window_secs;
        
        // Cleanup old connection attempts
        let mut activity = self.activity.write().unwrap();
        for record in activity.values_mut() {
            record.connection_attempts.retain(|&timestamp| timestamp > window_start);
        }
        activity.retain(|_, record| {
            !record.connection_attempts.is_empty() || 
            record.failed_pairings > 0 || 
            record.active_connections > 0
        });
        
        // Cleanup expired blocks
        let mut blocked_peers = self.blocked_peers.write().unwrap();
        blocked_peers.retain(|_, &mut unblock_time| now < unblock_time);
        
        Ok(())
    }
    
    /// Get activity summary for a peer
    pub fn get_activity_summary(&self, peer_id: &PeerId) -> Option<String> {
        let activity = self.activity.read().unwrap();
        
        if let Some(record) = activity.get(peer_id) {
            let config = self.config.read().unwrap();
            let now = Self::now();
            let window_start = now - config.detection_window_secs;
            
            let recent_attempts = record.connection_attempts.iter()
                .filter(|&&timestamp| timestamp > window_start)
                .count();
            
            Some(format!(
                "Recent attempts: {}, Failed pairings: {}, Active connections: {}",
                recent_attempts, record.failed_pairings, record.active_connections
            ))
        } else {
            None
        }
    }
    
    /// Update configuration
    pub fn update_config(&self, config: AttackDetectorConfig) -> SecurityResult<()> {
        let mut current_config = self.config.write().unwrap();
        *current_config = config;
        Ok(())
    }
}

impl Default for AttackDetector {
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
    fn test_rapid_connections_detection() {
        let detector = AttackDetector::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Record many rapid connection attempts
        for _ in 0..15 {
            detector.record_connection_attempt(&peer_id).unwrap();
        }
        
        let patterns = detector.detect_suspicious_patterns(&peer_id).unwrap();
        assert!(patterns.contains(&SuspiciousPattern::RapidConnections));
    }
    
    #[test]
    fn test_failed_pairings_detection() {
        let detector = AttackDetector::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Record multiple failed pairings
        for _ in 0..5 {
            detector.record_failed_pairing(&peer_id).unwrap();
        }
        
        let patterns = detector.detect_suspicious_patterns(&peer_id).unwrap();
        assert!(patterns.contains(&SuspiciousPattern::FailedPairings));
    }
    
    #[test]
    fn test_blocked_peer_detection() {
        let detector = AttackDetector::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Block the peer
        detector.block_peer(&peer_id, 3600).unwrap();
        
        // Record attempt from blocked peer
        detector.record_connection_attempt(&peer_id).unwrap();
        
        let patterns = detector.detect_suspicious_patterns(&peer_id).unwrap();
        assert!(patterns.contains(&SuspiciousPattern::BlockedPeerAttempt));
    }
    
    #[test]
    fn test_multiple_connections_detection() {
        let detector = AttackDetector::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Establish multiple connections
        for _ in 0..5 {
            detector.record_connection_established(&peer_id).unwrap();
        }
        
        let patterns = detector.detect_suspicious_patterns(&peer_id).unwrap();
        assert!(patterns.contains(&SuspiciousPattern::MultipleConnections));
    }
    
    #[test]
    fn test_should_block() {
        let detector = AttackDetector::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        // Initially should not block
        assert!(!detector.should_block(&peer_id).unwrap());
        
        // Record many rapid attempts
        for _ in 0..15 {
            detector.record_connection_attempt(&peer_id).unwrap();
        }
        
        // Should now block
        assert!(detector.should_block(&peer_id).unwrap());
    }
    
    #[test]
    fn test_block_unblock() {
        let detector = AttackDetector::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        assert!(!detector.is_blocked(&peer_id));
        
        detector.block_peer(&peer_id, 3600).unwrap();
        assert!(detector.is_blocked(&peer_id));
        
        detector.unblock_peer(&peer_id).unwrap();
        assert!(!detector.is_blocked(&peer_id));
    }
    
    #[test]
    fn test_activity_summary() {
        let detector = AttackDetector::new();
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        detector.record_connection_attempt(&peer_id).unwrap();
        detector.record_failed_pairing(&peer_id).unwrap();
        detector.record_connection_established(&peer_id).unwrap();
        
        let summary = detector.get_activity_summary(&peer_id);
        assert!(summary.is_some());
        assert!(summary.unwrap().contains("Recent attempts: 1"));
    }
    
    #[test]
    fn test_cleanup() {
        let config = AttackDetectorConfig {
            rapid_connection_threshold: 10,
            failed_pairing_threshold: 3,
            detection_window_secs: 1, // 1 second window
            max_simultaneous_connections: 3,
        };
        
        let detector = AttackDetector::with_config(config);
        let peer_id = PeerId::from_string("test_peer").unwrap();
        
        detector.record_connection_attempt(&peer_id).unwrap();
        
        // Wait for window to expire
        thread::sleep(Duration::from_secs(2));
        
        detector.cleanup().unwrap();
        
        // Activity should be cleaned up
        let summary = detector.get_activity_summary(&peer_id);
        assert!(summary.is_none() || summary.unwrap().contains("Recent attempts: 0"));
    }
}
