# Kizuna Flutter Plugin

A Flutter plugin for Kizuna - seamless peer-to-peer connectivity for file sharing, screen streaming, and device communication.

## Features

- **Peer Discovery**: Automatically discover nearby devices using mDNS, UDP broadcast, and Bluetooth
- **File Transfer**: Fast and reliable file transfers with resume support
- **Screen Streaming**: Share your screen with other devices in real-time
- **Camera Streaming**: Stream camera feed to other devices
- **Command Execution**: Execute commands on remote devices (with proper authorization)
- **Cross-Platform**: Works on Android, iOS, Windows, macOS, and Linux

## Platform Support

| Platform | Discovery | File Transfer | Screen Streaming | Camera Streaming |
|----------|-----------|---------------|------------------|------------------|
| Android  | ✅        | ✅            | ❌               | ✅               |
| iOS      | ✅        | ✅            | ❌               | ✅               |
| Windows  | ✅        | ✅            | ✅               | ✅               |
| macOS    | ✅        | ✅            | ✅               | ✅               |
| Linux    | ✅        | ✅            | ✅               | ✅               |

## Installation

Add this to your package's `pubspec.yaml` file:

```yaml
dependencies:
  kizuna: ^0.1.0
```

Then run:

```bash
flutter pub get
```

## Usage

### Initialize Kizuna

```dart
import 'package:kizuna/kizuna.dart';

void main() async {
  // Create Kizuna instance
  final kizuna = Kizuna();
  
  // Configure and initialize
  final config = KizunaConfig(
    deviceName: 'My Flutter Device',
    userName: 'John Doe',
    enableMdns: true,
    enableUdp: true,
    enableEncryption: true,
    requireAuthentication: true,
  );
  
  await kizuna.initialize(config);
}
```

### Discover Peers

```dart
// Discover peers on the network
final peers = await kizuna.discoverPeers();

for (final peer in peers) {
  print('Found peer: ${peer.name} (${peer.id})');
}
```

### Connect to a Peer

```dart
// Connect to a discovered peer
final connection = await kizuna.connectToPeer(peer.id);
print('Connected to ${connection.peerId}');
```

### Transfer Files

```dart
// Transfer a file to a peer
final transferHandle = await kizuna.transferFile(
  '/path/to/file.txt',
  peer.id,
);

print('Transfer started: ${transferHandle.transferId}');
```

### Start Screen Streaming

```dart
// Start screen streaming (desktop only)
final streamConfig = StreamConfig(
  streamType: 'screen',
  peerId: peer.id,
  quality: 80,
);

final streamHandle = await kizuna.startStream(streamConfig);
print('Stream started: ${streamHandle.streamId}');
```

### Start Camera Streaming

```dart
// Start camera streaming
final streamConfig = StreamConfig(
  streamType: 'camera',
  peerId: peer.id,
  quality: 80,
);

final streamHandle = await kizuna.startStream(streamConfig);
print('Camera stream started: ${streamHandle.streamId}');
```

### Execute Commands

```dart
// Execute a command on a remote peer
final result = await kizuna.executeCommand('ls -la', peer.id);
print('Exit code: ${result.exitCode}');
print('Output: ${result.stdout}');
```

### Listen to Events

```dart
// Subscribe to events
while (true) {
  final event = await kizuna.getNextEvent();
  if (event != null) {
    print('Event: ${event.eventType}');
    print('Data: ${event.data}');
  }
}
```

### Shutdown

```dart
// Shutdown Kizuna when done
await kizuna.shutdown();
```

## Platform-Specific Features

### Check Platform Support

```dart
import 'package:kizuna/kizuna.dart';

// Get current platform information
final platformInfo = PlatformInfo.getCurrentPlatform();
print('Platform: ${platformInfo.platform}');
print('Supported features: ${platformInfo.supportedFeatures}');

// Check if a specific feature is supported
if (platformInfo.isFeatureSupported('screen_streaming')) {
  print('Screen streaming is supported on this platform');
}
```

### Platform Optimizations

```dart
import 'package:kizuna/kizuna.dart';

// Get recommended buffer size for the platform
final bufferSize = PlatformOptimizations.getRecommendedBufferSize();
print('Recommended buffer size: $bufferSize bytes');

// Get max concurrent transfers
final maxTransfers = PlatformOptimizations.getMaxConcurrentTransfers();
print('Max concurrent transfers: $maxTransfers');

// Check background execution support
final supportsBackground = PlatformOptimizations.supportsBackgroundExecution();
print('Supports background execution: $supportsBackground');

// Get network preferences
final networkPrefs = PlatformOptimizations.getNetworkPreferences();
print('Prefer WiFi: ${networkPrefs.preferWifi}');
print('Allow cellular: ${networkPrefs.allowCellular}');
```

## Configuration Options

### KizunaConfig

- `deviceName`: Name of this device (optional)
- `userName`: Name of the user (optional)
- `enableMdns`: Enable mDNS discovery (default: true)
- `enableUdp`: Enable UDP broadcast discovery (default: true)
- `enableBluetooth`: Enable Bluetooth discovery (default: false)
- `enableEncryption`: Enable end-to-end encryption (default: true)
- `requireAuthentication`: Require peer authentication (default: true)
- `trustMode`: Trust mode - "trust_all", "manual", or "allowlist_only" (default: "manual")
- `listenPort`: Port to listen on (optional)
- `enableIpv6`: Enable IPv6 support (default: true)
- `enableQuic`: Enable QUIC protocol (default: true)
- `enableWebrtc`: Enable WebRTC protocol (default: true)
- `enableWebsocket`: Enable WebSocket protocol (default: true)

## Security

Kizuna uses industry-standard encryption and authentication:

- **End-to-end encryption** using ChaCha20-Poly1305
- **Key exchange** using X25519 (Elliptic Curve Diffie-Hellman)
- **Identity verification** using Ed25519 signatures
- **Trust management** with manual approval or allowlist modes

## Examples

See the [example](example/) directory for complete example applications:

- Basic peer discovery and file transfer
- Screen streaming application
- Camera streaming application
- Multi-platform demo

## Troubleshooting

### Android

- Ensure you have the required permissions in `AndroidManifest.xml`:
  ```xml
  <uses-permission android:name="android.permission.INTERNET" />
  <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
  <uses-permission android:name="android.permission.ACCESS_WIFI_STATE" />
  <uses-permission android:name="android.permission.CAMERA" />
  ```

### iOS

- Add required permissions to `Info.plist`:
  ```xml
  <key>NSCameraUsageDescription</key>
  <string>Camera access is required for video streaming</string>
  <key>NSLocalNetworkUsageDescription</key>
  <string>Local network access is required for peer discovery</string>
  ```

### macOS

- Enable network entitlements in your app's entitlements file

### Windows/Linux

- Ensure firewall allows the application to accept incoming connections

## Contributing

Contributions are welcome! Please read our [contributing guidelines](CONTRIBUTING.md) before submitting pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- Documentation: https://kizuna.dev/docs
- Issues: https://github.com/kizuna/kizuna/issues
- Discussions: https://github.com/kizuna/kizuna/discussions
