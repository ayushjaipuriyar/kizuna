// Progressive Web App functionality

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use web_sys::{Window, ServiceWorkerContainer, ServiceWorkerRegistration, CacheStorage, Cache};
#[cfg(target_arch = "wasm32")]
use js_sys::{Array, Promise};

/// PWA manifest configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PwaManifest {
    pub name: String,
    pub short_name: String,
    pub description: String,
    pub start_url: String,
    pub display: DisplayMode,
    pub background_color: String,
    pub theme_color: String,
    pub icons: Vec<Icon>,
    pub categories: Vec<String>,
    pub orientation: Orientation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DisplayMode {
    Fullscreen,
    Standalone,
    MinimalUi,
    Browser,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Icon {
    pub src: String,
    pub sizes: String,
    #[serde(rename = "type")]
    pub icon_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Orientation {
    Any,
    Natural,
    Landscape,
    LandscapePrimary,
    LandscapeSecondary,
    Portrait,
    PortraitPrimary,
    PortraitSecondary,
}

impl Default for PwaManifest {
    fn default() -> Self {
        Self {
            name: "Kizuna".to_string(),
            short_name: "Kizuna".to_string(),
            description: "Cross-platform device connectivity and file sharing".to_string(),
            start_url: "/".to_string(),
            display: DisplayMode::Standalone,
            background_color: "#ffffff".to_string(),
            theme_color: "#4a90e2".to_string(),
            icons: vec![
                Icon {
                    src: "/icons/icon-192.png".to_string(),
                    sizes: "192x192".to_string(),
                    icon_type: "image/png".to_string(),
                    purpose: Some("any maskable".to_string()),
                },
                Icon {
                    src: "/icons/icon-512.png".to_string(),
                    sizes: "512x512".to_string(),
                    icon_type: "image/png".to_string(),
                    purpose: Some("any maskable".to_string()),
                },
            ],
            categories: vec!["productivity".to_string(), "utilities".to_string()],
            orientation: Orientation::Any,
        }
    }
}

impl PwaManifest {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Service Worker manager for PWA functionality
pub struct ServiceWorkerManager {
    #[cfg(target_arch = "wasm32")]
    container: Option<ServiceWorkerContainer>,
}

impl ServiceWorkerManager {
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let container = web_sys::window()
                .and_then(|w| w.navigator().service_worker());
            
            Self { container }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {}
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    pub async fn register(&self, script_url: &str) -> Result<ServiceWorkerRegistration, JsValue> {
        let container = self.container.as_ref()
            .ok_or_else(|| JsValue::from_str("Service Worker not supported"))?;
        
        let promise = container.register(script_url)?;
        let result = wasm_bindgen_futures::JsFuture::from(promise).await?;
        
        result.dyn_into::<ServiceWorkerRegistration>()
    }
    
    #[cfg(target_arch = "wasm32")]
    pub fn is_supported(&self) -> bool {
        self.container.is_some()
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    pub fn is_supported(&self) -> bool {
        false
    }
    
    /// Generate service worker script content
    pub fn generate_service_worker_script() -> String {
        r#"
// Kizuna Service Worker
const CACHE_NAME = 'kizuna-v1';
const OFFLINE_URL = '/offline.html';

// Files to cache for offline functionality
const CACHE_URLS = [
    '/',
    '/index.html',
    '/offline.html',
    '/kizuna.js',
    '/kizuna_bg.wasm',
    '/icons/icon-192.png',
    '/icons/icon-512.png',
];

// Install event - cache essential files
self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open(CACHE_NAME).then((cache) => {
            return cache.addAll(CACHE_URLS);
        })
    );
    self.skipWaiting();
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
    event.waitUntil(
        caches.keys().then((cacheNames) => {
            return Promise.all(
                cacheNames.map((cacheName) => {
                    if (cacheName !== CACHE_NAME) {
                        return caches.delete(cacheName);
                    }
                })
            );
        })
    );
    self.clients.claim();
});

// Fetch event - serve from cache, fallback to network
self.addEventListener('fetch', (event) => {
    if (event.request.mode === 'navigate') {
        event.respondWith(
            fetch(event.request).catch(() => {
                return caches.match(OFFLINE_URL);
            })
        );
        return;
    }
    
    event.respondWith(
        caches.match(event.request).then((response) => {
            if (response) {
                return response;
            }
            
            return fetch(event.request).then((response) => {
                // Cache successful responses
                if (response.status === 200) {
                    const responseClone = response.clone();
                    caches.open(CACHE_NAME).then((cache) => {
                        cache.put(event.request, responseClone);
                    });
                }
                return response;
            });
        })
    );
});

// Background sync for offline operations
self.addEventListener('sync', (event) => {
    if (event.tag === 'sync-data') {
        event.waitUntil(syncData());
    }
});

async function syncData() {
    // Implement data synchronization logic
    console.log('Syncing data...');
}

// Push notifications
self.addEventListener('push', (event) => {
    const options = {
        body: event.data ? event.data.text() : 'New notification',
        icon: '/icons/icon-192.png',
        badge: '/icons/badge-72.png',
        vibrate: [200, 100, 200],
    };
    
    event.waitUntil(
        self.registration.showNotification('Kizuna', options)
    );
});

// Notification click handler
self.addEventListener('notificationclick', (event) => {
    event.notification.close();
    
    event.waitUntil(
        clients.openWindow('/')
    );
});
"#.to_string()
    }
}

/// Offline storage manager using browser storage APIs
pub struct OfflineStorageManager {
    #[cfg(target_arch = "wasm32")]
    window: Window,
}

impl OfflineStorageManager {
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
    pub async fn cache_resource(&self, url: &str, data: &[u8]) -> Result<(), JsValue> {
        let caches = self.window.caches()?;
        let cache_name = "kizuna-resources";
        
        let cache_promise = caches.open(cache_name)?;
        let cache = wasm_bindgen_futures::JsFuture::from(cache_promise).await?;
        let cache: Cache = cache.dyn_into()?;
        
        // Create a Response from the data
        let array = js_sys::Uint8Array::from(data);
        let response = web_sys::Response::new_with_opt_u8_array(Some(&array))?;
        
        let put_promise = cache.put_with_str(url, &response)?;
        wasm_bindgen_futures::JsFuture::from(put_promise).await?;
        
        Ok(())
    }
    
    #[cfg(target_arch = "wasm32")]
    pub async fn get_cached_resource(&self, url: &str) -> Result<Option<Vec<u8>>, JsValue> {
        let caches = self.window.caches()?;
        let cache_name = "kizuna-resources";
        
        let cache_promise = caches.open(cache_name)?;
        let cache = wasm_bindgen_futures::JsFuture::from(cache_promise).await?;
        let cache: Cache = cache.dyn_into()?;
        
        let match_promise = cache.match_with_str(url)?;
        let response = wasm_bindgen_futures::JsFuture::from(match_promise).await?;
        
        if response.is_undefined() {
            return Ok(None);
        }
        
        let response: web_sys::Response = response.dyn_into()?;
        let array_buffer_promise = response.array_buffer()?;
        let array_buffer = wasm_bindgen_futures::JsFuture::from(array_buffer_promise).await?;
        
        let array = js_sys::Uint8Array::new(&array_buffer);
        let mut data = vec![0u8; array.length() as usize];
        array.copy_to(&mut data);
        
        Ok(Some(data))
    }
    
    #[cfg(target_arch = "wasm32")]
    pub async fn clear_cache(&self) -> Result<(), JsValue> {
        let caches = self.window.caches()?;
        let keys_promise = caches.keys()?;
        let keys = wasm_bindgen_futures::JsFuture::from(keys_promise).await?;
        let keys: Array = keys.dyn_into()?;
        
        for i in 0..keys.length() {
            if let Some(key) = keys.get(i).as_string() {
                let delete_promise = caches.delete(&key)?;
                wasm_bindgen_futures::JsFuture::from(delete_promise).await?;
            }
        }
        
        Ok(())
    }
}

/// Background sync manager for offline operations
pub struct BackgroundSyncManager {
    #[cfg(target_arch = "wasm32")]
    registration: Option<ServiceWorkerRegistration>,
}

impl BackgroundSyncManager {
    pub fn new() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self { registration: None }
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            Self {}
        }
    }
    
    #[cfg(target_arch = "wasm32")]
    pub fn set_registration(&mut self, registration: ServiceWorkerRegistration) {
        self.registration = Some(registration);
    }
    
    #[cfg(target_arch = "wasm32")]
    pub async fn register_sync(&self, tag: &str) -> Result<(), JsValue> {
        let registration = self.registration.as_ref()
            .ok_or_else(|| JsValue::from_str("No service worker registration"))?;
        
        // Check if sync manager is available
        if let Ok(sync) = js_sys::Reflect::get(registration, &JsValue::from_str("sync")) {
            if !sync.is_undefined() {
                // Register background sync
                let register_method = js_sys::Reflect::get(&sync, &JsValue::from_str("register"))?;
                if let Ok(func) = register_method.dyn_into::<js_sys::Function>() {
                    let promise = func.call1(&sync, &JsValue::from_str(tag))?;
                    wasm_bindgen_futures::JsFuture::from(Promise::from(promise)).await?;
                }
            }
        }
        
        Ok(())
    }
}
