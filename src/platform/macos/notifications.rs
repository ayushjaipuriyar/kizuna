// macOS Notification Center integration

use crate::platform::{PlatformResult, PlatformError};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSString, NSAutoreleasePool};
use objc::runtime::Object;
use std::sync::Once;

static NOTIFICATION_INIT: Once = Once::new();

/// Initialize the notification center
pub fn initialize_notification_center() -> PlatformResult<()> {
    NOTIFICATION_INIT.call_once(|| {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);
            // Request notification permissions
            let center_class = objc::class!(NSUserNotificationCenter);
            let center: id = objc::msg_send![center_class, defaultUserNotificationCenter];
            
            if center != nil {
                // Set up notification center
                let _: () = objc::msg_send![center, setDelegate: nil];
            }
        }
    });
    Ok(())
}

/// Send a notification through macOS Notification Center
pub fn send_notification(
    title: &str,
    message: &str,
    sound: bool,
) -> PlatformResult<()> {
    unsafe {
        let pool = NSAutoreleasePool::new(nil);
        
        // Get notification center
        let center_class = objc::class!(NSUserNotificationCenter);
        let center: id = objc::msg_send![center_class, defaultUserNotificationCenter];
        
        if center == nil {
            return Err(PlatformError::IntegrationError(
                "Failed to get notification center".to_string()
            ));
        }
        
        // Create notification
        let notification_class = objc::class!(NSUserNotification);
        let notification: id = objc::msg_send![notification_class, alloc];
        let notification: id = objc::msg_send![notification, init];
        
        // Set title
        let title_str = NSString::alloc(nil);
        let title_str = NSString::init_str(title_str, title);
        let _: () = objc::msg_send![notification, setTitle: title_str];
        
        // Set message
        let message_str = NSString::alloc(nil);
        let message_str = NSString::init_str(message_str, message);
        let _: () = objc::msg_send![notification, setInformativeText: message_str];
        
        // Set sound if requested
        if sound {
            let sound_class = objc::class!(NSUserNotificationDefaultSoundName);
            let default_sound: id = objc::msg_send![sound_class, defaultSound];
            let _: () = objc::msg_send![notification, setSoundName: default_sound];
        }
        
        // Deliver notification
        let _: () = objc::msg_send![center, deliverNotification: notification];
        
        let _: () = objc::msg_send![pool, drain];
    }
    
    Ok(())
}

/// Remove all delivered notifications
pub fn remove_all_notifications() -> PlatformResult<()> {
    unsafe {
        let center_class = objc::class!(NSUserNotificationCenter);
        let center: id = objc::msg_send![center_class, defaultUserNotificationCenter];
        
        if center != nil {
            let _: () = objc::msg_send![center, removeAllDeliveredNotifications];
        }
    }
    
    Ok(())
}
