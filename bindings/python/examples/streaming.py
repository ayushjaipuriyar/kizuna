"""
Media Streaming Example

This example demonstrates how to start media streams (camera, screen, audio) to peers.
"""

import asyncio
import sys
from kizuna import Kizuna


async def start_streaming(stream_type: str, quality: int = 80):
    """
    Start a media stream to a peer.
    
    Args:
        stream_type: Type of stream - "camera", "screen", or "audio"
        quality: Stream quality 0-100 (default: 80)
    """
    
    # Validate stream type
    valid_types = ["camera", "screen", "audio"]
    if stream_type not in valid_types:
        print(f"Error: Invalid stream type. Must be one of: {', '.join(valid_types)}")
        return
    
    # Validate quality
    if not 0 <= quality <= 100:
        print("Error: Quality must be between 0 and 100")
        return
    
    print(f"Preparing to start {stream_type} stream (quality: {quality})")
    
    # Initialize Kizuna
    print("\nInitializing Kizuna...")
    kizuna = Kizuna({
        "identity": {
            "device_name": "Python Streaming Client"
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
        
        # Use first peer
        target_peer = peers[0]
        print(f"\nStreaming to: {target_peer.name}")
        
        # Connect to peer
        print("Connecting...")
        connection = await kizuna.connect_to_peer(target_peer.id)
        print(f"Connected to {connection.peer_id}")
        
        # Start stream
        print(f"Starting {stream_type} stream...")
        handle = await kizuna.start_stream(stream_type, target_peer.id, quality)
        print(f"Stream started: {handle.stream_id}")
        print(f"Streaming {stream_type} at {quality}% quality...")
        
        # Keep streaming for a while
        print("\nStreaming for 10 seconds... (Press Ctrl+C to stop)")
        try:
            await asyncio.sleep(10)
        except KeyboardInterrupt:
            print("\nStopping stream...")
        
        print("Stream completed!")
        
    except RuntimeError as e:
        print(f"Error during streaming: {e}")
    except ValueError as e:
        print(f"Invalid parameter: {e}")
    
    finally:
        print("\nShutting down...")
        await kizuna.shutdown()
        print("Done!")


async def main():
    """Main function."""
    
    if len(sys.argv) < 2:
        print("Usage: python streaming.py <stream_type> [quality]")
        print("\nStream types:")
        print("  camera  - Stream from camera")
        print("  screen  - Share screen")
        print("  audio   - Stream audio")
        print("\nQuality: 0-100 (default: 80)")
        print("\nExamples:")
        print("  python streaming.py screen")
        print("  python streaming.py camera 90")
        print("  python streaming.py audio 70")
        return
    
    stream_type = sys.argv[1]
    quality = int(sys.argv[2]) if len(sys.argv) > 2 else 80
    
    await start_streaming(stream_type, quality)


if __name__ == "__main__":
    asyncio.run(main())
