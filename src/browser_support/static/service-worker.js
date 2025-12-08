/**
 * Kizuna Service Worker
 * Provides offline functionality, caching, and background sync for PWA
 */

const CACHE_VERSION = 'kizuna-v1';
const CACHE_NAME = `${CACHE_VERSION}-static`;
const DATA_CACHE_NAME = `${CACHE_VERSION}-data`;

// Resources to cache for offline operation
const STATIC_RESOURCES = [
    '/',
    '/index.html',
    '/connect.html',
    '/kizuna-sdk.js',
    '/kizuna-ui.js',
    '/kizuna-ui.css',
    '/kizuna-file-transfer.js',
    '/kizuna-clipboard.js',
    '/kizuna-command.js',
    '/kizuna-feature-detection.js',
    '/kizuna-mobile.js',
    '/kizuna-mobile.css',
    '/kizuna-responsive.css',
];

// API endpoints that should be cached
const API_CACHE_PATTERNS = [
    /\/api\/manifest/,
    /\/api\/config/,
];

// Install event - cache static resources
self.addEventListener('install', (event) => {
    console.log('[Service Worker] Installing service worker...');

    event.waitUntil(
        caches.open(CACHE_NAME)
            .then((cache) => {
                console.log('[Service Worker] Caching static resources');
                return cache.addAll(STATIC_RESOURCES);
            })
            .then(() => {
                console.log('[Service Worker] Installation complete');
                return self.skipWaiting();
            })
            .catch((error) => {
                console.error('[Service Worker] Installation failed:', error);
            })
    );
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
    console.log('[Service Worker] Activating service worker...');

    event.waitUntil(
        caches.keys()
            .then((cacheNames) => {
                return Promise.all(
                    cacheNames.map((cacheName) => {
                        if (cacheName !== CACHE_NAME && cacheName !== DATA_CACHE_NAME) {
                            console.log('[Service Worker] Deleting old cache:', cacheName);
                            return caches.delete(cacheName);
                        }
                    })
                );
            })
            .then(() => {
                console.log('[Service Worker] Activation complete');
                return self.clients.claim();
            })
    );
});

// Fetch event - serve from cache with network fallback
self.addEventListener('fetch', (event) => {
    const { request } = event;
    const url = new URL(request.url);

    // Skip cross-origin requests
    if (url.origin !== location.origin) {
        return;
    }

    // Handle API requests with network-first strategy
    if (url.pathname.startsWith('/api/')) {
        event.respondWith(networkFirstStrategy(request));
        return;
    }

    // Handle static resources with cache-first strategy
    event.respondWith(cacheFirstStrategy(request));
});

/**
 * Cache-first strategy: Try cache first, then network
 * Best for static resources that don't change often
 */
async function cacheFirstStrategy(request) {
    try {
        const cache = await caches.open(CACHE_NAME);
        const cachedResponse = await cache.match(request);

        if (cachedResponse) {
            console.log('[Service Worker] Serving from cache:', request.url);
            return cachedResponse;
        }

        console.log('[Service Worker] Fetching from network:', request.url);
        const networkResponse = await fetch(request);

        // Cache successful responses
        if (networkResponse && networkResponse.status === 200) {
            cache.put(request, networkResponse.clone());
        }

        return networkResponse;
    } catch (error) {
        console.error('[Service Worker] Fetch failed:', error);

        // Return offline page for navigation requests
        if (request.mode === 'navigate') {
            const cache = await caches.open(CACHE_NAME);
            return cache.match('/index.html');
        }

        throw error;
    }
}

/**
 * Network-first strategy: Try network first, then cache
 * Best for API requests that need fresh data
 */
async function networkFirstStrategy(request) {
    try {
        console.log('[Service Worker] Fetching API from network:', request.url);
        const networkResponse = await fetch(request);

        // Cache successful API responses
        if (networkResponse && networkResponse.status === 200) {
            const cache = await caches.open(DATA_CACHE_NAME);
            cache.put(request, networkResponse.clone());
        }

        return networkResponse;
    } catch (error) {
        console.log('[Service Worker] Network failed, trying cache:', request.url);

        const cache = await caches.open(DATA_CACHE_NAME);
        const cachedResponse = await cache.match(request);

        if (cachedResponse) {
            console.log('[Service Worker] Serving API from cache:', request.url);
            return cachedResponse;
        }

        console.error('[Service Worker] No cached response available');
        throw error;
    }
}

// Background sync event - sync queued operations when online
self.addEventListener('sync', (event) => {
    console.log('[Service Worker] Background sync triggered:', event.tag);

    if (event.tag === 'sync-operations') {
        event.waitUntil(syncQueuedOperations());
    }
});

/**
 * Sync queued operations when connection is restored
 */
async function syncQueuedOperations() {
    try {
        console.log('[Service Worker] Syncing queued operations...');

        // Get queued operations from IndexedDB
        const db = await openDatabase();
        const operations = await getQueuedOperations(db);

        console.log(`[Service Worker] Found ${operations.length} queued operations`);

        // Process each operation
        for (const operation of operations) {
            try {
                await processOperation(operation);
                await removeQueuedOperation(db, operation.id);
                console.log('[Service Worker] Operation synced:', operation.id);
            } catch (error) {
                console.error('[Service Worker] Failed to sync operation:', operation.id, error);
            }
        }

        console.log('[Service Worker] Background sync complete');
    } catch (error) {
        console.error('[Service Worker] Background sync failed:', error);
        throw error;
    }
}

/**
 * Open IndexedDB for offline data storage
 */
function openDatabase() {
    return new Promise((resolve, reject) => {
        const request = indexedDB.open('KizunaOfflineDB', 1);

        request.onerror = () => reject(request.error);
        request.onsuccess = () => resolve(request.result);

        request.onupgradeneeded = (event) => {
            const db = event.target.result;

            // Create object stores
            if (!db.objectStoreNames.contains('operations')) {
                const operationsStore = db.createObjectStore('operations', { keyPath: 'id', autoIncrement: true });
                operationsStore.createIndex('timestamp', 'timestamp', { unique: false });
                operationsStore.createIndex('type', 'type', { unique: false });
            }

            if (!db.objectStoreNames.contains('settings')) {
                db.createObjectStore('settings', { keyPath: 'key' });
            }

            if (!db.objectStoreNames.contains('cache')) {
                const cacheStore = db.createObjectStore('cache', { keyPath: 'key' });
                cacheStore.createIndex('timestamp', 'timestamp', { unique: false });
            }
        };
    });
}

/**
 * Get queued operations from IndexedDB
 */
function getQueuedOperations(db) {
    return new Promise((resolve, reject) => {
        const transaction = db.transaction(['operations'], 'readonly');
        const store = transaction.objectStore('operations');
        const request = store.getAll();

        request.onerror = () => reject(request.error);
        request.onsuccess = () => resolve(request.result || []);
    });
}

/**
 * Remove queued operation from IndexedDB
 */
function removeQueuedOperation(db, operationId) {
    return new Promise((resolve, reject) => {
        const transaction = db.transaction(['operations'], 'readwrite');
        const store = transaction.objectStore('operations');
        const request = store.delete(operationId);

        request.onerror = () => reject(request.error);
        request.onsuccess = () => resolve();
    });
}

/**
 * Process a queued operation
 */
async function processOperation(operation) {
    const { type, data } = operation;

    switch (type) {
        case 'file-transfer':
            return await syncFileTransfer(data);
        case 'clipboard-sync':
            return await syncClipboard(data);
        case 'command-execution':
            return await syncCommand(data);
        default:
            console.warn('[Service Worker] Unknown operation type:', type);
    }
}

/**
 * Sync file transfer operation
 */
async function syncFileTransfer(data) {
    const response = await fetch('/api/file-transfer', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
    });

    if (!response.ok) {
        throw new Error(`File transfer sync failed: ${response.status}`);
    }

    return response.json();
}

/**
 * Sync clipboard operation
 */
async function syncClipboard(data) {
    const response = await fetch('/api/clipboard/sync', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
    });

    if (!response.ok) {
        throw new Error(`Clipboard sync failed: ${response.status}`);
    }

    return response.json();
}

/**
 * Sync command execution
 */
async function syncCommand(data) {
    const response = await fetch('/api/command/execute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
    });

    if (!response.ok) {
        throw new Error(`Command sync failed: ${response.status}`);
    }

    return response.json();
}

// Message event - handle messages from clients
self.addEventListener('message', (event) => {
    console.log('[Service Worker] Message received:', event.data);

    const { type, data } = event.data;

    switch (type) {
        case 'SKIP_WAITING':
            self.skipWaiting();
            break;

        case 'CACHE_URLS':
            event.waitUntil(cacheUrls(data.urls));
            break;

        case 'CLEAR_CACHE':
            event.waitUntil(clearCache(data.cacheName));
            break;

        case 'GET_CACHE_SIZE':
            event.waitUntil(getCacheSize().then((size) => {
                event.ports[0].postMessage({ size });
            }));
            break;

        default:
            console.warn('[Service Worker] Unknown message type:', type);
    }
});

/**
 * Cache additional URLs
 */
async function cacheUrls(urls) {
    const cache = await caches.open(CACHE_NAME);
    return cache.addAll(urls);
}

/**
 * Clear specific cache
 */
async function clearCache(cacheName) {
    if (cacheName) {
        return caches.delete(cacheName);
    }

    // Clear all caches
    const cacheNames = await caches.keys();
    return Promise.all(cacheNames.map((name) => caches.delete(name)));
}

/**
 * Get total cache size
 */
async function getCacheSize() {
    if ('storage' in navigator && 'estimate' in navigator.storage) {
        const estimate = await navigator.storage.estimate();
        return estimate.usage || 0;
    }
    return 0;
}

// Push notification event - handle incoming push messages
self.addEventListener('push', (event) => {
    console.log('[Service Worker] Push notification received');

    let notificationData = {
        title: 'Kizuna',
        body: 'You have a new notification',
        icon: '/icons/icon-192.png',
        badge: '/icons/badge-72.png',
        tag: 'kizuna-notification',
        data: {},
    };

    // Parse push data if available
    if (event.data) {
        try {
            const data = event.data.json();
            notificationData = {
                title: data.title || notificationData.title,
                body: data.body || notificationData.body,
                icon: data.icon || notificationData.icon,
                badge: data.badge || notificationData.badge,
                tag: data.tag || notificationData.tag,
                data: data.data || {},
                actions: data.actions || [],
                requireInteraction: data.requireInteraction || false,
                vibrate: data.vibrate || [200, 100, 200],
            };
        } catch (error) {
            console.error('[Service Worker] Failed to parse push data:', error);
        }
    }

    // Show notification
    event.waitUntil(
        self.registration.showNotification(notificationData.title, {
            body: notificationData.body,
            icon: notificationData.icon,
            badge: notificationData.badge,
            tag: notificationData.tag,
            data: notificationData.data,
            actions: notificationData.actions,
            requireInteraction: notificationData.requireInteraction,
            vibrate: notificationData.vibrate,
        })
    );
});

// Notification click event - handle notification interactions
self.addEventListener('notificationclick', (event) => {
    console.log('[Service Worker] Notification clicked:', event.notification.tag);

    event.notification.close();

    const notificationData = event.notification.data;
    const action = event.action;

    // Handle notification actions
    event.waitUntil(
        clients.matchAll({ type: 'window', includeUncontrolled: true })
            .then((clientList) => {
                // Focus existing window if available
                for (const client of clientList) {
                    if (client.url.includes(self.registration.scope) && 'focus' in client) {
                        return client.focus().then(() => {
                            // Send message to client about notification click
                            client.postMessage({
                                type: 'NOTIFICATION_CLICK',
                                action,
                                data: notificationData,
                            });
                        });
                    }
                }

                // Open new window if no existing window
                if (clients.openWindow) {
                    return clients.openWindow('/').then((client) => {
                        // Send message to new client
                        if (client) {
                            client.postMessage({
                                type: 'NOTIFICATION_CLICK',
                                action,
                                data: notificationData,
                            });
                        }
                    });
                }
            })
    );
});

// Notification close event - handle notification dismissal
self.addEventListener('notificationclose', (event) => {
    console.log('[Service Worker] Notification closed:', event.notification.tag);

    const notificationData = event.notification.data;

    // Track notification dismissal
    event.waitUntil(
        fetch('/api/notifications/dismissed', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
                tag: event.notification.tag,
                data: notificationData,
            }),
        }).catch((error) => {
            console.error('[Service Worker] Failed to track dismissal:', error);
        })
    );
});

console.log('[Service Worker] Service worker script loaded');
