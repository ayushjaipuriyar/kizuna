# Progressive Web App (PWA) Implementation

This module implements Progressive Web App functionality for Kizuna's browser support system, enabling offline capabilities, installability, and push notifications.

## Features Implemented

### 1. Service Worker (Subtask 6.1)
- **File**: `static/service-worker.js`
- **Capabilities**:
  - Offline functionality with intelligent caching strategies
  - Cache-first strategy for static resources
  - Network-first strategy for API requests
  - Background sync for queued operations when offline
  - IndexedDB integration for offline data storage
  - Automatic cache cleanup and versioning

### 2. PWA Manifest and Installation (Subtask 6.2)
- **Files**: 
  - `static/manifest.json` - PWA manifest with app metadata
  - `static/kizuna-install.js` - Installation manager
- **Capabilities**:
  - Installable web app with native-like experience
  - Installation prompts and banners
  - App shortcuts for quick actions
  - Share target integration
  - Platform-specific installation instructions
  - Standalone display mode support

### 3. Push Notifications (Subtask 6.3)
- **File**: `static/kizuna-notifications.js`
- **Capabilities**:
  - Push notification subscription management
  - Permission handling and user preferences
  - Notification delivery for various events:
    - File transfer completion
    - Clipboard synchronization
    - Command execution status
    - Peer connection changes
  - Notification click handling and interactions
  - VAPID key integration for secure push

### 4. Offline Data Management and Caching (Subtask 6.4)
- **Files**:
  - `static/kizuna-cache.js` - Cache manager
  - `static/kizuna-pwa.js` - PWA manager
- **Capabilities**:
  - Intelligent resource caching with TTL
  - Cache invalidation and pruning
  - Storage quota management
  - Persistent storage requests
  - IndexedDB for offline data
  - Settings and preferences storage
  - Operation queuing for background sync

## Architecture

```
PWA System
├── Service Worker (service-worker.js)
│   ├── Install & Activate lifecycle
│   ├── Fetch event handling
│   ├── Background sync
│   └── Push notification handling
│
├── PWA Manager (kizuna-pwa.js)
│   ├── Service worker registration
│   ├── Offline operation queuing
│   ├── Settings management
│   └── Network status monitoring
│
├── Installation Manager (kizuna-install.js)
│   ├── Installation prompts
│   ├── Platform detection
│   └── App interface setup
│
├── Notifications Manager (kizuna-notifications.js)
│   ├── Permission management
│   ├── Push subscription
│   └── Notification delivery
│
└── Cache Manager (kizuna-cache.js)
    ├── Resource caching
    ├── Cache invalidation
    └── Storage quota management
```

## Rust Integration

The Rust PWA controller (`pwa/mod.rs`) provides:

- **PWAController**: Main controller for PWA features
- **OfflineOperation**: Queued operations for background sync
- **CacheEntry**: Cached data with expiration
- **ServiceWorkerInfo**: Service worker registration info
- **PushNotification**: Push notification data structures
- **NotificationPreferences**: User notification preferences

## Usage

### Basic Setup

```html
<!-- Include PWA scripts -->
<script src="/kizuna-pwa.js"></script>
<script src="/kizuna-install.js"></script>
<script src="/kizuna-notifications.js"></script>
<script src="/kizuna-cache.js"></script>

<!-- Add manifest link -->
<link rel="manifest" href="/manifest.json">
```

### Installation

```javascript
// Show install banner
if (window.KizunaInstall.canInstall()) {
    window.KizunaInstall.showInstallBanner({
        message: 'Install Kizuna for offline access',
        position: 'bottom',
        dismissible: true,
    });
}

// Or create custom install button
const installBtn = window.KizunaInstall.createInstallButton({
    text: 'Install App',
    className: 'custom-install-btn',
});
document.body.appendChild(installBtn);
```

### Offline Data

```javascript
// Save data for offline access
await window.KizunaPWA.saveSetting('user-preferences', {
    theme: 'dark',
    notifications: true,
});

// Load offline data
const preferences = await window.KizunaPWA.getSetting('user-preferences');

// Queue operation for background sync
await window.KizunaPWA.queueOperation('file-transfer', {
    fileName: 'document.pdf',
    peerId: 'peer-123',
});
```

### Notifications

```javascript
// Request permission
await window.KizunaNotifications.requestPermission();

// Subscribe to push notifications
await window.KizunaNotifications.subscribe();

// Show notification
await window.KizunaNotifications.notifyFileTransfer(
    'document.pdf',
    'complete',
    { transferId: '123' }
);

// Update preferences
await window.KizunaNotifications.updatePreferences({
    fileTransfer: true,
    clipboardSync: false,
});
```

### Cache Management

```javascript
// Cache resources
await window.KizunaCache.cacheResources([
    '/app.js',
    '/styles.css',
    '/logo.png',
]);

// Cache data with TTL
await window.KizunaCache.cacheData('api-response', data, 3600000); // 1 hour

// Get cached data
const cachedData = await window.KizunaCache.getCachedData('api-response');

// Check storage quota
const quota = await window.KizunaCache.checkStorageQuota();
console.log(`Using ${quota.percentage}% of storage`);

// Clear cache
await window.KizunaCache.clearAllCaches();
```

## Demo

A comprehensive demo is available at `/pwa-demo.html` showcasing all PWA features:
- Installation status and prompts
- Service worker registration
- Network status monitoring
- Push notification management
- Cache statistics and management
- Offline data operations

## Requirements Validation

This implementation satisfies the following requirements:

### Requirement 6.1 (PWA Client with installable manifest)
✅ Implemented via `manifest.json` and `kizuna-install.js`

### Requirement 6.2 (Offline functionality for cached data)
✅ Implemented via service worker caching and IndexedDB storage

### Requirement 6.3 (Native-like user experience)
✅ Implemented via standalone display mode and app-like interface

### Requirement 6.4 (Push notifications for important events)
✅ Implemented via `kizuna-notifications.js` and service worker push handling

### Requirement 6.5 (Cache essential resources for offline operation)
✅ Implemented via service worker caching strategies and cache manager

## Browser Compatibility

- **Chrome/Edge**: Full PWA support including installation and push notifications
- **Firefox**: Service workers and caching, limited push notification support
- **Safari**: Service workers on iOS 11.3+, limited PWA features
- **Mobile browsers**: Varying levels of support, graceful degradation implemented

## Future Enhancements

- Background fetch for large file transfers
- Periodic background sync for automatic updates
- Web Share API integration
- Badging API for notification counts
- App shortcuts customization
- Advanced caching strategies (stale-while-revalidate)
