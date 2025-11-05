use crate::discovery::{Discovery, DiscoveryError, ServiceRecord};
use async_trait::async_trait;
use btleplug::api::{Central, Manager as _, Peripheral, ScanFilter};
use btleplug::platform::{Adapter, Manager, PeripheralId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BluetoothAdvertisementData {
    peer_id: String,
    name: String,
    port: u16,
    capabilities: HashMap<String, String>,
}

pub struct BluetoothDiscovery {
    peer_id: String,
    device_name: String,
    port: u16,
    service_uuid: Uuid,
    capabilities: HashMap<String, String>,
    manager: Arc<RwLock<Option<Manager>>>,
    adapter: Arc<RwLock<Option<Adapter>>>,
    is_advertising: Arc<RwLock<bool>>,
}

impl BluetoothDiscovery {
    pub fn new() -> Self {
        Self {
            peer_id: format!("kizuna-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            device_name: "Kizuna Device".to_string(),
            port: 41337,
            service_uuid: Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap(),
            capabilities: HashMap::new(),
            manager: Arc::new(RwLock::new(None)),
            adapter: Arc::new(RwLock::new(None)),
            is_advertising: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_config(peer_id: String, device_name: String, port: u16) -> Self {
        let mut capabilities = HashMap::new();
        capabilities.insert("version".to_string(), "0.1.0".to_string());
        
        Self {
            peer_id,
            device_name,
            port,
            service_uuid: Uuid::parse_str("6ba7b810-9dad-11d1-80b4-00c04fd430c8").unwrap(),
            capabilities,
            manager: Arc::new(RwLock::new(None)),
            adapter: Arc::new(RwLock::new(None)),
            is_advertising: Arc::new(RwLock::new(false)),
        }
    }

    async fn initialize_bluetooth(&self) -> Result<(), DiscoveryError> {
        let mut manager_lock = self.manager.write().await;
        let mut adapter_lock = self.adapter.write().await;

        if manager_lock.is_none() {
            let manager = Manager::new().await.map_err(|e| DiscoveryError::Bluetooth(format!("Failed to create BLE manager: {}", e)))?;
            let adapters = manager.adapters().await.map_err(|e| DiscoveryError::Bluetooth(format!("Failed to get BLE adapters: {}", e)))?;
            
            if adapters.is_empty() {
                return Err(DiscoveryError::Bluetooth("No Bluetooth adapters found".to_string()));
            }

            let adapter = adapters.into_iter().next().unwrap();
            *manager_lock = Some(manager);
            *adapter_lock = Some(adapter);
        }

        Ok(())
    }

    fn create_advertisement_data(&self) -> Vec<u8> {
        let data = BluetoothAdvertisementData {
            peer_id: self.peer_id.clone(),
            name: self.device_name.clone(),
            port: self.port,
            capabilities: self.capabilities.clone(),
        };

        // Serialize to JSON and convert to bytes
        serde_json::to_string(&data)
            .unwrap_or_default()
            .into_bytes()
    }

    fn parse_advertisement_data(&self, data: &[u8]) -> Option<ServiceRecord> {
        // Try to parse as JSON first
        if let Ok(json_str) = String::from_utf8(data.to_vec()) {
            if let Ok(adv_data) = serde_json::from_str::<BluetoothAdvertisementData>(&json_str) {
                let mut service_record = ServiceRecord::new(
                    adv_data.peer_id,
                    adv_data.name,
                    adv_data.port,
                );

                service_record.discovery_method = "bluetooth".to_string();
                
                for (key, value) in adv_data.capabilities {
                    service_record.add_capability(key, value);
                }

                return Some(service_record);
            }
        }

        // Fallback: try to parse as simple format (peer_id|name|port)
        if let Ok(data_str) = String::from_utf8(data.to_vec()) {
            let parts: Vec<&str> = data_str.split('|').collect();
            if parts.len() >= 3 {
                if let Ok(port) = parts[2].parse::<u16>() {
                    let mut service_record = ServiceRecord::new(
                        parts[0].to_string(),
                        parts[1].to_string(),
                        port,
                    );
                    service_record.discovery_method = "bluetooth".to_string();
                    return Some(service_record);
                }
            }
        }

        None
    }

    async fn establish_ble_connection(&self, peripheral_id: &PeripheralId) -> Result<ServiceRecord, DiscoveryError> {
        let adapter_lock = self.adapter.read().await;
        let adapter = adapter_lock.as_ref().ok_or_else(|| {
            DiscoveryError::Bluetooth("Bluetooth adapter not initialized".to_string())
        })?;

        let peripherals = adapter.peripherals().await.map_err(|e| {
            DiscoveryError::Bluetooth(format!("Failed to get peripherals: {}", e))
        })?;

        let peripheral = peripherals.into_iter()
            .find(|p| p.id() == *peripheral_id)
            .ok_or_else(|| DiscoveryError::Bluetooth("Peripheral not found".to_string()))?;

        // Connect to the peripheral
        peripheral.connect().await.map_err(|e| {
            DiscoveryError::Bluetooth(format!("Failed to connect to peripheral: {}", e))
        })?;

        // Discover services
        peripheral.discover_services().await.map_err(|e| {
            DiscoveryError::Bluetooth(format!("Failed to discover services: {}", e))
        })?;

        // Look for our service and characteristics
        let services = peripheral.services();
        for service in services {
            if service.uuid == self.service_uuid {
                for characteristic in &service.characteristics {
                    // Try to read peer information from characteristic
                    if let Ok(data) = peripheral.read(characteristic).await {
                        if let Some(service_record) = self.parse_advertisement_data(&data) {
                            // Disconnect after getting the data
                            let _ = peripheral.disconnect().await;
                            return Ok(service_record);
                        }
                    }
                }
            }
        }

        // Disconnect if we couldn't get the data
        let _ = peripheral.disconnect().await;
        Err(DiscoveryError::Bluetooth("No valid peer data found".to_string()))
    }

    fn handle_bluetooth_unavailable(&self) -> DiscoveryError {
        DiscoveryError::StrategyUnavailable {
            strategy: "bluetooth".to_string(),
        }
    }
}

#[async_trait]
impl Discovery for BluetoothDiscovery {
    async fn discover(&self, timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        if !self.is_available() {
            return Err(self.handle_bluetooth_unavailable());
        }

        // Try to initialize Bluetooth, gracefully handle failures
        if let Err(e) = self.initialize_bluetooth().await {
            eprintln!("Bluetooth initialization failed: {}", e);
            return Err(self.handle_bluetooth_unavailable());
        }
        
        let adapter_lock = self.adapter.read().await;
        let adapter = adapter_lock.as_ref().ok_or_else(|| {
            self.handle_bluetooth_unavailable()
        })?;

        // Start scanning for BLE devices with our service UUID filter
        let mut scan_filter = ScanFilter::default();
        scan_filter.services = vec![self.service_uuid];
        
        if let Err(e) = adapter.start_scan(scan_filter).await {
            eprintln!("Failed to start BLE scan: {}", e);
            return Err(self.handle_bluetooth_unavailable());
        }

        // Wait for the specified timeout (minimum 5 seconds for BLE discovery)
        let scan_duration = timeout.max(Duration::from_secs(5));
        tokio::time::sleep(scan_duration).await;

        // Stop scanning
        if let Err(e) = adapter.stop_scan().await {
            eprintln!("Failed to stop BLE scan: {}", e);
        }

        // Get discovered peripherals
        let peripherals = adapter.peripherals().await.map_err(|e| {
            DiscoveryError::Bluetooth(format!("Failed to get peripherals: {}", e))
        })?;

        let mut discovered_peers = Vec::new();

        for peripheral in peripherals {
            // Check if this peripheral advertises our service UUID
            if let Ok(properties) = peripheral.properties().await {
                if let Some(props) = properties {
                    let mut found_service = false;
                    
                    // Check service UUIDs
                    if props.services.contains(&self.service_uuid) {
                        found_service = true;
                    }

                    if found_service {
                        // Try to get advertisement data from manufacturer data
                        for (_, data) in &props.manufacturer_data {
                            if let Some(service_record) = self.parse_advertisement_data(data) {
                                discovered_peers.push(service_record);
                                break;
                            }
                        }

                        // If no manufacturer data, try to establish connection for data exchange
                        if discovered_peers.is_empty() {
                            if let Ok(service_record) = self.establish_ble_connection(&peripheral.id()).await {
                                discovered_peers.push(service_record);
                            }
                        }
                    }
                }
            }
        }

        Ok(discovered_peers)
    }

    async fn announce(&self) -> Result<(), DiscoveryError> {
        if !self.is_available() {
            return Err(self.handle_bluetooth_unavailable());
        }

        // Try to initialize Bluetooth, gracefully handle failures
        if let Err(e) = self.initialize_bluetooth().await {
            eprintln!("Bluetooth initialization failed for announcement: {}", e);
            return Err(self.handle_bluetooth_unavailable());
        }

        let mut is_advertising = self.is_advertising.write().await;
        if *is_advertising {
            return Ok(()); // Already advertising
        }

        // Create advertisement data
        let _advertisement_data = self.create_advertisement_data();
        
        // Note: btleplug doesn't support advertising on most platforms
        // This is a limitation of the library and underlying platform APIs
        // For production use, you would need platform-specific implementations:
        
        #[cfg(target_os = "linux")]
        {
            // On Linux, you could use BlueZ D-Bus APIs directly
            // or use a different crate like bluer
            eprintln!("BLE advertising not fully supported on Linux via btleplug");
        }
        
        #[cfg(target_os = "macos")]
        {
            // On macOS, you could use Core Bluetooth framework
            eprintln!("BLE advertising not fully supported on macOS via btleplug");
        }
        
        #[cfg(target_os = "windows")]
        {
            // On Windows, you could use Windows Runtime APIs
            eprintln!("BLE advertising not fully supported on Windows via btleplug");
        }
        
        // For now, we simulate advertising by marking the state
        // In a real implementation, you would:
        // 1. Create BLE advertisement with service UUID
        // 2. Set manufacturer data with peer information
        // 3. Start advertising with appropriate intervals
        // 4. Handle platform-specific permission requirements
        
        *is_advertising = true;
        println!("Bluetooth LE advertising started (simulated) for peer: {}", self.peer_id);
        
        Ok(())
    }

    async fn stop_announce(&self) -> Result<(), DiscoveryError> {
        let mut is_advertising = self.is_advertising.write().await;
        if !*is_advertising {
            return Ok(()); // Not advertising
        }

        // Stop advertising (platform-specific implementation needed)
        *is_advertising = false;
        
        Ok(())
    }

    fn strategy_name(&self) -> &'static str {
        "bluetooth"
    }

    fn is_available(&self) -> bool {
        // Check if Bluetooth is available on this platform
        // This is a simplified check - in production you'd want to:
        // 1. Check if Bluetooth hardware is present
        // 2. Check if Bluetooth is enabled
        // 3. Check if app has necessary permissions
        
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        {
            // Basic availability check - assume available on desktop platforms
            true
        }
        
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // Conservative approach for other platforms
            false
        }
    }

    fn priority(&self) -> u8 {
        // High priority for mobile/proximity scenarios
        70
    }
}

impl Default for BluetoothDiscovery {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_bluetooth_discovery_creation() {
        let discovery = BluetoothDiscovery::new();
        assert_eq!(discovery.strategy_name(), "bluetooth");
        assert_eq!(discovery.priority(), 70);
        assert!(discovery.peer_id.starts_with("kizuna-"));
        assert_eq!(discovery.device_name, "Kizuna Device");
        assert_eq!(discovery.port, 41337);
    }

    #[test]
    fn test_bluetooth_discovery_with_config() {
        let peer_id = "test-peer-123".to_string();
        let device_name = "Test Device".to_string();
        let port = 8080;

        let discovery = BluetoothDiscovery::with_config(peer_id.clone(), device_name.clone(), port);
        
        assert_eq!(discovery.peer_id, peer_id);
        assert_eq!(discovery.device_name, device_name);
        assert_eq!(discovery.port, port);
        assert_eq!(discovery.strategy_name(), "bluetooth");
    }

    #[test]
    fn test_create_advertisement_data() {
        let discovery = BluetoothDiscovery::with_config(
            "test-peer".to_string(),
            "Test Device".to_string(),
            8080,
        );

        let data = discovery.create_advertisement_data();
        let json_str = String::from_utf8(data).unwrap();
        
        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["peer_id"], "test-peer");
        assert_eq!(parsed["name"], "Test Device");
        assert_eq!(parsed["port"], 8080);
    }

    #[test]
    fn test_parse_advertisement_data_json() {
        let discovery = BluetoothDiscovery::new();
        
        let mut capabilities = HashMap::new();
        capabilities.insert("version".to_string(), "0.1.0".to_string());
        
        let adv_data = BluetoothAdvertisementData {
            peer_id: "test-peer".to_string(),
            name: "Test Device".to_string(),
            port: 8080,
            capabilities,
        };

        let json_data = serde_json::to_string(&adv_data).unwrap();
        let data_bytes = json_data.as_bytes();

        let service_record = discovery.parse_advertisement_data(data_bytes).unwrap();
        
        assert_eq!(service_record.peer_id, "test-peer");
        assert_eq!(service_record.name, "Test Device");
        assert_eq!(service_record.port, 8080);
        assert_eq!(service_record.discovery_method, "bluetooth");
        assert_eq!(service_record.capabilities.get("version"), Some(&"0.1.0".to_string()));
    }

    #[test]
    fn test_parse_advertisement_data_simple_format() {
        let discovery = BluetoothDiscovery::new();
        
        let simple_data = "test-peer|Test Device|8080";
        let data_bytes = simple_data.as_bytes();

        let service_record = discovery.parse_advertisement_data(data_bytes).unwrap();
        
        assert_eq!(service_record.peer_id, "test-peer");
        assert_eq!(service_record.name, "Test Device");
        assert_eq!(service_record.port, 8080);
        assert_eq!(service_record.discovery_method, "bluetooth");
    }

    #[test]
    fn test_parse_advertisement_data_invalid() {
        let discovery = BluetoothDiscovery::new();
        
        // Invalid JSON
        let invalid_json = b"invalid json data";
        assert!(discovery.parse_advertisement_data(invalid_json).is_none());
        
        // Invalid simple format
        let invalid_simple = b"incomplete|data";
        assert!(discovery.parse_advertisement_data(invalid_simple).is_none());
        
        // Non-UTF8 data
        let invalid_utf8 = &[0xFF, 0xFE, 0xFD];
        assert!(discovery.parse_advertisement_data(invalid_utf8).is_none());
    }

    #[test]
    fn test_is_available() {
        let discovery = BluetoothDiscovery::new();
        
        // Should return true on supported platforms
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        assert!(discovery.is_available());
        
        // Should return false on unsupported platforms
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        assert!(!discovery.is_available());
    }

    #[tokio::test]
    async fn test_announce_unavailable_bluetooth() {
        // Create a discovery instance that reports as unavailable
        let _discovery = BluetoothDiscovery::new();
        
        // Mock unavailable by testing on unsupported platform behavior
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            let result = _discovery.announce().await;
            assert!(result.is_err());
            
            if let Err(DiscoveryError::StrategyUnavailable { strategy }) = result {
                assert_eq!(strategy, "bluetooth");
            } else {
                panic!("Expected StrategyUnavailable error");
            }
        }
    }

    #[tokio::test]
    async fn test_discover_unavailable_bluetooth() {
        let _discovery = BluetoothDiscovery::new();
        
        // Test behavior when Bluetooth is unavailable
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            let result = _discovery.discover(Duration::from_secs(1)).await;
            assert!(result.is_err());
            
            if let Err(DiscoveryError::StrategyUnavailable { strategy }) = result {
                assert_eq!(strategy, "bluetooth");
            } else {
                panic!("Expected StrategyUnavailable error");
            }
        }
    }

    #[tokio::test]
    async fn test_stop_announce() {
        let discovery = BluetoothDiscovery::new();
        
        // Should succeed even if not advertising
        let result = discovery.stop_announce().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_service_record_creation_from_advertisement() {
        let discovery = BluetoothDiscovery::new();
        
        let mut capabilities = HashMap::new();
        capabilities.insert("version".to_string(), "0.1.0".to_string());
        capabilities.insert("features".to_string(), "file_transfer,chat".to_string());
        
        let adv_data = BluetoothAdvertisementData {
            peer_id: "bluetooth-peer-456".to_string(),
            name: "Mobile Device".to_string(),
            port: 9090,
            capabilities,
        };

        let json_data = serde_json::to_string(&adv_data).unwrap();
        let service_record = discovery.parse_advertisement_data(json_data.as_bytes()).unwrap();
        
        assert_eq!(service_record.peer_id, "bluetooth-peer-456");
        assert_eq!(service_record.name, "Mobile Device");
        assert_eq!(service_record.port, 9090);
        assert_eq!(service_record.discovery_method, "bluetooth");
        assert_eq!(service_record.capabilities.len(), 2);
        assert_eq!(service_record.capabilities.get("version"), Some(&"0.1.0".to_string()));
        assert_eq!(service_record.capabilities.get("features"), Some(&"file_transfer,chat".to_string()));
    }
}