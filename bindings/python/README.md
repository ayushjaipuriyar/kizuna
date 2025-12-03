# Kizuna Python Bindings

Python bindings for Kizuna - a secure peer-to-peer communication library for device discovery, file transfer, media streaming, and remote command execution.

## Installation

```bash
pip install kizuna
```

## Features

- **Peer Discovery**: Automatically discover devices on your local network using mDNS, UDP broadcast, or Bluetooth
- **File Transfer**: Securely transfer files between devices with progress tracking
- **Media Streaming**: Stream camera, screen, or audio to other devices
- **Remote Commands**: Execute commands on remote devices (with proper authorization)
- **Async/Await Support**: Full asyncio compatibility for modern Python applications
- **Type Hints**: Complete type annotations for better IDE support and type checking

## Quick Start

```python
import asyncio
from kizuna import Kizuna

async def main():
    # Initialize Kizuna
    kizuna = Kizuna()
    
    # Discover peers
    peers = await kizuna.discover_peers()
    print(f"Found {len(peers)} peers")
    
    for peer in peers:
        print(f"  - {peer.name} ({peer.id})")
    
    # Connect to a peer and transfer a file
    if peers:
        peer_id = peers[0].id
        connection = await kizuna.connect_to_peer(peer_id)
        print(f"Connected to {connection.peer_id}")
        
        # Transfer a file
        handle = await kizuna.transfer_file("document.pdf", peer_id)
        print(f"Transfer started: {handle.transfer_id}")
    
    # Cleanup
    await kizuna.shutdown()

if __name__ == "__main__":
    asyncio.run(main())
```

## Configuration

You can customize Kizuna's behavior by passing a configuration dictionary:

```python
config = {
    "identity": {
        "device_name": "My Python App",
        "user_name": "Alice"
    },
    "discovery": {
        "enable_mdns": True,
        "enable_udp": True,
        "enable_bluetooth": False,
        "interval_secs": 5,
        "timeout_secs": 30
    },
    "security": {
        "enable_encryption": True,
        "require_authentication": True,
        "trust_mode": "manual"  # Options: "trust_all", "manual", "allowlist_only"
    },
    "networking": {
        "listen_port": 8080,
        "enable_ipv6": True,
        "enable_quic": True,
        "enable_webrtc": True,
        "enable_websocket": True,
        "connection_timeout_secs": 30
    }
}

kizuna = Kizuna(config)
```

## API Reference

### Kizuna

Main class for interacting with the Kizuna library.

#### `__init__(config: Optional[Dict] = None)`

Initialize a new Kizuna instance.

**Parameters:**
- `config` (dict, optional): Configuration dictionary

**Raises:**
- `RuntimeError`: If initialization fails

#### `async discover_peers() -> List[PeerInfo]`

Discover peers on the local network.

**Returns:**
- List of `PeerInfo` objects

**Example:**
```python
peers = await kizuna.discover_peers()
for peer in peers:
    print(f"{peer.name}: {peer.addresses}")
```

#### `async connect_to_peer(peer_id: str) -> PeerConnection`

Establish a connection to a peer.

**Parameters:**
- `peer_id` (str): Unique identifier of the peer

**Returns:**
- `PeerConnection` object

**Example:**
```python
connection = await kizuna.connect_to_peer("peer-123")
```

#### `async transfer_file(file_path: str, peer_id: str) -> TransferHandle`

Transfer a file to a peer.

**Parameters:**
- `file_path` (str): Path to the file to transfer
- `peer_id` (str): Unique identifier of the destination peer

**Returns:**
- `TransferHandle` object

**Example:**
```python
handle = await kizuna.transfer_file("/path/to/file.txt", peer_id)
print(f"Transfer ID: {handle.transfer_id}")
```

#### `async start_stream(stream_type: str, peer_id: str, quality: int = 80) -> StreamHandle`

Start a media stream to a peer.

**Parameters:**
- `stream_type` (str): Type of stream - "camera", "screen", or "audio"
- `peer_id` (str): Unique identifier of the destination peer
- `quality` (int, optional): Stream quality 0-100 (default: 80)

**Returns:**
- `StreamHandle` object

**Example:**
```python
# Start screen sharing
stream = await kizuna.start_stream("screen", peer_id, quality=90)
```

#### `async execute_command(command: str, peer_id: str) -> CommandResult`

Execute a command on a remote peer.

**Parameters:**
- `command` (str): Command to execute
- `peer_id` (str): Unique identifier of the target peer

**Returns:**
- `CommandResult` object with exit code and output

**Example:**
```python
result = await kizuna.execute_command("ls -la", peer_id)
print(f"Exit code: {result.exit_code}")
print(f"Output:\n{result.stdout}")
```

#### `async subscribe_events() -> List[KizunaEvent]`

Subscribe to Kizuna events.

**Returns:**
- List of `KizunaEvent` objects

**Example:**
```python
events = await kizuna.subscribe_events()
for event in events:
    print(f"Event: {event.event_type}")
```

#### `async shutdown() -> None`

Shutdown the Kizuna instance and clean up resources.

**Example:**
```python
await kizuna.shutdown()
```

### Data Classes

#### PeerInfo

Information about a discovered peer.

**Attributes:**
- `id` (str): Unique identifier
- `name` (str): Human-readable name
- `addresses` (List[str]): Network addresses
- `capabilities` (List[str]): Supported capabilities
- `discovery_method` (str): Discovery method used

#### PeerConnection

Represents an active connection to a peer.

**Attributes:**
- `peer_id` (str): Unique identifier of the connected peer

#### TransferHandle

Handle for monitoring a file transfer.

**Attributes:**
- `transfer_id` (str): Unique identifier for the transfer

#### StreamHandle

Handle for controlling a media stream.

**Attributes:**
- `stream_id` (str): Unique identifier for the stream

#### CommandResult

Result of a remote command execution.

**Attributes:**
- `exit_code` (int): Exit code
- `stdout` (str): Standard output
- `stderr` (str): Standard error output

#### KizunaEvent

Event emitted by the Kizuna system.

**Attributes:**
- `event_type` (str): Type of event
- `data` (str): JSON-encoded event data

#### TransferProgress

Progress information for a file transfer.

**Attributes:**
- `transfer_id` (str): Transfer identifier
- `bytes_transferred` (int): Bytes transferred
- `total_bytes` (int): Total bytes
- `speed_bps` (int): Transfer speed in bytes/second

**Methods:**
- `percentage() -> float`: Calculate progress percentage

## Advanced Usage

### File Transfer with Progress Monitoring

```python
import asyncio
from kizuna import Kizuna

async def transfer_with_progress():
    kizuna = Kizuna()
    peers = await kizuna.discover_peers()
    
    if peers:
        peer_id = peers[0].id
        handle = await kizuna.transfer_file("large_file.zip", peer_id)
        
        # Monitor progress (you would implement polling or event-based monitoring)
        print(f"Transfer started: {handle.transfer_id}")
    
    await kizuna.shutdown()

asyncio.run(transfer_with_progress())
```

### Event Monitoring

```python
import asyncio
from kizuna import Kizuna

async def monitor_events():
    kizuna = Kizuna()
    
    # Subscribe to events
    events = await kizuna.subscribe_events()
    
    for event in events:
        if event.event_type == "peer_discovered":
            print(f"New peer discovered: {event.data}")
        elif event.event_type == "transfer_completed":
            print(f"Transfer completed: {event.data}")
    
    await kizuna.shutdown()

asyncio.run(monitor_events())
```

### Custom Configuration

```python
import asyncio
from kizuna import Kizuna

async def custom_config():
    # Configure for high-security environment
    config = {
        "security": {
            "enable_encryption": True,
            "require_authentication": True,
            "trust_mode": "allowlist_only"
        },
        "discovery": {
            "enable_mdns": True,
            "enable_udp": False,  # Disable UDP broadcast
            "enable_bluetooth": False
        }
    }
    
    kizuna = Kizuna(config)
    peers = await kizuna.discover_peers()
    print(f"Found {len(peers)} trusted peers")
    
    await kizuna.shutdown()

asyncio.run(custom_config())
```

## Error Handling

All async methods can raise `RuntimeError` if operations fail. Always use try-except blocks:

```python
try:
    peers = await kizuna.discover_peers()
except RuntimeError as e:
    print(f"Discovery failed: {e}")

try:
    connection = await kizuna.connect_to_peer(peer_id)
except RuntimeError as e:
    print(f"Connection failed: {e}")
```

## Type Checking

The library includes comprehensive type hints. Use with mypy or other type checkers:

```bash
pip install mypy
mypy your_script.py
```

## Requirements

- Python 3.8 or higher
- asyncio support

## Platform Support

- Linux (x86_64, ARM64)
- macOS (x86_64, ARM64)
- Windows (x86_64)

## License

See the main Kizuna repository for license information.

## Contributing

Contributions are welcome! Please see the main Kizuna repository for contribution guidelines.

## Support

For issues and questions:
- GitHub Issues: [kizuna/issues](https://github.com/kizuna/kizuna/issues)
- Documentation: [kizuna.dev/docs](https://kizuna.dev/docs)
