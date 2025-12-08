/**
 * Kizuna PWA Manager
 * Handles service worker registration, offline data management, and PWA features
 */

class KizunaPWA {
    constructor() {
        this.serviceWorker = null;
        this.db = null;
        this.isOnline = navigator.onLine;
        this.syncInProgress = false;
        this.listeners = {
            online: [],
            offline: [],
            sync: [],
            install: [],
        };

        this.init();
    }

    /**
     * Initialize PWA manager
     */
    async init() {
        // Set up online/offline listeners
        window.addEventListener('online', () => this.handleOnline());
        window.addEventListener('offline', () => this.handleOffline());

        // Open IndexedDB
        try {
            this.db = await this.openDatabase();
            console.log('[PWA] Database opened successfully');
        } catch (error) {
            console.error('[PWA] Failed to open database:', error);
        }

        // Register service worker if supported
        if ('serviceWorker' in navigator) {
            try {
                await this.registerServiceWorker();
            } catch (error) {
                console.error('[PWA] Service worker registration failed:', error);
            }
        } else {
            console.warn('[PWA] Service workers not supported');
        }
    }

    /**
     * Register service worker
     */
    async registerServiceWorker() {
        try {
            const registration = await navigator.serviceWorker.register('/service-worker.js', {
                scope: '/',
            });

            console.log('[PWA] Service worker registered:', registration.scope);
            this.serviceWorker = registration;

            // Handle updates
            registration.addEventListener('updatefound', () => {
                const newWorker = registration.installing;
                console.log('[PWA] New service worker found');

                newWorker.addEventListener('statechange', () => {
                    if (newWorker.state === 'installed' && navigator.serviceWorker.controller) {
                        console.log('[PWA] New service worker installed, update available');
                        this.notifyListeners('install', { registration, newWorker });
                    }
                });
            });

            // Request background sync permission
            if ('sync' in registration) {
                console.log('[PWA] Background sync supported');
            }

            return registration;
        } catch (error) {
            console.error('[PWA] Service worker registration failed:', error);
            throw error;
        }
    }

    /**
     * Update service worker
     */
    async updateServiceWorker() {
        if (!this.serviceWorker) {
            throw new Error('No service worker registered');
        }

        try {
            await this.serviceWorker.update();
            console.log('[PWA] Service worker update check complete');
        } catch (error) {
            console.error('[PWA] Service worker update failed:', error);
            throw error;
        }
    }

    /**
     * Skip waiting and activate new service worker
     */
    async skipWaiting() {
        if (!this.serviceWorker || !this.serviceWorker.waiting) {
            return;
        }

        this.serviceWorker.waiting.postMessage({ type: 'SKIP_WAITING' });

        // Reload page after activation
        navigator.serviceWorker.addEventListener('controllerchange', () => {
            window.location.reload();
        });
    }

    /**
     * Open IndexedDB for offline data storage
     */
    openDatabase() {
        return new Promise((resolve, reject) => {
            const request = indexedDB.open('KizunaOfflineDB', 1);

            request.onerror = () => reject(request.error);
            request.onsuccess = () => resolve(request.result);

            request.onupgradeneeded = (event) => {
                const db = event.target.result;

                // Create object stores
                if (!db.objectStoreNames.contains('operations')) {
                    const operationsStore = db.createObjectStore('operations', {
                        keyPath: 'id',
                        autoIncrement: true
                    });
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
     * Queue operation for background sync
     */
    async queueOperation(type, data) {
        if (!this.db) {
            throw new Error('Database not initialized');
        }

        const operation = {
            type,
            data,
            timestamp: Date.now(),
            status: 'queued',
        };

        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction(['operations'], 'readwrite');
            const store = transaction.objectStore('operations');
            const request = store.add(operation);

            request.onerror = () => reject(request.error);
            request.onsuccess = () => {
                console.log('[PWA] Operation queued:', type, request.result);
                resolve(request.result);
            };
        });
    }

    /**
     * Get queued operations
     */
    async getQueuedOperations() {
        if (!this.db) {
            throw new Error('Database not initialized');
        }

        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction(['operations'], 'readonly');
            const store = transaction.objectStore('operations');
            const request = store.getAll();

            request.onerror = () => reject(request.error);
            request.onsuccess = () => resolve(request.result || []);
        });
    }

    /**
     * Clear queued operations
     */
    async clearQueuedOperations() {
        if (!this.db) {
            throw new Error('Database not initialized');
        }

        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction(['operations'], 'readwrite');
            const store = transaction.objectStore('operations');
            const request = store.clear();

            request.onerror = () => reject(request.error);
            request.onsuccess = () => {
                console.log('[PWA] Queued operations cleared');
                resolve();
            };
        });
    }

    /**
     * Save setting to offline storage
     */
    async saveSetting(key, value) {
        if (!this.db) {
            throw new Error('Database not initialized');
        }

        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction(['settings'], 'readwrite');
            const store = transaction.objectStore('settings');
            const request = store.put({ key, value, timestamp: Date.now() });

            request.onerror = () => reject(request.error);
            request.onsuccess = () => {
                console.log('[PWA] Setting saved:', key);
                resolve();
            };
        });
    }

    /**
     * Get setting from offline storage
     */
    async getSetting(key) {
        if (!this.db) {
            throw new Error('Database not initialized');
        }

        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction(['settings'], 'readonly');
            const store = transaction.objectStore('settings');
            const request = store.get(key);

            request.onerror = () => reject(request.error);
            request.onsuccess = () => resolve(request.result?.value);
        });
    }

    /**
     * Cache data for offline access
     */
    async cacheData(key, data, ttl = 3600000) {
        if (!this.db) {
            throw new Error('Database not initialized');
        }

        const cacheEntry = {
            key,
            data,
            timestamp: Date.now(),
            expiresAt: Date.now() + ttl,
        };

        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction(['cache'], 'readwrite');
            const store = transaction.objectStore('cache');
            const request = store.put(cacheEntry);

            request.onerror = () => reject(request.error);
            request.onsuccess = () => {
                console.log('[PWA] Data cached:', key);
                resolve();
            };
        });
    }

    /**
     * Get cached data
     */
    async getCachedData(key) {
        if (!this.db) {
            throw new Error('Database not initialized');
        }

        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction(['cache'], 'readonly');
            const store = transaction.objectStore('cache');
            const request = store.get(key);

            request.onerror = () => reject(request.error);
            request.onsuccess = () => {
                const entry = request.result;

                // Check if expired
                if (entry && entry.expiresAt > Date.now()) {
                    resolve(entry.data);
                } else {
                    resolve(null);
                }
            };
        });
    }

    /**
     * Clear expired cache entries
     */
    async clearExpiredCache() {
        if (!this.db) {
            throw new Error('Database not initialized');
        }

        return new Promise((resolve, reject) => {
            const transaction = this.db.transaction(['cache'], 'readwrite');
            const store = transaction.objectStore('cache');
            const index = store.index('timestamp');
            const request = index.openCursor();

            let deletedCount = 0;

            request.onerror = () => reject(request.error);
            request.onsuccess = (event) => {
                const cursor = event.target.result;

                if (cursor) {
                    const entry = cursor.value;

                    if (entry.expiresAt <= Date.now()) {
                        cursor.delete();
                        deletedCount++;
                    }

                    cursor.continue();
                } else {
                    console.log(`[PWA] Cleared ${deletedCount} expired cache entries`);
                    resolve(deletedCount);
                }
            };
        });
    }

    /**
     * Request background sync
     */
    async requestBackgroundSync() {
        if (!this.serviceWorker || !('sync' in this.serviceWorker)) {
            console.warn('[PWA] Background sync not supported');
            return false;
        }

        try {
            await this.serviceWorker.sync.register('sync-operations');
            console.log('[PWA] Background sync requested');
            return true;
        } catch (error) {
            console.error('[PWA] Background sync request failed:', error);
            return false;
        }
    }

    /**
     * Handle online event
     */
    async handleOnline() {
        console.log('[PWA] Connection restored');
        this.isOnline = true;
        this.notifyListeners('online');

        // Request background sync
        await this.requestBackgroundSync();
    }

    /**
     * Handle offline event
     */
    handleOffline() {
        console.log('[PWA] Connection lost');
        this.isOnline = false;
        this.notifyListeners('offline');
    }

    /**
     * Add event listener
     */
    on(event, callback) {
        if (this.listeners[event]) {
            this.listeners[event].push(callback);
        }
    }

    /**
     * Remove event listener
     */
    off(event, callback) {
        if (this.listeners[event]) {
            this.listeners[event] = this.listeners[event].filter(cb => cb !== callback);
        }
    }

    /**
     * Notify listeners
     */
    notifyListeners(event, data) {
        if (this.listeners[event]) {
            this.listeners[event].forEach(callback => callback(data));
        }
    }

    /**
     * Get cache size
     */
    async getCacheSize() {
        if ('storage' in navigator && 'estimate' in navigator.storage) {
            const estimate = await navigator.storage.estimate();
            return {
                usage: estimate.usage || 0,
                quota: estimate.quota || 0,
                percentage: estimate.quota ? (estimate.usage / estimate.quota) * 100 : 0,
            };
        }
        return null;
    }

    /**
     * Check if app is installed as PWA
     */
    isInstalled() {
        return window.matchMedia('(display-mode: standalone)').matches ||
            window.navigator.standalone === true;
    }

    /**
     * Get online status
     */
    getOnlineStatus() {
        return this.isOnline;
    }
}

// Create global instance
window.KizunaPWA = new KizunaPWA();
