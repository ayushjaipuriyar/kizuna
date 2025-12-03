/**
 * Basic test for Kizuna Node.js bindings
 * 
 * This is a simple smoke test to verify the bindings work correctly.
 * For comprehensive testing, use a proper test framework like Jest or Mocha.
 */

const { Kizuna } = require('../index');

async function runTests() {
    console.log('Starting Kizuna Node.js bindings test...\n');

    // Test 1: Create instance
    console.log('Test 1: Creating Kizuna instance...');
    const kizuna = new Kizuna();
    console.log('✓ Instance created\n');

    // Test 2: Check initialization state
    console.log('Test 2: Checking initialization state...');
    const initializedBefore = await kizuna.isInitialized();
    if (!initializedBefore) {
        console.log('✓ Instance not initialized (as expected)\n');
    } else {
        throw new Error('Instance should not be initialized yet');
    }

    // Test 3: Initialize with config
    console.log('Test 3: Initializing Kizuna...');
    try {
        await kizuna.initialize({
            deviceName: 'Test Device',
            userName: 'Test User',
            enableMdns: true,
            enableUdp: false,
            enableBluetooth: false,
            enableEncryption: true,
            requireAuthentication: false
        });
        console.log('✓ Initialization successful\n');
    } catch (error) {
        console.error('✗ Initialization failed:', error.message);
        throw error;
    }

    // Test 4: Check initialization state after init
    console.log('Test 4: Verifying initialization...');
    const initializedAfter = await kizuna.isInitialized();
    if (initializedAfter) {
        console.log('✓ Instance is initialized\n');
    } else {
        throw new Error('Instance should be initialized');
    }

    // Test 5: Set up event listener
    console.log('Test 5: Setting up event listener...');
    let eventReceived = false;
    try {
        await kizuna.onEvent((event) => {
            console.log(`  Received event: ${event.eventType}`);
            eventReceived = true;
        });
        console.log('✓ Event listener registered\n');
    } catch (error) {
        console.error('✗ Failed to register event listener:', error.message);
        throw error;
    }

    // Test 6: Discover peers (may return empty list)
    console.log('Test 6: Discovering peers...');
    try {
        const peers = await kizuna.discoverPeers();
        console.log(`✓ Discovery completed, found ${peers.length} peer(s)\n`);

        if (peers.length > 0) {
            console.log('  Discovered peers:');
            peers.forEach(peer => {
                console.log(`    - ${peer.name} (${peer.id})`);
                console.log(`      Addresses: ${peer.addresses.join(', ')}`);
                console.log(`      Method: ${peer.discoveryMethod}`);
            });
            console.log();
        }
    } catch (error) {
        console.error('✗ Discovery failed:', error.message);
        // Don't throw - discovery failure is not critical for this test
    }

    // Test 7: Test error handling (try to initialize again)
    console.log('Test 7: Testing error handling...');
    try {
        await kizuna.initialize({ deviceName: 'Test' });
        console.error('✗ Should have thrown an error for double initialization');
        throw new Error('Expected error was not thrown');
    } catch (error) {
        if (error.message.includes('already initialized')) {
            console.log('✓ Error handling works correctly\n');
        } else {
            throw error;
        }
    }

    // Test 8: Shutdown
    console.log('Test 8: Shutting down...');
    try {
        await kizuna.shutdown();
        console.log('✓ Shutdown successful\n');
    } catch (error) {
        console.error('✗ Shutdown failed:', error.message);
        throw error;
    }

    // Test 9: Verify shutdown
    console.log('Test 9: Verifying shutdown...');
    const initializedAfterShutdown = await kizuna.isInitialized();
    if (!initializedAfterShutdown) {
        console.log('✓ Instance is shut down\n');
    } else {
        throw new Error('Instance should be shut down');
    }

    console.log('All tests passed! ✓');
}

// Run tests
runTests()
    .then(() => {
        console.log('\nTest suite completed successfully');
        process.exit(0);
    })
    .catch((error) => {
        console.error('\nTest suite failed:', error);
        process.exit(1);
    });
