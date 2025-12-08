/**
 * Kizuna Compatibility Integration
 * 
 * Integrates all compatibility, degradation, and versioning systems
 * to provide a unified compatibility layer for the Kizuna browser SDK.
 * 
 * @version 1.0.0
 */

(function (global) {
    'use strict';

    /**
     * Compatibility Integration Manager
     */
    class CompatibilityIntegration {
        constructor() {
            this.compatibility = null;
            this.degradation = null;
            this.versioning = null;
            this.featureDetection = null;
            this.initialized = false;
        }

        /**
         * Initialize all compatibility systems
         */
        async initialize() {
            if (this.initialized) {
                return;
            }

            // Initialize compatibility layer
            if (window.KizunaCompatibility) {
                this.compatibility = window.KizunaCompatibility.compatibilityLayer;
                console.log('[Kizuna] Compatibility layer initialized');
            }

            // Initialize graceful degradation
            if (window.KizunaGracefulDegradation) {
                this.degradation = window.KizunaGracefulDegradation.gracefulDegradation;
                console.log('[Kizuna] Graceful degradation initialized');
            }

            // Initialize API versioning
            if (window.KizunaAPIVersioning) {
                this.versioning = window.KizunaAPIVersioning.versionManager;
                console.log('[Kizuna] API versioning initialized');
            }

            // Initialize feature detection
            if (window.KizunaFeatureDetection) {
                this.featureDetection = window.KizunaFeatureDetection.detector;
                console.log('[Kizuna] Feature detection initialized');
            }

            this.initialized = true;

            // Display compatibility warnings if needed
            this._displayCompatibilityWarnings();
        }

        /**
         * Display compatibility warnings to user
         * @private
         */
        _displayCompatibilityWarnings() {
            if (!this.degradation) {
                return;
            }

            const notifications = this.degradation.getUserNotifications();
            const highSeverity = notifications.filter(n => n.severity === 'high');

            if (highSeverity.length > 0) {
                this._showWarningBanner(highSeverity);
            }
        }

        /**
         * Show warning banner for compatibility issues
         * @private
         */
        _showWarningBanner(issues) {
            const banner = document.createElement('div');
            banner.id = 'kizuna-compatibility-warning';
            banner.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                background: #ff9800;
                color: white;
                padding: 12px 20px;
                z-index: 10000;
                box-shadow: 0 2px 4px rgba(0,0,0,0.2);
                font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
                font-size: 14px;
            `;

            const message = document.createElement('div');
            message.innerHTML = `
                <strong>⚠️ Compatibility Notice:</strong> 
                Some features are not fully supported in your browser. 
                <a href="#" style="color: white; text-decoration: underline; margin-left: 10px;">
                    View Details
                </a>
                <button style="float: right; background: transparent; border: 1px solid white; color: white; padding: 4px 12px; cursor: pointer; border-radius: 3px;">
                    Dismiss
                </button>
            `;

            banner.appendChild(message);
            document.body.insertBefore(banner, document.body.firstChild);

            // Add event listeners
            const detailsLink = message.querySelector('a');
            detailsLink.addEventListener('click', (e) => {
                e.preventDefault();
                this._showCompatibilityDetails(issues);
            });

            const dismissBtn = message.querySelector('button');
            dismissBtn.addEventListener('click', () => {
                banner.remove();
            });
        }

        /**
         * Show detailed compatibility information
         * @private
         */
        _showCompatibilityDetails(issues) {
            const modal = document.createElement('div');
            modal.style.cssText = `
                position: fixed;
                top: 0;
                left: 0;
                right: 0;
                bottom: 0;
                background: rgba(0,0,0,0.5);
                z-index: 10001;
                display: flex;
                align-items: center;
                justify-content: center;
            `;

            const content = document.createElement('div');
            content.style.cssText = `
                background: white;
                padding: 24px;
                border-radius: 8px;
                max-width: 600px;
                max-height: 80vh;
                overflow-y: auto;
                box-shadow: 0 4px 6px rgba(0,0,0,0.1);
            `;

            let html = '<h2 style="margin-top: 0;">Browser Compatibility</h2>';
            html += '<p>The following features have limited support in your browser:</p>';
            html += '<ul style="line-height: 1.8;">';

            issues.forEach(issue => {
                html += `<li><strong>${issue.feature}:</strong> ${issue.message}`;
                if (issue.limitations.length > 0) {
                    html += '<ul>';
                    issue.limitations.forEach(limitation => {
                        html += `<li>${limitation}</li>`;
                    });
                    html += '</ul>';
                }
                html += '</li>';
            });

            html += '</ul>';
            html += '<button style="background: #2196F3; color: white; border: none; padding: 10px 20px; border-radius: 4px; cursor: pointer; margin-top: 16px;">Close</button>';

            content.innerHTML = html;
            modal.appendChild(content);
            document.body.appendChild(modal);

            // Close modal
            const closeBtn = content.querySelector('button');
            closeBtn.addEventListener('click', () => {
                modal.remove();
            });

            modal.addEventListener('click', (e) => {
                if (e.target === modal) {
                    modal.remove();
                }
            });
        }

        /**
         * Get comprehensive compatibility report
         */
        getCompatibilityReport() {
            const report = {
                timestamp: new Date().toISOString(),
                browser: {},
                features: {},
                degradation: {},
                versioning: {},
                recommendations: []
            };

            // Browser info
            if (this.compatibility) {
                report.browser = this.compatibility.browserInfo;
                report.polyfills = this.compatibility.polyfills;
            }

            // Feature detection
            if (this.featureDetection) {
                report.features = this.featureDetection.features;
                report.device = this.featureDetection.deviceInfo;
            }

            // Degradation info
            if (this.degradation) {
                report.degradation = this.degradation.getDegradationReport();
            }

            // Versioning info
            if (this.versioning) {
                report.versioning = this.versioning.getVersionInfo();
            }

            // Generate recommendations
            report.recommendations = this._generateRecommendations();

            return report;
        }

        /**
         * Generate recommendations based on compatibility status
         * @private
         */
        _generateRecommendations() {
            const recommendations = [];

            if (this.degradation) {
                const notifications = this.degradation.getUserNotifications();

                notifications.forEach(notification => {
                    if (notification.severity === 'high') {
                        recommendations.push({
                            priority: 'high',
                            category: notification.feature,
                            message: notification.message,
                            action: 'Consider using a modern browser for full functionality'
                        });
                    }
                });
            }

            if (this.compatibility) {
                const optimizations = this.compatibility.getBrowserOptimizations();
                optimizations.recommendations.forEach(opt => {
                    recommendations.push({
                        priority: 'medium',
                        category: opt.feature,
                        message: opt.optimization,
                        action: opt.reason
                    });
                });
            }

            return recommendations;
        }

        /**
         * Check if SDK can run in current environment
         */
        canRunSDK() {
            if (!this.degradation) {
                return true; // Assume compatible if degradation system not loaded
            }

            // Check critical features
            const webrtcSupport = this.degradation.getFeatureSupport('webrtc');
            const websocketSupport = this.degradation.getFeatureSupport('websocket');

            // Need at least one communication method
            const hasWebRTC = webrtcSupport && webrtcSupport.level !== 'unsupported';
            const hasWebSocket = websocketSupport && websocketSupport.level !== 'unsupported';

            if (!hasWebRTC && !hasWebSocket) {
                return {
                    canRun: false,
                    reason: 'No supported communication protocol (WebRTC or WebSocket)'
                };
            }

            // Check file API for file transfer
            const fileSupport = this.degradation.getFeatureSupport('fileAPI');
            const hasFileAPI = fileSupport && fileSupport.level !== 'unsupported';

            return {
                canRun: true,
                limitations: !hasFileAPI ? ['File transfer not available'] : []
            };
        }

        /**
         * Get recommended browser
         */
        getRecommendedBrowser() {
            const recommendations = [
                {
                    name: 'Google Chrome',
                    version: '90+',
                    reason: 'Best WebRTC support and performance',
                    url: 'https://www.google.com/chrome/'
                },
                {
                    name: 'Mozilla Firefox',
                    version: '88+',
                    reason: 'Good WebRTC support and privacy features',
                    url: 'https://www.mozilla.org/firefox/'
                },
                {
                    name: 'Microsoft Edge',
                    version: '90+',
                    reason: 'Chromium-based with good compatibility',
                    url: 'https://www.microsoft.com/edge'
                },
                {
                    name: 'Safari',
                    version: '14+',
                    reason: 'Best for macOS and iOS devices',
                    url: 'https://www.apple.com/safari/'
                }
            ];

            return recommendations;
        }

        /**
         * Export compatibility report as JSON
         */
        exportReport() {
            const report = this.getCompatibilityReport();
            const blob = new Blob([JSON.stringify(report, null, 2)], {
                type: 'application/json'
            });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `kizuna-compatibility-report-${Date.now()}.json`;
            a.click();
            URL.revokeObjectURL(url);
        }
    }

    /**
     * Initialize integration on page load
     */
    const integration = new CompatibilityIntegration();

    // Auto-initialize when DOM is ready
    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', () => {
            integration.initialize();
        });
    } else {
        integration.initialize();
    }

    // Export to global scope
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {
            CompatibilityIntegration,
            integration
        };
    } else {
        global.KizunaCompatibilityIntegration = {
            CompatibilityIntegration,
            integration
        };
    }

})(typeof window !== 'undefined' ? window : this);
