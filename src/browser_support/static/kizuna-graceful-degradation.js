/**
 * Kizuna Graceful Degradation System
 * 
 * Provides fallback mechanisms and progressive enhancement for browsers
 * without full feature support. Ensures core functionality remains available
 * even when advanced features are not supported.
 * 
 * @version 1.0.0
 */

(function (global) {
    'use strict';

    /**
     * Feature Support Levels
     */
    const SupportLevel = {
        FULL: 'full',           // Feature fully supported
        PARTIAL: 'partial',     // Feature partially supported with limitations
        FALLBACK: 'fallback',   // Feature available through fallback mechanism
        DEGRADED: 'degraded',   // Feature available but with reduced functionality
        UNSUPPORTED: 'unsupported' // Feature not available
    };

    /**
     * Graceful Degradation Manager
     */
    class GracefulDegradation {
        constructor() {
            this.featureSupport = new Map();
            this.fallbackHandlers = new Map();
            this.userNotifications = [];
            this.init();
        }

        init() {
            this._assessFeatureSupport();
            this._registerFallbackHandlers();
            this._setupProgressiveEnhancement();
        }

        /**
         * Assess support level for all features
         * @private
         */
        _assessFeatureSupport() {
            // WebRTC support
            this.featureSupport.set('webrtc', this._assessWebRTCSupport());

            // Clipboard support
            this.featureSupport.set('clipboard', this._assessClipboardSupport());

            // File API support
            this.featureSupport.set('fileAPI', this._assessFileAPISupport());

            // Media support
            this.featureSupport.set('media', this._assessMediaSupport());

            // Storage support
            this.featureSupport.set('storage', this._assessStorageSupport());

            // Service Worker support
            this.featureSupport.set('serviceWorker', this._assessServiceWorkerSupport());

            // Notifications support
            this.featureSupport.set('notifications', this._assessNotificationsSupport());

            // Fullscreen support
            this.featureSupport.set('fullscreen', this._assessFullscreenSupport());
        }

        /**
         * Assess WebRTC support level
         * @private
         */
        _assessWebRTCSupport() {
            if (!window.RTCPeerConnection && !window.webkitRTCPeerConnection && !window.mozRTCPeerConnection) {
                return {
                    level: SupportLevel.FALLBACK,
                    message: 'WebRTC not supported. Using WebSocket fallback for communication.',
                    limitations: ['No peer-to-peer video streaming', 'Higher latency for data transfer'],
                    fallback: 'websocket'
                };
            }

            // Check for DataChannel support
            try {
                const pc = new RTCPeerConnection();
                const dc = pc.createDataChannel('test');
                dc.close();
                pc.close();

                return {
                    level: SupportLevel.FULL,
                    message: 'WebRTC fully supported',
                    limitations: [],
                    fallback: null
                };
            } catch (error) {
                return {
                    level: SupportLevel.PARTIAL,
                    message: 'WebRTC partially supported. DataChannels may not work.',
                    limitations: ['DataChannel support uncertain'],
                    fallback: 'websocket'
                };
            }
        }

        /**
         * Assess Clipboard API support level
         * @private
         */
        _assessClipboardSupport() {
            if (!navigator.clipboard) {
                return {
                    level: SupportLevel.FALLBACK,
                    message: 'Clipboard API not supported. Using execCommand fallback.',
                    limitations: ['Requires user interaction', 'May not work in all contexts'],
                    fallback: 'execCommand'
                };
            }

            // Check for read/write permissions
            const hasRead = typeof navigator.clipboard.readText === 'function';
            const hasWrite = typeof navigator.clipboard.writeText === 'function';

            if (hasRead && hasWrite) {
                return {
                    level: SupportLevel.FULL,
                    message: 'Clipboard API fully supported',
                    limitations: ['Requires user permission'],
                    fallback: null
                };
            }

            return {
                level: SupportLevel.PARTIAL,
                message: 'Clipboard API partially supported',
                limitations: ['Some operations may require fallback'],
                fallback: 'execCommand'
            };
        }

        /**
         * Assess File API support level
         * @private
         */
        _assessFileAPISupport() {
            const hasFile = window.File && window.FileReader && window.FileList && window.Blob;

            if (!hasFile) {
                return {
                    level: SupportLevel.UNSUPPORTED,
                    message: 'File API not supported. File operations unavailable.',
                    limitations: ['Cannot transfer files', 'Cannot read file contents'],
                    fallback: null
                };
            }

            // Check for drag and drop
            const hasDragDrop = 'draggable' in document.createElement('div');

            if (hasDragDrop) {
                return {
                    level: SupportLevel.FULL,
                    message: 'File API fully supported with drag and drop',
                    limitations: [],
                    fallback: null
                };
            }

            return {
                level: SupportLevel.DEGRADED,
                message: 'File API supported but drag and drop unavailable',
                limitations: ['Use file input for file selection'],
                fallback: 'fileInput'
            };
        }

        /**
         * Assess Media API support level
         * @private
         */
        _assessMediaSupport() {
            const hasMediaDevices = navigator.mediaDevices && navigator.mediaDevices.getUserMedia;
            const hasLegacyGetUserMedia = navigator.getUserMedia ||
                navigator.webkitGetUserMedia ||
                navigator.mozGetUserMedia;

            if (!hasMediaDevices && !hasLegacyGetUserMedia) {
                return {
                    level: SupportLevel.UNSUPPORTED,
                    message: 'Media capture not supported',
                    limitations: ['Cannot access camera/microphone'],
                    fallback: null
                };
            }

            if (hasMediaDevices) {
                return {
                    level: SupportLevel.FULL,
                    message: 'Media API fully supported',
                    limitations: ['Requires user permission'],
                    fallback: null
                };
            }

            return {
                level: SupportLevel.FALLBACK,
                message: 'Using legacy getUserMedia API',
                limitations: ['May have compatibility issues'],
                fallback: 'legacyGetUserMedia'
            };
        }

        /**
         * Assess Storage API support level
         * @private
         */
        _assessStorageSupport() {
            try {
                const test = '__storage_test__';
                localStorage.setItem(test, test);
                localStorage.removeItem(test);

                return {
                    level: SupportLevel.FULL,
                    message: 'Local storage fully supported',
                    limitations: [],
                    fallback: null
                };
            } catch (e) {
                return {
                    level: SupportLevel.FALLBACK,
                    message: 'Local storage not available. Using in-memory storage.',
                    limitations: ['Data lost on page reload', 'Limited storage capacity'],
                    fallback: 'memoryStorage'
                };
            }
        }

        /**
         * Assess Service Worker support level
         * @private
         */
        _assessServiceWorkerSupport() {
            if (!('serviceWorker' in navigator)) {
                return {
                    level: SupportLevel.UNSUPPORTED,
                    message: 'Service Workers not supported. Offline functionality unavailable.',
                    limitations: ['No offline support', 'No background sync', 'No push notifications'],
                    fallback: null
                };
            }

            // Check for secure context (required for service workers)
            if (!window.isSecureContext && location.protocol !== 'https:') {
                return {
                    level: SupportLevel.UNSUPPORTED,
                    message: 'Service Workers require HTTPS',
                    limitations: ['No offline support', 'No background sync'],
                    fallback: null
                };
            }

            return {
                level: SupportLevel.FULL,
                message: 'Service Workers fully supported',
                limitations: [],
                fallback: null
            };
        }

        /**
         * Assess Notifications API support level
         * @private
         */
        _assessNotificationsSupport() {
            if (!('Notification' in window)) {
                return {
                    level: SupportLevel.FALLBACK,
                    message: 'Notifications not supported. Using in-app notifications.',
                    limitations: ['No system notifications', 'Notifications only visible when app is open'],
                    fallback: 'inAppNotifications'
                };
            }

            return {
                level: SupportLevel.FULL,
                message: 'Notifications API supported',
                limitations: ['Requires user permission'],
                fallback: null
            };
        }

        /**
         * Assess Fullscreen API support level
         * @private
         */
        _assessFullscreenSupport() {
            const hasFullscreen = document.fullscreenEnabled ||
                document.webkitFullscreenEnabled ||
                document.mozFullScreenEnabled ||
                document.msFullscreenEnabled;

            if (!hasFullscreen) {
                return {
                    level: SupportLevel.DEGRADED,
                    message: 'Fullscreen not supported. Using maximized viewport.',
                    limitations: ['Cannot enter true fullscreen mode'],
                    fallback: 'maximizedViewport'
                };
            }

            return {
                level: SupportLevel.FULL,
                message: 'Fullscreen API supported',
                limitations: ['Requires user gesture'],
                fallback: null
            };
        }

        /**
         * Register fallback handlers for unsupported features
         * @private
         */
        _registerFallbackHandlers() {
            // WebSocket fallback for WebRTC
            this.fallbackHandlers.set('websocket', {
                name: 'WebSocket Communication',
                description: 'Use WebSocket for data transfer when WebRTC is unavailable',
                handler: this._createWebSocketFallback.bind(this)
            });

            // execCommand fallback for Clipboard
            this.fallbackHandlers.set('execCommand', {
                name: 'execCommand Clipboard',
                description: 'Use document.execCommand for clipboard operations',
                handler: this._createExecCommandFallback.bind(this)
            });

            // File input fallback for drag and drop
            this.fallbackHandlers.set('fileInput', {
                name: 'File Input',
                description: 'Use file input element for file selection',
                handler: this._createFileInputFallback.bind(this)
            });

            // Memory storage fallback
            this.fallbackHandlers.set('memoryStorage', {
                name: 'In-Memory Storage',
                description: 'Use in-memory storage when localStorage is unavailable',
                handler: this._createMemoryStorageFallback.bind(this)
            });

            // In-app notifications fallback
            this.fallbackHandlers.set('inAppNotifications', {
                name: 'In-App Notifications',
                description: 'Show notifications within the application UI',
                handler: this._createInAppNotificationsFallback.bind(this)
            });

            // Maximized viewport fallback for fullscreen
            this.fallbackHandlers.set('maximizedViewport', {
                name: 'Maximized Viewport',
                description: 'Maximize viewport when fullscreen is unavailable',
                handler: this._createMaximizedViewportFallback.bind(this)
            });
        }

        /**
         * Create WebSocket fallback handler
         * @private
         */
        _createWebSocketFallback() {
            return {
                connect: (url) => {
                    return new WebSocket(url);
                },
                send: (ws, data) => {
                    if (ws.readyState === WebSocket.OPEN) {
                        ws.send(JSON.stringify(data));
                    }
                },
                onMessage: (ws, callback) => {
                    ws.onmessage = (event) => {
                        try {
                            const data = JSON.parse(event.data);
                            callback(data);
                        } catch (error) {
                            console.error('Failed to parse WebSocket message:', error);
                        }
                    };
                }
            };
        }

        /**
         * Create execCommand fallback handler
         * @private
         */
        _createExecCommandFallback() {
            return {
                copy: (text) => {
                    const textarea = document.createElement('textarea');
                    textarea.value = text;
                    textarea.style.position = 'fixed';
                    textarea.style.opacity = '0';
                    document.body.appendChild(textarea);
                    textarea.select();
                    const success = document.execCommand('copy');
                    document.body.removeChild(textarea);
                    return success;
                },
                paste: () => {
                    // Note: execCommand paste requires user interaction and may not work
                    return new Promise((resolve, reject) => {
                        reject(new Error('Paste requires user interaction'));
                    });
                }
            };
        }

        /**
         * Create file input fallback handler
         * @private
         */
        _createFileInputFallback() {
            return {
                createFileInput: (options = {}) => {
                    const input = document.createElement('input');
                    input.type = 'file';
                    input.multiple = options.multiple || false;
                    input.accept = options.accept || '*/*';
                    return input;
                },
                selectFiles: (options = {}) => {
                    return new Promise((resolve) => {
                        const input = this._createFileInputFallback().createFileInput(options);
                        input.onchange = (event) => {
                            resolve(Array.from(event.target.files));
                        };
                        input.click();
                    });
                }
            };
        }

        /**
         * Create memory storage fallback handler
         * @private
         */
        _createMemoryStorageFallback() {
            const storage = new Map();
            return {
                getItem: (key) => storage.get(key) || null,
                setItem: (key, value) => storage.set(key, String(value)),
                removeItem: (key) => storage.delete(key),
                clear: () => storage.clear(),
                get length() { return storage.size; },
                key: (index) => Array.from(storage.keys())[index] || null
            };
        }

        /**
         * Create in-app notifications fallback handler
         * @private
         */
        _createInAppNotificationsFallback() {
            return {
                show: (title, options = {}) => {
                    const notification = {
                        id: Date.now(),
                        title,
                        body: options.body || '',
                        icon: options.icon || null,
                        timestamp: Date.now()
                    };

                    // Dispatch custom event for in-app notification
                    const event = new CustomEvent('kizuna-notification', {
                        detail: notification
                    });
                    window.dispatchEvent(event);

                    return notification;
                }
            };
        }

        /**
         * Create maximized viewport fallback handler
         * @private
         */
        _createMaximizedViewportFallback() {
            return {
                maximize: (element) => {
                    element.style.position = 'fixed';
                    element.style.top = '0';
                    element.style.left = '0';
                    element.style.width = '100vw';
                    element.style.height = '100vh';
                    element.style.zIndex = '9999';
                },
                restore: (element, originalStyles) => {
                    Object.assign(element.style, originalStyles);
                }
            };
        }

        /**
         * Setup progressive enhancement
         * @private
         */
        _setupProgressiveEnhancement() {
            // Add CSS classes based on feature support
            const html = document.documentElement;

            this.featureSupport.forEach((support, feature) => {
                html.classList.add(`kizuna-${feature}-${support.level}`);
            });

            // Add general support level class
            const overallLevel = this._calculateOverallSupportLevel();
            html.classList.add(`kizuna-support-${overallLevel}`);
        }

        /**
         * Calculate overall support level
         * @private
         */
        _calculateOverallSupportLevel() {
            const levels = Array.from(this.featureSupport.values()).map(s => s.level);

            if (levels.every(l => l === SupportLevel.FULL)) {
                return SupportLevel.FULL;
            }

            if (levels.some(l => l === SupportLevel.UNSUPPORTED)) {
                return SupportLevel.DEGRADED;
            }

            if (levels.some(l => l === SupportLevel.FALLBACK || l === SupportLevel.DEGRADED)) {
                return SupportLevel.PARTIAL;
            }

            return SupportLevel.FULL;
        }

        /**
         * Get feature support information
         */
        getFeatureSupport(feature) {
            return this.featureSupport.get(feature);
        }

        /**
         * Get fallback handler for a feature
         */
        getFallbackHandler(fallbackType) {
            const handler = this.fallbackHandlers.get(fallbackType);
            return handler ? handler.handler() : null;
        }

        /**
         * Get user-facing messages about unsupported features
         */
        getUserNotifications() {
            const notifications = [];

            this.featureSupport.forEach((support, feature) => {
                if (support.level !== SupportLevel.FULL) {
                    notifications.push({
                        feature,
                        level: support.level,
                        message: support.message,
                        limitations: support.limitations,
                        severity: this._getSeverity(support.level)
                    });
                }
            });

            return notifications.sort((a, b) => {
                const severityOrder = { high: 0, medium: 1, low: 2 };
                return severityOrder[a.severity] - severityOrder[b.severity];
            });
        }

        /**
         * Get severity level for support level
         * @private
         */
        _getSeverity(level) {
            switch (level) {
                case SupportLevel.UNSUPPORTED:
                    return 'high';
                case SupportLevel.FALLBACK:
                case SupportLevel.DEGRADED:
                    return 'medium';
                case SupportLevel.PARTIAL:
                    return 'low';
                default:
                    return 'low';
            }
        }

        /**
         * Get comprehensive degradation report
         */
        getDegradationReport() {
            return {
                overallLevel: this._calculateOverallSupportLevel(),
                features: Object.fromEntries(this.featureSupport),
                notifications: this.getUserNotifications(),
                fallbacks: Array.from(this.fallbackHandlers.keys())
            };
        }

        /**
         * Check if feature is usable (either natively or through fallback)
         */
        isFeatureUsable(feature) {
            const support = this.featureSupport.get(feature);
            return support && support.level !== SupportLevel.UNSUPPORTED;
        }

        /**
         * Get recommended action for unsupported feature
         */
        getRecommendedAction(feature) {
            const support = this.featureSupport.get(feature);
            if (!support) {
                return null;
            }

            switch (support.level) {
                case SupportLevel.FULL:
                    return { action: 'use', message: 'Feature fully supported' };
                case SupportLevel.PARTIAL:
                    return { action: 'use-with-caution', message: support.message };
                case SupportLevel.FALLBACK:
                    return { action: 'use-fallback', message: support.message, fallback: support.fallback };
                case SupportLevel.DEGRADED:
                    return { action: 'use-degraded', message: support.message };
                case SupportLevel.UNSUPPORTED:
                    return { action: 'disable', message: support.message };
                default:
                    return null;
            }
        }
    }

    /**
     * Progressive Enhancement Helper
     */
    class ProgressiveEnhancement {
        constructor(degradation) {
            this.degradation = degradation;
        }

        /**
         * Enhance element based on feature support
         */
        enhance(element, feature, enhancementFn, fallbackFn) {
            const support = this.degradation.getFeatureSupport(feature);

            if (support && support.level === SupportLevel.FULL) {
                enhancementFn(element);
            } else if (fallbackFn) {
                fallbackFn(element, support);
            }
        }

        /**
         * Create feature-aware component
         */
        createComponent(feature, fullComponent, fallbackComponent) {
            const support = this.degradation.getFeatureSupport(feature);

            if (support && support.level === SupportLevel.FULL) {
                return fullComponent();
            } else {
                return fallbackComponent(support);
            }
        }

        /**
         * Add progressive enhancement CSS
         */
        addProgressiveCSS() {
            const style = document.createElement('style');
            style.textContent = `
                /* Hide enhanced features when not supported */
                .kizuna-webrtc-unsupported .webrtc-only { display: none !important; }
                .kizuna-clipboard-unsupported .clipboard-only { display: none !important; }
                .kizuna-serviceWorker-unsupported .pwa-only { display: none !important; }
                
                /* Show fallback UI when needed */
                .kizuna-webrtc-fallback .websocket-fallback { display: block !important; }
                .kizuna-clipboard-fallback .manual-clipboard { display: block !important; }
                
                /* Degraded mode styling */
                .kizuna-support-degraded .feature-warning {
                    display: block;
                    background: #fff3cd;
                    border: 1px solid #ffc107;
                    padding: 10px;
                    margin: 10px 0;
                    border-radius: 4px;
                }
            `;
            document.head.appendChild(style);
        }
    }

    /**
     * Initialize graceful degradation
     */
    const gracefulDegradation = new GracefulDegradation();
    const progressiveEnhancement = new ProgressiveEnhancement(gracefulDegradation);

    // Add progressive CSS
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
            progressiveEnhancement.addProgressiveCSS();
        });
    } else {
        progressiveEnhancement.addProgressiveCSS();
    }

    // Export to global scope
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {
            GracefulDegradation,
            ProgressiveEnhancement,
            SupportLevel,
            gracefulDegradation,
            progressiveEnhancement
        };
    } else {
        global.KizunaGracefulDegradation = {
            GracefulDegradation,
            ProgressiveEnhancement,
            SupportLevel,
            gracefulDegradation,
            progressiveEnhancement
        };
    }

    // Log degradation report in development
    if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
        console.log('Kizuna Graceful Degradation:', gracefulDegradation.getDegradationReport());
        console.log('User Notifications:', gracefulDegradation.getUserNotifications());
    }

})(typeof window !== 'undefined' ? window : this);
