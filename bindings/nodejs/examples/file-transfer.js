/**
 * Example: File Transfer
 * 
 * This example demonstrates how to transfer files between peers
 * using Kizuna's file transfer capabilities.
 */

const { Kizuna } = require('kizuna');
const fs = require('fs');
const path = require('path');

async function main() {
    const kizuna = new Kizuna();

    try {
        // Initialize Kizuna
        console.log('Initializing Kizuna...');
        await kizuna.initialize({
            deviceName: 'File Transfer Example',
            enableMdns: true,
            enableEncryption: true
        });

        // Discover peers
        console.log('Discovering peers...');
        const peers = await kizuna.discoverPeers();

        if (peers.length === 0) {
            console.log('No peers found. Please ensure another Kizuna instance is running.');
            return;
        }

        const targetPeer = peers[0];
        console.log(`\nTransferring file to: ${targetPeer.name} (${targetPeer.id})`);

        // Create a test file
        const testFile = path.join(__dirname, 'test-file.txt');
        fs.writeFileSync(testFile, 'Hello from Kizuna!\n'.repeat(1000));

        // Start the transfer
        const transfer = await kizuna.transferFile(testFile, targetPeer.id);
        console.log(`Transfer started: ${transfer.transferId()}`);

        // In a real application, you would listen for transfer progress events
        // For this example, we'll just wait a bit
        await new Promise(resolve => setTimeout(resolve, 2000));

        console.log('Transfer initiated successfully');

        // Clean up test file
        fs.unlinkSync(testFile);

    } catch (error) {
        console.error('Error:', error.message);
    } finally {
        await kizuna.shutdown();
        console.log('\nShutdown complete');
    }
}

main().catch(console.error);
