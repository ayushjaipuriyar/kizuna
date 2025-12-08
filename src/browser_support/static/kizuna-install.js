/**
 * Kizuna PWA Installation Manager
 * Handles PWA installation prompts and app-like interface
 */

class KizunaInstallManager {
    constructor() {
        this.deferredPrompt = null;
        this.isInstalled = this.checkInstallStatus();
        this.listeners = {
            beforeinstallprompt: [],
            appinstalled: [],
            installaccepted: [],
            installdismissed: [],
        };

        this.init();
    }

    /**
     * Initialize installation manager
     */
    init() {
        // Listen for beforeinstallprompt event
        window.addEventListener('beforeinstallprompt', (e) => {
            console.log('[Install] beforeinstallprompt event fired');

            // Prevent the default browser install prompt
            e.preventDefault();

            // Store the event for later use
            this.deferredPrompt = e;

            // Notify listeners
            this.notifyListeners('beforeinstallprompt', e);
        });

        // Listen for appinstalled event
        window.addEventListener('appinstalled', (e) => {
            console.log('[Install] App installed successfully');
            this.isInstalled = true;
            this.deferredPrompt = null;

            // Notify listeners
            this.notifyListeners('appinstalled', e);
        });

        // Check if already installed
        if (this.isInstalled) {
            console.log('[Install] App is already installed');
            this.setupAppInterface();
        }
    }

    /**
     * Check if app is installed
     */
    checkInstallStatus() {
        // Check if running in standalone mode
        if (window.matchMedia('(display-mode: standalone)').matches) {
            return true;
        }

        // Check iOS standalone mode
        if (window.navigator.standalone === true) {
            return true;
        }

        // Check if installed via related applications
        if (document.referrer.includes('android-app://')) {
            return true;
        }

        return false;
    }

    /**
     * Check if installation is available
     */
    canInstall() {
        return this.deferredPrompt !== null && !this.isInstalled;
    }

    /**
     * Show installation prompt
     */
    async showInstallPrompt() {
        if (!this.canInstall()) {
            console.warn('[Install] Installation not available');
            return { outcome: 'unavailable' };
        }

        try {
            // Show the install prompt
            this.deferredPrompt.prompt();

            // Wait for the user's response
            const choiceResult = await this.deferredPrompt.userChoice;

            console.log('[Install] User choice:', choiceResult.outcome);

            if (choiceResult.outcome === 'accepted') {
                console.log('[Install] User accepted the install prompt');
                this.notifyListeners('installaccepted', choiceResult);
            } else {
                console.log('[Install] User dismissed the install prompt');
                this.notifyListeners('installdismissed', choiceResult);
            }

            // Clear the deferred prompt
            this.deferredPrompt = null;

            return choiceResult;
        } catch (error) {
            console.error('[Install] Error showing install prompt:', error);
            throw error;
        }
    }

    /**
     * Create install button
     */
    createInstallButton(options = {}) {
        const {
            text = 'Install App',
            className = 'kizuna-install-button',
            style = {},
        } = options;

        const button = document.createElement('button');
        button.textContent = text;
        button.className = className;

        // Apply styles
        Object.assign(button.style, {
            padding: '12px 24px',
            fontSize: '16px',
            fontWeight: 'bold',
            color: '#ffffff',
            backgroundColor: '#2196F3',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
            transition: 'background-color 0.3s',
            ...style,
        });

        // Hover effect
        button.addEventListener('mouseenter', () => {
            button.style.backgroundColor = '#1976D2';
        });

        button.addEventListener('mouseleave', () => {
            button.style.backgroundColor = '#2196F3';
        });

        // Click handler
        button.addEventListener('click', async () => {
            try {
                const result = await this.showInstallPrompt();

                if (result.outcome === 'accepted') {
                    button.textContent = 'Installing...';
                    button.disabled = true;
                }
            } catch (error) {
                console.error('[Install] Install button error:', error);
            }
        });

        // Hide button if not installable
        if (!this.canInstall()) {
            button.style.display = 'none';
        }

        // Show button when installation becomes available
        this.on('beforeinstallprompt', () => {
            button.style.display = '';
        });

        // Hide button after installation
        this.on('appinstalled', () => {
            button.style.display = 'none';
        });

        return button;
    }

    /**
     * Show install banner
     */
    showInstallBanner(options = {}) {
        const {
            message = 'Install Kizuna for a better experience',
            position = 'bottom',
            dismissible = true,
        } = options;

        // Don't show if already installed or not installable
        if (this.isInstalled || !this.canInstall()) {
            return null;
        }

        // Create banner
        const banner = document.createElement('div');
        banner.className = 'kizuna-install-banner';

        Object.assign(banner.style, {
            position: 'fixed',
            left: '0',
            right: '0',
            [position]: '0',
            padding: '16px',
            backgroundColor: '#2196F3',
            color: '#ffffff',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            boxShadow: '0 -2px 10px rgba(0, 0, 0, 0.1)',
            zIndex: '9999',
            animation: 'slideIn 0.3s ease-out',
        });

        // Message
        const messageEl = document.createElement('span');
        messageEl.textContent = message;
        messageEl.style.flex = '1';
        banner.appendChild(messageEl);

        // Install button
        const installBtn = document.createElement('button');
        installBtn.textContent = 'Install';
        Object.assign(installBtn.style, {
            padding: '8px 16px',
            marginLeft: '16px',
            backgroundColor: '#ffffff',
            color: '#2196F3',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
            fontWeight: 'bold',
        });

        installBtn.addEventListener('click', async () => {
            try {
                await this.showInstallPrompt();
                banner.remove();
            } catch (error) {
                console.error('[Install] Banner install error:', error);
            }
        });

        banner.appendChild(installBtn);

        // Dismiss button
        if (dismissible) {
            const dismissBtn = document.createElement('button');
            dismissBtn.textContent = '×';
            Object.assign(dismissBtn.style, {
                padding: '8px 12px',
                marginLeft: '8px',
                backgroundColor: 'transparent',
                color: '#ffffff',
                border: 'none',
                fontSize: '24px',
                cursor: 'pointer',
                lineHeight: '1',
            });

            dismissBtn.addEventListener('click', () => {
                banner.remove();
            });

            banner.appendChild(dismissBtn);
        }

        // Add to page
        document.body.appendChild(banner);

        // Remove after installation
        this.on('appinstalled', () => {
            banner.remove();
        });

        return banner;
    }

    /**
     * Setup app-like interface for installed PWA
     */
    setupAppInterface() {
        // Add app-installed class to body
        document.body.classList.add('kizuna-app-installed');

        // Hide browser UI elements
        const metaThemeColor = document.querySelector('meta[name="theme-color"]');
        if (metaThemeColor) {
            metaThemeColor.setAttribute('content', '#2196F3');
        }

        // Add iOS status bar styling
        const metaAppleStatusBar = document.querySelector('meta[name="apple-mobile-web-app-status-bar-style"]');
        if (metaAppleStatusBar) {
            metaAppleStatusBar.setAttribute('content', 'black-translucent');
        }

        // Prevent pull-to-refresh on mobile
        document.body.style.overscrollBehavior = 'none';

        console.log('[Install] App interface configured');
    }

    /**
     * Get installation instructions for current platform
     */
    getInstallInstructions() {
        const userAgent = navigator.userAgent.toLowerCase();

        if (this.isInstalled) {
            return {
                platform: 'installed',
                instructions: 'App is already installed',
            };
        }

        if (this.canInstall()) {
            return {
                platform: 'supported',
                instructions: 'Click the install button to add Kizuna to your device',
            };
        }

        // iOS Safari
        if (/iphone|ipad|ipod/.test(userAgent) && /safari/.test(userAgent) && !/crios/.test(userAgent)) {
            return {
                platform: 'ios-safari',
                instructions: 'Tap the Share button, then tap "Add to Home Screen"',
            };
        }

        // Android Chrome
        if (/android/.test(userAgent) && /chrome/.test(userAgent)) {
            return {
                platform: 'android-chrome',
                instructions: 'Tap the menu button (⋮), then tap "Add to Home screen"',
            };
        }

        // Desktop Chrome/Edge
        if (/chrome|edg/.test(userAgent) && !/mobile/.test(userAgent)) {
            return {
                platform: 'desktop-chrome',
                instructions: 'Click the install icon in the address bar or menu',
            };
        }

        return {
            platform: 'unsupported',
            instructions: 'PWA installation not supported on this browser',
        };
    }

    /**
     * Check if app needs update
     */
    async checkForUpdates() {
        if ('serviceWorker' in navigator && navigator.serviceWorker.controller) {
            try {
                const registration = await navigator.serviceWorker.getRegistration();

                if (registration) {
                    await registration.update();
                    console.log('[Install] Update check complete');
                    return true;
                }
            } catch (error) {
                console.error('[Install] Update check failed:', error);
            }
        }

        return false;
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
     * Get installation status
     */
    getInstallStatus() {
        return {
            isInstalled: this.isInstalled,
            canInstall: this.canInstall(),
            platform: this.getInstallInstructions().platform,
        };
    }
}

// Create global instance
window.KizunaInstall = new KizunaInstallManager();
