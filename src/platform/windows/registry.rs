// Windows Registry integration for configuration and system settings

use crate::platform::{PlatformResult, PlatformError};
use std::collections::HashMap;

#[cfg(windows)]
use winapi::um::{
    winreg::{
        RegOpenKeyExW, RegCloseKey, RegQueryValueExW, RegSetValueExW,
        RegCreateKeyExW, RegDeleteKeyW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
    },
    winnt::{KEY_READ, KEY_WRITE, REG_SZ, REG_DWORD},
};

#[cfg(windows)]
use std::ptr;

/// Windows Registry manager for configuration and system settings
pub struct RegistryManager {
    app_key_path: String,
}

impl RegistryManager {
    pub fn new() -> Self {
        Self {
            app_key_path: "Software\\Kizuna".to_string(),
        }
    }

    /// Initialize registry keys for the application
    pub fn initialize(&self) -> PlatformResult<()> {
        #[cfg(windows)]
        {
            self.create_app_key()?;
        }
        Ok(())
    }

    /// Create application registry key
    #[cfg(windows)]
    fn create_app_key(&self) -> PlatformResult<()> {
        unsafe {
            let key_path = self.string_to_wide(&self.app_key_path);
            let mut hkey = ptr::null_mut();
            let mut disposition = 0u32;
            
            let result = RegCreateKeyExW(
                HKEY_CURRENT_USER,
                key_path.as_ptr(),
                0,
                ptr::null_mut(),
                0,
                KEY_WRITE,
                ptr::null_mut(),
                &mut hkey,
                &mut disposition,
            );
            
            if result != 0 {
                return Err(PlatformError::SystemError(
                    format!("Failed to create registry key: error code {}", result)
                ));
            }
            
            RegCloseKey(hkey);
        }
        Ok(())
    }

    /// Read a string value from the registry
    pub fn read_string(&self, key_name: &str, value_name: &str) -> PlatformResult<String> {
        #[cfg(windows)]
        {
            unsafe {
                let key_path = self.string_to_wide(key_name);
                let value_name_wide = self.string_to_wide(value_name);
                let mut hkey = ptr::null_mut();
                
                let result = RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    key_path.as_ptr(),
                    0,
                    KEY_READ,
                    &mut hkey,
                );
                
                if result != 0 {
                    return Err(PlatformError::SystemError(
                        format!("Failed to open registry key: error code {}", result)
                    ));
                }
                
                let mut buffer: [u16; 512] = [0; 512];
                let mut buffer_size = (buffer.len() * 2) as u32;
                let mut value_type = 0u32;
                
                let result = RegQueryValueExW(
                    hkey,
                    value_name_wide.as_ptr(),
                    ptr::null_mut(),
                    &mut value_type,
                    buffer.as_mut_ptr() as *mut u8,
                    &mut buffer_size,
                );
                
                RegCloseKey(hkey);
                
                if result != 0 {
                    return Err(PlatformError::SystemError(
                        format!("Failed to read registry value: error code {}", result)
                    ));
                }
                
                let len = (buffer_size / 2) as usize;
                let value = String::from_utf16_lossy(&buffer[..len.saturating_sub(1)]);
                Ok(value)
            }
        }
        
        #[cfg(not(windows))]
        {
            Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
        }
    }

    /// Write a string value to the registry
    pub fn write_string(&self, key_name: &str, value_name: &str, value: &str) -> PlatformResult<()> {
        #[cfg(windows)]
        {
            unsafe {
                let key_path = self.string_to_wide(key_name);
                let value_name_wide = self.string_to_wide(value_name);
                let value_wide = self.string_to_wide(value);
                let mut hkey = ptr::null_mut();
                
                let result = RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    key_path.as_ptr(),
                    0,
                    KEY_WRITE,
                    &mut hkey,
                );
                
                if result != 0 {
                    return Err(PlatformError::SystemError(
                        format!("Failed to open registry key: error code {}", result)
                    ));
                }
                
                let data_size = (value_wide.len() * 2) as u32;
                let result = RegSetValueExW(
                    hkey,
                    value_name_wide.as_ptr(),
                    0,
                    REG_SZ,
                    value_wide.as_ptr() as *const u8,
                    data_size,
                );
                
                RegCloseKey(hkey);
                
                if result != 0 {
                    return Err(PlatformError::SystemError(
                        format!("Failed to write registry value: error code {}", result)
                    ));
                }
            }
        }
        Ok(())
    }

    /// Read a DWORD value from the registry
    pub fn read_dword(&self, key_name: &str, value_name: &str) -> PlatformResult<u32> {
        #[cfg(windows)]
        {
            unsafe {
                let key_path = self.string_to_wide(key_name);
                let value_name_wide = self.string_to_wide(value_name);
                let mut hkey = ptr::null_mut();
                
                let result = RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    key_path.as_ptr(),
                    0,
                    KEY_READ,
                    &mut hkey,
                );
                
                if result != 0 {
                    return Err(PlatformError::SystemError(
                        format!("Failed to open registry key: error code {}", result)
                    ));
                }
                
                let mut value: u32 = 0;
                let mut buffer_size = std::mem::size_of::<u32>() as u32;
                let mut value_type = 0u32;
                
                let result = RegQueryValueExW(
                    hkey,
                    value_name_wide.as_ptr(),
                    ptr::null_mut(),
                    &mut value_type,
                    &mut value as *mut u32 as *mut u8,
                    &mut buffer_size,
                );
                
                RegCloseKey(hkey);
                
                if result != 0 {
                    return Err(PlatformError::SystemError(
                        format!("Failed to read registry value: error code {}", result)
                    ));
                }
                
                Ok(value)
            }
        }
        
        #[cfg(not(windows))]
        {
            Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
        }
    }

    /// Write a DWORD value to the registry
    pub fn write_dword(&self, key_name: &str, value_name: &str, value: u32) -> PlatformResult<()> {
        #[cfg(windows)]
        {
            unsafe {
                let key_path = self.string_to_wide(key_name);
                let value_name_wide = self.string_to_wide(value_name);
                let mut hkey = ptr::null_mut();
                
                let result = RegOpenKeyExW(
                    HKEY_CURRENT_USER,
                    key_path.as_ptr(),
                    0,
                    KEY_WRITE,
                    &mut hkey,
                );
                
                if result != 0 {
                    return Err(PlatformError::SystemError(
                        format!("Failed to open registry key: error code {}", result)
                    ));
                }
                
                let result = RegSetValueExW(
                    hkey,
                    value_name_wide.as_ptr(),
                    0,
                    REG_DWORD,
                    &value as *const u32 as *const u8,
                    std::mem::size_of::<u32>() as u32,
                );
                
                RegCloseKey(hkey);
                
                if result != 0 {
                    return Err(PlatformError::SystemError(
                        format!("Failed to write registry value: error code {}", result)
                    ));
                }
            }
        }
        Ok(())
    }

    /// Get application configuration from registry
    pub fn get_app_config(&self) -> PlatformResult<HashMap<String, String>> {
        let mut config = HashMap::new();
        
        #[cfg(windows)]
        {
            // Try to read common configuration values
            if let Ok(value) = self.read_string(&self.app_key_path, "InstallPath") {
                config.insert("install_path".to_string(), value);
            }
            if let Ok(value) = self.read_string(&self.app_key_path, "Version") {
                config.insert("version".to_string(), value);
            }
        }
        
        Ok(config)
    }

    /// Set application configuration in registry
    pub fn set_app_config(&self, key: &str, value: &str) -> PlatformResult<()> {
        self.write_string(&self.app_key_path, key, value)
    }

    /// Convert Rust string to wide string for Windows API
    #[cfg(windows)]
    fn string_to_wide(&self, s: &str) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        use std::ffi::OsStr;
        
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }
}

impl Default for RegistryManager {
    fn default() -> Self {
        Self::new()
    }
}
