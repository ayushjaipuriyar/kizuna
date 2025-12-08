# Kizuna Browser UI Components

This directory contains the web user interface components for the Kizuna browser support system.

## Components

### 1. File Transfer UI (`FileTransferUI`)

**Features:**
- Drag-and-drop file upload interface
- File selection dialog
- Transfer queue management with progress tracking
- Real-time transfer status updates
- Transfer cancellation
- Responsive design for mobile and desktop

**Usage:**
```javascript
const sdk = new KizunaSDK({ apiBaseUrl: 'http://localhost:8080' });
const fileTransferUI = new KizunaUI.FileTransferUI(
    document.getElementById('file-transfer-container'),
    sdk
);
```

**Requirements Validated:**
- 2.3: Drag-and-drop file transfer interface
- 2.4: Transfer progress and status display
- 9.1: Responsive web design
- 9.4: Mobile-optimized file transfer interfaces

### 2. Video Player UI (`VideoPlayerUI`)

**Features:**
- WebRTC video streaming player
- Playback controls (play/pause, volume, fullscreen)
- Adaptive quality selection (auto, high, medium, low)
- Connection status indicators
- Stream statistics display (resolution, bitrate, latency)
- Fullscreen viewing support

**Usage:**
```javascript
const sdk = new KizunaSDK({ apiBaseUrl: 'http://localhost:8080' });
const videoPlayerUI = new KizunaUI.VideoPlayerUI(
    document.getElementById('video-player-container'),
    sdk
);
```

**Requirements Validated:**
- 4.1: Stream camera feeds to browser clients
- 4.2: Display video streams with playback controls
- 4.4: Fullscreen viewing and basic video controls

### 3. Command Terminal UI (`CommandTerminalUI`)

**Features:**
- Web-based terminal interface
- Command input with history navigation (up/down arrows)
- Auto-completion (Tab key)
- Command suggestions
- Real-time command output streaming
- Saved command templates
- Command history persistence (localStorage)

**Usage:**
```javascript
const sdk = new KizunaSDK({ apiBaseUrl: 'http://localhost:8080' });
const terminalUI = new KizunaUI.CommandTerminalUI(
    document.getElementById('terminal-container'),
    sdk
);
```

**Requirements Validated:**
- 5.2: Web-based terminal interface for command execution
- 5.3: Display command output in real-time
- 5.5: Command history and saved command templates

### 4. Peer Management UI (`PeerManagementUI`)

**Features:**
- Peer discovery and listing
- Connection status display
- Peer details panel with:
  - Connection information
  - Connection quality indicators
  - Capability badges
  - Connection actions (connect/disconnect)
  - Connection testing and troubleshooting
- Signal strength visualization
- Real-time peer status updates

**Usage:**
```javascript
const sdk = new KizunaSDK({ apiBaseUrl: 'http://localhost:8080' });
const peerManagementUI = new KizunaUI.PeerManagementUI(
    document.getElementById('peers-container'),
    sdk
);
```

**Requirements Validated:**
- 8.4: Connection status and peer information display
- 1.5: Connection quality indicators and troubleshooting

## Files

- **kizuna-ui.js**: Main UI components implementation
- **kizuna-ui.css**: Responsive styles for all UI components
- **kizuna-sdk.js**: JavaScript SDK for browser-to-peer communication
- **ui-demo.html**: Interactive demo page showcasing all components
- **index.html**: Main browser support landing page
- **connect.html**: Browser connection setup page

## Responsive Design

All UI components are fully responsive and work on:
- Desktop browsers (Chrome, Firefox, Safari, Edge)
- Tablet devices
- Mobile browsers (iOS Safari, Chrome Mobile)

### Breakpoints

- Desktop: > 768px
- Tablet: 481px - 768px
- Mobile: ≤ 480px

## Browser Compatibility

- **Chrome**: Full support
- **Firefox**: Full support
- **Safari**: Full support (with WebRTC limitations on older versions)
- **Edge**: Full support (Chromium-based)
- **Mobile browsers**: Optimized touch interfaces

## Integration with Kizuna SDK

All UI components integrate with the Kizuna SDK (`KizunaSDK`) for:
- Connection management
- File transfer operations
- Video streaming
- Command execution
- Peer discovery and management

The SDK handles:
- WebRTC connection establishment
- WebSocket fallback for unsupported browsers
- Event-driven communication
- Automatic reconnection
- Browser capability detection

## Demo

To view the interactive demo:

1. Start the Kizuna browser support server
2. Navigate to `http://localhost:8080/ui-demo.html`
3. Explore each component using the tab navigation

## Styling

The UI uses CSS custom properties (CSS variables) for theming:

```css
:root {
    --primary-color: #2196F3;
    --success-color: #4CAF50;
    --error-color: #F44336;
    --background: #FAFAFA;
    --surface: #FFFFFF;
    /* ... more variables */
}
```

You can customize the theme by overriding these variables.

## Accessibility

All components follow accessibility best practices:
- Keyboard navigation support
- ARIA labels for screen readers
- Focus indicators
- Semantic HTML structure
- Color contrast compliance

## Performance

- Lazy loading of components
- Efficient DOM updates
- Debounced event handlers
- Optimized animations
- Memory management for long-running sessions

## Future Enhancements

- Internationalization (i18n) support
- Dark mode theme
- Customizable keyboard shortcuts
- Advanced file transfer features (pause/resume)
- Video recording capabilities
- Terminal themes and customization
