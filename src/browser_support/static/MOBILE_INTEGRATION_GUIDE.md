# Mobile Browser Optimization - Integration Guide

## Quick Start

### 1. Include Required Files

Add these files to your HTML in the following order:

```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
    <meta name="apple-mobile-web-app-capable" content="yes">
    <meta name="theme-color" content="#2196F3">
    
    <!-- Core styles -->
    <link rel="stylesheet" href="kizuna-ui.css">
    
    <!-- Responsive design system -->
    <link rel="stylesheet" href="kizuna-responsive.css">
    
    <!-- Mobile-specific styles -->
    <link rel="stylesheet" href="kizuna-mobile.css">
</head>
<body>
    <!-- Your content -->
    
    <!-- Core SDK -->
    <script src="kizuna-sdk.js"></script>
    
    <!-- Feature detection -->
    <script src="kizuna-feature-detection.js"></script>
    
    <!-- Mobile components -->
    <script src="kizuna-mobile.js"></script>
    
    <!-- UI components -->
    <script src="kizuna-ui.js"></script>
</body>
</html>
```

### 2. Initialize Feature Detection

```javascript
// Feature detection is automatically initialized
const detector = KizunaFeatureDetection.detector;
const fallbackManager = KizunaFeatureDetection.fallbackManager;

// Check if running on mobile
if (detector.deviceInfo.isMobile) {
    console.log('Running on mobile device');
    
    // Get mobile limitations
    const limitations = detector.getMobileLimitations();
    limitations.forEach(limitation => {
        console.warn(`${limitation.feature}: ${limitation.message}`);
    });
}

// Check specific feature support
if (!detector.supports('webrtc')) {
    console.log('WebRTC not supported, using fallback');
    const fallback = fallbackManager.getFallback('webrtc');
    // Use fallback.handler() to get fallback implementation
}
```

### 3. Use Mobile Components

#### Mobile File Transfer UI

```javascript
// Initialize Kizuna SDK
const sdk = new KizunaSDK({
    serverUrl: 'wss://your-server.com'
});

// Create mobile file transfer UI
const container = document.getElementById('file-transfer-container');
const fileTransferUI = new KizunaMobile.MobileFileTransferUI(container, sdk);

// The UI automatically handles:
// - Touch gestures (swipe to reveal actions)
// - File selection
// - Upload progress
// - Mobile-optimized layout
```

#### Mobile Video Player UI

```javascript
// Create mobile video player
const videoContainer = document.getElementById('video-container');
const videoPlayer = new KizunaMobile.MobileVideoPlayerUI(videoContainer, sdk);

// The player automatically handles:
// - Tap to show/hide controls
// - Double-tap to play/pause
// - Swipe gestures for fullscreen
// - Auto-hiding controls
```

#### Custom Touch Gestures

```javascript
// Create touch handler for custom element
const touchHandler = new KizunaMobile.MobileTouchHandler();
const element = document.getElementById('my-element');

touchHandler.init(element, {
    onSwipeLeft: (e) => {
        console.log('User swiped left');
        // Handle swipe left
    },
    onSwipeRight: (e) => {
        console.log('User swiped right');
        // Handle swipe right
    },
    onSwipeUp: (e) => {
        console.log('User swiped up');
        // Handle swipe up
    },
    onSwipeDown: (e) => {
        console.log('User swiped down');
        // Handle swipe down
    },
    onTap: (e) => {
        console.log('User tapped');
        // Handle tap
    },
    onDoubleTap: (e) => {
        console.log('User double-tapped');
        // Handle double tap
    },
    onLongPress: (e) => {
        console.log('User long-pressed');
        // Handle long press
    },
    onPinch: (data) => {
        console.log('User pinched', data.scale);
        // Handle pinch/zoom
    }
});
```

## Responsive Design System

### Using the Grid System

```html
<!-- Basic grid -->
<div class="grid">
    <div class="col-12 col-md-6 col-lg-4">Column 1</div>
    <div class="col-12 col-md-6 col-lg-4">Column 2</div>
    <div class="col-12 col-md-6 col-lg-4">Column 3</div>
</div>

<!-- Card grid (automatically responsive) -->
<div class="card-grid">
    <div class="card">Card 1</div>
    <div class="card">Card 2</div>
    <div class="card">Card 3</div>
</div>
```

### Using Flexbox Utilities

```html
<!-- Horizontal layout with space between -->
<div class="flex justify-between items-center gap-md">
    <span>Left</span>
    <span>Right</span>
</div>

<!-- Vertical layout on mobile, horizontal on desktop -->
<div class="flex flex-col md:flex-row gap-lg">
    <div class="flex-1">Content 1</div>
    <div class="flex-1">Content 2</div>
</div>
```

### Responsive Visibility

```html
<!-- Show only on mobile -->
<div class="show-mobile">
    Mobile-only content
</div>

<!-- Show only on desktop -->
<div class="show-desktop">
    Desktop-only content
</div>

<!-- Hide on specific breakpoints -->
<div class="hidden-xs hidden-sm">
    Hidden on extra small and small screens
</div>
```

### Spacing Utilities

```html
<!-- Margin utilities -->
<div class="mt-lg mb-md">Content with top and bottom margin</div>

<!-- Padding utilities -->
<div class="p-md">Content with padding</div>

<!-- Responsive spacing -->
<div class="p-sm md:p-lg">
    Small padding on mobile, large on desktop
</div>
```

## Feature Detection Patterns

### Pattern 1: Progressive Enhancement

```javascript
// Start with basic functionality
let transferMethod = 'basic';

// Enhance if WebRTC is available
if (detector.supports('webrtc')) {
    transferMethod = 'webrtc';
    console.log('Using WebRTC for transfers');
} else if (detector.supports('websocket')) {
    transferMethod = 'websocket';
    console.log('Using WebSocket fallback');
}

// Initialize with appropriate method
initializeTransfer(transferMethod);
```

### Pattern 2: Graceful Degradation

```javascript
// Try to use advanced feature
async function copyToClipboard(text) {
    if (detector.supports('clipboard')) {
        try {
            await navigator.clipboard.writeText(text);
            return true;
        } catch (err) {
            console.warn('Clipboard API failed, using fallback');
        }
    }
    
    // Fallback to execCommand
    const fallback = fallbackManager.getFallback('clipboard');
    if (fallback && fallback.available) {
        return execCommandCopy(text);
    }
    
    // Last resort: show manual copy prompt
    showManualCopyPrompt(text);
    return false;
}
```

### Pattern 3: Feature-Based Routing

```javascript
// Route to appropriate implementation based on features
function initializeApp() {
    const features = detector.features;
    
    if (features.webrtc && features.mediaDevices) {
        // Full-featured app
        initializeFullApp();
    } else if (features.websocket) {
        // Limited app with WebSocket
        initializeLimitedApp();
    } else {
        // Basic app with polling
        initializeBasicApp();
    }
}
```

## Mobile-Specific Optimizations

### 1. Touch Target Sizing

```css
/* All interactive elements should be at least 48x48px */
.btn {
    min-width: 48px;
    min-height: 48px;
}
```

### 2. Prevent Zoom on Input Focus

```html
<!-- Use 16px font size to prevent iOS zoom -->
<input type="text" style="font-size: 16px;">
```

### 3. Safe Area Insets

```css
/* Handle notched devices */
.header {
    padding-top: env(safe-area-inset-top);
    padding-left: env(safe-area-inset-left);
    padding-right: env(safe-area-inset-right);
}
```

### 4. Smooth Scrolling

```css
/* Enable momentum scrolling on iOS */
.scrollable {
    overflow-y: auto;
    -webkit-overflow-scrolling: touch;
}
```

### 5. Prevent Pull-to-Refresh

```css
body {
    overscroll-behavior-y: contain;
}
```

## Handling Orientation Changes

```javascript
// Listen for orientation changes
window.addEventListener('orientationchange', () => {
    // Wait for layout to settle
    setTimeout(() => {
        // Update UI based on new orientation
        const orientation = detector.getOrientation();
        console.log('New orientation:', orientation);
        
        // Adjust layout
        adjustLayoutForOrientation(orientation);
    }, 100);
});

// Or use matchMedia for more control
const portraitQuery = window.matchMedia('(orientation: portrait)');
portraitQuery.addListener((e) => {
    if (e.matches) {
        console.log('Portrait mode');
    } else {
        console.log('Landscape mode');
    }
});
```

## Performance Best Practices

### 1. Use Passive Event Listeners

```javascript
// Already implemented in MobileTouchHandler
element.addEventListener('touchstart', handler, { passive: true });
```

### 2. Debounce Resize Events

```javascript
let resizeTimeout;
window.addEventListener('resize', () => {
    clearTimeout(resizeTimeout);
    resizeTimeout = setTimeout(() => {
        // Handle resize
        handleResize();
    }, 250);
});
```

### 3. Use CSS Transforms for Animations

```css
/* Use transform instead of position changes */
.animated {
    transform: translateX(100px);
    transition: transform 0.3s ease;
}
```

### 4. Lazy Load Images

```html
<img src="placeholder.jpg" data-src="actual-image.jpg" loading="lazy">
```

## Testing on Mobile Devices

### Chrome DevTools Mobile Emulation

1. Open Chrome DevTools (F12)
2. Click the device toolbar icon (Ctrl+Shift+M)
3. Select a device or set custom dimensions
4. Test touch events and responsive layouts

### Real Device Testing

1. Connect device via USB
2. Enable USB debugging (Android) or Web Inspector (iOS)
3. Use Chrome Remote Debugging or Safari Web Inspector
4. Test actual touch gestures and performance

### Testing Checklist

- [ ] Touch gestures work correctly
- [ ] Layouts adapt to different screen sizes
- [ ] Safe area insets work on notched devices
- [ ] Orientation changes handled properly
- [ ] Performance is acceptable (60fps)
- [ ] No horizontal scrolling
- [ ] Text is readable without zooming
- [ ] Touch targets are at least 48x48px
- [ ] Forms don't cause zoom on focus

## Troubleshooting

### Issue: Touch events not working

**Solution**: Ensure you're using the correct event listeners and not preventing default on passive events.

```javascript
// Correct
element.addEventListener('touchstart', handler, { passive: false });

// If you need to preventDefault
element.addEventListener('touchstart', (e) => {
    e.preventDefault();
    // Handle touch
}, { passive: false });
```

### Issue: Viewport not scaling correctly

**Solution**: Check your viewport meta tag:

```html
<meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0, user-scalable=no">
```

### Issue: Feature detection returning false positives

**Solution**: Use try-catch blocks for actual feature testing:

```javascript
function testFeature() {
    try {
        // Try to use the feature
        const result = someAPI.doSomething();
        return true;
    } catch (err) {
        return false;
    }
}
```

### Issue: Gestures conflicting with browser defaults

**Solution**: Prevent default behavior on touch events:

```javascript
element.addEventListener('touchstart', (e) => {
    e.preventDefault(); // Prevent browser default
    // Handle touch
}, { passive: false });
```

## Additional Resources

- [MDN Touch Events](https://developer.mozilla.org/en-US/docs/Web/API/Touch_events)
- [CSS Grid Layout](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_Grid_Layout)
- [Responsive Web Design](https://web.dev/responsive-web-design-basics/)
- [Mobile Web Best Practices](https://developers.google.com/web/fundamentals/design-and-ux/principles)

## Support

For issues or questions:
1. Check the MOBILE_IMPLEMENTATION_SUMMARY.md
2. Review the mobile-demo.html for examples
3. Test with the feature detection tool
4. Check browser console for warnings/errors
