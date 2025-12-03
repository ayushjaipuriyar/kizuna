/**
 * Example: Media Streaming (TypeScript)
 * 
 * This example demonstrates how to start media streams
 * using Kizuna's streaming capabilities with TypeScript.
 */

import { Kizuna, StreamConfig, Peer } from 'kizuna';

async function main(): Promise<void> {
    const kizuna = new Kizuna();

    try {
        // Initialize Kizuna
        console.log('Initializing Kizuna...');
        await kizuna.initialize({
            deviceName: 'Streaming Example',
            enableMdns: true,
            enableEncryption: true
        });

        // Discover peers
        console.log('Discovering peers...');
        const peers: Peer[] = await kizuna.discoverPeers();

        if (peers.length === 0) {
            console.log('No peers found. Please ensure another Kizuna instance is running.');
            return;
        }

        const targetPeer = peers[0];
        console.log(`\nStarting stream to: ${targetPeer.name} (${targetPeer.id})`);

        // Start a screen sharing stream
        const streamConfig: StreamConfig = {
            streamType: 'screen',
            peerId: targetPeer.id,
            quality: 80
        };

        const stream = await kizuna.startStream(streamConfig);
        console.log(`Stream started: ${stream.streamId()}`);

        // Stream for 10 seconds
        console.log('Streaming for 10 seconds...');
        await new Promise(resolve => setTimeout(resolve, 10000));

        // Stop the stream
        console.log('Stopping stream...');
        await stream.stop();
        console.log('Stream stopped');

    } catch (error) {
        if (error instanceof Error) {
            console.error('Error:', error.message);
        } else {
            console.error('Unknown error:', error);
        }
    } finally {
        await kizuna.shutdown();
        console.log('\nShutdown complete');
    }
}

main().catch(console.error);
