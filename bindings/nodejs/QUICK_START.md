# Kizuna Node.js Quick Start

Get started with Kizuna in 5 minutes!

## Installation

```bash
npm install kizuna-node
```

## Basic Usage

```javascript
const { Kizuna } = require('kizuna-node');

async function main() {
  // 1. Create instance
  const kizuna = new Kizuna();
  
  // 2. Initialize
  await kizuna.initialize({
    deviceName: 'My Device',
    enableMdns: true,
    enableEncryption: true
  });
  
  // 3. Set up events
  await kizuna.onEvent((event) => {
    console.log('Event:', event.eventType);
  });
  
  // 4. Discover peers
  const peers = await kizuna.discoverPeers();
  console.log('Found', peers.length, 'peers');
  
  // 5. Transfer a file
  if (peers.length > 0) {
    const handle = await kizuna.transferFile('./file.txt', peers[0].id);
    console.log('Transfer started:', handle.transferId());
  }
  
  // 6. Clean up
  await kizuna.shutdown();
}

main().catch(console.error);
```

## TypeScript

```typescript
import { Kizuna, Config, Peer } from 'kizuna-node';

const config: Config = {
  deviceName: 'My Device',
  enableMdns: true
};

const kizuna = new Kizuna();
await kizuna.initialize(config);

const peers: Peer[] = await kizuna.discoverPeers();
```

## Common Patterns

### Event Handling

```javascript
await kizuna.onEvent((event) => {
  const data = JSON.parse(event.data);
  
  switch (event.eventType) {
    case 'peer_discovered':
      console.log('New peer:', data.name);
      break;
    case 'transfer_progress':
      console.log('Progress:', data.percentage + '%');
      break;
    case 'transfer_completed':
      console.log('Done!', data.success ? '✓' : '✗');
      break;
  }
});
```

### File Transfer with Progress

```javascript
await kizuna.onEvent((event) => {
  if (event.eventType === 'transfer_progress') {
    const { percentage, speedBps } = JSON.parse(event.data);
    console.log(`${percentage.toFixed(1)}% @ ${(speedBps/1024/1024).toFixed(2)} MB/s`);
  }
});

const handle = await kizuna.transferFile('./large-file.zip', peerId);
```

### Media Streaming

```javascript
const stream = await kizuna.startStream({
  streamType: 'camera',
  peerId: peerId,
  quality: 80
});

// Stream for 30 seconds
setTimeout(async () => {
  await stream.stop();
}, 30000);
```

### Error Handling

```javascript
try {
  await kizuna.initialize(config);
} catch (error) {
  console.error('Failed to initialize:', error.message);
}
```

## Configuration Options

```javascript
{
  deviceName: 'My Device',        // Device identifier
  userName: 'John Doe',           // User name
  enableMdns: true,               // mDNS discovery
  enableUdp: true,                // UDP broadcast
  enableBluetooth: false,         // Bluetooth (mobile)
  enableEncryption: true,         // E2E encryption
  requireAuthentication: true,    // Require auth
  listenPort: 8080               // Custom port
}
```

## API Reference

| Method | Description |
|--------|-------------|
| `initialize(config)` | Initialize Kizuna |
| `onEvent(callback)` | Register event listener |
| `discoverPeers()` | Find peers on network |
| `connectToPeer(id)` | Connect to a peer |
| `transferFile(path, id)` | Send a file |
| `startStream(config)` | Start media stream |
| `executeCommand(cmd, id)` | Run remote command |
| `isInitialized()` | Check if ready |
| `shutdown()` | Clean up resources |

## Event Types

- `peer_discovered` - New peer found
- `peer_connected` - Connected to peer
- `peer_disconnected` - Peer disconnected
- `transfer_started` - File transfer began
- `transfer_progress` - Transfer progress update
- `transfer_completed` - Transfer finished
- `stream_started` - Stream began
- `stream_ended` - Stream stopped
- `command_executed` - Command completed
- `error` - Error occurred

## Next Steps

- Read the [full documentation](README.md)
- Check out [advanced examples](examples/advanced.js)
- Learn about [publishing](PUBLISHING.md)
- Report issues on [GitHub](https://github.com/kizuna/kizuna/issues)

## Support

- Documentation: [README.md](README.md)
- Examples: [examples/](examples/)
- Issues: https://github.com/kizuna/kizuna/issues
- npm: https://www.npmjs.com/package/kizuna-node
