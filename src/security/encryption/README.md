# Encryption Engine Implementation

This module implements the encryption engine and session management for Kizuna's security system.

## Overview

The encryption engine provides end-to-end encryption for all communications using industry-standard cryptographic primitives:

- **X25519 ECDH** for key exchange
- **ChaCha20-Poly1305** for authenticated encryption
- **HKDF-like construction** for key derivation
- **Automatic key rotation** for forward secrecy

## Components

### 1. Session Key Exchange (Task 3.1)

**KeyExchange** - Implements X25519 ECDH key exchange protocol

```rust
let kx = KeyExchange::new();
let our_public_key = kx.public_key();
// Exchange public keys with peer...
let shared_secret = kx.exchange(&peer_public_key);
```

**Features:**
- Ephemeral key generation using secure random number generator
- Diffie-Hellman key exchange for shared secret derivation
- Session key derivation using HKDF-like construction with HMAC-SHA256
- Separate send/receive keys for bidirectional communication

**Requirements Addressed:**
- 2.2: Key exchange using X25519 Elliptic Curve Diffie-Hellman
- 2.3: Perfect forward secrecy through ephemeral keys

### 2. ChaCha20-Poly1305 Message Encryption (Task 3.2)

**SecuritySession** - Manages encryption state for a peer connection

```rust
let session = SecuritySession::new(peer_id, shared_secret)?;
```

**EncryptionEngineImpl** - Main encryption engine implementation

```rust
let engine = EncryptionEngineImpl::with_defaults();
let session_id = engine.establish_session(&peer_id).await?;
let encrypted = engine.encrypt_message(&session_id, data).await?;
let decrypted = engine.decrypt_message(&session_id, &encrypted).await?;
```

**Features:**
- Authenticated encryption using ChaCha20-Poly1305 AEAD
- Automatic nonce generation and management
- Nonce counter to prevent reuse
- Replay attack prevention through nonce validation
- Authentication tag verification on decryption

**Requirements Addressed:**
- 2.1: E2E encryption for all data transfers using ChaCha20-Poly1305
- 2.4: Message authentication using HMAC-SHA256
- 2.5: Connection rejection on encryption failure

### 3. Forward Secrecy with Key Rotation (Task 3.3)

**Key Rotation** - Automatic periodic key rotation

```rust
engine.rotate_session_keys(&session_id).await?;
```

**Features:**
- Periodic automatic key rotation (default: 15 minutes)
- Secure key zeroization after rotation
- Session timeout and cleanup (default: 1 hour)
- New keys derived from previous state + timestamp
- Nonce counter reset after rotation

**Requirements Addressed:**
- 2.3: Perfect forward secrecy through key rotation

## Security Properties

### Cryptographic Guarantees

1. **Confidentiality**: ChaCha20 stream cipher ensures data cannot be read without the key
2. **Authenticity**: Poly1305 MAC ensures messages cannot be forged
3. **Forward Secrecy**: Key rotation ensures past communications remain secure even if current keys are compromised
4. **Replay Protection**: Nonce validation prevents replay attacks
5. **Key Isolation**: Separate send/receive keys prevent reflection attacks

### Memory Safety

- All sensitive data (keys, shared secrets) use `zeroize` crate
- Automatic zeroization on drop via `ZeroizeOnDrop` derive
- Explicit zeroization before key rotation

### Session Management

- Sessions automatically expire after timeout period
- Expired sessions can be cleaned up with `cleanup_expired_sessions()`
- Session IDs are UUIDs to prevent guessing

## Usage Example

```rust
use kizuna::security::encryption::{EncryptionEngine, EncryptionEngineImpl};
use kizuna::security::identity::PeerId;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create encryption engine
    let engine = EncryptionEngineImpl::with_defaults();
    
    // Establish session with peer
    let peer_id = PeerId::from_fingerprint([1u8; 32]);
    let session_id = engine.establish_session(&peer_id).await?;
    
    // Encrypt message
    let message = b"Hello, secure world!";
    let encrypted = engine.encrypt_message(&session_id, message).await?;
    
    // Decrypt message
    let decrypted = engine.decrypt_message(&session_id, &encrypted).await?;
    assert_eq!(message, decrypted.as_slice());
    
    // Rotate keys for forward secrecy
    engine.rotate_session_keys(&session_id).await?;
    
    Ok(())
}
```

## Configuration

The encryption engine can be configured with custom timeouts:

```rust
use std::time::Duration;

let engine = EncryptionEngineImpl::new(
    Duration::from_secs(3600),  // Session timeout: 1 hour
    Duration::from_secs(900),   // Key rotation: 15 minutes
);
```

## Testing

Run the encryption tests:

```bash
cargo test --lib security::encryption::test_encryption
```

Run the demo:

```bash
cargo run --example encryption_demo
```

## Implementation Notes

### Nonce Format

Nonces are 12 bytes (96 bits) as required by ChaCha20-Poly1305:
- Bytes 0-3: Reserved (zeros)
- Bytes 4-11: 64-bit counter (little-endian)

### Message Format

Encrypted messages have the following format:
```
[12 bytes: nonce][N bytes: ciphertext + 16 byte auth tag]
```

### Key Derivation

Session keys are derived using HMAC-SHA256 as a KDF:
```
send_key = HMAC-SHA256(shared_secret, "kizuna-send-key-v1")
recv_key = HMAC-SHA256(shared_secret, "kizuna-recv-key-v1")
```

### Key Rotation Algorithm

1. Hash current shared_secret + timestamp to create new shared_secret
2. Zeroize old keys
3. Derive new send/recv keys from new shared_secret
4. Reset nonce counters
5. Update last_rotation timestamp

## Future Enhancements

- [ ] Support for multiple concurrent sessions per peer
- [ ] Ratcheting protocol for enhanced forward secrecy (Double Ratchet)
- [ ] Post-quantum key exchange (Kyber)
- [ ] Session resumption with pre-shared keys
- [ ] Bandwidth optimization for small messages
