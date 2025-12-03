"""
Basic Kizuna Usage Example

This example demonstrates the basic usage of Kizuna Python bindings:
- Initializing Kizuna
- Discovering peers
- Connecting to a peer
- Shutting down
"""

import asyncio
from kizuna import Kizuna


async def main():
    """Main function demonstrating basic Kizuna usage."""
    
    # Initialize Kizuna with default configuration
    print("Initializing Kizuna...")
    kizuna = Kizuna()
    
    try:
        # Discover peers on the network
        print("\nDiscovering peers...")
        peers = await kizuna.discover_peers()
        
        print(f"Found {len(peers)} peer(s):")
        for peer in peers:
            print(f"  - {peer.name}")
            print(f"    ID: {peer.id}")
            print(f"    Addresses: {', '.join(peer.addresses)}")
            print(f"    Capabilities: {', '.join(peer.capabilities)}")
            print(f"    Discovery method: {peer.discovery_method}")
            print()
        
        # Connect to the first peer if available
        if peers:
            peer_id = peers[0].id
            print(f"Connecting to peer: {peers[0].name}...")
            
            try:
                connection = await kizuna.connect_to_peer(peer_id)
                print(f"Successfully connected to {connection.peer_id}")
            except RuntimeError as e:
                print(f"Failed to connect: {e}")
        else:
            print("No peers found to connect to.")
    
    except RuntimeError as e:
        print(f"Error: {e}")
    
    finally:
        # Always shutdown to clean up resources
        print("\nShutting down...")
        await kizuna.shutdown()
        print("Done!")


if __name__ == "__main__":
    asyncio.run(main())
