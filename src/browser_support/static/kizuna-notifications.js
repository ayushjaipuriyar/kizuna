/**
 * Kizuna Push Notifications Manager
 * Handles push notification permissions, subscriptions, and delivery
 */

class KizunaNotifications {
    constructor() {
        this.permission = Notification.permission;
        this.subscription = null;
        this.preferences = {
            fileTransfer: true,
            clipboardSync: true,
            commandExecution: true,
            peerConnection: true,
            systemAlerts: true,
        };
        this.listeners = {
            permissionchange: [],
            subscriptionchange: [],
            notificationclick: [],
        };

        this.init();
    }

    /**
     * Initialize notifications manager
     */
    async init() {
        // Check if notifications are supported
        if (!('Notification' in window)) {
            console.warn('[Notifications] Notifications not supported');
            return;
        }

        // Load preferences from storage
        await this.loadPreferences();

        // Get existing subscription if available
        if ('serviceWorker' in navigator) {
            try {
                const registration = await navigator.serviceWorker.ready;
                this.subscription = await registration.pushManager.getSubscription();

                if (this.subscription) {
                    console.log('[Notifications] Existing subscription found');
                }
            } catch (error) {
                console.error('[Notifications] Failed to get subscription:', error);
            }
        }

        // Listen for permission changes
        if ('permissions' in navigator) {
            try {
                const status = await navigator.permissions.query({ name: 'notifications' });
                status.addEventListener('change', () => {
                    this.permission = Notification.permission;
                    this.notifyListeners('permissionchange', this.permission);
                });
            } catch (error) {
                console.warn('[Notifications] Permission query not supported');
            }
        }
    }

    /**
     * Check if notifications are supported
     */
    isSupported() {
        return 'Notification' in window && 'serviceWorker' in navigator && 'PushManager' in window;
    }

    /**
     * Check if notifications are enabled
     */
    isEnabled() {
        return this.permission === 'granted' && this.subscription !== null;
    }

    /**
     * Request notification permission
     */
    async requestPermission() {
        if (!this.isSupported()) {
            throw new Error('Notifications not supported');
        }

        if (this.permission === 'granted') {
            console.log('[Notifications] Permission already granted');
            return 'granted';
        }

        try {
            const permission = await Notification.requestPermission();
            this.permission = permission;

            console.log('[Notifications] Permission:', permission);
            this.notifyListeners('permissionchange', permission);

            if (permission === 'granted') {
                // Subscribe to push notifications
                await this.subscribe();
            }

            return permission;
        } catch (error) {
            console.error('[Notifications] Permission request failed:', error);
            throw error;
        }
    }

    /**
     * Subscribe to push notifications
     */
    async subscribe() {
        if (!this.isSupported()) {
            throw new Error('Push notifications not supported');
        }

        if (this.permission !== 'granted') {
            throw new Error('Notification permission not granted');
        }

        try {
            const registration = await navigator.serviceWorker.ready;

            // Get VAPID public key from server
            const response = await fetch('/api/notifications/vapid-key');
            const { publicKey } = await response.json();

            // Subscribe to push notifications
            const subscription = await registration.pushManager.subscribe({
                userVisibleOnly: true,
                applicationServerKey: this.urlBase64ToUint8Array(publicKey),
            });

            this.subscription = subscription;
            console.log('[Notifications] Subscribed to push notifications');

            // Send subscription to server
            await this.sendSubscriptionToServer(subscription);

            this.notifyListeners('subscriptionchange', subscription);

            return subscription;
        } catch (error) {
            console.error('[Notifications] Subscription failed:', error);
            throw error;
        }
    }

    /**
     * Unsubscribe from push notifications
     */
    async unsubscribe() {
        if (!this.subscription) {
            console.log('[Notifications] No active subscription');
            return;
        }

        try {
            await this.subscription.unsubscribe();

            // Remove subscription from server
            await this.removeSubscriptionFromServer(this.subscription);

            this.subscription = null;
            console.log('[Notifications] Unsubscribed from push notifications');

            this.notifyListeners('subscriptionchange', null);
        } catch (error) {
            console.error('[Notifications] Unsubscribe failed:', error);
            throw error;
        }
    }

    /**
     * Send subscription to server
     */
    async sendSubscriptionToServer(subscription) {
        try {
            const response = await fetch('/api/notifications/subscribe', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    subscription: subscription.toJSON(),
                    preferences: this.preferences,
                }),
            });

            if (!response.ok) {
                throw new Error(`Server error: ${response.status}`);
            }

            console.log('[Notifications] Subscription sent to server');
        } catch (error) {
            console.error('[Notifications] Failed to send subscription:', error);
            throw error;
        }
    }

    /**
     * Remove subscription from server
     */
    async removeSubscriptionFromServer(subscription) {
        try {
            const response = await fetch('/api/notifications/unsubscribe', {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    endpoint: subscription.endpoint,
                }),
            });

            if (!response.ok) {
                throw new Error(`Server error: ${response.status}`);
            }

            console.log('[Notifications] Subscription removed from server');
        } catch (error) {
            console.error('[Notifications] Failed to remove subscription:', error);
            throw error;
        }
    }

    /**
     * Show local notification
     */
    async showNotification(title, options = {}) {
        if (!this.isSupported()) {
            console.warn('[Notifications] Notifications not supported');
            return null;
        }

        if (this.permission !== 'granted') {
            console.warn('[Notifications] Permission not granted');
            return null;
        }

        try {
            const registration = await navigator.serviceWorker.ready;

            const notificationOptions = {
                body: options.body || '',
                icon: options.icon || '/icons/icon-192.png',
                badge: options.badge || '/icons/badge-72.png',
                tag: options.tag || 'kizuna-notification',
                data: options.data || {},
                requireInteraction: options.requireInteraction || false,
                silent: options.silent || false,
                vibrate: options.vibrate || [200, 100, 200],
                actions: options.actions || [],
            };

            await registration.showNotification(title, notificationOptions);
            console.log('[Notifications] Notification shown:', title);

            return notificationOptions;
        } catch (error) {
            console.error('[Notifications] Failed to show notification:', error);
            throw error;
        }
    }

    /**
     * Update notification preferences
     */
    async updatePreferences(preferences) {
        this.preferences = { ...this.preferences, ...preferences };

        // Save to storage
        await this.savePreferences();

        // Update server if subscribed
        if (this.subscription) {
            await this.sendSubscriptionToServer(this.subscription);
        }

        console.log('[Notifications] Preferences updated:', this.preferences);
    }

    /**
     * Get notification preferences
     */
    getPreferences() {
        return { ...this.preferences };
    }

    /**
     * Save preferences to storage
     */
    async savePreferences() {
        if (window.KizunaPWA) {
            await window.KizunaPWA.saveSetting('notification-preferences', this.preferences);
        } else {
            localStorage.setItem('kizuna-notification-preferences', JSON.stringify(this.preferences));
        }
    }

    /**
     * Load preferences from storage
     */
    async loadPreferences() {
        try {
            let preferences;

            if (window.KizunaPWA) {
                preferences = await window.KizunaPWA.getSetting('notification-preferences');
            } else {
                const stored = localStorage.getItem('kizuna-notification-preferences');
                preferences = stored ? JSON.parse(stored) : null;
            }

            if (preferences) {
                this.preferences = { ...this.preferences, ...preferences };
                console.log('[Notifications] Preferences loaded:', this.preferences);
            }
        } catch (error) {
            console.error('[Notifications] Failed to load preferences:', error);
        }
    }

    /**
     * Create notification for file transfer
     */
    async notifyFileTransfer(fileName, status, options = {}) {
        if (!this.preferences.fileTransfer) {
            return;
        }

        const title = status === 'complete'
            ? `File Transfer Complete`
            : `File Transfer ${status}`;

        return this.showNotification(title, {
            body: fileName,
            icon: '/icons/file-transfer.png',
            tag: `file-transfer-${options.transferId || Date.now()}`,
            data: { type: 'file-transfer', fileName, status, ...options },
            actions: status === 'complete' ? [
                { action: 'open', title: 'Open' },
                { action: 'dismiss', title: 'Dismiss' },
            ] : [],
        });
    }

    /**
     * Create notification for clipboard sync
     */
    async notifyClipboardSync(content, options = {}) {
        if (!this.preferences.clipboardSync) {
            return;
        }

        const preview = content.length > 50 ? content.substring(0, 50) + '...' : content;

        return this.showNotification('Clipboard Synced', {
            body: preview,
            icon: '/icons/clipboard.png',
            tag: 'clipboard-sync',
            data: { type: 'clipboard-sync', content, ...options },
        });
    }

    /**
     * Create notification for command execution
     */
    async notifyCommandExecution(command, status, options = {}) {
        if (!this.preferences.commandExecution) {
            return;
        }

        const title = status === 'complete'
            ? 'Command Completed'
            : `Command ${status}`;

        return this.showNotification(title, {
            body: command,
            icon: '/icons/command.png',
            tag: `command-${options.commandId || Date.now()}`,
            data: { type: 'command-execution', command, status, ...options },
        });
    }

    /**
     * Create notification for peer connection
     */
    async notifyPeerConnection(peerName, status, options = {}) {
        if (!this.preferences.peerConnection) {
            return;
        }

        const title = status === 'connected'
            ? 'Peer Connected'
            : 'Peer Disconnected';

        return this.showNotification(title, {
            body: peerName,
            icon: '/icons/peer.png',
            tag: `peer-${options.peerId || Date.now()}`,
            data: { type: 'peer-connection', peerName, status, ...options },
        });
    }

    /**
     * Convert VAPID key to Uint8Array
     */
    urlBase64ToUint8Array(base64String) {
        const padding = '='.repeat((4 - base64String.length % 4) % 4);
        const base64 = (base64String + padding)
            .replace(/\-/g, '+')
            .replace(/_/g, '/');

        const rawData = window.atob(base64);
        const outputArray = new Uint8Array(rawData.length);

        for (let i = 0; i < rawData.length; ++i) {
            outputArray[i] = rawData.charCodeAt(i);
        }

        return outputArray;
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
     * Get notification status
     */
    getStatus() {
        return {
            supported: this.isSupported(),
            enabled: this.isEnabled(),
            permission: this.permission,
            subscribed: this.subscription !== null,
            preferences: this.getPreferences(),
        };
    }
}

// Create global instance
window.KizunaNotifications = new KizunaNotifications();
