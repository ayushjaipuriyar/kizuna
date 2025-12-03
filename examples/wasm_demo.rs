// WebAssembly platform demonstration
// This example shows how to use the WASM platform adapter

#[cfg(target_arch = "wasm32")]
use kizuna::platform::wasm::{WasmAdapter, BrowserCapabilities};

#[cfg(target_arch = "wasm32")]
use kizuna::platform::PlatformAdapter;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn main() -> Result<(), JsValue> {
    // Set up panic hook for better error messages
    console_error_panic_hook::set_once();
    
    // Initialize logger
    wasm_logger::init(wasm_logger::Config::default());
    
    log::info!("Starting Kizuna WASM demo");
    
    // Create WASM adapter
    let adapter = WasmAdapter::new();
    
    // Initialize platform
    adapter.initialize_platform().await
        .map_err(|e| JsValue::from_str(&format!("Initialization failed: {}", e)))?;
    
    // Get browser capabilities
    let capabilities = adapter.get_capabilities();
    log::info!("Browser capabilities detected:");
    log::info!("  Notifications: {}", capabilities.notifications);
    log::info!("  Service Worker: {}", capabilities.service_worker);
    log::info!("  Local Storage: {}", capabilities.local_storage);
    log::info!("  WebRTC: {}", capabilities.web_rtc);
    log::info!("  WebSocket: {}", capabilities.web_socket);
    log::info!("  Clipboard API: {}", capabilities.clipboard_api);
    
    // Integrate system services
    let services = adapter.integrate_system_services().await
        .map_err(|e| JsValue::from_str(&format!("Service integration failed: {}", e)))?;
    
    log::info!("System services integrated:");
    log::info!("  Notifications: {}", services.notifications);
    log::info!("  File Manager: {}", services.file_manager);
    log::info!("  Network Manager: {}", services.network_manager);
    
    // Setup UI framework
    let ui = adapter.setup_ui_framework().await
        .map_err(|e| JsValue::from_str(&format!("UI setup failed: {}", e)))?;
    
    log::info!("UI framework: {:?}", ui.framework_type);
    log::info!("UI capabilities: {:?}", ui.capabilities);
    
    // Configure networking
    let network = adapter.configure_networking().await
        .map_err(|e| JsValue::from_str(&format!("Network config failed: {}", e)))?;
    
    log::info!("Preferred protocols: {:?}", network.preferred_protocols);
    
    // Setup security
    let security = adapter.setup_security_integration().await
        .map_err(|e| JsValue::from_str(&format!("Security setup failed: {}", e)))?;
    
    log::info!("Security sandbox enabled: {}", security.sandbox_enabled);
    
    // Request notification permission if available
    if capabilities.notifications {
        match adapter.request_notification_permission().await {
            Ok(granted) => {
                log::info!("Notification permission: {}", if granted { "granted" } else { "denied" });
            }
            Err(e) => {
                log::warn!("Failed to request notification permission: {}", e);
            }
        }
    }
    
    // Try to access local storage
    if capabilities.local_storage {
        match adapter.get_local_storage() {
            Ok(storage) => {
                log::info!("Local storage is available");
                
                // Store a test value
                if let Err(e) = storage.set_item("kizuna_test", "Hello from Kizuna!") {
                    log::warn!("Failed to write to local storage: {:?}", e);
                } else {
                    log::info!("Successfully wrote to local storage");
                    
                    // Read it back
                    match storage.get_item("kizuna_test") {
                        Ok(Some(value)) => {
                            log::info!("Read from local storage: {}", value);
                        }
                        Ok(None) => {
                            log::warn!("Value not found in local storage");
                        }
                        Err(e) => {
                            log::warn!("Failed to read from local storage: {:?}", e);
                        }
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to access local storage: {}", e);
            }
        }
    }
    
    log::info!("Kizuna WASM demo completed successfully!");
    
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("This example is only available for WASM target.");
    println!("Build with: wasm-pack build --target web");
}
