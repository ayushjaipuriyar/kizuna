/**
 * Kizuna Cache Manager
 * Handles intelligent resource caching, cache invalidation, and storage quota management
 */

class KizunaCacheManager {
    constructor() {
        this.cacheVersion = 'kizuna-v1';
        this.staticCacheName = `${this.cacheVersion}-static`;
        this.dataCacheName = `${this.cacheVersion}-data`;
        this.imageCacheName = `${this.cacheVersion}-images`;
        this.maxCacheAge = 7 * 24 * 60 * 60 * 1000; // 7 days
        this.maxCacheSize = 50 * 1024 * 1024; // 50 MB

        this.init();
    }

    /**
     * Initialize cache manager
     */
    async init() {
        // Check cache storage support
        if (!('caches' in window)) {
            console.warn('[Cache] Cache API not supported');
            return;
        }

        // Clean up expired caches
        await this.cleanupExpiredCaches();

        // Check storage quota
        await this.checkStorageQuota();
    }

    /**
     * Cache resource with strategy
     */
    async cacheResource(url, strategy = 'cache-first') {
        if (!('caches' in window)) {
            throw new Error('Cache API not supported');
        }

        try {
            const cache = await caches.open(this.staticCacheName);

            if (strategy === 'cache-first') {
                // Check cache first
                const cachedResponse = await cache.match(url);
                if (cachedResponse) {
                    return cachedResponse;
                }
            }

            // Fetch from network
            const response = await fetch(url);

            if (response && response.status === 200) {
                // Clone and cache the response
                cache.put(url, response.clone());
            }

            return response;
        } catch (error) {
            console.error('[Cache] Failed to cache resource:', url, error);
            throw error;
        }
    }

    /**
     * Cache multiple resources
     */
    async cacheResources(urls) {
        if (!('caches' in window)) {
            throw new Error('Cache API not supported');
        }

        try {
            const cache = await caches.open(this.staticCacheName);
            await cache.addAll(urls);
            console.log(`[Cache] Cached ${urls.length} resources`);
        } catch (error) {
            console.error('[Cache] Failed to cache resources:', error);
            throw error;
        }
    }

    /**
     * Cache data with expiration
     */
    async cacheData(key, data, ttl = this.maxCacheAge) {
        if (!('caches' in window)) {
            throw new Error('Cache API not supported');
        }

        try {
            const cache = await caches.open(this.dataCacheName);

            // Create response with metadata
            const cacheData = {
                data,
                timestamp: Date.now(),
                expiresAt: Date.now() + ttl,
            };

            const response = new Response(JSON.stringify(cacheData), {
                headers: {
                    'Content-Type': 'application/json',
                    'X-Cache-Timestamp': Date.now().toString(),
                    'X-Cache-Expires': (Date.now() + ttl).toString(),
                },
            });

            await cache.put(key, response);
            console.log('[Cache] Data cached:', key);
        } catch (error) {
            console.error('[Cache] Failed to cache data:', error);
            throw error;
        }
    }

    /**
     * Get cached data
     */
    async getCachedData(key) {
        if (!('caches' in window)) {
            return null;
        }

        try {
            const cache = await caches.open(this.dataCacheName);
            const response = await cache.match(key);

            if (!response) {
                return null;
            }

            const cacheData = await response.json();

            // Check if expired
            if (cacheData.expiresAt && cacheData.expiresAt < Date.now()) {
                console.log('[Cache] Data expired:', key);
                await cache.delete(key);
                return null;
            }

            return cacheData.data;
        } catch (error) {
            console.error('[Cache] Failed to get cached data:', error);
            return null;
        }
    }

    /**
     * Cache image with optimization
     */
    async cacheImage(url) {
        if (!('caches' in window)) {
            throw new Error('Cache API not supported');
        }

        try {
            const cache = await caches.open(this.imageCacheName);

            // Check if already cached
            const cachedResponse = await cache.match(url);
            if (cachedResponse) {
                return cachedResponse;
            }

            // Fetch and cache
            const response = await fetch(url);

            if (response && response.status === 200) {
                cache.put(url, response.clone());
            }

            return response;
        } catch (error) {
            console.error('[Cache] Failed to cache image:', url, error);
            throw error;
        }
    }

    /**
     * Invalidate cache entry
     */
    async invalidateCache(key, cacheName = null) {
        if (!('caches' in window)) {
            return false;
        }

        try {
            if (cacheName) {
                const cache = await caches.open(cacheName);
                const deleted = await cache.delete(key);
                console.log('[Cache] Cache invalidated:', key, deleted);
                return deleted;
            }

            // Try all caches
            const cacheNames = [this.staticCacheName, this.dataCacheName, this.imageCacheName];
            let deleted = false;

            for (const name of cacheNames) {
                const cache = await caches.open(name);
                if (await cache.delete(key)) {
                    deleted = true;
                }
            }

            console.log('[Cache] Cache invalidated:', key, deleted);
            return deleted;
        } catch (error) {
            console.error('[Cache] Failed to invalidate cache:', error);
            return false;
        }
    }

    /**
     * Clear specific cache
     */
    async clearCache(cacheName) {
        if (!('caches' in window)) {
            return false;
        }

        try {
            const deleted = await caches.delete(cacheName);
            console.log('[Cache] Cache cleared:', cacheName, deleted);
            return deleted;
        } catch (error) {
            console.error('[Cache] Failed to clear cache:', error);
            return false;
        }
    }

    /**
     * Clear all caches
     */
    async clearAllCaches() {
        if (!('caches' in window)) {
            return 0;
        }

        try {
            const cacheNames = await caches.keys();
            const results = await Promise.all(
                cacheNames.map(name => caches.delete(name))
            );

            const deletedCount = results.filter(r => r).length;
            console.log(`[Cache] Cleared ${deletedCount} caches`);
            return deletedCount;
        } catch (error) {
            console.error('[Cache] Failed to clear all caches:', error);
            return 0;
        }
    }

    /**
     * Clean up expired caches
     */
    async cleanupExpiredCaches() {
        if (!('caches' in window)) {
            return 0;
        }

        try {
            const cache = await caches.open(this.dataCacheName);
            const requests = await cache.keys();

            let deletedCount = 0;

            for (const request of requests) {
                const response = await cache.match(request);

                if (response) {
                    const expiresHeader = response.headers.get('X-Cache-Expires');

                    if (expiresHeader) {
                        const expiresAt = parseInt(expiresHeader, 10);

                        if (expiresAt < Date.now()) {
                            await cache.delete(request);
                            deletedCount++;
                        }
                    }
                }
            }

            console.log(`[Cache] Cleaned up ${deletedCount} expired entries`);
            return deletedCount;
        } catch (error) {
            console.error('[Cache] Failed to cleanup expired caches:', error);
            return 0;
        }
    }

    /**
     * Get cache size
     */
    async getCacheSize() {
        if (!('caches' in window)) {
            return 0;
        }

        try {
            const cacheNames = await caches.keys();
            let totalSize = 0;

            for (const cacheName of cacheNames) {
                const cache = await caches.open(cacheName);
                const requests = await cache.keys();

                for (const request of requests) {
                    const response = await cache.match(request);

                    if (response) {
                        const blob = await response.blob();
                        totalSize += blob.size;
                    }
                }
            }

            return totalSize;
        } catch (error) {
            console.error('[Cache] Failed to get cache size:', error);
            return 0;
        }
    }

    /**
     * Check storage quota
     */
    async checkStorageQuota() {
        if (!('storage' in navigator) || !('estimate' in navigator.storage)) {
            console.warn('[Cache] Storage API not supported');
            return null;
        }

        try {
            const estimate = await navigator.storage.estimate();
            const usage = estimate.usage || 0;
            const quota = estimate.quota || 0;
            const percentage = quota ? (usage / quota) * 100 : 0;

            console.log(`[Cache] Storage: ${this.formatBytes(usage)} / ${this.formatBytes(quota)} (${percentage.toFixed(2)}%)`);

            // Warn if approaching quota
            if (percentage > 80) {
                console.warn('[Cache] Storage quota approaching limit');
            }

            return {
                usage,
                quota,
                percentage,
                available: quota - usage,
            };
        } catch (error) {
            console.error('[Cache] Failed to check storage quota:', error);
            return null;
        }
    }

    /**
     * Request persistent storage
     */
    async requestPersistentStorage() {
        if (!('storage' in navigator) || !('persist' in navigator.storage)) {
            console.warn('[Cache] Persistent storage not supported');
            return false;
        }

        try {
            const isPersisted = await navigator.storage.persisted();

            if (isPersisted) {
                console.log('[Cache] Storage is already persistent');
                return true;
            }

            const granted = await navigator.storage.persist();
            console.log('[Cache] Persistent storage:', granted ? 'granted' : 'denied');
            return granted;
        } catch (error) {
            console.error('[Cache] Failed to request persistent storage:', error);
            return false;
        }
    }

    /**
     * Prune cache to fit within size limit
     */
    async pruneCache() {
        const currentSize = await this.getCacheSize();

        if (currentSize <= this.maxCacheSize) {
            console.log('[Cache] Cache size within limit');
            return 0;
        }

        console.log('[Cache] Cache size exceeds limit, pruning...');

        try {
            const cache = await caches.open(this.dataCacheName);
            const requests = await cache.keys();

            // Sort by timestamp (oldest first)
            const entries = [];

            for (const request of requests) {
                const response = await cache.match(request);

                if (response) {
                    const timestampHeader = response.headers.get('X-Cache-Timestamp');
                    const timestamp = timestampHeader ? parseInt(timestampHeader, 10) : 0;

                    entries.push({ request, timestamp });
                }
            }

            entries.sort((a, b) => a.timestamp - b.timestamp);

            // Delete oldest entries until within limit
            let deletedCount = 0;
            let newSize = currentSize;

            for (const entry of entries) {
                if (newSize <= this.maxCacheSize) {
                    break;
                }

                const response = await cache.match(entry.request);

                if (response) {
                    const blob = await response.blob();
                    await cache.delete(entry.request);
                    newSize -= blob.size;
                    deletedCount++;
                }
            }

            console.log(`[Cache] Pruned ${deletedCount} entries`);
            return deletedCount;
        } catch (error) {
            console.error('[Cache] Failed to prune cache:', error);
            return 0;
        }
    }

    /**
     * Get cache statistics
     */
    async getStatistics() {
        const cacheSize = await this.getCacheSize();
        const storageQuota = await this.checkStorageQuota();

        let entryCount = 0;

        if ('caches' in window) {
            const cacheNames = await caches.keys();

            for (const cacheName of cacheNames) {
                const cache = await caches.open(cacheName);
                const requests = await cache.keys();
                entryCount += requests.length;
            }
        }

        return {
            cacheSize,
            entryCount,
            storageQuota,
            maxCacheSize: this.maxCacheSize,
            maxCacheAge: this.maxCacheAge,
        };
    }

    /**
     * Format bytes to human-readable string
     */
    formatBytes(bytes) {
        if (bytes === 0) return '0 Bytes';

        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));

        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }
}

// Create global instance
window.KizunaCache = new KizunaCacheManager();
