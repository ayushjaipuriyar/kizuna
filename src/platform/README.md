# Cross-Platform Support Module

This module provides comprehensive cross-platform support for Kizuna, enabling it to run consistently across all target platforms including Linux, macOS, Windows, Android, iOS, and web browsers.

## Overview

The platform module consists of several key components:

- **Platform Detection**: Runtime detection of operating system, architecture, and capabilities
- **Platform Abstraction**: Common traits and interfaces for platform-specific implementations
- **Capability Management**: Feature availability tracking and graceful degradation
- **Platform Adapters**: Platform-specific implementations for each supported OS

## Architecture

```
platform/
├── mod.rs              # Module definition and exports
├── types.rs            # Core type definitions
├── traits.rs           # Platform abstraction traits
├── detection.rs        # Runtime platform detection
├── capabilities.rs     # Capability management
├── adapter.rs          # Platform adapter implementations
├── linux.rs            # Linux-specific adapter
├── macos.rs            # macOS-specific adapter
├── windows.rs          # Windows-specific adapter
├── android.rs          # Android-specific adapter
├── ios.rs              # iOS-specific adapter
└── wasm.rs             # WebAssembly/browser adapter
```

## Usage

### Basic Platform Detection

```rust
use kizuna::platform::{DefaultPlatformManager, PlatformManager};

let manager = DefaultPlatformManager::new()?;
let info = manager.detect_platform()?;

println!("OS: {:?}", info.os);
println!("Architecture: {:?}", info.architecture);
```

### Checking Feature Availability

```rust
use kizuna::platform::Feature;

let manager = DefaultPlatformManager::new()?;

if manager.is_feature_available(Feature::Clipboard) {
    // Use clipboard functionality
}
```

### Getting Platform Adapter

```rust
let manager = DefaultPlatformManager::new()?;
let adapter = manager.get_platform_adapter()?;

// Initialize platform-specific components
adapter.initialize_platform().await?;

// Get system services
let services = adapter.integrate_system_services().await?;
```

### Platform Optimization

```rust
use kizuna::platform::PlatformConfig;

let manager = DefaultPlatformManager::new()?;
let mut config = PlatformConfig::default();

// Apply platform-specific optimizations
manager.optimize_for_platform(&mut config)?;
```

## Supported Platforms

### Desktop Platforms

- **Linux**: Full support with X11/Wayland, systemd, D-Bus integration
- **macOS**: Native Cocoa framework, Keychain, code signing support
- **Windows**: Win32/WinRT APIs, Registry, Windows Security integration

### Mobile Platforms

- **Android**: Native Android UI, system services, battery optimization
- **iOS**: UIKit integration, Keychain, App Store compliance

### Web Platform

- **WebAssembly**: Browser execution with PWA capabilities, WebRTC support

### Container Platform

- **Docker/Kubernetes**: Containerized deployment with minimal footprint

## Feature Flags

The module supports various Cargo feature flags for conditional compilation:

```toml
[features]
platform-native = []      # Native platform support (default)
platform-linux = []       # Linux-specific features
platform-macos = []       # macOS-specific features
platform-windows = []     # Windows-specific features
platform-android = []     # Android-specific features
platform-ios = []         # iOS-specific features
platform-wasm = []        # WebAssembly/browser features
platform-container = []   # Container-specific features
```

## Platform Capabilities

Each platform provides different capabilities:

- **GUI Framework**: Native, Web, or None
- **System Tray**: Desktop notification area integration
- **Notifications**: System notification support
- **File Associations**: File type registration
- **Auto Start**: Launch on system startup
- **Hardware Acceleration**: SIMD, GPU, codec support
- **Network Features**: TCP, UDP, QUIC, WebRTC, mDNS, Bluetooth
- **Security Features**: Keychain, secure enclave, hardware crypto, sandboxing

## Graceful Degradation

When a feature is unavailable, the capability manager provides fallback options:

```rust
let manager = CapabilityManager::new(capabilities);
let fallbacks = manager.get_fallback_options(Feature::Discovery);

// Use fallback discovery methods if mDNS is unavailable
```

## Platform-Specific Implementation

Each platform adapter implements the `PlatformAdapter` trait:

```rust
#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    async fn initialize_platform(&self) -> PlatformResult<()>;
    async fn integrate_system_services(&self) -> PlatformResult<SystemServices>;
    async fn setup_ui_framework(&self) -> PlatformResult<UIFramework>;
    async fn configure_networking(&self) -> PlatformResult<NetworkConfig>;
    async fn setup_security_integration(&self) -> PlatformResult<SecurityConfig>;
    fn platform_name(&self) -> &str;
}
```

## Testing

Run platform-specific tests:

```bash
cargo test --lib platform
```

Run the platform demo:

```bash
cargo run --example platform_demo
```

## Future Work

The following tasks will implement full platform-specific functionality:

- Task 2: Platform detection and capability management
- Task 3: Linux platform support
- Task 4: macOS platform support
- Task 5: Windows platform support
- Task 6: Android platform support
- Task 7: iOS platform support
- Task 8: WebAssembly and browser support
- Task 9: Container and deployment support
- Task 10: Cross-platform optimization and performance tuning

## Requirements

This module implements requirements from the Cross-Platform Footprint specification:

- **Requirement 10.1**: Cross-compilation support for all target platforms
- **Requirement 10.3**: Platform-specific build scripts and deployment tools

See `.kiro/specs/cross-platform-footprint/requirements.md` for full requirements.
