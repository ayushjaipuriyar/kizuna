/**
 * Advanced Kizuna Usage Examples
 * 
 * This file demonstrates advanced usage patterns including:
 * - Event-driven architecture
 * - Error handling and recovery
 * - Multiple concurrent operations
 * - Resource management
 */

const { Kizuna } = require('../index');

/**
 * Example 1: Event-driven peer discovery and file transfer
 */
async function eventDrivenTransfer() {
    console.log('=== Event-Driven Transfer Example ===\n');

    const kizuna = new Kizuna();
    await kizuna.initialize({
        deviceName: 'Advanced Example',
        enableMdns: true,
        enableEncryption: true
    });

    const discoveredPeers = new Map();
    const activeTransfers = new Map();

    // Set up comprehensive event handling
    await kizuna.onEvent((event) => {
        const data = JSON.parse(event.data);

        switch (event.eventType) {
            case 'peer_discovered':
                console.log(`[Discovery] Found peer: ${data.name} (${data.id})`);
                discoveredPeers.set(data.id, data);
                break;

            case 'peer_connected':
                console.log(`[Connection] Connected to peer: ${data.peerId}`);
                break;

            case 'peer_disconnected':
                console.log(`[Connection] Disconnected from peer: ${data.peerId}`);
                discoveredPeers.delete(data.peerId);
                break;

            case 'transfer_started':
                console.log(`[Transfer] Started: ${data.fileName} (${data.id})`);
                activeTransfers.set(data.id, {
                    fileName: data.fileName,
                    fileSize: data.fileSize,
                    startTime: Date.now()
                });
                break;

            case 'transfer_progress':
                const transfer = activeTransfers.get(data.id);
                if (transfer) {
                    const speedMBps = (data.speedBps / 1024 / 1024).toFixed(2);
                    const eta = ((data.totalBytes - data.bytesTransferred) / data.speedBps).toFixed(0);
                    console.log(
                        `[Transfer] ${transfer.fileName}: ${data.percentage.toFixed(1)}% ` +
                        `(${speedMBps} MB/s, ETA: ${eta}s)`
                    );
                }
                break;

            case 'transfer_completed':
                const completedTransfer = activeTransfers.get(data.id);
                if (completedTransfer) {
                    const duration = (data.durationMs / 1000).toFixed(2);
                    if (data.success) {
                        console.log(
                            `[Transfer] Completed: ${completedTransfer.fileName} ` +
                            `in ${duration}s`
                        );
                    } else {
                        console.error(
                            `[Transfer] Failed: ${completedTransfer.fileName} - ${data.error}`
                        );
                    }
                    activeTransfers.delete(data.id);
                }
                break;

            case 'error':
                console.error(`[Error] ${data.message}`);
                if (data.code) {
                    console.error(`  Code: ${data.code}`);
                }
                break;

            default:
                console.log(`[Event] ${event.eventType}:`, data);
        }
    });

    // Discover peers
    console.log('Discovering peers...\n');
    await kizuna.discoverPeers();

    // Wait a bit for events to come in
    await new Promise(resolve => setTimeout(resolve, 3000));

    console.log(`\nDiscovered ${discoveredPeers.size} peer(s)`);

    // Clean up
    await kizuna.shutdown();
    console.log('\nShutdown complete\n');
}

/**
 * Example 2: Robust error handling and retry logic
 */
async function robustTransfer(filePath, peerId, maxRetries = 3) {
    console.log('=== Robust Transfer Example ===\n');

    const kizuna = new Kizuna();

    try {
        await kizuna.initialize({
            deviceName: 'Robust Transfer',
            enableEncryption: true
        });

        let attempt = 0;
        let success = false;

        while (attempt < maxRetries && !success) {
            attempt++;
            console.log(`Transfer attempt ${attempt}/${maxRetries}...`);

            try {
                const handle = await kizuna.transferFile(filePath, peerId);
                console.log(`Transfer started: ${handle.transferId()}`);

                // In a real application, you would wait for the transfer_completed event
                // For this example, we'll just wait a bit
                await new Promise(resolve => setTimeout(resolve, 5000));

                success = true;
                console.log('Transfer completed successfully');
            } catch (error) {
                console.error(`Attempt ${attempt} failed:`, error.message);

                if (attempt < maxRetries) {
                    const backoff = Math.pow(2, attempt) * 1000; // Exponential backoff
                    console.log(`Retrying in ${backoff}ms...`);
                    await new Promise(resolve => setTimeout(resolve, backoff));
                }
            }
        }

        if (!success) {
            throw new Error(`Transfer failed after ${maxRetries} attempts`);
        }
    } finally {
        await kizuna.shutdown();
        console.log('Shutdown complete\n');
    }
}

/**
 * Example 3: Managing multiple concurrent operations
 */
async function concurrentOperations() {
    console.log('=== Concurrent Operations Example ===\n');

    const kizuna = new Kizuna();
    await kizuna.initialize({
        deviceName: 'Concurrent Example',
        enableMdns: true
    });

    // Track operations
    const operations = [];

    // Operation 1: Continuous peer discovery
    operations.push(
        (async () => {
            console.log('[Op1] Starting peer discovery...');
            const peers = await kizuna.discoverPeers();
            console.log(`[Op1] Found ${peers.length} peers`);
            return peers;
        })()
    );

    // Operation 2: Connect to a specific peer (if available)
    operations.push(
        (async () => {
            await new Promise(resolve => setTimeout(resolve, 1000));
            console.log('[Op2] Attempting to connect to peer...');
            // In a real scenario, you'd use an actual peer ID
            // For this example, we'll just simulate
            console.log('[Op2] Connection operation completed');
        })()
    );

    // Operation 3: Monitor system state
    operations.push(
        (async () => {
            for (let i = 0; i < 5; i++) {
                await new Promise(resolve => setTimeout(resolve, 500));
                const initialized = await kizuna.isInitialized();
                console.log(`[Op3] System check ${i + 1}: ${initialized ? 'OK' : 'NOT OK'}`);
            }
        })()
    );

    // Wait for all operations to complete
    console.log('\nWaiting for all operations to complete...\n');
    const results = await Promise.allSettled(operations);

    // Report results
    console.log('\nOperation Results:');
    results.forEach((result, index) => {
        if (result.status === 'fulfilled') {
            console.log(`  Operation ${index + 1}: Success`);
        } else {
            console.log(`  Operation ${index + 1}: Failed - ${result.reason}`);
        }
    });

    await kizuna.shutdown();
    console.log('\nShutdown complete\n');
}

/**
 * Example 4: Resource cleanup with proper error handling
 */
async function properCleanup() {
    console.log('=== Proper Cleanup Example ===\n');

    const kizuna = new Kizuna();
    let initialized = false;

    try {
        await kizuna.initialize({ deviceName: 'Cleanup Example' });
        initialized = true;
        console.log('Initialized successfully');

        // Simulate some work
        await kizuna.discoverPeers();
        console.log('Work completed');

        // Simulate an error
        throw new Error('Simulated error');
    } catch (error) {
        console.error('Error occurred:', error.message);
    } finally {
        // Always clean up, even if an error occurred
        if (initialized) {
            try {
                await kizuna.shutdown();
                console.log('Cleanup successful');
            } catch (cleanupError) {
                console.error('Cleanup failed:', cleanupError.message);
            }
        }
    }

    console.log();
}

/**
 * Main function to run all examples
 */
async function main() {
    const examples = [
        { name: 'Event-Driven Transfer', fn: eventDrivenTransfer },
        { name: 'Concurrent Operations', fn: concurrentOperations },
        { name: 'Proper Cleanup', fn: properCleanup }
    ];

    for (const example of examples) {
        try {
            console.log(`\n${'='.repeat(60)}`);
            console.log(`Running: ${example.name}`);
            console.log('='.repeat(60) + '\n');
            await example.fn();
        } catch (error) {
            console.error(`Example "${example.name}" failed:`, error);
        }
    }

    console.log('\nAll examples completed!');
}

// Run examples if this file is executed directly
if (require.main === module) {
    main().catch(console.error);
}

// Export examples for use in other files
module.exports = {
    eventDrivenTransfer,
    robustTransfer,
    concurrentOperations,
    properCleanup
};
