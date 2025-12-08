# Kizuna Browser Compatibility System

## Overview

The Kizuna Browser Compatibility System provides comprehensive cross-browser support, graceful degradation, and API versioning for the Kizuna WebRTC browser client. This system ensures that Kizuna works reliably across different browsers and platforms while providing fallback mechanisms for unsupported features.

## Components

### 1. Compatibility Layer (`kizuna-compatibility.js`)

Provides polyfills and compatibility shims for cross-browser support.

**Features:**
- WebRTC API standardization (RTCPeerConnection, RTCSessionDescription, RTCIceCandidate)
- Clipboard API polyfills with execCommand fallback
- File API polyfills including FileReader enhancements
- Media API polyfills (getUserMedia standardization)
- Storage API polyfills with in-memory fallback
- Promise polyfills (allSettled, any)
- Browser-specific fixes for Safari, Firefox, and Edge

**Usage:**
```javascript
// Automatically applied on page load
const report = KizunaCompatibility.compatibilityLayer.getCompatibilityReport();
console.log('Browser:', report.browser);
console.log('Polyfills:', report.polyfills);
```

### 2. Graceful Degradation (`kizuna-graceful-degradation.js`)

Implements fallback mechanisms and progressive enhancement for unsupported features.

**Support Levels:**
- `FULL`: Feature fully supported
- `PARTIAL`: Feature partially supported with limitations
- `FALLBACK`: Feature available through fallback mechanism
- `DEGRADED`: Feature available but with reduced functionality
- `UNSUPPORTED`: Feature not available

**Features:**
- Automatic feature support assessment
- WebSocket fallback for WebRTC
- execCommand fallback for Clipboard API
- File input fallback for drag-and-drop
- In-memory storage fallback for localStorage
- In-app notifications fallback
- Progressive CSS classes based on support

**Usage:**
```javascript
const degradation = KizunaGracefulDegradation.gracefulDegradation;

// Check feature support
const webrtcSupport = degradation.getFeatureSupport('webrtc');
console.log('WebRTC:', webrtcSupport.level, webrtcSupport.message);

// Get fallback handler
if (webrtcSupport.level === 'fallback') {
    const fallback = degradation.getFallbackHandler('websocket');
    // Use WebSocket fallback
}

// Get user notifications
const notifications = degradation.getUserNotifications();
notifications.forEach(n => {
    console.log(`${n.feature}: ${n.message} (${n.severity})`);
});
```

### 3. API Versioning (`kizuna-api-versioning.js`)

Provides API versioning system with backward compatibility.

**Features:**
- API version management (currently v1.0)
- Version adapters for backward compatibility
- Feature negotiation between browser and peer
- Deprecation warnings for old APIs
- Migration guides between versions

**Usage:**
```javascript
const versioning = KizunaAPIVersioning;

// Get local capabilities
const capabilities = versioning.getLocalCapabilities();
console.log('Supported versions:', capabilities.supportedVersions);
console.log('Features:', capabilities.features);

// Negotiate with peer
const peerCapabilities = {
    supportedVersions: ['1.0'],
    features: { /* peer features */ }
};
const negotiated = versioning.negotiateWithPeer(peerCapabilities);
console.log('Negotiated version:', negotiated.version);
console.log('Available features:', negotiated.features);

// Create versioned API wrapper
const versionedAPI = versioning.createVersionedAPI(baseAPI);
versionedAPI.setVersion('1.0');
```

### 4. Feature Detection (`kizuna-feature-detection.js`)

Comprehensive feature detection for mobile and desktop browsers.

**Detected Features:**
- WebRTC, WebSocket, Service Workers
- Media APIs (getUserMedia, MediaRecorder)
- Storage APIs (localStorage, sessionStorage, IndexedDB)
- Clipboard API (read/write)
- File APIs (FileReader, drag-and-drop)
- Network APIs (fetch, XHR, beacon)
- UI features (fullscreen, notifications, vibration)
- Mobile-specific features (touch events, orientation)

**Usage:**
```javascript
const detector = KizunaFeatureDetection.detector;

// Get all detected information
const info = detector.getInfo();
console.log('Device:', info.device);
console.log('Browser:', info.browser);
console.log('Features:', info.features);

// Check specific feature
if (detector.supports('webrtc')) {
    console.log('WebRTC is supported');
}

// Get mobile limitations
const limitations = detector.getMobileLimitations();
limitations.forEach(limit => {
    console.log(`${limit.feature}: ${limit.message}`);
    console.log('Fallback:', limit.fallback);
});
```

### 5. Compatibility Integration (`kizuna-compatibility-integration.js`)

Integrates all compatibility systems and provides unified interface.

**Features:**
- Automatic initialization of all compatibility systems
- Compatibility warning banners for users
- Comprehensive compatibility reports
- SDK compatibility checks
- Browser recommendations

**Usage:**
```javascript
const integration = KizunaCompatibilityIntegration.integration;

// Get comprehensive report
const report = integration.getCompatibilityReport();
console.log('Overall support level:', report.degradation.overallLevel);
console.log('Recommendations:', report.recommendations);

// Check if SDK can run
const canRun = integration.canRunSDK();
if (!canRun.canRun) {
    console.error('Cannot run SDK:', canRun.reason);
}

// Get recommended browsers
const browsers = integration.getRecommendedBrowser();
browsers.forEach(browser => {
    console.log(`${browser.name} ${browser.version}: ${browser.reason}`);
});

// Export compatibility report
integration.exportReport(); // Downloads JSON file
```

## Browser Support Matrix

### Desktop Browsers

| Browser | Version | WebRTC | Clipboard | File API | Service Worker | Notes |
|---------|---------|--------|-----------|----------|----------------|-------|
| Chrome | 90+ | ✅ Full | ✅ Full | ✅ Full | ✅ Full | Best support |
| Firefox | 88+ | ✅ Full | ✅ Full | ✅ Full | ✅ Full | Good support |
| Safari | 14+ | ⚠️ Partial | ⚠️ Partial | ✅ Full | ✅ Full | Requires user gestures |
| Edge | 90+ | ✅ Full | ✅ Full | ✅ Full | ✅ Full | Chromium-based |

### Mobile Browsers

| Browser | Platform | WebRTC | Clipboard | File API | Service Worker | Notes |
|---------|----------|--------|-----------|----------|----------------|-------|
| Chrome Mobile | Android | ✅ Full | ⚠️ Partial | ✅ Full | ✅ Full | Best mobile support |
| Safari Mobile | iOS | ⚠️ Partial | ⚠️ Partial | ✅ Full | ✅ Full | Limited WebRTC |
| Firefox Mobile | Android | ✅ Full | ⚠️ Partial | ✅ Full | ✅ Full | Good support |
| Samsung Internet | Android | ✅ Full | ⚠️ Partial | ✅ Full | ✅ Full | Chromium-based |

**Legend:**
- ✅ Full: Complete support
- ⚠️ Partial: Supported with limitations
- ❌ None: Not supported (fallback available)

## Fallback Mechanisms

### WebRTC → WebSocket
When WebRTC is not supported or fails:
- Automatically falls back to WebSocket communication
- Maintains same API interface
- Slightly higher latency but reliable

### Clipboard API → execCommand
When Clipboard API is not available:
- Uses `document.execCommand('copy')` for writing
- Requires user interaction
- Limited paste functionality

### Drag-and-Drop → File Input
When drag-and-drop is not supported:
- Uses standard file input element
- Same file selection functionality
- Less convenient UX

### localStorage → Memory Storage
When localStorage is blocked:
- Uses in-memory Map storage
- Data lost on page reload
- Same API interface

### Notifications → In-App
When system notifications are unavailable:
- Shows notifications within app UI
- Custom event system
- Only visible when app is open

## Progressive Enhancement

The system automatically adds CSS classes to enable progressive enhancement:

```css
/* Feature-specific classes */
.kizuna-webrtc-full { /* WebRTC fully supported */ }
.kizuna-webrtc-fallback { /* Using WebSocket fallback */ }
.kizuna-webrtc-unsupported { /* WebRTC not available */ }

/* Overall support level */
.kizuna-support-full { /* All features supported */ }
.kizuna-support-partial { /* Some features limited */ }
.kizuna-support-degraded { /* Significant limitations */ }

/* Hide/show based on support */
.kizuna-webrtc-unsupported .webrtc-only { display: none; }
.kizuna-webrtc-fallback .websocket-fallback { display: block; }
```

## Integration with Kizuna SDK

The compatibility system is automatically integrated with the Kizuna SDK:

```javascript
// SDK automatically uses compatibility layer
const sdk = new KizunaSDK({
    apiVersion: '1.0',
    debug: true
});

// Connect with automatic fallback
await sdk.connect(setupId);

// Check negotiated features
if (sdk.isFeatureAvailable('fileTransfer')) {
    // File transfer is available
}

// Get API version info
const versionInfo = sdk.getAPIVersion();
console.log('API version:', versionInfo.current);
```

## User Notifications

The system automatically displays warnings for significant compatibility issues:

- **High severity**: Critical features unavailable (e.g., no WebRTC or WebSocket)
- **Medium severity**: Features available through fallback (e.g., WebSocket fallback active)
- **Low severity**: Minor limitations (e.g., clipboard requires user gesture)

Users can view detailed compatibility information through the UI.

## Development and Debugging

### Enable Debug Logging

All compatibility modules log detailed information in development:

```javascript
// Automatically enabled on localhost
// Check console for:
// - Kizuna Compatibility Layer
// - Kizuna Graceful Degradation
// - Kizuna API Versioning
// - Kizuna Feature Detection
```

### Export Compatibility Report

```javascript
// Export detailed JSON report
KizunaCompatibilityIntegration.integration.exportReport();
```

### Manual Testing

Test specific browsers and scenarios:

1. **Safari**: Test clipboard and WebRTC limitations
2. **Firefox**: Test WebRTC event differences
3. **Mobile browsers**: Test touch interfaces and limitations
4. **Private browsing**: Test storage fallbacks
5. **Older browsers**: Test polyfills and fallbacks

## API Versioning

### Current Version: 1.0

**Features:**
- Connection management
- File transfer
- Clipboard synchronization
- Command execution
- Video streaming
- WebRTC and WebSocket protocols

### Future Versions

Version adapters will be added for backward compatibility when new versions are released.

## Best Practices

1. **Always check feature availability** before using advanced features
2. **Provide fallback UI** for unsupported features
3. **Test on multiple browsers** including mobile
4. **Monitor compatibility reports** in production
5. **Keep compatibility scripts updated** with new browser versions

## Troubleshooting

### WebRTC Connection Fails
- Check if browser supports WebRTC
- Verify HTTPS is used (required for WebRTC)
- Check firewall and NAT settings
- Fallback to WebSocket should activate automatically

### Clipboard Not Working
- Ensure user interaction triggered the operation
- Check if Clipboard API is supported
- Verify HTTPS context
- execCommand fallback should activate automatically

### Storage Issues
- Check if localStorage is blocked (private browsing)
- Verify storage quota
- Memory storage fallback should activate automatically

### Service Worker Not Registering
- Ensure HTTPS is used
- Check browser support
- Verify service worker file path
- Check console for registration errors

## Performance Considerations

- Polyfills add minimal overhead (~5-10KB)
- Feature detection runs once on initialization
- Fallback mechanisms activate only when needed
- Progressive enhancement improves perceived performance

## Security Considerations

- All polyfills maintain security boundaries
- Fallback mechanisms respect same-origin policy
- Clipboard operations require user interaction
- Storage fallbacks are memory-only (no persistence)

## Contributing

When adding new features:
1. Add feature detection in `kizuna-feature-detection.js`
2. Add polyfills in `kizuna-compatibility.js`
3. Add fallback in `kizuna-graceful-degradation.js`
4. Update API version if needed
5. Update this documentation

## License

Part of the Kizuna project. See main LICENSE file.
