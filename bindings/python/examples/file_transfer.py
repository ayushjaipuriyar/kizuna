"""
File Transfer Example

This example demonstrates how to transfer files between peers using Kizuna.
"""

import asyncio
import sys
from pathlib import Path
from kizuna import Kizuna


async def transfer_file(file_path: str, peer_name: str = None):
    """
    Transfer a file to a peer.
    
    Args:
        file_path: Path to the file to transfer
        peer_name: Optional name of the peer to transfer to (uses first peer if not specified)
    """
    
    # Validate file exists
    path = Path(file_path)
    if not path.exists():
        print(f"Error: File not found: {file_path}")
        return
    
    if not path.is_file():
        print(f"Error: Not a file: {file_path}")
        return
    
    print(f"Preparing to transfer: {path.name} ({path.stat().st_size} bytes)")
    
    # Initialize Kizuna
    print("\nInitializing Kizuna...")
    kizuna = Kizuna({
        "identity": {
            "device_name": "Python File Transfer Client"
        }
    })
    
    try:
        # Discover peers
        print("Discovering peers...")
        peers = await kizuna.discover_peers()
        
        if not peers:
            print("No peers found. Make sure another Kizuna instance is running.")
            return
        
        print(f"Found {len(peers)} peer(s):")
        for i, peer in enumerate(peers):
            print(f"  {i + 1}. {peer.name} ({peer.id})")
        
        # Select peer
        target_peer = None
        if peer_name:
            # Find peer by name
            for peer in peers:
                if peer.name == peer_name:
                    target_peer = peer
                    break
            if not target_peer:
                print(f"Peer '{peer_name}' not found.")
                return
        else:
            # Use first peer
            target_peer = peers[0]
        
        print(f"\nTransferring to: {target_peer.name}")
        
        # Connect to peer
        print("Connecting...")
        connection = await kizuna.connect_to_peer(target_peer.id)
        print(f"Connected to {connection.peer_id}")
        
        # Start file transfer
        print(f"Starting transfer of {path.name}...")
        handle = await kizuna.transfer_file(str(path), target_peer.id)
        print(f"Transfer started: {handle.transfer_id}")
        print("Transfer in progress...")
        
        # In a real application, you would monitor progress here
        # For now, we just wait a bit
        await asyncio.sleep(2)
        
        print("Transfer initiated successfully!")
        print(f"Transfer ID: {handle.transfer_id}")
        
    except RuntimeError as e:
        print(f"Error during transfer: {e}")
    
    finally:
        print("\nShutting down...")
        await kizuna.shutdown()
        print("Done!")


async def main():
    """Main function."""
    
    if len(sys.argv) < 2:
        print("Usage: python file_transfer.py <file_path> [peer_name]")
        print("\nExample:")
        print("  python file_transfer.py document.pdf")
        print("  python file_transfer.py document.pdf 'Alice's Device'")
        return
    
    file_path = sys.argv[1]
    peer_name = sys.argv[2] if len(sys.argv) > 2 else None
    
    await transfer_file(file_path, peer_name)


if __name__ == "__main__":
    asyncio.run(main())
