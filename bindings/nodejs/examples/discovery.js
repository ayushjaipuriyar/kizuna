/**
 * Example: Peer Discovery
 * 
 * This example demonstrates how to discover peers on the network
 * using Kizuna's automatic discovery mechanisms.
 */

const { Kizuna } = require('kizuna');

async function main() {
    const kizuna = new Kizuna();

    try {
        // Initialize with discovery enabled
        console.log('Initializing Kizuna...');
        await kizuna.initialize({
            deviceName: 'Discovery Example',
            userName: 'Example User',
            enableMdns: true,
            enableUdp: true,
            enableBluetooth: false
        });

        console.log('Discovering peers...');
        const peers = await kizuna.discoverPeers();

        if (peers.length === 0) {
            console.log('No peers found');
        } else {
            console.log(`Found ${peers.length} peer(s):`);

            for (const peer of peers) {
                console.log(`\n  Peer: ${peer.name}`);
                console.log(`    ID: ${peer.id}`);
                console.log(`    Addresses: ${peer.addresses.join(', ')}`);
                console.log(`    Capabilities: ${peer.capabilities.join(', ')}`);
                console.log(`    Discovery Method: ${peer.discoveryMethod}`);
            }
        }

    } catch (error) {
        console.error('Error:', error.message);
    } finally {
        // Clean up
        await kizuna.shutdown();
        console.log('\nShutdown complete');
    }
}

main().catch(console.error);
