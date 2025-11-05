# Implementation Plan

- [x] 1. Set up discovery module structure and core interfaces
  - Create the discovery module directory structure with mod.rs, manager.rs, service_record.rs, and error.rs
  - Define the Discovery trait with async methods for discover, announce, stop_announce, and utility methods
  - Implement ServiceRecord data structure with peer information and helper methods
  - Create DiscoveryError enum with comprehensive error variants and proper error handling
  - _Requirements: 1.1, 7.1, 7.2, 7.3_

- [x] 2. Implement ServiceRecord and core data structures
  - [x] 2.1 Create ServiceRecord with all required fields and methods
    - Implement ServiceRecord struct with peer_id, name, addresses, port, discovery_method, capabilities, and last_seen
    - Add methods for adding addresses, capabilities, and checking expiration
    - Implement proper serialization/deserialization for network protocols
    - _Requirements: 1.4, 7.4_

  - [x] 2.2 Implement DiscoveryManager core functionality
    - Create DiscoveryManager struct with strategy management and peer caching
    - Implement methods for adding strategies, discovering peers, and announcing presence
    - Add auto-selection logic and active strategy tracking
    - _Requirements: 1.1, 1.2, 8.1, 8.2_

  - [x] 2.3 Write unit tests for core data structures
    - Create unit tests for ServiceRecord creation, modification, and expiration
    - Test DiscoveryManager strategy management and peer caching
    - _Requirements: 1.4, 7.4_

- [x] 3. Implement mDNS discovery strategy
  - [x] 3.1 Create mDNS strategy implementation
    - Implement Discovery trait for mDNS using the mdns crate
    - Add service announcement for "_kizuna._tcp.local" with TXT records
    - Implement peer browsing and TXT record parsing for peer metadata
    - Handle IPv4/IPv6 dual-stack scenarios and proper cleanup
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5_

  - [x] 3.2 Implement mDNS protocol handling
    - Create TXT record format with peer_id, name, version, and capabilities
    - Parse incoming mDNS responses and convert to ServiceRecord format
    - Handle mDNS service resolution for IP address discovery
    - Implement proper error handling for mDNS network failures
    - _Requirements: 2.2, 2.4_

  - [x] 3.3 Write mDNS strategy tests
    - Create unit tests for mDNS announcement and discovery
    - Test TXT record parsing and ServiceRecord conversion
    - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [x] 4. Implement UDP broadcast discovery strategy
  - [x] 4.1 Create UDP broadcast strategy implementation
    - Implement Discovery trait for UDP broadcast discovery
    - Create UDP broadcast sender for "DISCOVER_KIZUNA" messages
    - Implement UDP listener for discovery requests and responses
    - Add rate limiting to prevent network flooding
    - _Requirements: 3.1, 3.2, 3.3, 3.5_

  - [x] 4.2 Implement UDP protocol message handling
    - Define UDP message format for discovery requests and responses
    - Parse UDP messages and extract peer information into ServiceRecord
    - Handle broadcast addressing and multi-interface scenarios
    - Implement proper error handling for UDP network operations
    - _Requirements: 3.1, 3.4, 3.5_

  - [x] 4.3 Write UDP strategy tests
    - Create unit tests for UDP broadcast and response handling
    - Test message parsing and ServiceRecord creation
    - _Requirements: 3.1, 3.2, 3.4_

- [-] 5. Implement TCP handshake beacon strategy
  - [x] 5.1 Create TCP handshake strategy implementation
    - Implement Discovery trait for TCP handshake beacon
    - Create TCP port scanner for common Kizuna service ports
    - Implement TCP handshake protocol with peer verification
    - Add connection timeouts and parallel scanning with limits
    - _Requirements: 4.1, 4.2, 4.3, 4.5_

  - [x] 5.2 Implement TCP handshake protocol
    - Define TCP handshake message format (KIZUNA_HELLO/KIZUNA_PEER)
    - Parse handshake responses and extract peer information
    - Handle connection failures and timeout scenarios gracefully
    - Implement proper cleanup of TCP connections
    - _Requirements: 4.3, 4.4, 4.5_

  - [ ] 5.3 Write TCP strategy tests
    - Create unit tests for TCP handshake protocol
    - Test port scanning and connection handling
    - _Requirements: 4.1, 4.2, 4.3_

- [x] 6. Implement Bluetooth LE discovery strategy
  - [x] 6.1 Create Bluetooth LE strategy implementation
    - Implement Discovery trait for Bluetooth LE discovery
    - Add Bluetooth LE service advertisement with Kizuna service UUID
    - Implement BLE scanning for nearby Kizuna devices
    - Handle platform-specific Bluetooth permission requirements
    - _Requirements: 5.1, 5.2, 5.4, 5.5_

  - [x] 6.2 Implement Bluetooth LE protocol handling
    - Define BLE advertisement data format with peer information
    - Parse BLE scan results and convert to ServiceRecord format
    - Handle BLE connection establishment for data exchange
    - Implement graceful degradation when Bluetooth is unavailable
    - _Requirements: 5.2, 5.3, 5.5_

  - [x] 6.3 Write Bluetooth LE strategy tests
    - Create unit tests for BLE advertisement and scanning
    - Test advertisement data parsing and ServiceRecord creation
    - _Requirements: 5.1, 5.2, 5.3_

- [x] 7. Implement libp2p hybrid discovery strategy
  - [x] 7.1 Create libp2p strategy implementation
    - Implement Discovery trait for libp2p hybrid discovery
    - Set up libp2p swarm with mDNS and Kademlia DHT protocols
    - Implement peer ID generation and management for libp2p
    - Add NAT traversal support with relay and hole punching
    - _Requirements: 6.1, 6.2, 6.3, 6.5_

  - [x] 7.2 Implement libp2p protocol integration
    - Configure libp2p mDNS for local peer discovery
    - Set up Kademlia DHT for global peer lookup and routing
    - Handle bootstrap node configuration and connection management
    - Implement proper connection lifecycle and cleanup
    - _Requirements: 6.1, 6.4, 6.5_

  - [x] 7.3 Write libp2p strategy tests
    - Create unit tests for libp2p peer discovery and DHT operations
    - Test NAT traversal and connection establishment
    - _Requirements: 6.1, 6.2, 6.4_

- [x] 8. Implement auto-selection strategy and discovery manager
  - [x] 8.1 Create auto-selection algorithm
    - Implement strategy availability checking and priority ranking
    - Add network condition evaluation for optimal strategy selection
    - Create strategy performance testing and latency measurement
    - Implement fallback logic when primary strategies fail
    - _Requirements: 8.1, 8.2, 8.3, 8.4_

  - [x] 8.2 Integrate all strategies into DiscoveryManager
    - Register all discovery strategies with the DiscoveryManager
    - Implement concurrent discovery across multiple strategies
    - Add peer deduplication and result merging logic
    - Create strategy switching and adaptation mechanisms
    - _Requirements: 1.2, 1.5, 8.1, 8.5_

  - [x] 8.3 Write integration tests for discovery system
    - Create integration tests for multi-strategy discovery
    - Test auto-selection algorithm and fallback behavior
    - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [-] 9. Add comprehensive error handling and logging
  - [x] 9.1 Implement robust error handling across all strategies
    - Add proper error propagation and recovery mechanisms
    - Implement retry logic with exponential backoff for network failures
    - Create strategy-specific error handling and graceful degradation
    - Add comprehensive logging for debugging and monitoring
    - _Requirements: 7.3, 1.3_

  - [ ] 9.2 Add discovery performance monitoring
    - Implement timing metrics for each discovery strategy
    - Add peer cache management with TTL and cleanup
    - Create resource usage monitoring and optimization
    - Add configuration options for timeouts and retry behavior
    - _Requirements: 1.3, 8.5_

- [x] 10. Create public API and CLI integration
  - [x] 10.1 Implement public discovery API
    - Create clean public API for the discovery system
    - Add configuration options for strategy selection and timeouts
    - Implement async API with proper cancellation support
    - Create documentation and usage examples
    - _Requirements: 7.1, 7.5_

  - [x] 10.2 Integrate discovery system with main application
    - Wire discovery system into main Kizuna application structure
    - Add CLI commands for discovery testing and debugging
    - Implement proper initialization and shutdown procedures
    - Create configuration file support for discovery settings
    - _Requirements: 1.1, 7.5_