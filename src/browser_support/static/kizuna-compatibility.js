/**
 * Kizuna Browser Compatibility Layer
 * 
 * Provides polyfills, shims, and compatibility wrappers for cross-browser support.
 * Ensures consistent API behavior across Chrome, Firefox, Safari, Edge, and mobile browsers.
 * 
 * @version 1.0.0
 */

(function (global) {
    'use strict';

    /**
     * Browser Compatibility Manager
     */
    class CompatibilityLayer {
        constructor() {
            this.browserInfo = this._detectBrowser();
            this.polyfills = {};
            this.init();
        }

        init() {
            this._applyWebRTCPolyfills();
            this._applyClipboardPolyfills();
            this._applyFileAPIPolyfills();
            this._applyMediaPolyfills();
            this._applyStoragePolyfills();
            this._applyPromisePolyfills();
        }

        /**
         * Detect browser type and version
         * @private
         */
        _detectBrowser() {
            const ua = navigator.userAgent;
            let name = 'Unknown';
            let version = 0;
            let engine = 'Unknown';

            // Chrome
            if (/Chrome/.test(ua) && !/Edge|Edg/.test(ua)) {
                name = 'Chrome';
                const match = ua.match(/Chrome\/(\d+)/);
                version = match ? parseInt(match[1]) : 0;
                engine = 'Blink';
            }
            // Safari
            else if (/Safari/.test(ua) && !/Chrome/.test(ua)) {
                name = 'Safari';
                const match = ua.match(/Version\/(\d+)/);
                version = match ? parseInt(match[1]) : 0;
                engine = 'WebKit';
            }
            // Firefox
            else if (/Firefox/.test(ua)) {
                name = 'Firefox';
                const match = ua.match(/Firefox\/(\d+)/);
                version = match ? parseInt(match[1]) : 0;
                engine = 'Gecko';
            }
            // Edge (Chromium)
            else if (/Edg/.test(ua)) {
                name = 'Edge';
                const match = ua.match(/Edg\/(\d+)/);
                version = match ? parseInt(match[1]) : 0;
                engine = 'Blink';
            }
            // Edge (Legacy)
            else if (/Edge/.test(ua)) {
                name = 'EdgeLegacy';
                const match = ua.match(/Edge\/(\d+)/);
                version = match ? parseInt(match[1]) : 0;
                engine = 'EdgeHTML';
            }

            return { name, version, engine, userAgent: ua };
        }

        /**
         * Apply WebRTC polyfills and compatibility shims
         * @private
         */
        _applyWebRTCPolyfills() {
            // Standardize RTCPeerConnection
            if (!window.RTCPeerConnection) {
                window.RTCPeerConnection =
                    window.webkitRTCPeerConnection ||
                    window.mozRTCPeerConnection;
            }

            // Standardize RTCSessionDescription
            if (!window.RTCSessionDescription) {
                window.RTCSessionDescription =
                    window.webkitRTCSessionDescription ||
                    window.mozRTCSessionDescription;
            }

            // Standardize RTCIceCandidate
            if (!window.RTCIceCandidate) {
                window.RTCIceCandidate =
                    window.webkitRTCIceCandidate ||
                    window.mozRTCIceCandidate;
            }

            // Safari-specific WebRTC fixes
            if (this.browserInfo.name === 'Safari') {
                this._applySafariWebRTCFixes();
            }

            // Firefox-specific WebRTC fixes
            if (this.browserInfo.name === 'Firefox') {
                this._applyFirefoxWebRTCFixes();
            }

            this.polyfills.webrtc = true;
        }

        /**
         * Apply Safari-specific WebRTC fixes
         * @private
         */
        _applySafariWebRTCFixes() {
            // Safari has issues with unified plan
            if (window.RTCPeerConnection) {
                const originalRTCPeerConnection = window.RTCPeerConnection;
                window.RTCPeerConnection = function (config) {
                    // Force unified plan for Safari 12.1+
                    if (config && !config.sdpSemantics) {
                        config.sdpSemantics = 'unified-plan';
                    }
                    return new originalRTCPeerConnection(config);
                };
                // Copy static methods
                Object.setPrototypeOf(window.RTCPeerConnection, originalRTCPeerConnection);
                window.RTCPeerConnection.prototype = originalRTCPeerConnection.prototype;
            }
        }

        /**
         * Apply Firefox-specific WebRTC fixes
         * @private
         */
        _applyFirefoxWebRTCFixes() {
            // Firefox uses different event names for some WebRTC events
            if (window.RTCPeerConnection && !window.RTCPeerConnection.prototype.ontrack) {
                // Older Firefox versions use onaddstream instead of ontrack
                const originalAddEventListener = window.RTCPeerConnection.prototype.addEventListener;
                window.RTCPeerConnection.prototype.addEventListener = function (type, listener, options) {
                    if (type === 'track' && !this.ontrack) {
                        type = 'addstream';
                    }
                    return originalAddEventListener.call(this, type, listener, options);
                };
            }
        }

        /**
         * Apply Clipboard API polyfills
         * @private
         */
        _applyClipboardPolyfills() {
            // Create clipboard API if it doesn't exist
            if (!navigator.clipboard) {
                navigator.clipboard = {
                    writeText: this._clipboardWriteTextPolyfill.bind(this),
                    readText: this._clipboardReadTextPolyfill.bind(this),
                    write: this._clipboardWritePolyfill.bind(this),
                    read: this._clipboardReadPolyfill.bind(this)
                };
                this.polyfills.clipboard = 'polyfill';
            } else {
                // Enhance existing clipboard API with fallbacks
                const originalWriteText = navigator.clipboard.writeText;
                const originalReadText = navigator.clipboard.readText;

                navigator.clipboard.writeText = async (text) => {
                    try {
                        return await originalWriteText.call(navigator.clipboard, text);
                    } catch (error) {
                        return this._clipboardWriteTextPolyfill(text);
                    }
                };

                navigator.clipboard.readText = async () => {
                    try {
                        return await originalReadText.call(navigator.clipboard);
                    } catch (error) {
                        return this._clipboardReadTextPolyfill();
                    }
                };

                this.polyfills.clipboard = 'enhanced';
            }
        }

        /**
         * Polyfill for clipboard writeText
         * @private
         */
        _clipboardWriteTextPolyfill(text) {
            return new Promise((resolve, reject) => {
                // Try execCommand as fallback
                const textArea = document.createElement('textarea');
                textArea.value = text;
                textArea.style.position = 'fixed';
                textArea.style.left = '-999999px';
                textArea.style.top = '-999999px';
                document.body.appendChild(textArea);
                textArea.focus();
                textArea.select();

                try {
                    const successful = document.execCommand('copy');
                    document.body.removeChild(textArea);
                    if (successful) {
                        resolve();
                    } else {
                        reject(new Error('execCommand copy failed'));
                    }
                } catch (error) {
                    document.body.removeChild(textArea);
                    reject(error);
                }
            });
        }

        /**
         * Polyfill for clipboard readText
         * @private
         */
        _clipboardReadTextPolyfill() {
            return new Promise((resolve, reject) => {
                // Try execCommand paste as fallback
                const textArea = document.createElement('textarea');
                textArea.style.position = 'fixed';
                textArea.style.left = '-999999px';
                textArea.style.top = '-999999px';
                document.body.appendChild(textArea);
                textArea.focus();

                try {
                    const successful = document.execCommand('paste');
                    const text = textArea.value;
                    document.body.removeChild(textArea);
                    if (successful) {
                        resolve(text);
                    } else {
                        reject(new Error('execCommand paste failed'));
                    }
                } catch (error) {
                    document.body.removeChild(textArea);
                    reject(error);
                }
            });
        }

        /**
         * Polyfill for clipboard write
         * @private
         */
        _clipboardWritePolyfill(data) {
            return Promise.reject(new Error('Clipboard write not supported'));
        }

        /**
         * Polyfill for clipboard read
         * @private
         */
        _clipboardReadPolyfill() {
            return Promise.reject(new Error('Clipboard read not supported'));
        }

        /**
         * Apply File API polyfills
         * @private
         */
        _applyFileAPIPolyfills() {
            // Ensure FileReader exists
            if (!window.FileReader) {
                console.warn('FileReader not supported - file operations will be limited');
                this.polyfills.fileReader = false;
                return;
            }

            // Add readAsBinaryString if missing (Safari)
            if (!FileReader.prototype.readAsBinaryString) {
                FileReader.prototype.readAsBinaryString = function (blob) {
                    const reader = new FileReader();
                    reader.onload = () => {
                        const bytes = new Uint8Array(reader.result);
                        let binary = '';
                        for (let i = 0; i < bytes.length; i++) {
                            binary += String.fromCharCode(bytes[i]);
                        }
                        const event = new Event('load');
                        event.target = { result: binary };
                        this.onload(event);
                    };
                    reader.onerror = (error) => {
                        if (this.onerror) this.onerror(error);
                    };
                    reader.readAsArrayBuffer(blob);
                };
            }

            this.polyfills.fileAPI = true;
        }

        /**
         * Apply Media API polyfills
         * @private
         */
        _applyMediaPolyfills() {
            // Standardize getUserMedia
            if (!navigator.mediaDevices) {
                navigator.mediaDevices = {};
            }

            if (!navigator.mediaDevices.getUserMedia) {
                navigator.mediaDevices.getUserMedia = this._getUserMediaPolyfill.bind(this);
                this.polyfills.getUserMedia = 'polyfill';
            } else {
                this.polyfills.getUserMedia = 'native';
            }

            // Standardize enumerateDevices
            if (!navigator.mediaDevices.enumerateDevices) {
                navigator.mediaDevices.enumerateDevices = () => {
                    return Promise.resolve([]);
                };
                this.polyfills.enumerateDevices = 'polyfill';
            } else {
                this.polyfills.enumerateDevices = 'native';
            }
        }

        /**
         * Polyfill for getUserMedia
         * @private
         */
        _getUserMediaPolyfill(constraints) {
            const getUserMedia =
                navigator.getUserMedia ||
                navigator.webkitGetUserMedia ||
                navigator.mozGetUserMedia ||
                navigator.msGetUserMedia;

            if (!getUserMedia) {
                return Promise.reject(new Error('getUserMedia not supported'));
            }

            return new Promise((resolve, reject) => {
                getUserMedia.call(navigator, constraints, resolve, reject);
            });
        }

        /**
         * Apply Storage API polyfills
         * @private
         */
        _applyStoragePolyfills() {
            // Test localStorage availability
            try {
                const test = '__storage_test__';
                localStorage.setItem(test, test);
                localStorage.removeItem(test);
                this.polyfills.localStorage = 'native';
            } catch (e) {
                // Create in-memory storage fallback
                this._createMemoryStorage();
                this.polyfills.localStorage = 'memory';
            }

            // Test sessionStorage availability
            try {
                const test = '__storage_test__';
                sessionStorage.setItem(test, test);
                sessionStorage.removeItem(test);
                this.polyfills.sessionStorage = 'native';
            } catch (e) {
                // sessionStorage uses same fallback as localStorage
                this.polyfills.sessionStorage = 'memory';
            }
        }

        /**
         * Create in-memory storage fallback
         * @private
         */
        _createMemoryStorage() {
            const memoryStorage = new Map();

            const storageInterface = {
                getItem: (key) => memoryStorage.get(key) || null,
                setItem: (key, value) => memoryStorage.set(key, String(value)),
                removeItem: (key) => memoryStorage.delete(key),
                clear: () => memoryStorage.clear(),
                get length() { return memoryStorage.size; },
                key: (index) => {
                    const keys = Array.from(memoryStorage.keys());
                    return keys[index] || null;
                }
            };

            if (!window.localStorage || typeof window.localStorage.getItem !== 'function') {
                window.localStorage = storageInterface;
            }
            if (!window.sessionStorage || typeof window.sessionStorage.getItem !== 'function') {
                window.sessionStorage = storageInterface;
            }
        }

        /**
         * Apply Promise polyfills
         * @private
         */
        _applyPromisePolyfills() {
            // Add Promise.allSettled if missing
            if (!Promise.allSettled) {
                Promise.allSettled = function (promises) {
                    return Promise.all(
                        promises.map(promise =>
                            Promise.resolve(promise)
                                .then(value => ({ status: 'fulfilled', value }))
                                .catch(reason => ({ status: 'rejected', reason }))
                        )
                    );
                };
                this.polyfills.promiseAllSettled = true;
            }

            // Add Promise.any if missing
            if (!Promise.any) {
                Promise.any = function (promises) {
                    return new Promise((resolve, reject) => {
                        let errors = [];
                        let rejectedCount = 0;

                        promises.forEach((promise, index) => {
                            Promise.resolve(promise)
                                .then(resolve)
                                .catch(error => {
                                    errors[index] = error;
                                    rejectedCount++;
                                    if (rejectedCount === promises.length) {
                                        reject(new AggregateError(errors, 'All promises rejected'));
                                    }
                                });
                        });
                    });
                };
                this.polyfills.promiseAny = true;
            }
        }

        /**
         * Get browser-specific optimizations
         */
        getBrowserOptimizations() {
            const optimizations = {
                browser: this.browserInfo.name,
                version: this.browserInfo.version,
                recommendations: []
            };

            // Chrome optimizations
            if (this.browserInfo.name === 'Chrome') {
                optimizations.recommendations.push({
                    feature: 'WebRTC',
                    optimization: 'Use unified-plan SDP semantics',
                    reason: 'Better performance and standards compliance'
                });
            }

            // Safari optimizations
            if (this.browserInfo.name === 'Safari') {
                optimizations.recommendations.push({
                    feature: 'WebRTC',
                    optimization: 'Explicitly set unified-plan',
                    reason: 'Safari requires explicit SDP semantics'
                });
                optimizations.recommendations.push({
                    feature: 'Clipboard',
                    optimization: 'Use execCommand fallback',
                    reason: 'Safari has strict clipboard API restrictions'
                });
            }

            // Firefox optimizations
            if (this.browserInfo.name === 'Firefox') {
                optimizations.recommendations.push({
                    feature: 'WebRTC',
                    optimization: 'Handle addstream events',
                    reason: 'Older Firefox versions use different event names'
                });
            }

            // Mobile browser optimizations
            if (/Mobile|Android|iPhone|iPad/.test(this.browserInfo.userAgent)) {
                optimizations.recommendations.push({
                    feature: 'Performance',
                    optimization: 'Reduce resource usage',
                    reason: 'Mobile devices have limited resources'
                });
                optimizations.recommendations.push({
                    feature: 'UI',
                    optimization: 'Use touch-optimized interfaces',
                    reason: 'Mobile devices use touch input'
                });
            }

            return optimizations;
        }

        /**
         * Get compatibility report
         */
        getCompatibilityReport() {
            return {
                browser: this.browserInfo,
                polyfills: this.polyfills,
                optimizations: this.getBrowserOptimizations()
            };
        }

        /**
         * Check if a feature needs polyfill
         */
        needsPolyfill(feature) {
            return this.polyfills[feature] === 'polyfill' || this.polyfills[feature] === 'memory';
        }

        /**
         * Get polyfill status
         */
        getPolyfillStatus(feature) {
            return this.polyfills[feature] || 'unknown';
        }
    }

    /**
     * Browser-specific workarounds
     */
    class BrowserWorkarounds {
        constructor(compatLayer) {
            this.compatLayer = compatLayer;
        }

        /**
         * Apply Safari-specific workarounds
         */
        applySafariWorkarounds() {
            // Safari requires user interaction for certain APIs
            return {
                clipboardRequiresGesture: true,
                autoplayRequiresGesture: true,
                fullscreenRequiresGesture: true,
                recommendations: [
                    'Trigger clipboard operations from user events',
                    'Use muted autoplay or require user interaction',
                    'Request fullscreen from user gesture handlers'
                ]
            };
        }

        /**
         * Apply Firefox-specific workarounds
         */
        applyFirefoxWorkarounds() {
            return {
                webrtcEventDifferences: true,
                recommendations: [
                    'Handle both track and addstream events',
                    'Test WebRTC functionality thoroughly'
                ]
            };
        }

        /**
         * Apply mobile browser workarounds
         */
        applyMobileWorkarounds() {
            return {
                limitedClipboardAccess: true,
                limitedBackgroundProcessing: true,
                limitedStorageQuota: true,
                recommendations: [
                    'Use manual copy/paste prompts',
                    'Minimize background operations',
                    'Monitor storage quota usage',
                    'Optimize for touch interactions'
                ]
            };
        }

        /**
         * Get all applicable workarounds
         */
        getWorkarounds() {
            const workarounds = [];
            const browser = this.compatLayer.browserInfo.name;
            const userAgent = this.compatLayer.browserInfo.userAgent;

            if (browser === 'Safari') {
                workarounds.push({
                    browser: 'Safari',
                    ...this.applySafariWorkarounds()
                });
            }

            if (browser === 'Firefox') {
                workarounds.push({
                    browser: 'Firefox',
                    ...this.applyFirefoxWorkarounds()
                });
            }

            if (/Mobile|Android|iPhone|iPad/.test(userAgent)) {
                workarounds.push({
                    browser: 'Mobile',
                    ...this.applyMobileWorkarounds()
                });
            }

            return workarounds;
        }
    }

    /**
     * Initialize compatibility layer
     */
    const compatibilityLayer = new CompatibilityLayer();
    const browserWorkarounds = new BrowserWorkarounds(compatibilityLayer);

    // Export to global scope
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {
            CompatibilityLayer,
            BrowserWorkarounds,
            compatibilityLayer,
            browserWorkarounds
        };
    } else {
        global.KizunaCompatibility = {
            CompatibilityLayer,
            BrowserWorkarounds,
            compatibilityLayer,
            browserWorkarounds
        };
    }

    // Log compatibility info in development
    if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
        console.log('Kizuna Compatibility Layer:', compatibilityLayer.getCompatibilityReport());
        console.log('Browser Workarounds:', browserWorkarounds.getWorkarounds());
    }

})(typeof window !== 'undefined' ? window : this);
