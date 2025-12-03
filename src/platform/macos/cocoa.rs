// Cocoa framework integration for macOS

use crate::platform::{PlatformResult, PlatformError};
use cocoa::base::{id, nil};
use cocoa::foundation::NSAutoreleasePool;
use objc::runtime::Object;
use std::sync::Once;

static COCOA_INIT: Once = Once::new();

/// Initialize Cocoa framework
pub fn initialize_cocoa() -> PlatformResult<()> {
    COCOA_INIT.call_once(|| {
        unsafe {
            // Create autorelease pool for Cocoa objects
            let _pool = NSAutoreleasePool::new(nil);
        }
    });
    Ok(())
}

/// Get the shared NSApplication instance
pub fn get_shared_application() -> PlatformResult<id> {
    unsafe {
        let app_class = objc::class!(NSApplication);
        let app: id = objc::msg_send![app_class, sharedApplication];
        
        if app == nil {
            return Err(PlatformError::IntegrationError(
                "Failed to get NSApplication instance".to_string()
            ));
        }
        
        Ok(app)
    }
}

/// Activate the application (bring to foreground)
pub fn activate_application() -> PlatformResult<()> {
    unsafe {
        let app = get_shared_application()?;
        let _: () = objc::msg_send![app, activateIgnoringOtherApps: true];
    }
    Ok(())
}

/// Check if running in a sandboxed environment
pub fn is_sandboxed() -> bool {
    use std::env;
    env::var("APP_SANDBOX_CONTAINER_ID").is_ok()
}
