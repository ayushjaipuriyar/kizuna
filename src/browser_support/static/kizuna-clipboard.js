/**
 * Kizuna Clipboard Synchronization API
 * 
 * Clipboard synchronization functionality for browser clients.
 * Supports text clipboard sharing with permission handling and automatic sync.
 */

(function (global) {
    'use strict';

    /**
     * Clipboard Manager
     */
    class ClipboardManager {
        constructor(sdk) {
            this.sdk = sdk;
            this.enabled = false;
            this.autoSync = false;
            this.lastClipboardContent = null;
            this.pollInterval = null;
            this.pollIntervalMs = 1000; // Check clipboard every second
            this.hasClipboardAPI = !!navigator.clipboard;
            this.hasPermission = false;

            // Listen for clipboard sync messages
            this.sdk.on('message:ClipboardSync', (msg) => this._handleClipboardMessage(msg));

            this.sdk._log('Clipboard Manager initialized', {
                hasClipboardAPI: this.hasClipboardAPI
            });
        }

        /**
         * Check if clipboard API is supported
         * @returns {boolean} True if supported
         */
        isSupported() {
            return this.hasClipboardAPI;
        }

        /**
         * Request clipboard permissions
         * @returns {Promise<boolean>} True if permission granted
         */
        async requestPermission() {
            if (!this.hasClipboardAPI) {
                throw new Error('Clipboard API not supported in this browser');
            }

            try {
                // Try to read clipboard to trigger permission prompt
                await navigator.clipboard.readText();
                this.hasPermission = true;
                this.sdk._log('Clipboard permission granted');
                this.sdk._emit('clipboardPermissionGranted');
                return true;
            } catch (error) {
                this.sdk._log('Clipboard permission denied:', error);
                this.hasPermission = false;
                this.sdk._emit('clipboardPermissionDenied', { error });
                return false;
            }
        }

        /**
         * Enable clipboard synchronization
         * @param {Object} options - Options
         * @returns {Promise<void>}
         */
        async enable(options = {}) {
            if (!this.hasClipboardAPI) {
                throw new Error('Clipboard API not supported');
            }

            if (!this.hasPermission) {
                const granted = await this.requestPermission();
                if (!granted) {
                    throw new Error('Clipboard permission required');
                }
            }

            this.enabled = true;
            this.autoSync = options.autoSync !== false;

            // Send enable message to peer
            await this.sdk.sendMessage({
                message_type: 'ClipboardSync',
                payload: {
                    action: 'enable',
                    auto_sync: this.autoSync
                }
            }, 'clipboard');

            // Start monitoring if auto-sync enabled
            if (this.autoSync) {
                this._startMonitoring();
            }

            this.sdk._log('Clipboard sync enabled', { autoSync: this.autoSync });
            this.sdk._emit('clipboardEnabled', { autoSync: this.autoSync });
        }

        /**
         * Disable clipboard synchronization
         * @returns {Promise<void>}
         */
        async disable() {
            this.enabled = false;
            this.autoSync = false;
            this._stopMonitoring();

            // Send disable message to peer
            await this.sdk.sendMessage({
                message_type: 'ClipboardSync',
                payload: {
                    action: 'disable'
                }
            }, 'clipboard');

            this.sdk._log('Clipboard sync disabled');
            this.sdk._emit('clipboardDisabled');
        }

        /**
         * Start monitoring clipboard changes
         * @private
         */
        _startMonitoring() {
            if (this.pollInterval) {
                return; // Already monitoring
            }

            this.sdk._log('Starting clipboard monitoring');

            this.pollInterval = setInterval(async () => {
                try {
                    await this._checkClipboardChange();
                } catch (error) {
                    this.sdk._error('Error checking clipboard:', error);
                }
            }, this.pollIntervalMs);
        }

        /**
         * Stop monitoring clipboard changes
         * @private
         */
        _stopMonitoring() {
            if (this.pollInterval) {
                clearInterval(this.pollInterval);
                this.pollInterval = null;
                this.sdk._log('Stopped clipboard monitoring');
            }
        }

        /**
         * Check for clipboard changes
         * @private
         */
        async _checkClipboardChange() {
            if (!this.enabled || !this.hasPermission) {
                return;
            }

            try {
                const text = await navigator.clipboard.readText();

                // Check if content changed
                if (text !== this.lastClipboardContent) {
                    this.lastClipboardContent = text;

                    if (text && text.length > 0) {
                        this.sdk._log('Clipboard changed, syncing...');
                        await this._syncToRemote(text);
                    }
                }
            } catch (error) {
                // Permission might have been revoked
                if (error.name === 'NotAllowedError') {
                    this.hasPermission = false;
                    this._stopMonitoring();
                    this.sdk._emit('clipboardPermissionRevoked');
                }
            }
        }

        /**
         * Sync clipboard content to remote peer
         * @private
         */
        async _syncToRemote(content) {
            try {
                await this.sdk.sendMessage({
                    message_type: 'ClipboardSync',
                    payload: {
                        action: 'sync',
                        content: content,
                        content_type: 'text',
                        timestamp: Date.now()
                    }
                }, 'clipboard');

                this.sdk._emit('clipboardSynced', {
                    direction: 'outgoing',
                    content: content
                });
            } catch (error) {
                this.sdk._error('Failed to sync clipboard:', error);
                this.sdk._emit('clipboardSyncError', {
                    direction: 'outgoing',
                    error
                });
            }
        }

        /**
         * Handle clipboard messages from peer
         * @private
         */
        async _handleClipboardMessage(message) {
            const payload = message.payload;

            switch (payload.action) {
                case 'sync':
                    await this._handleRemoteSync(payload);
                    break;

                case 'request':
                    await this._handleSyncRequest();
                    break;

                case 'enabled':
                    this.sdk._emit('remoteClipboardEnabled');
                    break;

                case 'disabled':
                    this.sdk._emit('remoteClipboardDisabled');
                    break;

                case 'error':
                    this.sdk._emit('clipboardSyncError', {
                        direction: 'incoming',
                        error: payload.error
                    });
                    break;
            }
        }

        /**
         * Handle remote clipboard sync
         * @private
         */
        async _handleRemoteSync(payload) {
            if (!this.enabled) {
                this.sdk._log('Clipboard sync disabled, ignoring remote sync');
                return;
            }

            try {
                const content = payload.content;

                // Update local clipboard
                await this._writeToClipboard(content);

                this.lastClipboardContent = content;

                this.sdk._log('Clipboard synced from remote');
                this.sdk._emit('clipboardSynced', {
                    direction: 'incoming',
                    content: content
                });
            } catch (error) {
                this.sdk._error('Failed to write to clipboard:', error);
                this.sdk._emit('clipboardSyncError', {
                    direction: 'incoming',
                    error
                });
            }
        }

        /**
         * Handle sync request from peer
         * @private
         */
        async _handleSyncRequest() {
            if (!this.enabled || !this.hasPermission) {
                return;
            }

            try {
                const text = await navigator.clipboard.readText();
                if (text) {
                    await this._syncToRemote(text);
                }
            } catch (error) {
                this.sdk._error('Failed to handle sync request:', error);
            }
        }

        /**
         * Write content to clipboard
         * @private
         */
        async _writeToClipboard(content) {
            if (!this.hasClipboardAPI) {
                // Fallback for browsers without Clipboard API
                this._fallbackCopyToClipboard(content);
                return;
            }

            try {
                await navigator.clipboard.writeText(content);
            } catch (error) {
                // Try fallback method
                this._fallbackCopyToClipboard(content);
            }
        }

        /**
         * Fallback method to copy to clipboard
         * @private
         */
        _fallbackCopyToClipboard(content) {
            const textarea = document.createElement('textarea');
            textarea.value = content;
            textarea.style.position = 'fixed';
            textarea.style.opacity = '0';
            document.body.appendChild(textarea);
            textarea.select();

            try {
                document.execCommand('copy');
                this.sdk._log('Copied to clipboard using fallback method');
            } catch (error) {
                this.sdk._error('Fallback copy failed:', error);
                throw error;
            } finally {
                document.body.removeChild(textarea);
            }
        }

        /**
         * Manually copy text to clipboard
         * @param {string} text - Text to copy
         * @returns {Promise<void>}
         */
        async copyToClipboard(text) {
            await this._writeToClipboard(text);
            this.lastClipboardContent = text;

            if (this.enabled && this.autoSync) {
                await this._syncToRemote(text);
            }
        }

        /**
         * Manually paste from clipboard
         * @returns {Promise<string>} Clipboard content
         */
        async pasteFromClipboard() {
            if (!this.hasClipboardAPI) {
                throw new Error('Clipboard API not supported');
            }

            if (!this.hasPermission) {
                const granted = await this.requestPermission();
                if (!granted) {
                    throw new Error('Clipboard permission required');
                }
            }

            const text = await navigator.clipboard.readText();
            return text;
        }

        /**
         * Request clipboard content from peer
         * @returns {Promise<void>}
         */
        async requestRemoteClipboard() {
            await this.sdk.sendMessage({
                message_type: 'ClipboardSync',
                payload: {
                    action: 'request'
                }
            }, 'clipboard');

            this.sdk._log('Requested remote clipboard content');
        }

        /**
         * Get current clipboard status
         * @returns {Object} Status object
         */
        getStatus() {
            return {
                enabled: this.enabled,
                autoSync: this.autoSync,
                hasClipboardAPI: this.hasClipboardAPI,
                hasPermission: this.hasPermission,
                monitoring: !!this.pollInterval
            };
        }

        /**
         * Set auto-sync enabled/disabled
         * @param {boolean} enabled - Enable auto-sync
         */
        setAutoSync(enabled) {
            this.autoSync = enabled;

            if (this.enabled) {
                if (enabled) {
                    this._startMonitoring();
                } else {
                    this._stopMonitoring();
                }
            }

            this.sdk._log('Auto-sync', enabled ? 'enabled' : 'disabled');
            this.sdk._emit('clipboardAutoSyncChanged', { enabled });
        }

        /**
         * Set polling interval for clipboard monitoring
         * @param {number} intervalMs - Interval in milliseconds
         */
        setPollingInterval(intervalMs) {
            this.pollIntervalMs = intervalMs;

            if (this.pollInterval) {
                this._stopMonitoring();
                this._startMonitoring();
            }

            this.sdk._log('Polling interval set to', intervalMs, 'ms');
        }

        /**
         * Clear clipboard
         * @returns {Promise<void>}
         */
        async clearClipboard() {
            await this._writeToClipboard('');
            this.lastClipboardContent = '';

            if (this.enabled && this.autoSync) {
                await this._syncToRemote('');
            }

            this.sdk._log('Clipboard cleared');
            this.sdk._emit('clipboardCleared');
        }

        /**
         * Get last synced content
         * @returns {string|null} Last clipboard content
         */
        getLastContent() {
            return this.lastClipboardContent;
        }

        /**
         * Cleanup
         */
        destroy() {
            this._stopMonitoring();
            this.enabled = false;
            this.autoSync = false;
            this.lastClipboardContent = null;
            this.sdk._log('Clipboard Manager destroyed');
        }
    }

    // Export
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = ClipboardManager;
    } else {
        global.KizunaClipboard = ClipboardManager;
    }

})(typeof window !== 'undefined' ? window : this);
