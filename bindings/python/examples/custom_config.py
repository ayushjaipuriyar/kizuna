"""
Custom Configuration Example

This example demonstrates how to use custom configuration options with Kizuna.
"""

import asyncio
from kizuna import Kizuna


async def main():
    """Main function demonstrating custom configuration."""
    
    # Create a custom configuration
    config = {
        "identity": {
            "device_name": "Python Custom Config Demo",
            "user_name": "Demo User"
        },
        "discovery": {
            "enable_mdns": True,
            "enable_udp": True,
            "enable_bluetooth": False,  # Disable Bluetooth discovery
            "interval_secs": 3,  # Discover every 3 seconds
            "timeout_secs": 20   # 20 second timeout
        },
        "security": {
            "enable_encryption": True,
            "require_authentication": True,
            "trust_mode": "manual"  # Require manual approval for each peer
        },
        "networking": {
            "listen_port": 9000,  # Custom port
            "enable_ipv6": True,
            "enable_quic": True,
            "enable_webrtc": True,
            "enable_websocket": True,
            "connection_timeout_secs": 15
        }
    }
    
    print("Initializing Kizuna with custom configuration...")
    print("\nConfiguration:")
    print(f"  Device name: {config['identity']['device_name']}")
    print(f"  User name: {config['identity']['user_name']}")
    print(f"  Discovery interval: {config['discovery']['interval_secs']}s")
    print(f"  Trust mode: {config['security']['trust_mode']}")
    print(f"  Listen port: {config['networking']['listen_port']}")
    
    kizuna = Kizuna(config)
    
    try:
        print("\nDiscovering peers...")
        peers = await kizuna.discover_peers()
        
        print(f"Found {len(peers)} peer(s):")
        for peer in peers:
            print(f"  - {peer.name} ({peer.id})")
            print(f"    Discovery method: {peer.discovery_method}")
        
        if not peers:
            print("\nNo peers found. This could be because:")
            print("  - No other Kizuna instances are running")
            print("  - Firewall is blocking connections")
            print("  - Network discovery is disabled on other devices")
    
    except RuntimeError as e:
        print(f"Error: {e}")
    
    finally:
        print("\nShutting down...")
        await kizuna.shutdown()
        print("Done!")


async def high_security_config():
    """Example of a high-security configuration."""
    
    config = {
        "identity": {
            "device_name": "Secure Python Client"
        },
        "discovery": {
            "enable_mdns": True,
            "enable_udp": False,  # Disable UDP broadcast for security
            "enable_bluetooth": False
        },
        "security": {
            "enable_encryption": True,
            "require_authentication": True,
            "trust_mode": "allowlist_only"  # Only connect to allowlisted peers
        },
        "networking": {
            "enable_ipv6": True,
            "enable_quic": True,  # Use QUIC for better security
            "enable_webrtc": False,  # Disable WebRTC
            "enable_websocket": False  # Disable WebSocket
        }
    }
    
    print("High-security configuration:")
    print("  - Encryption: Enabled")
    print("  - Authentication: Required")
    print("  - Trust mode: Allowlist only")
    print("  - UDP broadcast: Disabled")
    print("  - QUIC only: Enabled")
    
    kizuna = Kizuna(config)
    
    try:
        peers = await kizuna.discover_peers()
        print(f"\nFound {len(peers)} trusted peer(s)")
    finally:
        await kizuna.shutdown()


async def minimal_config():
    """Example of minimal configuration (uses defaults)."""
    
    print("Using minimal configuration (defaults)...")
    kizuna = Kizuna()
    
    try:
        peers = await kizuna.discover_peers()
        print(f"Found {len(peers)} peer(s) with default settings")
    finally:
        await kizuna.shutdown()


if __name__ == "__main__":
    print("=== Custom Configuration Example ===\n")
    asyncio.run(main())
    
    print("\n\n=== High Security Configuration ===\n")
    asyncio.run(high_security_config())
    
    print("\n\n=== Minimal Configuration ===\n")
    asyncio.run(minimal_config())
