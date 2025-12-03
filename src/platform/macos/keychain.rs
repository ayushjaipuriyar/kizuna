// macOS Keychain integration

use crate::platform::{PlatformResult, PlatformError};
use core_foundation::base::{TCFType, ToVoid};
use core_foundation::string::CFString;
use core_foundation::dictionary::CFDictionary;
use std::ptr;

/// Check if Keychain is available
pub fn is_keychain_available() -> bool {
    // Keychain is always available on macOS
    true
}

/// Store a value in the Keychain
pub fn store_keychain_item(
    service: &str,
    account: &str,
    password: &[u8],
) -> PlatformResult<()> {
    use keyring::Entry;
    
    let entry = Entry::new(service, account)
        .map_err(|e| PlatformError::IntegrationError(format!("Keychain entry creation failed: {}", e)))?;
    
    let password_str = String::from_utf8_lossy(password);
    entry.set_password(&password_str)
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to store in Keychain: {}", e)))?;
    
    Ok(())
}

/// Retrieve a value from the Keychain
pub fn retrieve_keychain_item(
    service: &str,
    account: &str,
) -> PlatformResult<Vec<u8>> {
    use keyring::Entry;
    
    let entry = Entry::new(service, account)
        .map_err(|e| PlatformError::IntegrationError(format!("Keychain entry creation failed: {}", e)))?;
    
    let password = entry.get_password()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to retrieve from Keychain: {}", e)))?;
    
    Ok(password.into_bytes())
}

/// Delete a value from the Keychain
pub fn delete_keychain_item(
    service: &str,
    account: &str,
) -> PlatformResult<()> {
    use keyring::Entry;
    
    let entry = Entry::new(service, account)
        .map_err(|e| PlatformError::IntegrationError(format!("Keychain entry creation failed: {}", e)))?;
    
    entry.delete_credential()
        .map_err(|e| PlatformError::IntegrationError(format!("Failed to delete from Keychain: {}", e)))?;
    
    Ok(())
}

/// Check if a Keychain item exists
pub fn keychain_item_exists(
    service: &str,
    account: &str,
) -> bool {
    use keyring::Entry;
    
    if let Ok(entry) = Entry::new(service, account) {
        entry.get_password().is_ok()
    } else {
        false
    }
}
