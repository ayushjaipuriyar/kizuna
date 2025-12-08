/**
 * Kizuna Browser SDK
 * 
 * JavaScript SDK for browser clients to interact with Kizuna peers.
 * Provides connection management, file transfer, clipboard sync, and command execution.
 * 
 * @version 1.0.0
 */

(function (global) {
    'use strict';

    /**
     * Main Kizuna SDK class
     */
    class KizunaSDK {
        constructor(config = {}) {
            this.config = {
                apiBaseUrl: config.apiBaseUrl || window.location.origin,
                autoReconnect: config.autoReconnect !== false,
                reconnectInterval: config.reconnectInterval || 5000,
                maxReconnectAttempts: config.maxReconnectAttempts || 5,
                debug: config.debug || false,
                apiVersion: config.apiVersion || '1.0',
                ...config
            };

            this.connection = null;
            this.protocol = null;
            this.sessionId = null;
            this.peerConnection = null;
            this.websocketConnection = null;
            this.dataChannels = {};
            this.eventListeners = {};
            this.reconnectAttempts = 0;
            this.connectionState = 'disconnected';
            this.capabilities = this._detectCapabilities();
            this.negotiatedFeatures = null;

            this._log('Kizuna SDK initialized', this.config);
        }

        /**
         * Detect browser capabilities
         * @private
         */
        _detectCapabilities() {
            return {
                supports_webrtc: !!(window.RTCPeerConnection || window.webkitRTCPeerConnection || window.mozRTCPeerConnection),
                supports_websocket: !!window.WebSocket,
                supports_clipboard_api: !!navigator.clipboard,
                supports_file_api: !!(window.File && window.FileReader && window.FileList && window.Blob),
                browser_type: this._getBrowserType(),
                browser_version: this._getBrowserVersion(),
                platform: navigator.platform,
                user_agent: navigator.userAgent
            };
        }

        /**
         * Get browser type
         * @private
         */
        _getBrowserType() {
            const userAgent = navigator.userAgent;
            if (userAgent.includes('Chrome') && !userAgent.includes('Edge')) return 'Chrome';
            if (userAgent.includes('Firefox')) return 'Firefox';
            if (userAgent.includes('Safari') && !userAgent.includes('Chrome')) return 'Safari';
            if (userAgent.includes('Edge')) return 'Edge';
            return 'Other';
        }

        /**
         * Get browser version
         * @private
         */
        _getBrowserVersion() {
            const userAgent = navigator.userAgent;
            const match = userAgent.match(/(Chrome|Firefox|Safari|Edge)\/([0-9.]+)/);
            return match ? match[2] : 'Unknown';
        }

        /**
         * Log debug messages
         * @private
         */
        _log(...args) {
            if (this.config.debug) {
                console.log('[Kizuna SDK]', ...args);
            }
        }

        /**
         * Log errors
         * @private
         */
        _error(...args) {
            console.error('[Kizuna SDK]', ...args);
        }

        /**
         * Emit an event to registered listeners
         * @private
         */
        _emit(eventName, data) {
            const listeners = this.eventListeners[eventName] || [];
            listeners.forEach(listener => {
                try {
                    listener(data);
                } catch (error) {
                    this._error('Error in event listener:', error);
                }
            });
        }

        /**
         * Register an event listener
         * @param {string} eventName - Event name
         * @param {Function} callback - Callback function
         */
        on(eventName, callback) {
            if (!this.eventListeners[eventName]) {
                this.eventListeners[eventName] = [];
            }
            this.eventListeners[eventName].push(callback);
            return this;
        }

        /**
         * Remove an event listener
         * @param {string} eventName - Event name
         * @param {Function} callback - Callback function to remove
         */
        off(eventName, callback) {
            if (!this.eventListeners[eventName]) return this;

            if (callback) {
                this.eventListeners[eventName] = this.eventListeners[eventName]
                    .filter(listener => listener !== callback);
            } else {
                delete this.eventListeners[eventName];
            }
            return this;
        }

        /**
         * Register a one-time event listener
         * @param {string} eventName - Event name
         * @param {Function} callback - Callback function
         */
        once(eventName, callback) {
            const onceWrapper = (data) => {
                callback(data);
                this.off(eventName, onceWrapper);
            };
            return this.on(eventName, onceWrapper);
        }

        /**
         * Connect to a Kizuna peer using setup ID
         * @param {string} setupId - Connection setup ID
         * @param {Object} options - Connection options
         * @returns {Promise<Object>} Connection info
         */
        async connect(setupId, options = {}) {
            try {
                this._log('Connecting with setup ID:', setupId);
                this.connectionState = 'connecting';
                this._emit('connectionStateChange', { state: 'connecting' });

                // Load connection setup
                const setupResponse = await fetch(`${this.config.apiBaseUrl}/api/setup/${setupId}`);
                if (!setupResponse.ok) {
                    throw new Error('Setup not found or expired');
                }
                const connectionInfo = await setupResponse.json();
                this._log('Connection info loaded:', connectionInfo);

                // Try WebRTC first if supported
                if (this.capabilities.supports_webrtc && !options.forceWebSocket) {
                    try {
                        await this._connectViaWebRTC(setupId, connectionInfo);
                        return { protocol: 'webrtc', sessionId: this.sessionId };
                    } catch (error) {
                        this._log('WebRTC connection failed, falling back to WebSocket:', error);
                        if (this.capabilities.supports_websocket) {
                            await this._connectViaWebSocket(setupId, connectionInfo);
                            return { protocol: 'websocket', sessionId: this.sessionId };
                        }
                        throw error;
                    }
                } else if (this.capabilities.supports_websocket) {
                    await this._connectViaWebSocket(setupId, connectionInfo);
                    return { protocol: 'websocket', sessionId: this.sessionId };
                } else {
                    throw new Error('No supported connection protocol available');
                }
            } catch (error) {
                this.connectionState = 'failed';
                this._emit('connectionStateChange', { state: 'failed', error });
                this._emit('error', { type: 'connection', error });
                throw error;
            }
        }

        /**
         * Connect via WebRTC
         * @private
         */
        async _connectViaWebRTC(setupId, connectionInfo) {
            this._log('Establishing WebRTC connection...');

            const iceServers = connectionInfo.ice_servers || [
                { urls: 'stun:stun.l.google.com:19302' },
                { urls: 'stun:stun1.l.google.com:19302' }
            ];

            this.peerConnection = new RTCPeerConnection({ iceServers });
            this.protocol = 'webrtc';

            // Set up connection state handlers
            this.peerConnection.oniceconnectionstatechange = () => {
                this._log('ICE connection state:', this.peerConnection.iceConnectionState);
                this._emit('iceConnectionStateChange', {
                    state: this.peerConnection.iceConnectionState
                });

                if (this.peerConnection.iceConnectionState === 'connected' ||
                    this.peerConnection.iceConnectionState === 'completed') {
                    this.connectionState = 'connected';
                    this._emit('connectionStateChange', { state: 'connected' });
                    this._emit('connected', { protocol: 'webrtc' });
                } else if (this.peerConnection.iceConnectionState === 'failed' ||
                    this.peerConnection.iceConnectionState === 'disconnected') {
                    this._handleConnectionFailure('WebRTC connection failed');
                }
            };

            this.peerConnection.onconnectionstatechange = () => {
                this._log('Connection state:', this.peerConnection.connectionState);
                if (this.peerConnection.connectionState === 'failed') {
                    this._handleConnectionFailure('WebRTC peer connection failed');
                }
            };

            // Handle incoming data channels
            this.peerConnection.ondatachannel = (event) => {
                this._log('Received data channel:', event.channel.label);
                this._setupDataChannel(event.channel);
            };

            // Create control data channel
            const controlChannel = this.peerConnection.createDataChannel('control');
            this._setupDataChannel(controlChannel);

            // Create offer
            const offer = await this.peerConnection.createOffer();
            await this.peerConnection.setLocalDescription(offer);

            // Send offer to server
            const response = await fetch(`${this.config.apiBaseUrl}/api/connect?setup_id=${setupId}`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    browser_info: this._getBrowserInfo(),
                    protocol: 'webrtc',
                    offer: offer.sdp
                })
            });

            if (!response.ok) {
                throw new Error('Failed to establish WebRTC connection');
            }

            const connectionData = await response.json();
            this.sessionId = connectionData.session_id;

            if (connectionData.answer) {
                await this.peerConnection.setRemoteDescription({
                    type: 'answer',
                    sdp: connectionData.answer
                });
            }

            this._log('WebRTC connection established');
        }

        /**
         * Connect via WebSocket
         * @private
         */
        async _connectViaWebSocket(setupId, connectionInfo) {
            return new Promise((resolve, reject) => {
                this._log('Establishing WebSocket connection...');

                const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
                const wsUrl = `${protocol}//${window.location.host}/api/websocket?setup_id=${setupId}`;

                this.websocketConnection = new WebSocket(wsUrl);
                this.protocol = 'websocket';

                const timeout = setTimeout(() => {
                    reject(new Error('WebSocket connection timeout'));
                    this.websocketConnection.close();
                }, 10000);

                this.websocketConnection.onopen = () => {
                    this._log('WebSocket opened, sending handshake...');

                    const handshake = {
                        message_type: 'WebSocketHandshake',
                        payload: {
                            browser_info: this._getBrowserInfo(),
                            protocol: 'websocket'
                        },
                        timestamp: Date.now()
                    };

                    this.websocketConnection.send(JSON.stringify(handshake));
                };

                this.websocketConnection.onmessage = (event) => {
                    const message = JSON.parse(event.data);

                    if (message.message_type === 'WebSocketHandshake' &&
                        message.payload.status === 'connected') {
                        clearTimeout(timeout);
                        this.sessionId = message.payload.session_id;
                        this.connectionState = 'connected';
                        this._emit('connectionStateChange', { state: 'connected' });
                        this._emit('connected', { protocol: 'websocket' });
                        this._log('WebSocket connection established');
                        resolve();
                    } else {
                        this._handleMessage(message);
                    }
                };

                this.websocketConnection.onerror = (error) => {
                    clearTimeout(timeout);
                    this._error('WebSocket error:', error);
                    reject(error);
                };

                this.websocketConnection.onclose = () => {
                    this._log('WebSocket closed');
                    if (this.connectionState === 'connected') {
                        this._handleConnectionFailure('WebSocket connection closed');
                    }
                };
            });
        }

        /**
         * Setup data channel handlers
         * @private
         */
        _setupDataChannel(channel) {
            const channelName = channel.label;
            this.dataChannels[channelName] = channel;

            channel.onopen = () => {
                this._log(`Data channel '${channelName}' opened`);
                this._emit('dataChannelOpen', { channel: channelName });
            };

            channel.onmessage = (event) => {
                try {
                    const message = JSON.parse(event.data);
                    this._handleMessage(message);
                } catch (error) {
                    this._error('Error parsing data channel message:', error);
                }
            };

            channel.onclose = () => {
                this._log(`Data channel '${channelName}' closed`);
                this._emit('dataChannelClose', { channel: channelName });
                delete this.dataChannels[channelName];
            };

            channel.onerror = (error) => {
                this._error(`Data channel '${channelName}' error:`, error);
                this._emit('dataChannelError', { channel: channelName, error });
            };
        }

        /**
         * Handle incoming messages
         * @private
         */
        _handleMessage(message) {
            this._log('Received message:', message);
            this._emit('message', message);
            this._emit(`message:${message.message_type}`, message);
        }

        /**
         * Handle connection failure
         * @private
         */
        _handleConnectionFailure(reason) {
            this._log('Connection failure:', reason);
            this.connectionState = 'disconnected';
            this._emit('connectionStateChange', { state: 'disconnected', reason });
            this._emit('disconnected', { reason });

            if (this.config.autoReconnect &&
                this.reconnectAttempts < this.config.maxReconnectAttempts) {
                this.reconnectAttempts++;
                this._log(`Attempting reconnection (${this.reconnectAttempts}/${this.config.maxReconnectAttempts})...`);

                setTimeout(() => {
                    this._emit('reconnecting', { attempt: this.reconnectAttempts });
                    // Reconnection logic would go here
                }, this.config.reconnectInterval);
            }
        }

        /**
         * Get browser info for connection
         * @private
         */
        _getBrowserInfo() {
            return {
                user_agent: this.capabilities.user_agent,
                browser_type: this.capabilities.browser_type,
                version: this.capabilities.browser_version,
                platform: this.capabilities.platform,
                supports_webrtc: this.capabilities.supports_webrtc,
                supports_clipboard_api: this.capabilities.supports_clipboard_api,
                api_version: this.config.apiVersion,
                supported_versions: ['1.0']
            };
        }

        /**
         * Negotiate features with peer
         * @param {Object} peerCapabilities - Peer's capabilities
         * @private
         */
        _negotiateFeatures(peerCapabilities) {
            // Use API versioning system if available
            if (window.KizunaAPIVersioning) {
                this.negotiatedFeatures = window.KizunaAPIVersioning.negotiateWithPeer(peerCapabilities);
                this._log('Features negotiated:', this.negotiatedFeatures);
            } else {
                // Fallback to basic negotiation
                this.negotiatedFeatures = {
                    version: this.config.apiVersion,
                    features: {
                        connection: { available: true },
                        fileTransfer: { available: true },
                        clipboard: { available: true },
                        command: { available: true },
                        video: { available: true }
                    }
                };
            }
            return this.negotiatedFeatures;
        }

        /**
         * Check if feature is available
         * @param {string} feature - Feature name
         * @returns {boolean}
         */
        isFeatureAvailable(feature) {
            if (!this.negotiatedFeatures) {
                return false;
            }
            const featureInfo = this.negotiatedFeatures.features[feature];
            return featureInfo && featureInfo.available;
        }

        /**
         * Get API version info
         * @returns {Object}
         */
        getAPIVersion() {
            if (window.KizunaAPIVersioning) {
                return window.KizunaAPIVersioning.versionManager.getVersionInfo();
            }
            return {
                current: this.config.apiVersion,
                supported: ['1.0']
            };
        }

        /**
         * Send a message through the connection
         * @param {Object} message - Message to send
         * @param {string} channel - Data channel name (for WebRTC)
         * @returns {Promise<void>}
         */
        async sendMessage(message, channel = 'control') {
            if (this.connectionState !== 'connected') {
                throw new Error('Not connected');
            }

            const messageData = {
                message_id: this._generateUUID(),
                session_id: this.sessionId,
                timestamp: Date.now(),
                ...message
            };

            if (this.protocol === 'webrtc') {
                const dataChannel = this.dataChannels[channel];
                if (!dataChannel || dataChannel.readyState !== 'open') {
                    throw new Error(`Data channel '${channel}' not available`);
                }
                dataChannel.send(JSON.stringify(messageData));
            } else if (this.protocol === 'websocket') {
                if (this.websocketConnection.readyState !== WebSocket.OPEN) {
                    throw new Error('WebSocket not open');
                }
                this.websocketConnection.send(JSON.stringify(messageData));
            }

            this._log('Message sent:', messageData);
        }

        /**
         * Disconnect from the peer
         * @returns {Promise<void>}
         */
        async disconnect() {
            this._log('Disconnecting...');

            if (this.peerConnection) {
                this.peerConnection.close();
                this.peerConnection = null;
            }

            if (this.websocketConnection) {
                this.websocketConnection.close();
                this.websocketConnection = null;
            }

            this.dataChannels = {};
            this.protocol = null;
            this.sessionId = null;
            this.connectionState = 'disconnected';
            this.reconnectAttempts = 0;

            this._emit('connectionStateChange', { state: 'disconnected' });
            this._emit('disconnected', { reason: 'manual' });
        }

        /**
         * Get current connection status
         * @returns {Object} Connection status
         */
        getConnectionStatus() {
            const status = {
                state: this.connectionState,
                protocol: this.protocol,
                sessionId: this.sessionId,
                capabilities: this.capabilities
            };

            if (this.protocol === 'webrtc' && this.peerConnection) {
                status.webrtc = {
                    connectionState: this.peerConnection.connectionState,
                    iceConnectionState: this.peerConnection.iceConnectionState,
                    signalingState: this.peerConnection.signalingState,
                    dataChannels: Object.keys(this.dataChannels)
                };
            } else if (this.protocol === 'websocket' && this.websocketConnection) {
                status.websocket = {
                    readyState: this.websocketConnection.readyState
                };
            }

            return status;
        }

        /**
         * Generate a UUID
         * @private
         */
        _generateUUID() {
            return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function (c) {
                const r = Math.random() * 16 | 0;
                const v = c === 'x' ? r : (r & 0x3 | 0x8);
                return v.toString(16);
            });
        }
    }

    // Export SDK
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = KizunaSDK;
    } else {
        global.KizunaSDK = KizunaSDK;
    }

})(typeof window !== 'undefined' ? window : this);
