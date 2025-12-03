// WebAssembly/Browser platform adapter

use async_trait::async_trait;
use crate::platform::{
    PlatformResult, PlatformAdapter, SystemServices, UIFramework,
    NetworkConfig, SecurityConfig, GUIFramework, PlatformError,
};
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{Window, Navigator, Storage, Notification, NotificationPermission};
#[cfg(target_arch = "wasm32")]
use js_sys::Promise;

/// WebAssembly platform adapter with browser API integration
pub struct WasmAdapter {
    #[cfg(target_arch = "wasm32")]
    window: Window,
    #[cfg(target_arch = "wasm32")]
    navigator: Navigator,
    browser_capabilities: BrowserCapabilities,
}

/// Browser feature detection and capabilities
#[derive(Debug, Clone)]
pub struct BrowserCapabilities {
    pub notifications: bool,
    pub service_worker: bool,
    pub local_storage: bool,
    pub session_storage: bool,
    pub web_rtc: bool,
    pub web_socket: bool,
    pub clipboard_api: bool,
    pub file_api: bool,
    pub cache_api: bool,
    pub indexeddb: bool,
    pub web_workers: bool,
    pub shared_array_buffer: bool,
}

impl WasmAdapter {
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().expect("no global window exists");
            let navigator = window.navigator();
            let capabilities = Self::detect_browser_capabilities(&window, &navigator);
            
            Self {
                window,
                navigator,
                browser_capabilities: capabilities,
            }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                browser_capabilities: BrowserCapabilities::default(),
            }
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    fn detect_browser_capabilities(window: &Window, navigator: &Navigator) -> BrowserCapabilities {
        BrowserCapabilities {
            notifications: js_sys::Reflect::has(&window, &JsValue::from_str("Notification"))
                .unwrap_or(false),
            service_worker: navigator.service_worker().is_some(),
            local_storage: window.local_storage().ok().flatten().is_some(),
            session_storage: window.session_storage().ok().flatten().is_some(),
            web_rtc: js_sys::Reflect::has(&window, &JsValue::from_str("RTCPeerConnection"))
                .unwrap_or(false),
            web_socket: js_sys::Reflect::has(&window, &JsValue::from_str("WebSocket"))
                .unwrap_or(false),
            clipboard_api: navigator.clipboard().is_some(),
            file_api: js_sys::Reflect::has(&window, &JsValue::from_str("File"))
                .unwrap_or(false),
            cache_api: js_sys::Reflect::has(&window, &JsValue::from_str("caches"))
                .unwrap_or(false),
            indexeddb: js_sys::Reflect::has(&window, &JsValue::from_str("indexedDB"))
                .unwrap_or(false),
            web_workers: js_sys::Reflect::has(&window, &JsValue::from_str("Worker"))
                .unwrap_or(false),
            shared_array_buffer: js_sys::Reflect::has(&window, &JsValue::from_str("SharedArrayBuffer"))
                .unwrap_or(false),
        }
    }
    
    pub fn get_capabilities(&self) -> &BrowserCapabilities {
        &self.browser_capabilities
    }
    
    #[cfg(target_arch = "wasm32")]
    pub async fn request_notification_permission(&self) -> Result<bool, PlatformError> {
        if !self.browser_capabilities.notifications {
            return Ok(false);
        }
        
        let permission = Notification::permission();
        match permission {
            NotificationPermission::Granted => Ok(true),
            NotificationPermission::Denied => Ok(false),
            NotificationPermission::Default => {
                // Request permission
                let promise = Notification::request_permission()
                    .map_err(|e| PlatformError::InitializationError(
                        format!("Failed to request notification permission: {:?}", e)
                    ))?;
                
                let result = wasm_bindgen_futures::JsFuture::from(promise)
                    .await
                    .map_err(|e| PlatformError::InitializationError(
                        format!("Permission request failed: {:?}", e)
                    ))?;
                
                let permission_str = result.as_string().unwrap_or_default();
                Ok(permission_str == "granted")
            }
            _ => Ok(false),
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    pub fn get_local_storage(&self) -> Result<Storage, PlatformError> {
        self.window
            .local_storage()
            .map_err(|e| PlatformError::InitializationError(
                format!("Failed to access local storage: {:?}", e)
            ))?
            .ok_or_else(|| PlatformError::InitializationError(
                "Local storage not available".to_string()
            ))
    }
    
    #[cfg(target_arch = "wasm32")]
    pub fn get_session_storage(&self) -> Result<Storage, PlatformError> {
        self.window
            .session_storage()
            .map_err(|e| PlatformError::InitializationError(
                format!("Failed to access session storage: {:?}", e)
            ))?
            .ok_or_else(|| PlatformError::InitializationError(
                "Session storage not available".to_string()
            ))
    }
}

impl Default for BrowserCapabilities {
    fn default() -> Self {
        Self {
            notifications: false,
            service_worker: false,
            local_storage: false,
            session_storage: false,
            web_rtc: false,
            web_socket: false,
            clipboard_api: false,
            file_api: false,
            cache_api: false,
            indexeddb: false,
            web_workers: false,
            shared_array_buffer: false,
        }
    }
}

#[async_trait]
impl PlatformAdapter for WasmAdapter {
    async fn initialize_platform(&self) -> PlatformResult<()> {
        #[cfg(target_arch = "wasm32")]
        {
            // Set up panic hook for better error messages in browser console
            console_error_panic_hook::set_once();
            
            // Initialize wasm logger
            wasm_logger::init(wasm_logger::Config::default());
            
            log::info!("WASM platform initialized");
            log::info!("Browser capabilities: {:?}", self.browser_capabilities);
        }
        
        Ok(())
    }

    async fn integrate_system_services(&self) -> PlatformResult<SystemServices> {
        let mut metadata = HashMap::new();
        
        #[cfg(target_arch = "wasm32")]
        {
            metadata.insert("user_agent".to_string(), self.navigator.user_agent().unwrap_or_default());
            metadata.insert("platform".to_string(), self.navigator.platform().unwrap_or_default());
            metadata.insert("language".to_string(), self.navigator.language().unwrap_or_default());
            
            if let Some(connection) = self.navigator.connection() {
                if let Ok(effective_type) = js_sys::Reflect::get(&connection, &JsValue::from_str("effectiveType")) {
                    if let Some(type_str) = effective_type.as_string() {
                        metadata.insert("connection_type".to_string(), type_str);
                    }
                }
            }
        }
        
        Ok(SystemServices {
            notifications: self.browser_capabilities.notifications,
            system_tray: false,
            file_manager: self.browser_capabilities.file_api,
            network_manager: self.browser_capabilities.web_socket || self.browser_capabilities.web_rtc,
            metadata,
        })
    }

    async fn setup_ui_framework(&self) -> PlatformResult<UIFramework> {
        let mut capabilities = vec![
            "dom".to_string(),
            "canvas".to_string(),
        ];
        
        #[cfg(target_arch = "wasm32")]
        {
            if js_sys::Reflect::has(&self.window, &JsValue::from_str("WebGLRenderingContext"))
                .unwrap_or(false) {
                capabilities.push("webgl".to_string());
            }
            
            if js_sys::Reflect::has(&self.window, &JsValue::from_str("WebGL2RenderingContext"))
                .unwrap_or(false) {
                capabilities.push("webgl2".to_string());
            }
            
            if self.browser_capabilities.web_workers {
                capabilities.push("web_workers".to_string());
            }
        }
        
        Ok(UIFramework {
            framework_type: GUIFramework::Web,
            version: "wasm32".to_string(),
            capabilities,
        })
    }

    async fn configure_networking(&self) -> PlatformResult<NetworkConfig> {
        let mut config = NetworkConfig::default();
        
        let mut protocols = Vec::new();
        if self.browser_capabilities.web_rtc {
            protocols.push("webrtc".to_string());
        }
        if self.browser_capabilities.web_socket {
            protocols.push("websocket".to_string());
        }
        
        config.preferred_protocols = protocols;
        Ok(config)
    }

    async fn setup_security_integration(&self) -> PlatformResult<SecurityConfig> {
        let mut config = SecurityConfig::default();
        config.sandbox_enabled = true;
        
        #[cfg(target_arch = "wasm32")]
        {
            // Check if running in secure context
            if let Ok(is_secure) = self.window.is_secure_context() {
                config.metadata.insert("secure_context".to_string(), is_secure.to_string());
            }
            
            // Check for cross-origin isolation (required for SharedArrayBuffer)
            config.metadata.insert(
                "cross_origin_isolated".to_string(),
                self.browser_capabilities.shared_array_buffer.to_string()
            );
        }
        
        Ok(config)
    }

    fn platform_name(&self) -> &str {
        "wasm"
    }
}
