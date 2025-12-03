// Win32 API integration for Windows platform

use crate::platform::{PlatformResult, PlatformError};

#[cfg(windows)]
use winapi::um::{
    combaseapi::{CoInitializeEx, CoUninitialize},
    objbase::COINIT_MULTITHREADED,
    winsock2::{WSAStartup, WSACleanup, WSADATA},
    winnt::OSVERSIONINFOEXW,
    sysinfoapi::GetVersionExW,
    winbase::GetComputerNameW,
};

#[cfg(windows)]
use std::ptr;

/// Initialize COM library for Windows API usage
#[cfg(windows)]
pub fn initialize_com() -> PlatformResult<()> {
    unsafe {
        let hr = CoInitializeEx(ptr::null_mut(), COINIT_MULTITHREADED);
        if hr < 0 {
            return Err(PlatformError::InitializationFailed(
                format!("Failed to initialize COM: HRESULT {:#x}", hr)
            ));
        }
    }
    Ok(())
}

/// Cleanup COM library
#[cfg(windows)]
pub fn cleanup_com() {
    unsafe {
        CoUninitialize();
    }
}

/// Initialize Winsock for networking
#[cfg(windows)]
pub fn initialize_winsock() -> PlatformResult<()> {
    unsafe {
        let mut wsa_data: WSADATA = std::mem::zeroed();
        let result = WSAStartup(0x0202, &mut wsa_data); // Request Winsock 2.2
        if result != 0 {
            return Err(PlatformError::InitializationFailed(
                format!("Failed to initialize Winsock: error code {}", result)
            ));
        }
    }
    Ok(())
}

/// Cleanup Winsock
#[cfg(windows)]
pub fn cleanup_winsock() {
    unsafe {
        WSACleanup();
    }
}

/// Get Windows version information
#[cfg(windows)]
pub fn get_windows_version() -> PlatformResult<String> {
    unsafe {
        let mut version_info: OSVERSIONINFOEXW = std::mem::zeroed();
        version_info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOEXW>() as u32;
        
        let result = GetVersionExW(&mut version_info as *mut _ as *mut _);
        if result == 0 {
            return Err(PlatformError::SystemError(
                "Failed to get Windows version".to_string()
            ));
        }
        
        Ok(format!(
            "{}.{}.{}",
            version_info.dwMajorVersion,
            version_info.dwMinorVersion,
            version_info.dwBuildNumber
        ))
    }
}

/// Get computer name
#[cfg(windows)]
pub fn get_computer_name() -> PlatformResult<String> {
    unsafe {
        let mut buffer: [u16; 256] = [0; 256];
        let mut size = buffer.len() as u32;
        
        let result = GetComputerNameW(buffer.as_mut_ptr(), &mut size);
        if result == 0 {
            return Err(PlatformError::SystemError(
                "Failed to get computer name".to_string()
            ));
        }
        
        let name = String::from_utf16_lossy(&buffer[..size as usize]);
        Ok(name)
    }
}

/// Check if Windows Defender is enabled
#[cfg(windows)]
pub fn is_windows_defender_enabled() -> PlatformResult<bool> {
    // This is a simplified check - in production, you'd query Windows Security Center
    // For now, we'll assume it's enabled on modern Windows systems
    Ok(true)
}

/// Get system information
#[cfg(windows)]
pub fn get_system_info() -> PlatformResult<SystemInfo> {
    Ok(SystemInfo {
        version: get_windows_version().unwrap_or_else(|_| "Unknown".to_string()),
        computer_name: get_computer_name().unwrap_or_else(|_| "Unknown".to_string()),
        defender_enabled: is_windows_defender_enabled().unwrap_or(false),
    })
}

#[cfg(windows)]
#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub version: String,
    pub computer_name: String,
    pub defender_enabled: bool,
}

// Stub implementations for non-Windows platforms
#[cfg(not(windows))]
pub fn initialize_com() -> PlatformResult<()> {
    Ok(())
}

#[cfg(not(windows))]
pub fn cleanup_com() {}

#[cfg(not(windows))]
pub fn initialize_winsock() -> PlatformResult<()> {
    Ok(())
}

#[cfg(not(windows))]
pub fn cleanup_winsock() {}

#[cfg(not(windows))]
pub fn get_windows_version() -> PlatformResult<String> {
    Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
}

#[cfg(not(windows))]
pub fn get_computer_name() -> PlatformResult<String> {
    Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
}

#[cfg(not(windows))]
pub fn is_windows_defender_enabled() -> PlatformResult<bool> {
    Err(PlatformError::UnsupportedPlatform("Not on Windows".to_string()))
}
