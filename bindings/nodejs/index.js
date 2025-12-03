/**
 * Kizuna Node.js Bindings
 * 
 * This module provides Node.js bindings for Kizuna, enabling peer-to-peer
 * file transfer, media streaming, and command execution.
 */

const { existsSync, readFileSync } = require('fs');
const { join } = require('path');

const { platform, arch } = process;

// Map Node.js platform/arch to Rust target triples
function getTargetTriple() {
    const platformMap = {
        'linux': {
            'x64': 'x86_64-unknown-linux-gnu',
            'arm64': 'aarch64-unknown-linux-gnu'
        },
        'darwin': {
            'x64': 'x86_64-apple-darwin',
            'arm64': 'aarch64-apple-darwin'
        },
        'win32': {
            'x64': 'x86_64-pc-windows-msvc'
        }
    };

    const archMap = platformMap[platform];
    if (!archMap) {
        throw new Error(`Unsupported platform: ${platform}`);
    }

    const triple = archMap[arch];
    if (!triple) {
        throw new Error(`Unsupported architecture: ${arch} on ${platform}`);
    }

    return triple;
}

// Try to load the native module
function loadNativeModule() {
    const triple = getTargetTriple();

    // Try loading from the platform-specific optional dependency
    const packageName = `kizuna-${platform}-${arch}${platform === 'linux' ? '-gnu' : ''}`;
    try {
        return require(packageName);
    } catch (e) {
        // Fall back to local build
    }

    // Try loading from local build directory
    const localPath = join(__dirname, `../../target/release/libkizuna.node`);
    if (existsSync(localPath)) {
        return require(localPath);
    }

    // Try loading from debug build
    const debugPath = join(__dirname, `../../target/debug/libkizuna.node`);
    if (existsSync(debugPath)) {
        return require(debugPath);
    }

    throw new Error(
        `Failed to load Kizuna native module for ${platform}-${arch}. ` +
        `Please ensure the module is built or the platform-specific package is installed.`
    );
}

// Load and export the native module
const nativeModule = loadNativeModule();

module.exports = {
    Kizuna: nativeModule.Kizuna,
    PeerConnectionHandle: nativeModule.PeerConnectionHandle,
    TransferHandle: nativeModule.TransferHandle,
    StreamHandle: nativeModule.StreamHandle
};

// Also support ES6 imports
module.exports.default = nativeModule.Kizuna;
