// JavaScript bindings for browser API integration

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{Window, Storage, Notification, NotificationOptions};
#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Object, Reflect};
use serde::{Serialize, Deserialize};

/// JavaScript API wrapper for Kizuna WASM module
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct KizunaWasm {
    initialized: bool,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl KizunaWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        wasm_logger::init(wasm_logger::Config::default());
        
        Self {
            initialized: false,
        }
    }
    
    /// Initialize the Kizuna WASM module
    #[wasm_bindgen]
    pub async fn initialize(&mut self) -> Result<JsValue, JsValue> {
        if self.initialized {
            return Ok(JsValue::from_str("Already initialized"));
        }
        
        log::info!("Initializing Kizuna WASM module");
        self.initialized = true;
        
        Ok(JsValue::from_str("Initialized successfully"))
    }
    
    /// Check if a browser feature is available
    #[wasm_bindgen]
    pub fn check_feature(&self, feature: &str) -> bool {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return false,
        };
        
        match feature {
            "notifications" => Reflect::has(&window, &JsValue::from_str("Notification"))
                .unwrap_or(false),
            "serviceWorker" => window.navigator().service_worker().is_some(),
            "localStorage" => window.local_storage().ok().flatten().is_some(),
            "webrtc" => Reflect::has(&window, &JsValue::from_str("RTCPeerConnection"))
                .unwrap_or(false),
            "websocket" => Reflect::has(&window, &JsValue::from_str("WebSocket"))
                .unwrap_or(false),
            "clipboard" => window.navigator().clipboard().is_some(),
            _ => false,
        }
    }
    
    /// Get all available browser capabilities
    #[wasm_bindgen]
    pub fn get_capabilities(&self) -> JsValue {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return JsValue::NULL,
        };
        
        let navigator = window.navigator();
        let obj = Object::new();
        
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("notifications"),
            &JsValue::from_bool(Reflect::has(&window, &JsValue::from_str("Notification")).unwrap_or(false))
        );
        
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("serviceWorker"),
            &JsValue::from_bool(navigator.service_worker().is_some())
        );
        
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("localStorage"),
            &JsValue::from_bool(window.local_storage().ok().flatten().is_some())
        );
        
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("webrtc"),
            &JsValue::from_bool(Reflect::has(&window, &JsValue::from_str("RTCPeerConnection")).unwrap_or(false))
        );
        
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("websocket"),
            &JsValue::from_bool(Reflect::has(&window, &JsValue::from_str("WebSocket")).unwrap_or(false))
        );
        
        obj.into()
    }
    
    /// Show a browser notification
    #[wasm_bindgen]
    pub async fn show_notification(&self, title: &str, body: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        
        if !Reflect::has(&window, &JsValue::from_str("Notification")).unwrap_or(false) {
            return Err(JsValue::from_str("Notifications not supported"));
        }
        
        let mut options = NotificationOptions::new();
        options.body(body);
        
        Notification::new_with_options(title, &options)?;
        Ok(())
    }
    
    /// Store data in local storage
    #[wasm_bindgen]
    pub fn store_local(&self, key: &str, value: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let storage = window
            .local_storage()?
            .ok_or_else(|| JsValue::from_str("Local storage not available"))?;
        
        storage.set_item(key, value)?;
        Ok(())
    }
    
    /// Retrieve data from local storage
    #[wasm_bindgen]
    pub fn get_local(&self, key: &str) -> Result<Option<String>, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let storage = window
            .local_storage()?
            .ok_or_else(|| JsValue::from_str("Local storage not available"))?;
        
        storage.get_item(key)
    }
    
    /// Remove data from local storage
    #[wasm_bindgen]
    pub fn remove_local(&self, key: &str) -> Result<(), JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("No window"))?;
        let storage = window
            .local_storage()?
            .ok_or_else(|| JsValue::from_str("Local storage not available"))?;
        
        storage.remove_item(key)?;
        Ok(())
    }
    
    /// Get browser information
    #[wasm_bindgen]
    pub fn get_browser_info(&self) -> JsValue {
        let window = match web_sys::window() {
            Some(w) => w,
            None => return JsValue::NULL,
        };
        
        let navigator = window.navigator();
        let obj = Object::new();
        
        if let Ok(user_agent) = navigator.user_agent() {
            let _ = Reflect::set(&obj, &JsValue::from_str("userAgent"), &JsValue::from_str(&user_agent));
        }
        
        if let Ok(platform) = navigator.platform() {
            let _ = Reflect::set(&obj, &JsValue::from_str("platform"), &JsValue::from_str(&platform));
        }
        
        if let Some(language) = navigator.language() {
            let _ = Reflect::set(&obj, &JsValue::from_str("language"), &JsValue::from_str(&language));
        }
        
        obj.into()
    }
}

/// Polyfill detection and loading
#[cfg(target_arch = "wasm32")]
pub struct PolyfillManager;

#[cfg(target_arch = "wasm32")]
impl PolyfillManager {
    /// Check which polyfills are needed
    pub fn detect_needed_polyfills() -> Vec<String> {
        let mut needed = Vec::new();
        let window = match web_sys::window() {
            Some(w) => w,
            None => return needed,
        };
        
        // Check for Promise support
        if !Reflect::has(&window, &JsValue::from_str("Promise")).unwrap_or(false) {
            needed.push("promise".to_string());
        }
        
        // Check for fetch API
        if !Reflect::has(&window, &JsValue::from_str("fetch")).unwrap_or(false) {
            needed.push("fetch".to_string());
        }
        
        // Check for WebSocket
        if !Reflect::has(&window, &JsValue::from_str("WebSocket")).unwrap_or(false) {
            needed.push("websocket".to_string());
        }
        
        // Check for IndexedDB
        if !Reflect::has(&window, &JsValue::from_str("indexedDB")).unwrap_or(false) {
            needed.push("indexeddb".to_string());
        }
        
        needed
    }
    
    /// Generate polyfill script tags
    pub fn generate_polyfill_script() -> String {
        let needed = Self::detect_needed_polyfills();
        
        if needed.is_empty() {
            return String::new();
        }
        
        let polyfills = needed.join(",");
        format!(
            r#"<script src="https://polyfill.io/v3/polyfill.min.js?features={}"></script>"#,
            polyfills
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct KizunaWasm;

#[cfg(not(target_arch = "wasm32"))]
impl KizunaWasm {
    pub fn new() -> Self {
        Self
    }
}
