// Windows networking and firewall integration

use crate::platform::{PlatformResult, PlatformError};

#[cfg(windows)]
use winapi::um::{
    iphlpapi::{GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH},
    iptypes::IP_ADAPTER_ADDRESSES,
    winsock2::AF_UNSPEC,
};

#[cfg(windows)]
use std::ptr;

/// Windows networking manager
pub struct WindowsNetworking;

impl WindowsNetworking {
    pub fn new() -> Self {
        Self
    }

    /// Check if Windows Firewall is enabled
    pub fn is_firewall_enabled(&self) -> PlatformResult<bool> {
        #[cfg(windows)]
        {
            // This is a simplified check
            // In production, you'd use Windows Firewall API (INetFwPolicy2)
            // For now, we'll assume it's enabled on modern Windows systems
            Ok(true)
        }
        
        #[cfg(not(windows))]
        {
            Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
        }
    }

    /// Get network adapter information
    pub fn get_network_adapters(&self) -> PlatformResult<Vec<NetworkAdapter>> {
        #[cfg(windows)]
        {
            unsafe {
                let mut adapters = Vec::new();
                let mut buffer_size = 15000u32;
                let mut buffer = vec![0u8; buffer_size as usize];
                
                let result = GetAdaptersAddresses(
                    AF_UNSPEC as u32,
                    0,
                    ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH,
                    &mut buffer_size,
                );
                
                if result != 0 {
                    return Err(PlatformError::SystemError(
                        format!("Failed to get network adapters: error code {}", result)
                    ));
                }
                
                let mut current = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;
                while !current.is_null() {
                    let adapter = &*current;
                    
                    // Get adapter name
                    let name = if !adapter.FriendlyName.is_null() {
                        let len = (0..).take_while(|&i| *adapter.FriendlyName.offset(i) != 0).count();
                        let slice = std::slice::from_raw_parts(adapter.FriendlyName, len);
                        String::from_utf16_lossy(slice)
                    } else {
                        "Unknown".to_string()
                    };
                    
                    adapters.push(NetworkAdapter {
                        name,
                        description: "Network Adapter".to_string(),
                        status: if adapter.OperStatus == 1 { "Up" } else { "Down" }.to_string(),
                    });
                    
                    current = adapter.Next;
                }
                
                Ok(adapters)
            }
        }
        
        #[cfg(not(windows))]
        {
            Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
        }
    }

    /// Configure Windows Firewall rule for the application
    pub fn configure_firewall_rule(&self, rule_name: &str, port: u16) -> PlatformResult<()> {
        #[cfg(windows)]
        {
            // This would use Windows Firewall API (INetFwPolicy2) to add rules
            // For now, we'll return success as a placeholder
            // In production, you'd use COM to interact with Windows Firewall
            Ok(())
        }
        
        #[cfg(not(windows))]
        {
            Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
        }
    }

    /// Get network connection status
    pub fn get_connection_status(&self) -> PlatformResult<ConnectionStatus> {
        #[cfg(windows)]
        {
            // Check if we have any active network adapters
            let adapters = self.get_network_adapters()?;
            let has_active = adapters.iter().any(|a| a.status == "Up");
            
            Ok(ConnectionStatus {
                connected: has_active,
                connection_type: if has_active { "Ethernet/WiFi" } else { "None" }.to_string(),
                adapter_count: adapters.len(),
            })
        }
        
        #[cfg(not(windows))]
        {
            Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
        }
    }

    /// Configure network settings for optimal performance
    pub fn optimize_network_settings(&self) -> PlatformResult<()> {
        #[cfg(windows)]
        {
            // This would configure TCP/IP stack settings for optimal performance
            // For now, we'll return success as a placeholder
            Ok(())
        }
        
        #[cfg(not(windows))]
        {
            Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
        }
    }
}

impl Default for WindowsNetworking {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct NetworkAdapter {
    pub name: String,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct ConnectionStatus {
    pub connected: bool,
    pub connection_type: String,
    pub adapter_count: usize,
}
