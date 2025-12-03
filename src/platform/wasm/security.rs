// Browser security model compliance and API restrictions

use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::Window;
#[cfg(target_arch = "wasm32")]
use js_sys::Reflect;

/// Browser security context information
#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub is_secure_context: bool,
    pub cross_origin_isolated: bool,
    pub permissions: PermissionStatus,
    pub content_security_policy: Option<String>,
    pub restrictions: Vec<ApiRestriction>,
}

/// Permission status for various browser APIs
#[derive(Debug, Clone, Default)]
pub struct PermissionStatus {
    pub notifications: PermissionState,
    pub clipboard: PermissionState,
    pub geolocation: PermissionState,
    pub camera: PermissionState,
    pub microphone: PermissionState,
    pub storage: PermissionState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionState {
    Granted,
    Denied,
    Prompt,
    Unknown,
}

impl Default for PermissionState {
    fn default() -> Self {
        Self::Unknown
    }
}

/// API restrictions based on browser security model
#[derive(Debug, Clone)]
pub enum ApiRestriction {
    RequiresSecureContext(String),
    RequiresCrossOriginIsolation(String),
    RequiresUserGesture(String),
    RequiresPermission(String),
    NotAvailable(String),
}

/// Browser security manager
pub struct BrowserSecurityManager {
    #[cfg(target_arch = "wasm32")]
    window: Window,
    context: SecurityContext,
}

impl BrowserSecurityManager {
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().expect("no global window exists");
            let context = Self::detect_security_context(&window);
            
            Self { window, context }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {
                context: SecurityContext::default(),
            }
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    fn detect_security_context(window: &Window) -> SecurityContext {
        let is_secure = window.is_secure_context().unwrap_or(false);
        
        // Check for cross-origin isolation (required for SharedArrayBuffer)
        let cross_origin_isolated = Reflect::has(window, &JsValue::from_str("crossOriginIsolated"))
            .and_then(|has| {
                if has {
                    Reflect::get(window, &JsValue::from_str("crossOriginIsolated"))
                        .ok()
                        .and_then(|v| v.as_bool())
                } else {
                    Some(false)
                }
            })
            .unwrap_or(false);
        
        let mut restrictions = Vec::new();
        
        // Check for APIs that require secure context
        if !is_secure {
            restrictions.push(ApiRestriction::RequiresSecureContext("Clipboard API".to_string()));
            restrictions.push(ApiRestriction::RequiresSecureContext("Service Workers".to_string()));
            restrictions.push(ApiRestriction::RequiresSecureContext("WebRTC".to_string()));
            restrictions.push(ApiRestriction::RequiresSecureContext("Geolocation".to_string()));
        }
        
        // Check for APIs that require cross-origin isolation
        if !cross_origin_isolated {
            restrictions.push(ApiRestriction::RequiresCrossOriginIsolation("SharedArrayBuffer".to_string()));
            restrictions.push(ApiRestriction::RequiresCrossOriginIsolation("High-resolution timers".to_string()));
        }
        
        // APIs that require user gesture
        restrictions.push(ApiRestriction::RequiresUserGesture("Fullscreen API".to_string()));
        restrictions.push(ApiRestriction::RequiresUserGesture("Clipboard write".to_string()));
        
        SecurityContext {
            is_secure_context: is_secure,
            cross_origin_isolated,
            permissions: PermissionStatus::default(),
            content_security_policy: None,
            restrictions,
        }
    }
    
    pub fn get_context(&self) -> &SecurityContext {
        &self.context
    }
    
    pub fn is_api_available(&self, api_name: &str) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            match api_name {
                "clipboard" => self.context.is_secure_context && self.window.navigator().clipboard().is_some(),
                "serviceWorker" => self.context.is_secure_context && self.window.navigator().service_worker().is_some(),
                "webrtc" => self.context.is_secure_context && 
                    Reflect::has(&self.window, &JsValue::from_str("RTCPeerConnection")).unwrap_or(false),
                "sharedArrayBuffer" => self.context.cross_origin_isolated &&
                    Reflect::has(&self.window, &JsValue::from_str("SharedArrayBuffer")).unwrap_or(false),
                "notifications" => Reflect::has(&self.window, &JsValue::from_str("Notification")).unwrap_or(false),
                "localStorage" => self.window.local_storage().ok().flatten().is_some(),
                "indexedDB" => Reflect::has(&self.window, &JsValue::from_str("indexedDB")).unwrap_or(false),
                _ => false,
            }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            false
        }
    }
    
    pub fn get_api_restrictions(&self, api_name: &str) -> Vec<ApiRestriction> {
        self.context.restrictions.iter()
            .filter(|r| match r {
                ApiRestriction::RequiresSecureContext(name) |
                ApiRestriction::RequiresCrossOriginIsolation(name) |
                ApiRestriction::RequiresUserGesture(name) |
                ApiRestriction::RequiresPermission(name) |
                ApiRestriction::NotAvailable(name) => name.to_lowercase().contains(&api_name.to_lowercase()),
            })
            .cloned()
            .collect()
    }
    
    #[cfg(target_arch = "wasm32")]
    pub async fn request_permission(&self, permission: &str) -> Result<PermissionState, JsValue> {
        let navigator = self.window.navigator();
        
        match permission {
            "notifications" => {
                if let Ok(has_notif) = Reflect::has(&self.window, &JsValue::from_str("Notification")) {
                    if has_notif {
                        let notif_class = Reflect::get(&self.window, &JsValue::from_str("Notification"))?;
                        let request_fn = Reflect::get(&notif_class, &JsValue::from_str("requestPermission"))?;
                        
                        if let Ok(func) = request_fn.dyn_into::<js_sys::Function>() {
                            let promise = func.call0(&notif_class)?;
                            let result = wasm_bindgen_futures::JsFuture::from(js_sys::Promise::from(promise)).await?;
                            
                            if let Some(state) = result.as_string() {
                                return Ok(match state.as_str() {
                                    "granted" => PermissionState::Granted,
                                    "denied" => PermissionState::Denied,
                                    "prompt" => PermissionState::Prompt,
                                    _ => PermissionState::Unknown,
                                });
                            }
                        }
                    }
                }
                Ok(PermissionState::Unknown)
            }
            _ => Ok(PermissionState::Unknown),
        }
    }
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            is_secure_context: false,
            cross_origin_isolated: false,
            permissions: PermissionStatus::default(),
            content_security_policy: None,
            restrictions: Vec::new(),
        }
    }
}

/// Graceful degradation manager for unsupported features
pub struct GracefulDegradationManager {
    fallbacks: HashMap<String, FallbackStrategy>,
}

#[derive(Debug, Clone)]
pub enum FallbackStrategy {
    Alternative(String),
    Polyfill(String),
    Disabled,
    UserNotification(String),
}

impl GracefulDegradationManager {
    pub fn new() -> Self {
        let mut fallbacks = HashMap::new();
        
        // Define fallback strategies for common APIs
        fallbacks.insert(
            "clipboard".to_string(),
            FallbackStrategy::Alternative("Use manual copy/paste with textarea".to_string())
        );
        
        fallbacks.insert(
            "notifications".to_string(),
            FallbackStrategy::Alternative("Use in-app notifications".to_string())
        );
        
        fallbacks.insert(
            "serviceWorker".to_string(),
            FallbackStrategy::UserNotification("Offline functionality not available".to_string())
        );
        
        fallbacks.insert(
            "webrtc".to_string(),
            FallbackStrategy::Alternative("Use WebSocket for communication".to_string())
        );
        
        fallbacks.insert(
            "sharedArrayBuffer".to_string(),
            FallbackStrategy::Alternative("Use regular ArrayBuffer with message passing".to_string())
        );
        
        fallbacks.insert(
            "localStorage".to_string(),
            FallbackStrategy::Alternative("Use in-memory storage (data will not persist)".to_string())
        );
        
        Self { fallbacks }
    }
    
    pub fn get_fallback(&self, api_name: &str) -> Option<&FallbackStrategy> {
        self.fallbacks.get(api_name)
    }
    
    pub fn add_fallback(&mut self, api_name: String, strategy: FallbackStrategy) {
        self.fallbacks.insert(api_name, strategy);
    }
    
    pub fn handle_unavailable_api(&self, api_name: &str) -> String {
        match self.get_fallback(api_name) {
            Some(FallbackStrategy::Alternative(alt)) => {
                format!("{} is not available. Using alternative: {}", api_name, alt)
            }
            Some(FallbackStrategy::Polyfill(url)) => {
                format!("{} is not available. Load polyfill from: {}", api_name, url)
            }
            Some(FallbackStrategy::Disabled) => {
                format!("{} is not available and has been disabled", api_name)
            }
            Some(FallbackStrategy::UserNotification(msg)) => {
                msg.clone()
            }
            None => {
                format!("{} is not available and no fallback is configured", api_name)
            }
        }
    }
}

/// Performance optimization manager for browser environment
pub struct BrowserPerformanceManager {
    #[cfg(target_arch = "wasm32")]
    window: Window,
}

impl BrowserPerformanceManager {
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let window = web_sys::window().expect("no global window exists");
            Self { window }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {}
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    pub fn get_memory_info(&self) -> Option<MemoryInfo> {
        let performance = self.window.performance()?;
        
        // Try to get memory info (Chrome-specific)
        if let Ok(memory) = Reflect::get(&performance, &JsValue::from_str("memory")) {
            if !memory.is_undefined() {
                let used = Reflect::get(&memory, &JsValue::from_str("usedJSHeapSize"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;
                
                let total = Reflect::get(&memory, &JsValue::from_str("totalJSHeapSize"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;
                
                let limit = Reflect::get(&memory, &JsValue::from_str("jsHeapSizeLimit"))
                    .ok()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0) as u64;
                
                return Some(MemoryInfo { used, total, limit });
            }
        }
        
        None
    }
    
    #[cfg(target_arch = "wasm32")]
    pub fn request_idle_callback<F>(&self, callback: F) -> Result<(), JsValue>
    where
        F: FnOnce() + 'static,
    {
        // Check if requestIdleCallback is available
        if Reflect::has(&self.window, &JsValue::from_str("requestIdleCallback")).unwrap_or(false) {
            let closure = Closure::once(callback);
            let func = Reflect::get(&self.window, &JsValue::from_str("requestIdleCallback"))?;
            
            if let Ok(request_idle) = func.dyn_into::<js_sys::Function>() {
                request_idle.call1(&self.window, closure.as_ref())?;
                closure.forget();
            }
        } else {
            // Fallback to setTimeout
            let closure = Closure::once(callback);
            self.window.set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref(),
                0
            )?;
            closure.forget();
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub used: u64,
    pub total: u64,
    pub limit: u64,
}
