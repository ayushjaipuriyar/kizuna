/**
 * Kizuna Feature Detection
 * 
 * Comprehensive feature detection for mobile browsers with fallbacks
 * and optimization handling for unsupported features.
 * 
 * @version 1.0.0
 */

(function (global) {
    'use strict';

    /**
     * Mobile Browser Feature Detector
     */
    class FeatureDetector {
        constructor() {
            this.features = {};
            this.deviceInfo = {};
            this.browserInfo = {};
            this.init();
        }

        init() {
            this.detectDevice();
            this.detectBrowser();
            this.detectFeatures();
        }

        /**
         * Detect device type and capabilities
         */
        detectDevice() {
            const ua = navigator.userAgent;

            this.deviceInfo = {
                isMobile: this.isMobileDevice(),
                isTablet: this.isTabletDevice(),
                isDesktop: !this.isMobileDevice() && !this.isTabletDevice(),
                isIOS: /iPad|iPhone|iPod/.test(ua) && !window.MSStream,
                isAndroid: /Android/.test(ua),
                isWindows: /Windows/.test(ua),
                isMac: /Macintosh/.test(ua),
                hasTouch: this.hasTouchSupport(),
                screenSize: {
                    width: window.screen.width,
                    height: window.screen.height,
                    availWidth: window.screen.availWidth,
                    availHeight: window.screen.availHeight,
                    pixelRatio: window.devicePixelRatio || 1
                },
                orientation: this.getOrientation(),
                isStandalone: this.isStandaloneMode()
            };
        }

        /**
         * Detect browser type and version
         */
        detectBrowser() {
            const ua = navigator.userAgent;
            let browserName = 'Unknown';
            let browserVersion = 'Unknown';

            // Chrome
            if (/Chrome/.test(ua) && !/Edge|Edg/.test(ua)) {
                browserName = 'Chrome';
                const match = ua.match(/Chrome\/(\d+)/);
                browserVersion = match ? match[1] : 'Unknown';
            }
            // Safari
            else if (/Safari/.test(ua) && !/Chrome/.test(ua)) {
                browserName = 'Safari';
                const match = ua.match(/Version\/(\d+)/);
                browserVersion = match ? match[1] : 'Unknown';
            }
            // Firefox
            else if (/Firefox/.test(ua)) {
                browserName = 'Firefox';
                const match = ua.match(/Firefox\/(\d+)/);
                browserVersion = match ? match[1] : 'Unknown';
            }
            // Edge
            else if (/Edg/.test(ua)) {
                browserName = 'Edge';
                const match = ua.match(/Edg\/(\d+)/);
                browserVersion = match ? match[1] : 'Unknown';
            }
            // Samsung Internet
            else if (/SamsungBrowser/.test(ua)) {
                browserName = 'Samsung Internet';
                const match = ua.match(/SamsungBrowser\/(\d+)/);
                browserVersion = match ? match[1] : 'Unknown';
            }

            this.browserInfo = {
                name: browserName,
                version: browserVersion,
                userAgent: ua,
                language: navigator.language || navigator.userLanguage,
                cookieEnabled: navigator.cookieEnabled,
                onLine: navigator.onLine
            };
        }

        /**
         * Detect all relevant features
         */
        detectFeatures() {
            this.features = {
                // Core web APIs
                webrtc: this.detectWebRTC(),
                websocket: this.detectWebSocket(),
                serviceWorker: this.detectServiceWorker(),

                // Media APIs
                mediaDevices: this.detectMediaDevices(),
                getUserMedia: this.detectGetUserMedia(),
                mediaRecorder: this.detectMediaRecorder(),

                // Storage APIs
                localStorage: this.detectLocalStorage(),
                sessionStorage: this.detectSessionStorage(),
                indexedDB: this.detectIndexedDB(),

                // Clipboard API
                clipboard: this.detectClipboardAPI(),
                clipboardRead: this.detectClipboardRead(),
                clipboardWrite: this.detectClipboardWrite(),

                // File APIs
                fileAPI: this.detectFileAPI(),
                fileReader: this.detectFileReader(),
                dragAndDrop: this.detectDragAndDrop(),

                // Network APIs
                fetch: this.detectFetch(),
                xhr: this.detectXHR(),
                beacon: this.detectBeacon(),

                // UI/UX features
                fullscreen: this.detectFullscreen(),
                notifications: this.detectNotifications(),
                vibration: this.detectVibration(),

                // Performance features
                webWorker: this.detectWebWorker(),
                webAssembly: this.detectWebAssembly(),

                // Mobile-specific
                touchEvents: this.detectTouchEvents(),
                pointerEvents: this.detectPointerEvents(),
                orientationAPI: this.detectOrientationAPI(),

                // Security
                secureContext: this.isSecureContext(),
                permissions: this.detectPermissionsAPI()
            };
        }

        /**
         * Feature detection methods
         */

        detectWebRTC() {
            return !!(
                window.RTCPeerConnection ||
                window.webkitRTCPeerConnection ||
                window.mozRTCPeerConnection
            );
        }

        detectWebSocket() {
            return 'WebSocket' in window;
        }

        detectServiceWorker() {
            return 'serviceWorker' in navigator;
        }

        detectMediaDevices() {
            return !!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia);
        }

        detectGetUserMedia() {
            return !!(
                navigator.getUserMedia ||
                navigator.webkitGetUserMedia ||
                navigator.mozGetUserMedia ||
                navigator.msGetUserMedia
            );
        }

        detectMediaRecorder() {
            return 'MediaRecorder' in window;
        }

        detectLocalStorage() {
            try {
                const test = '__storage_test__';
                localStorage.setItem(test, test);
                localStorage.removeItem(test);
                return true;
            } catch (e) {
                return false;
            }
        }

        detectSessionStorage() {
            try {
                const test = '__storage_test__';
                sessionStorage.setItem(test, test);
                sessionStorage.removeItem(test);
                return true;
            } catch (e) {
                return false;
            }
        }

        detectIndexedDB() {
            return !!(window.indexedDB || window.mozIndexedDB || window.webkitIndexedDB);
        }

        detectClipboardAPI() {
            return !!(navigator.clipboard);
        }

        detectClipboardRead() {
            return !!(navigator.clipboard && navigator.clipboard.readText);
        }

        detectClipboardWrite() {
            return !!(navigator.clipboard && navigator.clipboard.writeText);
        }

        detectFileAPI() {
            return !!(window.File && window.FileReader && window.FileList && window.Blob);
        }

        detectFileReader() {
            return 'FileReader' in window;
        }

        detectDragAndDrop() {
            const div = document.createElement('div');
            return ('draggable' in div) || ('ondragstart' in div && 'ondrop' in div);
        }

        detectFetch() {
            return 'fetch' in window;
        }

        detectXHR() {
            return 'XMLHttpRequest' in window;
        }

        detectBeacon() {
            return 'sendBeacon' in navigator;
        }

        detectFullscreen() {
            return !!(
                document.fullscreenEnabled ||
                document.webkitFullscreenEnabled ||
                document.mozFullScreenEnabled ||
                document.msFullscreenEnabled
            );
        }

        detectNotifications() {
            return 'Notification' in window;
        }

        detectVibration() {
            return 'vibrate' in navigator;
        }

        detectWebWorker() {
            return 'Worker' in window;
        }

        detectWebAssembly() {
            return 'WebAssembly' in window;
        }

        detectTouchEvents() {
            return 'ontouchstart' in window || navigator.maxTouchPoints > 0;
        }

        detectPointerEvents() {
            return 'PointerEvent' in window;
        }

        detectOrientationAPI() {
            return 'orientation' in window || 'onorientationchange' in window;
        }

        isSecureContext() {
            return window.isSecureContext || location.protocol === 'https:';
        }

        detectPermissionsAPI() {
            return 'permissions' in navigator;
        }

        /**
         * Helper methods
         */

        isMobileDevice() {
            return /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
        }

        isTabletDevice() {
            const ua = navigator.userAgent;
            return /iPad|Android/i.test(ua) && !/Mobile/i.test(ua);
        }

        hasTouchSupport() {
            return 'ontouchstart' in window || navigator.maxTouchPoints > 0;
        }

        getOrientation() {
            if (window.screen.orientation) {
                return window.screen.orientation.type;
            }
            return window.innerWidth > window.innerHeight ? 'landscape' : 'portrait';
        }

        isStandaloneMode() {
            return window.matchMedia('(display-mode: standalone)').matches ||
                window.navigator.standalone === true;
        }

        /**
         * Get all detected information
         */
        getInfo() {
            return {
                device: this.deviceInfo,
                browser: this.browserInfo,
                features: this.features
            };
        }

        /**
         * Check if a specific feature is supported
         */
        supports(feature) {
            return this.features[feature] === true;
        }

        /**
         * Get feature support report
         */
        getFeatureReport() {
            const supported = [];
            const unsupported = [];

            for (const [feature, isSupported] of Object.entries(this.features)) {
                if (isSupported) {
                    supported.push(feature);
                } else {
                    unsupported.push(feature);
                }
            }

            return { supported, unsupported };
        }

        /**
         * Get mobile-specific limitations
         */
        getMobileLimitations() {
            const limitations = [];

            if (this.deviceInfo.isMobile) {
                if (!this.features.webrtc) {
                    limitations.push({
                        feature: 'WebRTC',
                        impact: 'high',
                        message: 'Real-time video streaming not available',
                        fallback: 'Use WebSocket for data transfer'
                    });
                }

                if (!this.features.clipboard) {
                    limitations.push({
                        feature: 'Clipboard API',
                        impact: 'medium',
                        message: 'Clipboard sync may require user interaction',
                        fallback: 'Use manual copy/paste prompts'
                    });
                }

                if (!this.features.serviceWorker) {
                    limitations.push({
                        feature: 'Service Worker',
                        impact: 'medium',
                        message: 'Offline functionality not available',
                        fallback: 'Require active internet connection'
                    });
                }

                if (!this.features.notifications) {
                    limitations.push({
                        feature: 'Notifications',
                        impact: 'low',
                        message: 'Push notifications not available',
                        fallback: 'Use in-app notifications only'
                    });
                }

                if (!this.features.fullscreen) {
                    limitations.push({
                        feature: 'Fullscreen',
                        impact: 'low',
                        message: 'Fullscreen mode not available',
                        fallback: 'Use maximized viewport'
                    });
                }
            }

            return limitations;
        }

        /**
         * Get recommended optimizations
         */
        getOptimizations() {
            const optimizations = [];

            if (this.deviceInfo.isMobile) {
                optimizations.push({
                    category: 'Performance',
                    recommendation: 'Use lazy loading for images and components',
                    reason: 'Mobile devices have limited bandwidth and processing power'
                });

                optimizations.push({
                    category: 'UI/UX',
                    recommendation: 'Increase touch target sizes to at least 48x48px',
                    reason: 'Improve touch accuracy on mobile devices'
                });

                if (this.deviceInfo.screenSize.pixelRatio > 2) {
                    optimizations.push({
                        category: 'Graphics',
                        recommendation: 'Serve high-DPI images for retina displays',
                        reason: 'Device has high pixel density'
                    });
                }

                if (!this.features.webrtc) {
                    optimizations.push({
                        category: 'Connectivity',
                        recommendation: 'Implement WebSocket fallback',
                        reason: 'WebRTC not supported on this browser'
                    });
                }
            }

            return optimizations;
        }
    }

    /**
     * Feature Fallback Manager
     */
    class FallbackManager {
        constructor(detector) {
            this.detector = detector;
            this.fallbacks = new Map();
            this.initFallbacks();
        }

        initFallbacks() {
            // WebRTC fallback
            if (!this.detector.supports('webrtc')) {
                this.fallbacks.set('webrtc', {
                    primary: 'WebSocket',
                    handler: this.useWebSocketFallback.bind(this)
                });
            }

            // Clipboard fallback
            if (!this.detector.supports('clipboard')) {
                this.fallbacks.set('clipboard', {
                    primary: 'execCommand',
                    handler: this.useExecCommandFallback.bind(this)
                });
            }

            // LocalStorage fallback
            if (!this.detector.supports('localStorage')) {
                this.fallbacks.set('localStorage', {
                    primary: 'Memory',
                    handler: this.useMemoryStorageFallback.bind(this)
                });
            }

            // Drag and drop fallback
            if (!this.detector.supports('dragAndDrop')) {
                this.fallbacks.set('dragAndDrop', {
                    primary: 'File Input',
                    handler: this.useFileInputFallback.bind(this)
                });
            }
        }

        useWebSocketFallback() {
            console.log('Using WebSocket fallback for WebRTC');
            return {
                type: 'websocket',
                available: this.detector.supports('websocket')
            };
        }

        useExecCommandFallback() {
            console.log('Using execCommand fallback for Clipboard API');
            return {
                type: 'execCommand',
                available: document.queryCommandSupported('copy')
            };
        }

        useMemoryStorageFallback() {
            console.log('Using in-memory storage fallback');
            const storage = new Map();
            return {
                type: 'memory',
                setItem: (key, value) => storage.set(key, value),
                getItem: (key) => storage.get(key),
                removeItem: (key) => storage.delete(key),
                clear: () => storage.clear()
            };
        }

        useFileInputFallback() {
            console.log('Using file input fallback for drag and drop');
            return {
                type: 'fileInput',
                available: true
            };
        }

        getFallback(feature) {
            return this.fallbacks.get(feature);
        }

        hasFallback(feature) {
            return this.fallbacks.has(feature);
        }
    }

    /**
     * Initialize and export
     */
    const detector = new FeatureDetector();
    const fallbackManager = new FallbackManager(detector);

    // Export to global scope
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {
            FeatureDetector,
            FallbackManager,
            detector,
            fallbackManager
        };
    } else {
        global.KizunaFeatureDetection = {
            FeatureDetector,
            FallbackManager,
            detector,
            fallbackManager
        };
    }

    // Log feature detection results in development
    if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
        console.log('Kizuna Feature Detection:', detector.getInfo());
        console.log('Feature Report:', detector.getFeatureReport());
        console.log('Mobile Limitations:', detector.getMobileLimitations());
        console.log('Recommended Optimizations:', detector.getOptimizations());
    }

})(typeof window !== 'undefined' ? window : this);
