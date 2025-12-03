# Changelog

All notable changes to the Kizuna Flutter plugin will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2024-01-01

### Added
- Initial release of Kizuna Flutter plugin
- Core API implementation with Rust backend via Flutter Rust Bridge
- Peer discovery using mDNS, UDP broadcast, and Bluetooth
- File transfer with resume support
- Screen streaming for desktop platforms (Windows, macOS, Linux)
- Camera streaming for all platforms
- Command execution on remote peers
- Event system for real-time notifications
- End-to-end encryption using ChaCha20-Poly1305
- Identity verification using Ed25519 signatures
- Multi-platform support (Android, iOS, Windows, macOS, Linux)
- Platform-specific optimizations and feature detection
- Comprehensive documentation and examples

### Platform Support
- Android: API level 21+ (Android 5.0+)
- iOS: iOS 12.0+
- Windows: Windows 10 build 17763+
- macOS: macOS 10.14+
- Linux: Ubuntu 20.04+

### Known Limitations
- Screen streaming not available on mobile platforms (Android, iOS)
- Web platform support is experimental
- Background execution on iOS requires additional configuration

## [Unreleased]

### Planned Features
- Web platform support
- Background transfer improvements for mobile
- Improved error handling and recovery
- Performance optimizations
- Additional streaming codecs
- Plugin system for extensibility
