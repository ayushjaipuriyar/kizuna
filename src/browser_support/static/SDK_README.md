# Kizuna Browser SDK

The Kizuna Browser SDK provides a comprehensive JavaScript API for browser clients to interact with Kizuna peers. It supports WebRTC and WebSocket connections, file transfer, clipboard synchronization, and command execution.

## Features

- **Connection Management**: WebRTC and WebSocket support with automatic fallback
- **File Transfer**: Upload/download with chunking, progress tracking, and resume capability
- **Clipboard Sync**: Automatic clipboard synchronization with permission handling
- **Command Execution**: Remote command execution with real-time output streaming
- **Event System**: Comprehensive event system for real-time updates
- **Promise & Callback Support**: Both modern Promise-based and traditional callback patterns

## Installation

Include the SDK scripts in your HTML:

```html
<script src="kizuna-sdk.js"></script>
<script src="kizuna-file-transfer.js"></script>
<script src="kizuna-clipboard.js"></script>
<script src="kizuna-command.js"></script>
```

## Quick Start

### 1. Initialize the SDK

```javascript
const sdk = new KizunaSDK({
    apiBaseUrl: 'http://localhost:8080',
    autoReconnect: true,
    debug: true
});
```

### 2. Connect to a Peer

```javascript
// Connect using setup ID
await sdk.connect('setup-id-here');

// Listen for connection events
sdk.on('connected', (data) => {
    console.log('Connected via', data.protocol);
});

sdk.on('disconnected', (data) => {
    console.log('Disconnected:', data.reason);
});
```

### 3. File Transfer

```javascript
const fileTransfer = new KizunaFileTransfer(sdk);

// Upload a file
const file = document.getElementById('file-input').files[0];
await fileTransfer.uploadFile(file, 'peer-id', {
    onProgress: (progress) => {
        console.log(`Progress: ${progress.progress}%`);
    },
    onComplete: (transfer) => {
        console.log('Upload complete!');
    }
});

// Download a file
await fileTransfer.downloadFile('filename.txt', 'peer-id', {
    onProgress: (progress) => {
        console.log(`Downloaded: ${progress.bytesTransferred} bytes`);
    }
});

// Setup drag and drop
fileTransfer.setupDragAndDrop(element, {
    peerId: 'peer-id',
    onFilesSelected: (files) => {
        console.log(`${files.length} files selected`);
    }
});
```

### 4. Clipboard Synchronization

```javascript
const clipboard = new KizunaClipboard(sdk);

// Request permission
await clipboard.requestPermission();

// Enable clipboard sync
await clipboard.enable({ autoSync: true });

// Listen for clipboard events
sdk.on('clipboardSynced', ({ direction, content }) => {
    console.log(`Clipboard synced (${direction}):`, content);
});

// Manually copy to clipboard
await clipboard.copyToClipboard('Hello, World!');

// Manually paste from clipboard
const content = await clipboard.pasteFromClipboard();
```

### 5. Command Execution

```javascript
const commandManager = new KizunaCommand(sdk);

// Execute a command
const execution = await commandManager.executeCommand('ls -la', 'peer-id', {
    onOutput: (output) => {
        console.log(output.content);
    },
    onComplete: (execution) => {
        console.log('Exit code:', execution.exitCode);
    }
});

// Create a terminal interface
const terminal = new KizunaTerminal(commandManager, containerElement);
terminal.setPeerId('peer-id');

// Save command templates
commandManager.saveTemplate('list-files', 'ls -la {directory}');

// Execute template
await commandManager.executeTemplate('list-files', 'peer-id', {
    directory: '/home/user'
});
```

## API Reference

### KizunaSDK

#### Constructor Options

```javascript
{
    apiBaseUrl: string,           // API base URL (default: window.location.origin)
    autoReconnect: boolean,       // Auto-reconnect on disconnect (default: true)
    reconnectInterval: number,    // Reconnect interval in ms (default: 5000)
    maxReconnectAttempts: number, // Max reconnect attempts (default: 5)
    debug: boolean                // Enable debug logging (default: false)
}
```

#### Methods

- `connect(setupId, options)` - Connect to a peer
- `disconnect()` - Disconnect from peer
- `sendMessage(message, channel)` - Send a message
- `getConnectionStatus()` - Get connection status
- `on(eventName, callback)` - Register event listener
- `off(eventName, callback)` - Remove event listener
- `once(eventName, callback)` - Register one-time listener

#### Events

- `connectionStateChange` - Connection state changed
- `connected` - Connected to peer
- `disconnected` - Disconnected from peer
- `reconnecting` - Attempting to reconnect
- `error` - Error occurred
- `message` - Message received
- `dataChannelOpen` - Data channel opened
- `dataChannelClose` - Data channel closed

### FileTransferManager

#### Methods

- `uploadFile(file, peerId, options)` - Upload a file
- `downloadFile(fileName, peerId, options)` - Download a file
- `pauseTransfer(transferId)` - Pause a transfer
- `resumeTransfer(transferId)` - Resume a transfer
- `cancelTransfer(transferId)` - Cancel a transfer
- `getActiveTransfers()` - Get active transfers
- `getTransferHistory()` - Get transfer history
- `setupDragAndDrop(element, options)` - Setup drag and drop

#### Events

- `fileTransferStarted` - Transfer started
- `fileTransferProgress` - Transfer progress update
- `fileTransferComplete` - Transfer completed
- `fileTransferError` - Transfer error
- `fileTransferPaused` - Transfer paused
- `fileTransferResumed` - Transfer resumed
- `fileTransferCancelled` - Transfer cancelled

### ClipboardManager

#### Methods

- `isSupported()` - Check if clipboard API is supported
- `requestPermission()` - Request clipboard permissions
- `enable(options)` - Enable clipboard sync
- `disable()` - Disable clipboard sync
- `copyToClipboard(text)` - Copy text to clipboard
- `pasteFromClipboard()` - Paste from clipboard
- `requestRemoteClipboard()` - Request remote clipboard content
- `setAutoSync(enabled)` - Enable/disable auto-sync
- `clearClipboard()` - Clear clipboard
- `getStatus()` - Get clipboard status

#### Events

- `clipboardEnabled` - Clipboard sync enabled
- `clipboardDisabled` - Clipboard sync disabled
- `clipboardSynced` - Clipboard synced
- `clipboardPermissionGranted` - Permission granted
- `clipboardPermissionDenied` - Permission denied
- `clipboardPermissionRevoked` - Permission revoked
- `clipboardSyncError` - Sync error

### CommandExecutionManager

#### Methods

- `executeCommand(command, peerId, options)` - Execute a command
- `terminateCommand(commandId)` - Terminate a command
- `sendInput(commandId, input)` - Send input to command
- `authorizeCommand(commandId, approved)` - Authorize command
- `getActiveCommands()` - Get active commands
- `getHistory()` - Get command history
- `clearHistory()` - Clear history
- `searchHistory(query)` - Search history
- `saveTemplate(name, command, metadata)` - Save template
- `getTemplate(name)` - Get template
- `executeTemplate(name, peerId, variables, options)` - Execute template

#### Events

- `commandStarted` - Command started
- `commandAccepted` - Command accepted
- `commandOutput` - Command output received
- `commandComplete` - Command completed
- `commandError` - Command error
- `commandTerminated` - Command terminated
- `commandAuthorizationRequired` - Authorization required

### TerminalInterface

#### Constructor

```javascript
const terminal = new KizunaTerminal(commandManager, containerElement);
```

#### Methods

- `setPeerId(peerId)` - Set target peer ID
- `clear()` - Clear terminal
- `focus()` - Focus input

#### Features

- Command history navigation (Up/Down arrows)
- Tab completion from history
- Built-in commands: `clear`, `history`

## Browser Compatibility

- **Chrome/Edge**: Full support (WebRTC + Clipboard API)
- **Firefox**: Full support (WebRTC + Clipboard API)
- **Safari**: WebRTC support with some limitations, Clipboard API supported
- **Mobile browsers**: WebSocket fallback recommended

## Security Considerations

1. **HTTPS Required**: Clipboard API and some WebRTC features require HTTPS
2. **Permissions**: User must grant clipboard permissions
3. **CORS**: Ensure proper CORS configuration on the server
4. **CSP**: Configure Content Security Policy appropriately

## Examples

See `sdk-demo.html` for a complete working example demonstrating all SDK features.

## Error Handling

```javascript
try {
    await sdk.connect(setupId);
} catch (error) {
    console.error('Connection failed:', error);
}

// Or use event listeners
sdk.on('error', ({ type, error }) => {
    console.error(`Error (${type}):`, error);
});
```

## Best Practices

1. **Always check connection state** before sending messages
2. **Handle permission denials** gracefully for clipboard
3. **Implement progress callbacks** for file transfers
4. **Use templates** for frequently used commands
5. **Enable debug mode** during development
6. **Implement proper error handling** for all async operations

## License

See main Kizuna project license.
