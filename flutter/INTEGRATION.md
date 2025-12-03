# Kizuna Flutter Plugin Integration Guide

This guide explains how to integrate the Kizuna Flutter plugin into your Flutter application and configure it for different platforms.

## Table of Contents

1. [Installation](#installation)
2. [Platform Configuration](#platform-configuration)
3. [Building the Native Library](#building-the-native-library)
4. [Flutter Rust Bridge Setup](#flutter-rust-bridge-setup)
5. [Platform-Specific Setup](#platform-specific-setup)
6. [Troubleshooting](#troubleshooting)

## Installation

### 1. Add Dependency

Add Kizuna to your `pubspec.yaml`:

```yaml
dependencies:
  kizuna: ^0.1.0
```

Or for local development:

```yaml
dependencies:
  kizuna:
    path: ../path/to/kizuna/flutter
```

### 2. Install Dependencies

```bash
flutter pub get
```

## Platform Configuration

### Android

#### 1. Update `android/app/build.gradle`

```gradle
android {
    // ... existing configuration
    
    defaultConfig {
        // ... existing configuration
        minSdkVersion 21
        targetSdkVersion 34
    }
    
    // Add NDK configuration
    ndkVersion "25.1.8937393"
}
```

#### 2. Add Permissions to `android/app/src/main/AndroidManifest.xml`

```xml
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <!-- Network permissions -->
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    <uses-permission android:name="android.permission.ACCESS_WIFI_STATE" />
    <uses-permission android:name="android.permission.CHANGE_WIFI_MULTICAST_STATE" />
    
    <!-- Camera permission (if using camera streaming) -->
    <uses-permission android:name="android.permission.CAMERA" />
    
    <!-- Bluetooth permissions (if using Bluetooth discovery) -->
    <uses-permission android:name="android.permission.BLUETOOTH" />
    <uses-permission android:name="android.permission.BLUETOOTH_ADMIN" />
    <uses-permission android:name="android.permission.BLUETOOTH_SCAN" />
    <uses-permission android:name="android.permission.BLUETOOTH_CONNECT" />
    
    <!-- Storage permissions (for file transfers) -->
    <uses-permission android:name="android.permission.READ_EXTERNAL_STORAGE" />
    <uses-permission android:name="android.permission.WRITE_EXTERNAL_STORAGE" />
    
    <application>
        <!-- ... existing configuration -->
    </application>
</manifest>
```

### iOS

#### 1. Update `ios/Podfile`

```ruby
platform :ios, '12.0'

# ... existing configuration

post_install do |installer|
  installer.pods_project.targets.each do |target|
    flutter_additional_ios_build_settings(target)
    
    # Enable bitcode
    target.build_configurations.each do |config|
      config.build_settings['ENABLE_BITCODE'] = 'NO'
    end
  end
end
```

#### 2. Add Permissions to `ios/Runner/Info.plist`

```xml
<dict>
    <!-- ... existing configuration -->
    
    <!-- Camera permission -->
    <key>NSCameraUsageDescription</key>
    <string>Camera access is required for video streaming</string>
    
    <!-- Microphone permission -->
    <key>NSMicrophoneUsageDescription</key>
    <string>Microphone access is required for audio streaming</string>
    
    <!-- Local network permission -->
    <key>NSLocalNetworkUsageDescription</key>
    <string>Local network access is required for peer discovery</string>
    
    <!-- Bonjour services -->
    <key>NSBonjourServices</key>
    <array>
        <string>_kizuna._tcp</string>
        <string>_kizuna._udp</string>
    </array>
</dict>
```

### macOS

#### 1. Update `macos/Runner/DebugProfile.entitlements` and `Release.entitlements`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <!-- ... existing entitlements -->
    
    <!-- Network entitlements -->
    <key>com.apple.security.network.client</key>
    <true/>
    <key>com.apple.security.network.server</key>
    <true/>
    
    <!-- Camera entitlement -->
    <key>com.apple.security.device.camera</key>
    <true/>
    
    <!-- Microphone entitlement -->
    <key>com.apple.security.device.audio-input</key>
    <true/>
    
    <!-- Screen recording entitlement -->
    <key>com.apple.security.device.screen-recording</key>
    <true/>
</dict>
</plist>
```

#### 2. Update `macos/Runner/Info.plist`

```xml
<dict>
    <!-- ... existing configuration -->
    
    <key>NSCameraUsageDescription</key>
    <string>Camera access is required for video streaming</string>
    
    <key>NSMicrophoneUsageDescription</key>
    <string>Microphone access is required for audio streaming</string>
</dict>
```

### Windows

#### 1. Update `windows/runner/main.cpp`

Ensure the application has network capabilities enabled.

#### 2. Firewall Configuration

The application will need to be allowed through the Windows Firewall for incoming connections. This can be configured programmatically or manually by the user.

### Linux

#### 1. Update `linux/CMakeLists.txt`

```cmake
# ... existing configuration

# Add required libraries
find_package(PkgConfig REQUIRED)
pkg_check_modules(GTK REQUIRED gtk+-3.0)

# Link libraries
target_link_libraries(${BINARY_NAME} PRIVATE
    ${GTK_LIBRARIES}
    # Add other required libraries
)
```

#### 2. System Dependencies

Install required system libraries:

```bash
# Ubuntu/Debian
sudo apt-get install libgtk-3-dev libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev

# Fedora
sudo dnf install gtk3-devel gstreamer1-devel gstreamer1-plugins-base-devel

# Arch Linux
sudo pacman -S gtk3 gstreamer gst-plugins-base
```

## Building the Native Library

### Prerequisites

1. Install Rust: https://rustup.rs/
2. Install Flutter Rust Bridge CLI:

```bash
cargo install flutter_rust_bridge_codegen
```

### Build Steps

#### 1. Generate Dart Bindings

From the root of the Kizuna project:

```bash
flutter_rust_bridge_codegen \
  --rust-input src/developer_api/bindings/flutter.rs \
  --dart-output flutter/lib/src/bridge_generated.dart \
  --dart-decl-output flutter/lib/src/bridge_definitions.dart
```

#### 2. Build Native Library

```bash
# For Android
cargo build --release --target aarch64-linux-android --features flutter
cargo build --release --target armv7-linux-androideabi --features flutter
cargo build --release --target x86_64-linux-android --features flutter

# For iOS
cargo build --release --target aarch64-apple-ios --features flutter
cargo build --release --target x86_64-apple-ios --features flutter

# For macOS
cargo build --release --target aarch64-apple-darwin --features flutter
cargo build --release --target x86_64-apple-darwin --features flutter

# For Windows
cargo build --release --target x86_64-pc-windows-msvc --features flutter

# For Linux
cargo build --release --target x86_64-unknown-linux-gnu --features flutter
```

#### 3. Copy Libraries to Flutter Plugin

```bash
# Android
mkdir -p flutter/android/src/main/jniLibs/{arm64-v8a,armeabi-v7a,x86_64}
cp target/aarch64-linux-android/release/libkizuna.so flutter/android/src/main/jniLibs/arm64-v8a/
cp target/armv7-linux-androideabi/release/libkizuna.so flutter/android/src/main/jniLibs/armeabi-v7a/
cp target/x86_64-linux-android/release/libkizuna.so flutter/android/src/main/jniLibs/x86_64/

# iOS
mkdir -p flutter/ios/Frameworks
cp target/aarch64-apple-ios/release/libkizuna.a flutter/ios/Frameworks/

# macOS
mkdir -p flutter/macos/Frameworks
cp target/aarch64-apple-darwin/release/libkizuna.dylib flutter/macos/Frameworks/

# Windows
mkdir -p flutter/windows
cp target/x86_64-pc-windows-msvc/release/kizuna.dll flutter/windows/

# Linux
mkdir -p flutter/linux
cp target/x86_64-unknown-linux-gnu/release/libkizuna.so flutter/linux/
```

## Flutter Rust Bridge Setup

### 1. Configure `flutter_rust_bridge.yaml`

Create a `flutter_rust_bridge.yaml` file in the root:

```yaml
rust_input: src/developer_api/bindings/flutter.rs
dart_output: flutter/lib/src/bridge_generated.dart
dart_decl_output: flutter/lib/src/bridge_definitions.dart
c_output: flutter/ios/Classes/bridge_generated.h
extra_c_output_path:
  - flutter/macos/Classes/
llvm_path:
  - /usr/lib/llvm-14
llvm_compiler_opts: -I /usr/include
```

### 2. Generate Bindings

```bash
flutter_rust_bridge_codegen
```

## Platform-Specific Setup

### Android NDK Setup

1. Install Android NDK via Android Studio SDK Manager
2. Add NDK targets to Rust:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

3. Configure cargo for Android in `~/.cargo/config.toml`:

```toml
[target.aarch64-linux-android]
ar = "path/to/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android-ar"
linker = "path/to/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang"

[target.armv7-linux-androideabi]
ar = "path/to/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/arm-linux-androideabi-ar"
linker = "path/to/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/armv7a-linux-androideabi21-clang"

[target.x86_64-linux-android]
ar = "path/to/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android-ar"
linker = "path/to/ndk/toolchains/llvm/prebuilt/linux-x86_64/bin/x86_64-linux-android21-clang"
```

### iOS/macOS Setup

1. Add iOS targets to Rust:

```bash
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin
```

2. Install cargo-lipo for universal binaries:

```bash
cargo install cargo-lipo
```

3. Build universal library:

```bash
cargo lipo --release --features flutter
```

## Troubleshooting

### Common Issues

#### 1. "Library not found" on Android

- Ensure the native library is in the correct `jniLibs` directory
- Check that the library is built for the correct architecture
- Verify NDK version compatibility

#### 2. "Symbol not found" on iOS/macOS

- Rebuild the library with the correct target
- Ensure all dependencies are linked
- Check that bitcode is disabled

#### 3. "Permission denied" errors

- Verify all required permissions are declared in the manifest/Info.plist
- Request runtime permissions in your Dart code
- Check platform-specific permission requirements

#### 4. Build failures

- Ensure Rust toolchain is up to date: `rustup update`
- Clean build artifacts: `cargo clean && flutter clean`
- Verify all dependencies are installed
- Check Flutter Rust Bridge version compatibility

### Getting Help

- GitHub Issues: https://github.com/kizuna/kizuna/issues
- Documentation: https://kizuna.dev/docs
- Discussions: https://github.com/kizuna/kizuna/discussions

## Version Compatibility

| Kizuna Version | Flutter Version | Rust Version | FRB Version |
|----------------|-----------------|--------------|-------------|
| 0.1.0          | >=3.0.0         | >=1.70.0     | ^2.0.0      |

## Next Steps

- Review the [API documentation](https://pub.dev/documentation/kizuna/latest/)
- Check out the [example application](example/)
- Read the [best practices guide](BEST_PRACTICES.md)
