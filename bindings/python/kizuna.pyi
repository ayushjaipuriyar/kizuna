"""
Kizuna Python Bindings - Type Stubs

This module provides Python bindings for the Kizuna peer-to-peer communication library.
Kizuna enables secure device discovery, file transfer, media streaming, and remote command execution.
"""

from typing import Optional, List, Dict, Any, AsyncIterator
from typing_extensions import Literal

class Kizuna:
    """
    Main Kizuna API class for peer-to-peer communication.
    
    This class provides the primary interface for all Kizuna functionality including
    peer discovery, file transfers, media streaming, and remote command execution.
    
    Example:
        ```python
        import asyncio
        from kizuna import Kizuna
        
        async def main():
            # Initialize Kizuna with default configuration
            kizuna = Kizuna()
            
            # Discover peers on the network
            peers = await kizuna.discover_peers()
            for peer in peers:
                print(f"Found peer: {peer.name} ({peer.id})")
            
            # Shutdown when done
            await kizuna.shutdown()
        
        asyncio.run(main())
        ```
    """
    
    def __init__(self, config: Optional[Dict[str, Any]] = None) -> None:
        """
        Initialize a new Kizuna instance.
        
        Args:
            config: Optional configuration dictionary. If not provided, uses default configuration.
                   See KizunaConfig for available options.
        
        Raises:
            RuntimeError: If initialization fails or configuration is invalid.
        
        Example:
            ```python
            # Initialize with default config
            kizuna = Kizuna()
            
            # Initialize with custom config
            kizuna = Kizuna({
                "identity": {
                    "device_name": "My Device",
                    "user_name": "John Doe"
                },
                "discovery": {
                    "enable_mdns": True,
                    "enable_udp": True,
                    "interval_secs": 5
                },
                "security": {
                    "enable_encryption": True,
                    "trust_mode": "manual"
                }
            })
            ```
        """
        ...
    
    async def discover_peers(self) -> List[PeerInfo]:
        """
        Discover peers on the local network.
        
        This method initiates peer discovery using configured discovery methods
        (mDNS, UDP broadcast, Bluetooth) and returns a list of discovered peers.
        
        Returns:
            List of PeerInfo objects representing discovered peers.
        
        Raises:
            RuntimeError: If discovery fails or the system is not initialized.
        
        Example:
            ```python
            peers = await kizuna.discover_peers()
            for peer in peers:
                print(f"Discovered: {peer.name} at {peer.addresses}")
            ```
        """
        ...
    
    async def connect_to_peer(self, peer_id: str) -> PeerConnection:
        """
        Establish a connection to a peer.
        
        Args:
            peer_id: The unique identifier of the peer to connect to.
        
        Returns:
            PeerConnection object representing the established connection.
        
        Raises:
            RuntimeError: If connection fails or peer is not reachable.
        
        Example:
            ```python
            peers = await kizuna.discover_peers()
            if peers:
                connection = await kizuna.connect_to_peer(peers[0].id)
                print(f"Connected to {connection.peer_id}")
            ```
        """
        ...
    
    async def transfer_file(self, file_path: str, peer_id: str) -> TransferHandle:
        """
        Transfer a file to a peer.
        
        Args:
            file_path: Path to the file to transfer.
            peer_id: The unique identifier of the destination peer.
        
        Returns:
            TransferHandle object for monitoring and controlling the transfer.
        
        Raises:
            RuntimeError: If transfer fails to start or file doesn't exist.
        
        Example:
            ```python
            handle = await kizuna.transfer_file("/path/to/file.txt", peer_id)
            print(f"Transfer started: {handle.transfer_id}")
            ```
        """
        ...
    
    async def start_stream(
        self, 
        stream_type: Literal["camera", "screen", "audio"], 
        peer_id: str, 
        quality: int = 80
    ) -> StreamHandle:
        """
        Start a media stream to a peer.
        
        Args:
            stream_type: Type of stream - "camera", "screen", or "audio".
            peer_id: The unique identifier of the destination peer.
            quality: Stream quality from 0-100 (default: 80).
        
        Returns:
            StreamHandle object for controlling the stream.
        
        Raises:
            RuntimeError: If stream fails to start.
            ValueError: If stream_type is invalid or quality is out of range.
        
        Example:
            ```python
            # Start screen sharing
            stream = await kizuna.start_stream("screen", peer_id, quality=90)
            print(f"Stream started: {stream.stream_id}")
            ```
        """
        ...
    
    async def execute_command(self, command: str, peer_id: str) -> CommandResult:
        """
        Execute a command on a remote peer.
        
        Args:
            command: The command to execute.
            peer_id: The unique identifier of the target peer.
        
        Returns:
            CommandResult object containing exit code and output.
        
        Raises:
            RuntimeError: If command execution fails or is not supported.
        
        Example:
            ```python
            result = await kizuna.execute_command("ls -la", peer_id)
            print(f"Exit code: {result.exit_code}")
            print(f"Output: {result.stdout}")
            ```
        """
        ...
    
    async def subscribe_events(self) -> List[KizunaEvent]:
        """
        Subscribe to Kizuna events.
        
        Returns a list of events that have occurred. For real-time event monitoring,
        consider implementing a polling mechanism or event callback system.
        
        Returns:
            List of KizunaEvent objects.
        
        Raises:
            RuntimeError: If event subscription fails.
        
        Example:
            ```python
            events = await kizuna.subscribe_events()
            for event in events:
                print(f"Event: {event.event_type}")
            ```
        """
        ...
    
    async def shutdown(self) -> None:
        """
        Shutdown the Kizuna instance and clean up resources.
        
        This method should be called when you're done using Kizuna to ensure
        proper cleanup of network connections and system resources.
        
        Raises:
            RuntimeError: If shutdown fails.
        
        Example:
            ```python
            await kizuna.shutdown()
            ```
        """
        ...


class PeerInfo:
    """
    Information about a discovered peer.
    
    Attributes:
        id: Unique identifier for the peer.
        name: Human-readable name of the peer.
        addresses: List of network addresses where the peer can be reached.
        capabilities: List of capabilities supported by the peer.
        discovery_method: Method used to discover this peer (e.g., "mdns", "udp").
    """
    
    id: str
    name: str
    addresses: List[str]
    capabilities: List[str]
    discovery_method: str
    
    def __repr__(self) -> str: ...


class PeerConnection:
    """
    Represents an active connection to a peer.
    
    Attributes:
        peer_id: Unique identifier of the connected peer.
    """
    
    peer_id: str
    
    def __repr__(self) -> str: ...


class TransferHandle:
    """
    Handle for monitoring and controlling a file transfer.
    
    Attributes:
        transfer_id: Unique identifier for this transfer.
    """
    
    transfer_id: str
    
    def __repr__(self) -> str: ...


class StreamHandle:
    """
    Handle for controlling a media stream.
    
    Attributes:
        stream_id: Unique identifier for this stream.
    """
    
    stream_id: str
    
    def __repr__(self) -> str: ...


class CommandResult:
    """
    Result of a remote command execution.
    
    Attributes:
        exit_code: Exit code returned by the command.
        stdout: Standard output from the command.
        stderr: Standard error output from the command.
    """
    
    exit_code: int
    stdout: str
    stderr: str
    
    def __repr__(self) -> str: ...


class KizunaEvent:
    """
    Event emitted by the Kizuna system.
    
    Attributes:
        event_type: Type of event (e.g., "peer_discovered", "transfer_started").
        data: JSON-encoded event data.
    """
    
    event_type: str
    data: str
    
    def __repr__(self) -> str: ...


class TransferProgress:
    """
    Progress information for a file transfer.
    
    Attributes:
        transfer_id: Unique identifier for the transfer.
        bytes_transferred: Number of bytes transferred so far.
        total_bytes: Total number of bytes to transfer.
        speed_bps: Current transfer speed in bytes per second.
    """
    
    transfer_id: str
    bytes_transferred: int
    total_bytes: int
    speed_bps: int
    
    def percentage(self) -> float:
        """
        Calculate the transfer progress as a percentage.
        
        Returns:
            Progress percentage (0.0 to 100.0).
        """
        ...
    
    def __repr__(self) -> str: ...


# Type definitions for configuration

class KizunaConfig:
    """
    Configuration for Kizuna instance.
    
    This is a TypedDict-style configuration object passed to Kizuna.__init__().
    """
    identity: Optional['IdentityConfig']
    discovery: Optional['DiscoveryConfig']
    security: Optional['SecurityConfig']
    networking: Optional['NetworkConfig']
    plugins: Optional[List['PluginConfig']]


class IdentityConfig:
    """
    Identity configuration.
    
    Attributes:
        device_name: Name of this device.
        user_name: Optional user name.
        identity_path: Optional path to identity file.
    """
    device_name: str
    user_name: Optional[str]
    identity_path: Optional[str]


class DiscoveryConfig:
    """
    Discovery configuration.
    
    Attributes:
        enable_mdns: Enable mDNS discovery (default: True).
        enable_udp: Enable UDP broadcast discovery (default: True).
        enable_bluetooth: Enable Bluetooth discovery (default: False).
        interval_secs: Discovery interval in seconds (default: 5).
        timeout_secs: Discovery timeout in seconds (default: 30).
    """
    enable_mdns: bool
    enable_udp: bool
    enable_bluetooth: bool
    interval_secs: int
    timeout_secs: int


class SecurityConfig:
    """
    Security configuration.
    
    Attributes:
        enable_encryption: Enable encryption (default: True).
        require_authentication: Require authentication (default: True).
        trust_mode: Trust mode - "trust_all", "manual", or "allowlist_only" (default: "manual").
        key_storage_path: Optional path for key storage.
    """
    enable_encryption: bool
    require_authentication: bool
    trust_mode: Literal["trust_all", "manual", "allowlist_only"]
    key_storage_path: Optional[str]


class NetworkConfig:
    """
    Networking configuration.
    
    Attributes:
        listen_port: Optional port to listen on.
        enable_ipv6: Enable IPv6 (default: True).
        enable_quic: Enable QUIC transport (default: True).
        enable_webrtc: Enable WebRTC transport (default: True).
        enable_websocket: Enable WebSocket transport (default: True).
        connection_timeout_secs: Connection timeout in seconds (default: 30).
    """
    listen_port: Optional[int]
    enable_ipv6: bool
    enable_quic: bool
    enable_webrtc: bool
    enable_websocket: bool
    connection_timeout_secs: int


class PluginConfig:
    """
    Plugin configuration.
    
    Attributes:
        name: Plugin name.
        path: Path to plugin library.
        enabled: Whether the plugin is enabled.
        config: Plugin-specific configuration.
    """
    name: str
    path: str
    enabled: bool
    config: Dict[str, Any]
