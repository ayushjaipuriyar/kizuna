/**
 * Kizuna Node.js Bindings
 * 
 * TypeScript definitions for the Kizuna API providing type-safe access
 * to peer discovery, file transfer, streaming, and command execution.
 * 
 * @module kizuna-node
 */

/**
 * Configuration options for initializing Kizuna
 */
export interface Config {
    /** Device name to identify this device on the network */
    deviceName?: string;

    /** User name associated with this device */
    userName?: string;

    /** Enable mDNS discovery (default: true) */
    enableMdns?: boolean;

    /** Enable UDP broadcast discovery (default: true) */
    enableUdp?: boolean;

    /** Enable Bluetooth discovery (default: false) */
    enableBluetooth?: boolean;

    /** Enable end-to-end encryption (default: true) */
    enableEncryption?: boolean;

    /** Require authentication for connections (default: true) */
    requireAuthentication?: boolean;

    /** Port to listen on for incoming connections */
    listenPort?: number;
}

/**
 * Information about a discovered peer
 */
export interface Peer {
    /** Unique identifier for the peer */
    id: string;

    /** Human-readable name of the peer */
    name: string;

    /** Network addresses where the peer can be reached */
    addresses: string[];

    /** Capabilities supported by the peer */
    capabilities: string[];

    /** Method used to discover this peer (mdns, udp, bluetooth) */
    discoveryMethod: string;
}

/**
 * Information about a file transfer
 */
export interface Transfer {
    /** Unique identifier for the transfer */
    id: string;

    /** Name of the file being transferred */
    fileName: string;

    /** Total size of the file in bytes */
    fileSize: number;

    /** ID of the peer involved in the transfer */
    peerId: string;

    /** Direction of the transfer */
    direction: 'send' | 'receive';
}

/**
 * Progress information for an ongoing file transfer
 */
export interface Progress {
    /** Transfer ID */
    id: string;

    /** Number of bytes transferred so far */
    bytesTransferred: number;

    /** Total number of bytes to transfer */
    totalBytes: number;

    /** Current transfer speed in bytes per second */
    speedBps: number;

    /** Progress percentage (0-100) */
    percentage: number;
}

/**
 * Result of a completed file transfer
 */
export interface TransferResult {
    /** Transfer ID */
    id: string;

    /** Whether the transfer completed successfully */
    success: boolean;

    /** Error message if the transfer failed */
    error?: string;

    /** Total bytes transferred */
    bytesTransferred: number;

    /** Duration of the transfer in milliseconds */
    durationMs: number;
}

/**
 * Configuration for starting a media stream
 */
export interface StreamConfig {
    /** Type of stream to create */
    streamType: 'camera' | 'screen' | 'audio';

    /** ID of the peer to stream to */
    peerId: string;

    /** Video quality (0-100, higher is better) */
    quality: number;
}

/**
 * Information about an active media stream
 */
export interface Stream {
    /** Unique identifier for the stream */
    id: string;

    /** Type of stream */
    streamType: 'camera' | 'screen' | 'audio';

    /** ID of the peer receiving the stream */
    peerId: string;
}

/**
 * Result of a command execution
 */
export interface CommandResult {
    /** Command that was executed */
    command: string;

    /** ID of the peer where the command was executed */
    peerId: string;

    /** Exit code of the command */
    exitCode: number;

    /** Standard output from the command */
    stdout: string;

    /** Standard error from the command */
    stderr: string;
}

/**
 * Event emitted by Kizuna
 */
export interface Event {
    /** Type of event */
    eventType: 'peer_discovered' | 'peer_connected' | 'peer_disconnected' |
    'transfer_started' | 'transfer_progress' | 'transfer_completed' |
    'stream_started' | 'stream_ended' | 'command_executed' | 'error';

    /** Event data as JSON string */
    data: string;
}

/**
 * Handle to a peer connection
 */
export class PeerConnectionHandle {
    /** Gets the peer ID */
    peerId(): string;
}

/**
 * Handle to manage an ongoing file transfer
 */
export class TransferHandle {
    /** Gets the transfer ID */
    transferId(): string;

    /** Cancels the transfer */
    cancel(): Promise<void>;
}

/**
 * Handle to manage an active media stream
 */
export class StreamHandle {
    /** Gets the stream ID */
    streamId(): string;

    /** Stops the stream */
    stop(): Promise<void>;
}

/**
 * Main Kizuna API class
 * 
 * Provides access to all Kizuna functionality including peer discovery,
 * file transfer, media streaming, and remote command execution.
 * 
 * @example
 * ```typescript
 * import { Kizuna } from 'kizuna-node';
 * 
 * const kizuna = new Kizuna();
 * await kizuna.initialize({
 *   deviceName: 'My Device',
 *   enableMdns: true,
 *   enableEncryption: true
 * });
 * 
 * // Set up event listener
 * kizuna.onEvent((event) => {
 *   console.log('Event:', event.eventType);
 * });
 * 
 * // Discover peers
 * const peers = await kizuna.discoverPeers();
 * console.log('Found peers:', peers);
 * 
 * // Clean up
 * await kizuna.shutdown();
 * ```
 */
export class Kizuna {
    /**
     * Creates a new Kizuna instance
     * 
     * Note: This does not start any services. Call `initialize()` to start Kizuna.
     */
    constructor();

    /**
     * Initializes Kizuna with the given configuration
     * 
     * @param config - Configuration options
     * @returns Promise that resolves when initialization is complete
     * @throws Error if already initialized or if initialization fails
     */
    initialize(config: Config): Promise<void>;

    /**
     * Registers an event callback for real-time notifications
     * 
     * The callback will be invoked whenever a Kizuna event occurs.
     * Events are delivered on the Node.js event loop.
     * 
     * @param callback - Function to call with event data
     * @returns Promise that resolves when the callback is registered
     * @throws Error if not initialized
     * 
     * @example
     * ```typescript
     * kizuna.onEvent((event) => {
     *   if (event.eventType === 'peer_discovered') {
     *     const peer = JSON.parse(event.data);
     *     console.log('Found peer:', peer.name);
     *   }
     * });
     * ```
     */
    onEvent(callback: (event: Event) => void): Promise<void>;

    /**
     * Discovers peers on the network
     * 
     * Returns a snapshot of currently discovered peers. For real-time
     * discovery updates, use the `onEvent` callback.
     * 
     * @returns Promise that resolves with an array of discovered peers
     * @throws Error if not initialized or if discovery fails
     * 
     * @example
     * ```typescript
     * const peers = await kizuna.discoverPeers();
     * peers.forEach(peer => {
     *   console.log(`Found ${peer.name} at ${peer.addresses.join(', ')}`);
     * });
     * ```
     */
    discoverPeers(): Promise<Peer[]>;

    /**
     * Connects to a peer
     * 
     * @param peerId - ID of the peer to connect to
     * @returns Promise that resolves with a connection handle
     * @throws Error if not initialized or if connection fails
     */
    connectToPeer(peerId: string): Promise<PeerConnectionHandle>;

    /**
     * Transfers a file to a peer
     * 
     * @param filePath - Path to the file to transfer
     * @param peerId - ID of the peer to send the file to
     * @returns Promise that resolves with a transfer handle
     * @throws Error if not initialized, file not found, or transfer fails
     * 
     * @example
     * ```typescript
     * const handle = await kizuna.transferFile('/path/to/file.txt', peerId);
     * console.log('Transfer started:', handle.transferId());
     * ```
     */
    transferFile(filePath: string, peerId: string): Promise<TransferHandle>;

    /**
     * Starts a media stream to a peer
     * 
     * @param config - Stream configuration
     * @returns Promise that resolves with a stream handle
     * @throws Error if not initialized or if stream fails to start
     * 
     * @example
     * ```typescript
     * const handle = await kizuna.startStream({
     *   streamType: 'camera',
     *   peerId: peerId,
     *   quality: 80
     * });
     * console.log('Stream started:', handle.streamId());
     * ```
     */
    startStream(config: StreamConfig): Promise<StreamHandle>;

    /**
     * Executes a command on a remote peer
     * 
     * @param command - Command to execute
     * @param peerId - ID of the peer to execute the command on
     * @returns Promise that resolves with the command result
     * @throws Error if not initialized or if execution fails
     * 
     * @example
     * ```typescript
     * const result = await kizuna.executeCommand('ls -la', peerId);
     * console.log('Output:', result.stdout);
     * console.log('Exit code:', result.exitCode);
     * ```
     */
    executeCommand(command: string, peerId: string): Promise<CommandResult>;

    /**
     * Checks if Kizuna is initialized and ready
     * 
     * @returns Promise that resolves with true if initialized
     */
    isInitialized(): Promise<boolean>;

    /**
     * Shuts down the Kizuna instance
     * 
     * Performs graceful shutdown of all systems and cleans up resources.
     * After calling this, the instance cannot be reused.
     * 
     * @returns Promise that resolves when shutdown is complete
     * 
     * @example
     * ```typescript
     * await kizuna.shutdown();
     * console.log('Kizuna shut down successfully');
     * ```
     */
    shutdown(): Promise<void>;
}
