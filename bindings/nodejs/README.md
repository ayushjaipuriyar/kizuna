# Kizuna Node.js Bindings

Node.js bindings for Kizuna, providing peer-to-peer file transfer, media streaming, and remote command execution capabilities.

## Features

- 🔍 **Peer Discovery**: Automatic discovery of peers on the network using mDNS, UDP broadcast, and Bluetooth
- 📁 **File Transfer**: Fast, reliable file transfers with progress tracking
- 🎥 **Media Streaming**: Camera, screen, and audio streaming
- 💻 **Command Execution**: Execute commands on remote peers
- 🔒 **Security**: End-to-end encryption and authentication
- ⚡ **Async/Await**: Promise-based API with full async/await support
- 📦 **TypeScript**: Complete TypeScript definitions included

## Installation

```bash
npm install kizuna-node
```

## Quick Start

```javascript
const { Kizuna } = require('kizuna-node');

async function main() {
  // Create and initialize Kizuna
  const kizuna = new Kizuna();
  await kizuna.initialize({
    deviceName: 'My Device',
    enableMdns: true,
    enableEncryption: true
  });

  // Set up event listener for real-time updates
  await kizuna.onEvent((event) => {
    console.log('Event:', event.eventType);
    const data = JSON.parse(event.data);
    
    if (event.eventType === 'peer_discovered') {
      console.log('Found peer:', data.name);
    } else if (event.eventType === 'transfer_progress') {
      console.log(`Transfer progress: ${data.percentage.toFixed(1)}%`);
    }
  });

  // Discover peers
  const peers = await kizuna.discoverPeers();
  console.log(`Found ${peers.length} peers`);

  // Transfer a file
  if (peers.length > 0) {
    const handle = await kizuna.transferFile('./myfile.txt', peers[0].id);
    console.log('Transfer started:', handle.transferId());
  }

  // Clean up
  await kizuna.shutdown();
}

main().catch(console.error);
```

## TypeScript Usage

```typescript
import { Kizuna, Config, Peer, Event } from 'kizuna-node';

const config: Config = {
  deviceName: 'My TypeScript App',
  enableMdns: true,
  enableEncryption: true,
  listenPort: 8080
};

const kizuna = new Kizuna();
await kizuna.initialize(config);

await kizuna.onEvent((event: Event) => {
  console.log('Event type:', event.eventType);
});

const peers: Peer[] = await kizuna.discoverPeers();
```

## API Reference

### Class: Kizuna

#### Constructor

```javascript
const kizuna = new Kizuna();
```

Creates a new Kizuna instance. Does not start any services until `initialize()` is called.

#### initialize(config)

```javascript
await kizuna.initialize({
  deviceName: 'My Device',
  userName: 'John Doe',
  enableMdns: true,
  enableUdp: true,
  enableBluetooth: false,
  enableEncryption: true,
  requireAuthentication: true,
  listenPort: 8080
});
```

Initializes Kizuna with the given configuration. All configuration options are optional.

**Parameters:**
- `config` (Object): Configuration options
  - `deviceName` (string, optional): Name to identify this device
  - `userName` (string, optional): User name for this device
  - `enableMdns` (boolean, optional): Enable mDNS discovery (default: true)
  - `enableUdp` (boolean, optional): Enable UDP broadcast discovery (default: true)
  - `enableBluetooth` (boolean, optional): Enable Bluetooth discovery (default: false)
  - `enableEncryption` (boolean, optional): Enable encryption (default: true)
  - `requireAuthentication` (boolean, optional): Require authentication (default: true)
  - `listenPort` (number, optional): Port to listen on

**Returns:** Promise<void>

#### onEvent(callback)

```javascript
await kizuna.onEvent((event) => {
  console.log('Event:', event.eventType);
  const data = JSON.parse(event.data);
  // Handle event...
});
```

Registers a callback for real-time event notifications. The callback is invoked on the Node.js event loop.

**Event Types:**
- `peer_discovered`: A new peer was discovered
- `peer_connected`: Connected to a peer
- `peer_disconnected`: Disconnected from a peer
- `transfer_started`: File transfer started
- `transfer_progress`: File transfer progress update
- `transfer_completed`: File transfer completed
- `stream_started`: Media stream started
- `stream_ended`: Media stream ended
- `command_executed`: Command execution completed
- `error`: An error occurred

**Parameters:**
- `callback` (Function): Function to call with event data

**Returns:** Promise<void>

#### discoverPeers()

```javascript
const peers = await kizuna.discoverPeers();
peers.forEach(peer => {
  console.log(`${peer.name} (${peer.id})`);
  console.log(`  Addresses: ${peer.addresses.join(', ')}`);
  console.log(`  Capabilities: ${peer.capabilities.join(', ')}`);
});
```

Discovers peers on the network. Returns a snapshot of currently discovered peers.

**Returns:** Promise<Peer[]>

#### connectToPeer(peerId)

```javascript
const connection = await kizuna.connectToPeer(peerId);
console.log('Connected to:', connection.peerId());
```

Establishes a connection to a peer.

**Parameters:**
- `peerId` (string): ID of the peer to connect to

**Returns:** Promise<PeerConnectionHandle>

#### transferFile(filePath, peerId)

```javascript
const handle = await kizuna.transferFile('./document.pdf', peerId);
console.log('Transfer ID:', handle.transferId());

// Cancel the transfer if needed
await handle.cancel();
```

Transfers a file to a peer.

**Parameters:**
- `filePath` (string): Path to the file to transfer
- `peerId` (string): ID of the peer to send to

**Returns:** Promise<TransferHandle>

#### startStream(config)

```javascript
const handle = await kizuna.startStream({
  streamType: 'camera',
  peerId: peerId,
  quality: 80
});
console.log('Stream ID:', handle.streamId());

// Stop the stream when done
await handle.stop();
```

Starts a media stream to a peer.

**Parameters:**
- `config` (Object): Stream configuration
  - `streamType` (string): Type of stream ('camera', 'screen', or 'audio')
  - `peerId` (string): ID of the peer to stream to
  - `quality` (number): Video quality (0-100)

**Returns:** Promise<StreamHandle>

#### executeCommand(command, peerId)

```javascript
const result = await kizuna.executeCommand('ls -la', peerId);
console.log('Exit code:', result.exitCode);
console.log('Output:', result.stdout);
if (result.stderr) {
  console.error('Errors:', result.stderr);
}
```

Executes a command on a remote peer.

**Parameters:**
- `command` (string): Command to execute
- `peerId` (string): ID of the peer to execute on

**Returns:** Promise<CommandResult>

#### isInitialized()

```javascript
const initialized = await kizuna.isInitialized();
if (initialized) {
  console.log('Kizuna is ready');
}
```

Checks if Kizuna is initialized and ready.

**Returns:** Promise<boolean>

#### shutdown()

```javascript
await kizuna.shutdown();
console.log('Kizuna shut down');
```

Shuts down Kizuna and cleans up all resources.

**Returns:** Promise<void>

## Examples

### File Transfer with Progress

```javascript
const { Kizuna } = require('kizuna-node');

async function transferWithProgress() {
  const kizuna = new Kizuna();
  await kizuna.initialize({ deviceName: 'Sender' });

  // Track transfer progress
  await kizuna.onEvent((event) => {
    if (event.eventType === 'transfer_progress') {
      const progress = JSON.parse(event.data);
      const percent = progress.percentage.toFixed(1);
      const speedMB = (progress.speedBps / 1024 / 1024).toFixed(2);
      console.log(`Progress: ${percent}% (${speedMB} MB/s)`);
    } else if (event.eventType === 'transfer_completed') {
      const result = JSON.parse(event.data);
      if (result.success) {
        console.log('Transfer completed successfully!');
      } else {
        console.error('Transfer failed:', result.error);
      }
    }
  });

  const peers = await kizuna.discoverPeers();
  if (peers.length > 0) {
    await kizuna.transferFile('./largefile.zip', peers[0].id);
  }

  // Keep running to receive progress events
  await new Promise(resolve => setTimeout(resolve, 60000));
  await kizuna.shutdown();
}

transferWithProgress().catch(console.error);
```

### Peer Discovery Monitor

```javascript
const { Kizuna } = require('kizuna-node');

async function monitorPeers() {
  const kizuna = new Kizuna();
  await kizuna.initialize({
    deviceName: 'Monitor',
    enableMdns: true,
    enableUdp: true
  });

  const discoveredPeers = new Set();

  await kizuna.onEvent((event) => {
    if (event.eventType === 'peer_discovered') {
      const peer = JSON.parse(event.data);
      if (!discoveredPeers.has(peer.id)) {
        discoveredPeers.add(peer.id);
        console.log(`New peer: ${peer.name}`);
        console.log(`  ID: ${peer.id}`);
        console.log(`  Method: ${peer.discoveryMethod}`);
        console.log(`  Addresses: ${peer.addresses.join(', ')}`);
      }
    } else if (event.eventType === 'peer_disconnected') {
      const data = JSON.parse(event.data);
      console.log(`Peer disconnected: ${data.peerId}`);
      discoveredPeers.delete(data.peerId);
    }
  });

  // Keep monitoring
  console.log('Monitoring for peers... Press Ctrl+C to stop');
  await new Promise(() => {}); // Run forever
}

monitorPeers().catch(console.error);
```

### Media Streaming

```javascript
const { Kizuna } = require('kizuna-node');

async function streamCamera() {
  const kizuna = new Kizuna();
  await kizuna.initialize({ deviceName: 'Streamer' });

  const peers = await kizuna.discoverPeers();
  if (peers.length === 0) {
    console.log('No peers found');
    return;
  }

  const streamHandle = await kizuna.startStream({
    streamType: 'camera',
    peerId: peers[0].id,
    quality: 80
  });

  console.log('Streaming camera to', peers[0].name);
  console.log('Stream ID:', streamHandle.streamId());

  // Stream for 30 seconds
  await new Promise(resolve => setTimeout(resolve, 30000));

  await streamHandle.stop();
  console.log('Stream stopped');

  await kizuna.shutdown();
}

streamCamera().catch(console.error);
```

## Error Handling

All async methods can throw errors. Always use try-catch or .catch():

```javascript
try {
  await kizuna.initialize(config);
} catch (error) {
  console.error('Failed to initialize:', error.message);
}

// Or with promises
kizuna.discoverPeers()
  .then(peers => console.log('Found peers:', peers))
  .catch(error => console.error('Discovery failed:', error));
```

## Platform Support

- Linux (x64, ARM64)
- macOS (x64, ARM64/Apple Silicon)
- Windows (x64)

## License

MIT

## Contributing

Contributions are welcome! Please see the main Kizuna repository for contribution guidelines.
