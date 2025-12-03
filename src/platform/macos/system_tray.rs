// macOS System Tray (Status Bar) integration

use crate::platform::{PlatformResult, PlatformError};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSString, NSAutoreleasePool};
use cocoa::appkit::{NSStatusBar, NSVariableStatusItemLength};
use objc::runtime::Object;

/// Initialize system tray
pub fn initialize_system_tray() -> PlatformResult<()> {
    unsafe {
        let _pool = NSAutoreleasePool::new(nil);
        
        // Get system status bar
        let status_bar_class = objc::class!(NSStatusBar);
        let status_bar: id = objc::msg_send![status_bar_class, systemStatusBar];
        
        if status_bar == nil {
            return Err(PlatformError::IntegrationError(
                "Failed to get system status bar".to_string()
            ));
        }
    }
    
    Ok(())
}

/// Create a status bar item (system tray icon)
pub fn create_status_item(title: &str) -> PlatformResult<id> {
    unsafe {
        let pool = NSAutoreleasePool::new(nil);
        
        // Get system status bar
        let status_bar_class = objc::class!(NSStatusBar);
        let status_bar: id = objc::msg_send![status_bar_class, systemStatusBar];
        
        if status_bar == nil {
            return Err(PlatformError::IntegrationError(
                "Failed to get system status bar".to_string()
            ));
        }
        
        // Create status item
        let status_item: id = objc::msg_send![
            status_bar,
            statusItemWithLength: NSVariableStatusItemLength
        ];
        
        if status_item == nil {
            return Err(PlatformError::IntegrationError(
                "Failed to create status item".to_string()
            ));
        }
        
        // Set title
        let title_str = NSString::alloc(nil);
        let title_str = NSString::init_str(title_str, title);
        let _: () = objc::msg_send![status_item, setTitle: title_str];
        
        let _: () = objc::msg_send![pool, drain];
        
        Ok(status_item)
    }
}

/// Remove a status bar item
pub fn remove_status_item(status_item: id) -> PlatformResult<()> {
    unsafe {
        let status_bar_class = objc::class!(NSStatusBar);
        let status_bar: id = objc::msg_send![status_bar_class, systemStatusBar];
        
        if status_bar != nil && status_item != nil {
            let _: () = objc::msg_send![status_bar, removeStatusItem: status_item];
        }
    }
    
    Ok(())
}

/// Update status item title
pub fn update_status_item_title(status_item: id, title: &str) -> PlatformResult<()> {
    unsafe {
        if status_item == nil {
            return Err(PlatformError::IntegrationError(
                "Invalid status item".to_string()
            ));
        }
        
        let title_str = NSString::alloc(nil);
        let title_str = NSString::init_str(title_str, title);
        let _: () = objc::msg_send![status_item, setTitle: title_str];
    }
    
    Ok(())
}
