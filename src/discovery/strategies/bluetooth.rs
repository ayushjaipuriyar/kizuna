use crate::discovery::{Discovery, DiscoveryError, ServiceRecord};
use async_trait::async_trait;
use std::time::Duration;

pub struct BluetoothDiscovery {
    peer_id: String,
    device_name: String,
    service_uuid: String,
}

impl BluetoothDiscovery {
    pub fn new() -> Self {
        Self {
            peer_id: format!("kizuna-{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
            device_name: "Kizuna Device".to_string(),
            service_uuid: "6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
        }
    }

    pub fn with_config(peer_id: String, device_name: String, service_uuid: String) -> Self {
        Self {
            peer_id,
            device_name,
            service_uuid,
        }
    }
}

#[async_trait]
impl Discovery for BluetoothDiscovery {
    async fn discover(&self, _timeout: Duration) -> Result<Vec<ServiceRecord>, DiscoveryError> {
        // TODO: Implement Bluetooth LE discovery
        Err(DiscoveryError::StrategyUnavailable {
            strategy: "bluetooth".to_string(),
        })
    }

    async fn announce(&self) -> Result<(), DiscoveryError> {
        // TODO: Implement Bluetooth LE advertisement
        Err(DiscoveryError::StrategyUnavailable {
            strategy: "bluetooth".to_string(),
        })
    }

    async fn stop_announce(&self) -> Result<(), DiscoveryError> {
        // TODO: Stop Bluetooth LE advertisement
        Ok(())
    }

    fn strategy_name(&self) -> &'static str {
        "bluetooth"
    }

    fn is_available(&self) -> bool {
        // TODO: Check if Bluetooth is available and has permissions
        false // Set to false until implemented
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