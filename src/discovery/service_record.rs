use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, SystemTime};
use std::fmt;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

// Helper functions for SystemTime serialization
fn serialize_system_time<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    use std::time::UNIX_EPOCH;
    let duration = time.duration_since(UNIX_EPOCH)
        .map_err(|_| serde::ser::Error::custom("SystemTime before UNIX_EPOCH"))?;
    serializer.serialize_u64(duration.as_secs())
}

fn deserialize_system_time<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
where
    D: Deserializer<'de>,
{
    use std::time::UNIX_EPOCH;
    let secs = u64::deserialize(deserializer)?;
    Ok(UNIX_EPOCH + Duration::from_secs(secs))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceRecord {
    pub peer_id: String,
    pub name: String,
    pub addresses: Vec<SocketAddr>,
    pub port: u16,
    pub discovery_method: String,
    pub capabilities: HashMap<String, String>,
    #[serde(serialize_with = "serialize_system_time", deserialize_with = "deserialize_system_time")]
    pub last_seen: SystemTime,
}

impl ServiceRecord {
    pub fn new(peer_id: String, name: String, port: u16) -> Self {
        Self {
            peer_id,
            name,
            addresses: Vec::new(),
            port,
            discovery_method: String::new(),
            capabilities: HashMap::new(),
            last_seen: SystemTime::now(),
        }
    }

    pub fn add_address(&mut self, addr: SocketAddr) {
        if !self.addresses.contains(&addr) {
            self.addresses.push(addr);
        }
    }

    pub fn add_capability(&mut self, key: String, value: String) {
        self.capabilities.insert(key, value);
    }

    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_seen.elapsed().unwrap_or(Duration::MAX) > timeout
    }

    pub fn update_last_seen(&mut self) {
        self.last_seen = SystemTime::now();
    }

    pub fn set_discovery_method(&mut self, method: String) {
        self.discovery_method = method;
    }

    pub fn primary_address(&self) -> Option<SocketAddr> {
        self.addresses.first().copied()
    }

    /// Get all IPv4 addresses
    pub fn ipv4_addresses(&self) -> Vec<SocketAddr> {
        self.addresses
            .iter()
            .filter(|addr| addr.is_ipv4())
            .copied()
            .collect()
    }

    /// Get all IPv6 addresses
    pub fn ipv6_addresses(&self) -> Vec<SocketAddr> {
        self.addresses
            .iter()
            .filter(|addr| addr.is_ipv6())
            .copied()
            .collect()
    }

    /// Check if the service record has a specific capability
    pub fn has_capability(&self, key: &str) -> bool {
        self.capabilities.contains_key(key)
    }

    /// Get a capability value
    pub fn get_capability(&self, key: &str) -> Option<&String> {
        self.capabilities.get(key)
    }

    /// Remove a capability
    pub fn remove_capability(&mut self, key: &str) -> Option<String> {
        self.capabilities.remove(key)
    }

    /// Clear all capabilities
    pub fn clear_capabilities(&mut self) {
        self.capabilities.clear();
    }

    /// Check if this record represents the same peer (by peer_id)
    pub fn is_same_peer(&self, other: &ServiceRecord) -> bool {
        self.peer_id == other.peer_id
    }

    /// Merge another service record into this one (useful for combining discovery results)
    pub fn merge(&mut self, other: ServiceRecord) {
        if !self.is_same_peer(&other) {
            return; // Only merge records from the same peer
        }

        // Update name if the other record has a more descriptive name
        if self.name == "Unknown" && other.name != "Unknown" {
            self.name = other.name;
        }

        // Merge addresses
        for addr in other.addresses {
            self.add_address(addr);
        }

        // Merge capabilities
        for (key, value) in other.capabilities {
            self.capabilities.insert(key, value);
        }

        // Update last seen to the more recent time
        if other.last_seen > self.last_seen {
            self.last_seen = other.last_seen;
        }

        // Update discovery method if it's more specific
        if self.discovery_method.is_empty() && !other.discovery_method.is_empty() {
            self.discovery_method = other.discovery_method;
        }
    }

    /// Serialize to a simple string format for network transmission
    pub fn to_network_string(&self) -> String {
        let addresses_str = self.addresses
            .iter()
            .map(|addr| addr.to_string())
            .collect::<Vec<_>>()
            .join(",");
        
        let capabilities_str = self.capabilities
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(",");

        format!("{}|{}|{}|{}|{}|{}", 
            self.peer_id, 
            self.name, 
            self.port, 
            addresses_str,
            self.discovery_method,
            capabilities_str
        )
    }

    /// Parse from a network string format
    pub fn from_network_string(s: &str) -> Result<Self, crate::discovery::DiscoveryError> {
        let parts: Vec<&str> = s.split('|').collect();
        if parts.len() < 6 {
            return Err(crate::discovery::DiscoveryError::Parse(
                "Invalid network string format".to_string()
            ));
        }

        let peer_id = parts[0].to_string();
        let name = parts[1].to_string();
        let port: u16 = parts[2].parse()
            .map_err(|_| crate::discovery::DiscoveryError::Parse("Invalid port".to_string()))?;
        
        let mut record = ServiceRecord::new(peer_id, name, port);
        
        // Parse addresses
        if !parts[3].is_empty() {
            for addr_str in parts[3].split(',') {
                if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                    record.add_address(addr);
                }
            }
        }

        // Set discovery method
        record.set_discovery_method(parts[4].to_string());

        // Parse capabilities
        if !parts[5].is_empty() {
            for cap_str in parts[5].split(',') {
                if let Some((key, value)) = cap_str.split_once('=') {
                    record.add_capability(key.to_string(), value.to_string());
                }
            }
        }

        Ok(record)
    }

    /// Get a human-readable description of the service record
    pub fn description(&self) -> String {
        let addr_count = self.addresses.len();
        let cap_count = self.capabilities.len();
        
        format!("{} ({}) - {} address(es), {} capability(ies) via {}", 
            self.name, 
            self.peer_id, 
            addr_count, 
            cap_count,
            if self.discovery_method.is_empty() { "unknown" } else { &self.discovery_method }
        )
    }
}

// Convert from the existing Peer struct for backward compatibility
impl From<crate::discovery::Peer> for ServiceRecord {
    fn from(peer: crate::discovery::Peer) -> Self {
        let mut record = ServiceRecord::new(peer.id, "Unknown".to_string(), peer.port);
        
        // Try to parse the addr as a SocketAddr, otherwise create one with the port
        if let Ok(socket_addr) = peer.addr.parse::<SocketAddr>() {
            record.add_address(socket_addr);
        } else if let Ok(ip) = peer.addr.parse::<std::net::IpAddr>() {
            record.add_address(SocketAddr::new(ip, peer.port));
        }
        
        record
    }
}

// Convert to the existing Peer struct for backward compatibility
impl From<ServiceRecord> for crate::discovery::Peer {
    fn from(record: ServiceRecord) -> Self {
        let addr = record.primary_address()
            .map(|a| a.ip().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        crate::discovery::Peer {
            id: record.peer_id,
            addr,
            port: record.port,
        }
    }
}

impl fmt::Display for ServiceRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_service_record_creation() {
        let record = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        
        assert_eq!(record.peer_id, "peer-123");
        assert_eq!(record.name, "Test Device");
        assert_eq!(record.port, 8080);
        assert!(record.addresses.is_empty());
        assert!(record.capabilities.is_empty());
        assert!(record.discovery_method.is_empty());
    }

    #[test]
    fn test_add_address() {
        let mut record = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        let addr2 = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)), 8080);
        
        record.add_address(addr1);
        record.add_address(addr2);
        record.add_address(addr1); // Duplicate should be ignored
        
        assert_eq!(record.addresses.len(), 2);
        assert_eq!(record.primary_address(), Some(addr1));
    }

    #[test]
    fn test_ipv4_ipv6_addresses() {
        let mut record = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        let ipv4_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        let ipv6_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)), 8080);
        
        record.add_address(ipv4_addr);
        record.add_address(ipv6_addr);
        
        let ipv4_addrs = record.ipv4_addresses();
        let ipv6_addrs = record.ipv6_addresses();
        
        assert_eq!(ipv4_addrs.len(), 1);
        assert_eq!(ipv6_addrs.len(), 1);
        assert_eq!(ipv4_addrs[0], ipv4_addr);
        assert_eq!(ipv6_addrs[0], ipv6_addr);
    }

    #[test]
    fn test_capabilities() {
        let mut record = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        
        record.add_capability("version".to_string(), "1.0.0".to_string());
        record.add_capability("features".to_string(), "file-transfer,chat".to_string());
        
        assert!(record.has_capability("version"));
        assert!(record.has_capability("features"));
        assert!(!record.has_capability("nonexistent"));
        
        assert_eq!(record.get_capability("version"), Some(&"1.0.0".to_string()));
        assert_eq!(record.get_capability("nonexistent"), None);
        
        let removed = record.remove_capability("version");
        assert_eq!(removed, Some("1.0.0".to_string()));
        assert!(!record.has_capability("version"));
        
        record.clear_capabilities();
        assert!(!record.has_capability("features"));
    }

    #[test]
    fn test_expiration() {
        let mut record = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        
        // Should not be expired immediately
        assert!(!record.is_expired(Duration::from_secs(60)));
        
        // Simulate old timestamp
        record.last_seen = SystemTime::now() - Duration::from_secs(120);
        assert!(record.is_expired(Duration::from_secs(60)));
        assert!(!record.is_expired(Duration::from_secs(180)));
        
        // Update last seen
        record.update_last_seen();
        assert!(!record.is_expired(Duration::from_secs(60)));
    }

    #[test]
    fn test_merge_records() {
        let mut record1 = ServiceRecord::new("peer-123".to_string(), "Unknown".to_string(), 8080);
        record1.add_address(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080));
        record1.add_capability("version".to_string(), "1.0.0".to_string());
        record1.set_discovery_method("udp".to_string());
        
        let mut record2 = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        record2.add_address(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101)), 8080));
        record2.add_capability("features".to_string(), "file-transfer".to_string());
        record2.last_seen = SystemTime::now() + Duration::from_secs(10); // Future time
        
        record1.merge(record2);
        
        // Should have updated name from "Unknown" to "Test Device"
        assert_eq!(record1.name, "Test Device");
        
        // Should have both addresses
        assert_eq!(record1.addresses.len(), 2);
        
        // Should have both capabilities
        assert!(record1.has_capability("version"));
        assert!(record1.has_capability("features"));
        
        // Should keep the discovery method
        assert_eq!(record1.discovery_method, "udp");
    }

    #[test]
    fn test_merge_different_peers() {
        let mut record1 = ServiceRecord::new("peer-123".to_string(), "Device 1".to_string(), 8080);
        let record2 = ServiceRecord::new("peer-456".to_string(), "Device 2".to_string(), 8080);
        
        let original_name = record1.name.clone();
        record1.merge(record2);
        
        // Should not merge records from different peers
        assert_eq!(record1.name, original_name);
    }

    #[test]
    fn test_network_string_serialization() {
        let mut record = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        record.add_address(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080));
        record.add_capability("version".to_string(), "1.0.0".to_string());
        record.set_discovery_method("udp".to_string());
        
        let network_string = record.to_network_string();
        let parsed_record = ServiceRecord::from_network_string(&network_string).unwrap();
        
        assert_eq!(parsed_record.peer_id, record.peer_id);
        assert_eq!(parsed_record.name, record.name);
        assert_eq!(parsed_record.port, record.port);
        assert_eq!(parsed_record.addresses.len(), 1);
        assert_eq!(parsed_record.discovery_method, record.discovery_method);
        assert!(parsed_record.has_capability("version"));
        assert_eq!(parsed_record.get_capability("version"), Some(&"1.0.0".to_string()));
    }

    #[test]
    fn test_network_string_parsing_errors() {
        // Test invalid format
        let result = ServiceRecord::from_network_string("invalid");
        assert!(result.is_err());
        
        // Test invalid port
        let result = ServiceRecord::from_network_string("peer|name|invalid_port|||");
        assert!(result.is_err());
    }

    #[test]
    fn test_peer_conversion() {
        let peer = crate::discovery::Peer {
            id: "peer-123".to_string(),
            addr: "192.168.1.100".to_string(),
            port: 8080,
        };
        
        let record: ServiceRecord = peer.clone().into();
        assert_eq!(record.peer_id, peer.id);
        assert_eq!(record.port, peer.port);
        assert_eq!(record.addresses.len(), 1);
        
        let converted_peer: crate::discovery::Peer = record.into();
        assert_eq!(converted_peer.id, peer.id);
        assert_eq!(converted_peer.port, peer.port);
    }

    #[test]
    fn test_description_and_display() {
        let mut record = ServiceRecord::new("peer-123".to_string(), "Test Device".to_string(), 8080);
        record.add_address(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080));
        record.add_capability("version".to_string(), "1.0.0".to_string());
        record.set_discovery_method("udp".to_string());
        
        let description = record.description();
        assert!(description.contains("Test Device"));
        assert!(description.contains("peer-123"));
        assert!(description.contains("1 address"));
        assert!(description.contains("1 capability"));
        assert!(description.contains("udp"));
        
        let display_string = format!("{}", record);
        assert_eq!(display_string, description);
    }
}