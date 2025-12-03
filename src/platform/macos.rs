// macOS platform implementation

mod adapter;
mod cocoa;
mod keychain;
mod notifications;
mod system_tray;
mod spotlight;
mod security;
mod app_bundle;

pub use adapter::MacOSAdapter;

// Re-export security functions for external use
pub use security::{
    is_code_signed, check_code_signature, get_code_signing_info,
    sign_binary, check_gatekeeper_status, assess_gatekeeper,
    submit_for_notarization, staple_notarization, is_hardened_runtime,
    get_entitlements, CodeSigningInfo, GatekeeperAssessment,
};

// Re-export app bundle functions for external use
pub use app_bundle::{
    AppBundle, create_dmg, is_apple_silicon, get_architecture,
    create_universal_binary,
};
