/**
 * Kizuna API Versioning and Backward Compatibility
 * 
 * Provides API versioning system for JavaScript SDK with backward compatibility
 * layer for older browser clients. Implements feature negotiation between
 * browser and peer.
 * 
 * @version 1.0.0
 */

(function (global) {
    'use strict';

    /**
     * API Version Constants
     */
    const API_VERSIONS = {
        V1_0: '1.0',
        V1_1: '1.1',
        V2_0: '2.0',
        CURRENT: '1.0'
    };

    /**
     * API Version Manager
     */
    class APIVersionManager {
        constructor() {
            this.currentVersion = API_VERSIONS.CURRENT;
            this.supportedVersions = [API_VERSIONS.V1_0];
            this.versionAdapters = new Map();
            this.featureMatrix = new Map();
            this.init();
        }

        init() {
            this._registerVersionAdapters();
            this._buildFeatureMatrix();
        }

        /**
         * Register version adapters for backward compatibility
         * @private
         */
        _registerVersionAdapters() {
            // V1.0 adapter (current version, no adaptation needed)
            this.versionAdapters.set(API_VERSIONS.V1_0, {
                version: API_VERSIONS.V1_0,
                adapter: (api) => api, // No transformation needed
                features: [
                    'connection',
                    'fileTransfer',
                    'clipboard',
                    'command',
                    'video',
                    'webrtc',
                    'websocket'
                ]
            });

            // Future version adapters would be added here
            // Example: V1.1 with additional features
            // this.versionAdapters.set(API_VERSIONS.V1_1, {
            //     version: API_VERSIONS.V1_1,
            //     adapter: this._adaptV1_1ToV1_0.bind(this),
            //     features: [...v1_0_features, 'newFeature']
            // });
        }

        /**
         * Build feature availability matrix
         * @private
         */
        _buildFeatureMatrix() {
            // V1.0 features
            this.featureMatrix.set(API_VERSIONS.V1_0, {
                connection: {
                    available: true,
                    methods: ['connect', 'disconnect', 'getConnectionStatus'],
                    events: ['connected', 'disconnected', 'connectionStateChange']
                },
                fileTransfer: {
                    available: true,
                    methods: ['uploadFile', 'downloadFile', 'cancelTransfer'],
                    events: ['transferProgress', 'transferComplete', 'transferError']
                },
                clipboard: {
                    available: true,
                    methods: ['syncClipboard', 'getClipboard', 'setClipboard'],
                    events: ['clipboardSync', 'clipboardChange']
                },
                command: {
                    available: true,
                    methods: ['executeCommand', 'getCommandHistory'],
                    events: ['commandOutput', 'commandComplete', 'commandError']
                },
                video: {
                    available: true,
                    methods: ['startVideoStream', 'stopVideoStream'],
                    events: ['videoStreamStart', 'videoStreamStop', 'videoStreamError']
                },
                webrtc: {
                    available: true,
                    methods: ['createPeerConnection', 'createDataChannel'],
                    events: ['iceCandidate', 'dataChannelOpen', 'dataChannelClose']
                },
                websocket: {
                    available: true,
                    methods: ['connectWebSocket', 'sendWebSocketMessage'],
                    events: ['websocketOpen', 'websocketMessage', 'websocketClose']
                }
            });
        }

        /**
         * Get adapter for specific version
         */
        getAdapter(version) {
            return this.versionAdapters.get(version);
        }

        /**
         * Check if version is supported
         */
        isVersionSupported(version) {
            return this.supportedVersions.includes(version);
        }

        /**
         * Get features available in specific version
         */
        getVersionFeatures(version) {
            return this.featureMatrix.get(version);
        }

        /**
         * Negotiate API version with peer
         */
        negotiateVersion(peerSupportedVersions) {
            // Find highest mutually supported version
            const mutualVersions = this.supportedVersions.filter(v =>
                peerSupportedVersions.includes(v)
            );

            if (mutualVersions.length === 0) {
                throw new Error('No mutually supported API version');
            }

            // Sort versions and return highest
            return mutualVersions.sort((a, b) => {
                const aNum = parseFloat(a);
                const bNum = parseFloat(b);
                return bNum - aNum;
            })[0];
        }

        /**
         * Get version info
         */
        getVersionInfo() {
            return {
                current: this.currentVersion,
                supported: this.supportedVersions,
                features: this.featureMatrix.get(this.currentVersion)
            };
        }
    }

    /**
     * Backward Compatibility Layer
     */
    class BackwardCompatibility {
        constructor(versionManager) {
            this.versionManager = versionManager;
            this.deprecatedAPIs = new Map();
            this.migrationGuides = new Map();
            this.init();
        }

        init() {
            this._registerDeprecatedAPIs();
            this._registerMigrationGuides();
        }

        /**
         * Register deprecated APIs
         * @private
         */
        _registerDeprecatedAPIs() {
            // Example deprecated APIs (for future versions)
            // this.deprecatedAPIs.set('oldMethodName', {
            //     deprecated: '1.1',
            //     removed: '2.0',
            //     replacement: 'newMethodName',
            //     message: 'Use newMethodName instead'
            // });
        }

        /**
         * Register migration guides
         * @private
         */
        _registerMigrationGuides() {
            // V1.0 to V1.1 migration guide (example for future)
            // this.migrationGuides.set('1.0->1.1', {
            //     from: '1.0',
            //     to: '1.1',
            //     changes: [],
            //     guide: 'Migration guide content'
            // });
        }

        /**
         * Wrap API method with deprecation warning
         */
        wrapDeprecatedMethod(methodName, method, deprecationInfo) {
            return function (...args) {
                console.warn(
                    `[Kizuna API] Method '${methodName}' is deprecated since version ${deprecationInfo.deprecated}. ` +
                    `${deprecationInfo.message}`
                );
                return method.apply(this, args);
            };
        }

        /**
         * Create compatibility wrapper for older API version
         */
        createCompatibilityWrapper(targetVersion, api) {
            const adapter = this.versionManager.getAdapter(targetVersion);
            if (!adapter) {
                throw new Error(`No adapter found for version ${targetVersion}`);
            }

            return adapter.adapter(api);
        }

        /**
         * Check if method is deprecated
         */
        isDeprecated(methodName) {
            return this.deprecatedAPIs.has(methodName);
        }

        /**
         * Get deprecation info
         */
        getDeprecationInfo(methodName) {
            return this.deprecatedAPIs.get(methodName);
        }

        /**
         * Get migration guide
         */
        getMigrationGuide(fromVersion, toVersion) {
            const key = `${fromVersion}->${toVersion}`;
            return this.migrationGuides.get(key);
        }
    }

    /**
     * Feature Negotiation
     */
    class FeatureNegotiation {
        constructor(versionManager) {
            this.versionManager = versionManager;
            this.negotiatedFeatures = new Map();
        }

        /**
         * Negotiate features with peer
         */
        negotiateFeatures(peerCapabilities) {
            const localVersion = this.versionManager.currentVersion;
            const localFeatures = this.versionManager.getVersionFeatures(localVersion);

            const negotiatedVersion = this.versionManager.negotiateVersion(
                peerCapabilities.supportedVersions || [API_VERSIONS.V1_0]
            );

            const peerFeatures = peerCapabilities.features || {};
            const negotiated = {};

            // Negotiate each feature
            for (const [featureName, featureInfo] of Object.entries(localFeatures)) {
                const peerFeature = peerFeatures[featureName];

                if (peerFeature && peerFeature.available) {
                    // Feature supported by both sides
                    negotiated[featureName] = {
                        available: true,
                        methods: this._intersectArrays(
                            featureInfo.methods,
                            peerFeature.methods || []
                        ),
                        events: this._intersectArrays(
                            featureInfo.events,
                            peerFeature.events || []
                        )
                    };
                } else {
                    // Feature not supported by peer
                    negotiated[featureName] = {
                        available: false,
                        reason: 'Not supported by peer'
                    };
                }
            }

            this.negotiatedFeatures.set(peerCapabilities.peerId, {
                version: negotiatedVersion,
                features: negotiated
            });

            return {
                version: negotiatedVersion,
                features: negotiated
            };
        }

        /**
         * Get intersecting elements of two arrays
         * @private
         */
        _intersectArrays(arr1, arr2) {
            return arr1.filter(item => arr2.includes(item));
        }

        /**
         * Check if feature is available for peer
         */
        isFeatureAvailable(peerId, featureName) {
            const negotiated = this.negotiatedFeatures.get(peerId);
            if (!negotiated) {
                return false;
            }

            const feature = negotiated.features[featureName];
            return feature && feature.available;
        }

        /**
         * Get negotiated features for peer
         */
        getNegotiatedFeatures(peerId) {
            return this.negotiatedFeatures.get(peerId);
        }

        /**
         * Get local capabilities
         */
        getLocalCapabilities() {
            const version = this.versionManager.currentVersion;
            const features = this.versionManager.getVersionFeatures(version);

            return {
                supportedVersions: this.versionManager.supportedVersions,
                currentVersion: version,
                features: features
            };
        }
    }

    /**
     * Versioned API Wrapper
     */
    class VersionedAPI {
        constructor(baseAPI, versionManager, backwardCompat) {
            this.baseAPI = baseAPI;
            this.versionManager = versionManager;
            this.backwardCompat = backwardCompat;
            this.activeVersion = versionManager.currentVersion;
        }

        /**
         * Set active API version
         */
        setVersion(version) {
            if (!this.versionManager.isVersionSupported(version)) {
                throw new Error(`API version ${version} is not supported`);
            }
            this.activeVersion = version;
        }

        /**
         * Get API for specific version
         */
        getAPI(version = this.activeVersion) {
            if (version === this.versionManager.currentVersion) {
                return this.baseAPI;
            }

            return this.backwardCompat.createCompatibilityWrapper(version, this.baseAPI);
        }

        /**
         * Call versioned method
         */
        call(methodName, ...args) {
            const api = this.getAPI();

            // Check if method is deprecated
            if (this.backwardCompat.isDeprecated(methodName)) {
                const deprecationInfo = this.backwardCompat.getDeprecationInfo(methodName);
                console.warn(
                    `[Kizuna API] Method '${methodName}' is deprecated. ${deprecationInfo.message}`
                );
            }

            if (typeof api[methodName] !== 'function') {
                throw new Error(`Method '${methodName}' not found in API version ${this.activeVersion}`);
            }

            return api[methodName](...args);
        }

        /**
         * Get version info
         */
        getVersionInfo() {
            return {
                active: this.activeVersion,
                ...this.versionManager.getVersionInfo()
            };
        }
    }

    /**
     * API Version Detector
     */
    class APIVersionDetector {
        /**
         * Detect API version from peer handshake
         */
        static detectFromHandshake(handshake) {
            if (handshake.apiVersion) {
                return handshake.apiVersion;
            }

            // Try to infer version from capabilities
            if (handshake.capabilities) {
                return this._inferVersionFromCapabilities(handshake.capabilities);
            }

            // Default to oldest version for maximum compatibility
            return API_VERSIONS.V1_0;
        }

        /**
         * Infer API version from capabilities
         * @private
         */
        static _inferVersionFromCapabilities(capabilities) {
            // Logic to infer version based on available features
            // For now, return V1.0 as default
            return API_VERSIONS.V1_0;
        }

        /**
         * Validate version string
         */
        static isValidVersion(version) {
            return Object.values(API_VERSIONS).includes(version);
        }

        /**
         * Compare versions
         */
        static compareVersions(v1, v2) {
            const v1Num = parseFloat(v1);
            const v2Num = parseFloat(v2);

            if (v1Num > v2Num) return 1;
            if (v1Num < v2Num) return -1;
            return 0;
        }
    }

    /**
     * Initialize versioning system
     */
    const versionManager = new APIVersionManager();
    const backwardCompatibility = new BackwardCompatibility(versionManager);
    const featureNegotiation = new FeatureNegotiation(versionManager);

    /**
     * Create versioned API wrapper
     */
    function createVersionedAPI(baseAPI) {
        return new VersionedAPI(baseAPI, versionManager, backwardCompatibility);
    }

    /**
     * Negotiate API version and features with peer
     */
    function negotiateWithPeer(peerCapabilities) {
        return featureNegotiation.negotiateFeatures(peerCapabilities);
    }

    /**
     * Get local API capabilities
     */
    function getLocalCapabilities() {
        return featureNegotiation.getLocalCapabilities();
    }

    // Export to global scope
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {
            API_VERSIONS,
            APIVersionManager,
            BackwardCompatibility,
            FeatureNegotiation,
            VersionedAPI,
            APIVersionDetector,
            versionManager,
            backwardCompatibility,
            featureNegotiation,
            createVersionedAPI,
            negotiateWithPeer,
            getLocalCapabilities
        };
    } else {
        global.KizunaAPIVersioning = {
            API_VERSIONS,
            APIVersionManager,
            BackwardCompatibility,
            FeatureNegotiation,
            VersionedAPI,
            APIVersionDetector,
            versionManager,
            backwardCompatibility,
            featureNegotiation,
            createVersionedAPI,
            negotiateWithPeer,
            getLocalCapabilities
        };
    }

    // Log versioning info in development
    if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
        console.log('Kizuna API Versioning:', versionManager.getVersionInfo());
        console.log('Local Capabilities:', getLocalCapabilities());
    }

})(typeof window !== 'undefined' ? window : this);
