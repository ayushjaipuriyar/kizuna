#!/bin/bash
# Build script for Kizuna native libraries across all platforms

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
	echo -e "${GREEN}[INFO]${NC} $1"
}

print_warn() {
	echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
	echo -e "${RED}[ERROR]${NC} $1"
}

# Check if Rust is installed
if ! command -v cargo &>/dev/null; then
	print_error "Rust is not installed. Please install from https://rustup.rs/"
	exit 1
fi

# Check if flutter_rust_bridge_codegen is installed
if ! command -v flutter_rust_bridge_codegen &>/dev/null; then
	print_warn "flutter_rust_bridge_codegen not found. Installing..."
	cargo install flutter_rust_bridge_codegen
fi

# Parse command line arguments
BUILD_ANDROID=false
BUILD_IOS=false
BUILD_MACOS=false
BUILD_WINDOWS=false
BUILD_LINUX=false
BUILD_ALL=false

if [ $# -eq 0 ]; then
	BUILD_ALL=true
else
	for arg in "$@"; do
		case $arg in
		--android)
			BUILD_ANDROID=true
			;;
		--ios)
			BUILD_IOS=true
			;;
		--macos)
			BUILD_MACOS=true
			;;
		--windows)
			BUILD_WINDOWS=true
			;;
		--linux)
			BUILD_LINUX=true
			;;
		--all)
			BUILD_ALL=true
			;;
		*)
			print_error "Unknown argument: $arg"
			echo "Usage: $0 [--android] [--ios] [--macos] [--windows] [--linux] [--all]"
			exit 1
			;;
		esac
	done
fi

# Set all flags if --all is specified
if [ "$BUILD_ALL" = true ]; then
	BUILD_ANDROID=true
	BUILD_IOS=true
	BUILD_MACOS=true
	BUILD_WINDOWS=true
	BUILD_LINUX=true
fi

# Navigate to project root
cd "$(dirname "$0")/../.."

print_info "Starting native library build process..."

# Generate Flutter Rust Bridge bindings
print_info "Generating Flutter Rust Bridge bindings..."
flutter_rust_bridge_codegen \
	--rust-input src/developer_api/bindings/flutter.rs \
	--dart-output flutter/lib/src/bridge_generated.dart \
	--dart-decl-output flutter/lib/src/bridge_definitions.dart

# Build for Android
if [ "$BUILD_ANDROID" = true ]; then
	print_info "Building for Android..."

	# Add Android targets if not already added
	rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android 2>/dev/null || true

	# Build for each Android architecture
	print_info "Building for Android arm64-v8a..."
	cargo build --release --target aarch64-linux-android --features flutter

	print_info "Building for Android armeabi-v7a..."
	cargo build --release --target armv7-linux-androideabi --features flutter

	print_info "Building for Android x86_64..."
	cargo build --release --target x86_64-linux-android --features flutter

	# Copy libraries to Flutter plugin
	print_info "Copying Android libraries..."
	mkdir -p flutter/android/src/main/jniLibs/{arm64-v8a,armeabi-v7a,x86_64}
	cp target/aarch64-linux-android/release/libkizuna.so flutter/android/src/main/jniLibs/arm64-v8a/
	cp target/armv7-linux-androideabi/release/libkizuna.so flutter/android/src/main/jniLibs/armeabi-v7a/
	cp target/x86_64-linux-android/release/libkizuna.so flutter/android/src/main/jniLibs/x86_64/

	print_info "Android build complete!"
fi

# Build for iOS
if [ "$BUILD_IOS" = true ]; then
	print_info "Building for iOS..."

	# Add iOS targets if not already added
	rustup target add aarch64-apple-ios x86_64-apple-ios 2>/dev/null || true

	# Check if cargo-lipo is installed
	if ! command -v cargo-lipo &>/dev/null; then
		print_warn "cargo-lipo not found. Installing..."
		cargo install cargo-lipo
	fi

	# Build universal library
	print_info "Building iOS universal library..."
	cargo lipo --release --features flutter --targets aarch64-apple-ios,x86_64-apple-ios

	# Copy library to Flutter plugin
	print_info "Copying iOS library..."
	mkdir -p flutter/ios/Frameworks
	cp target/universal/release/libkizuna.a flutter/ios/Frameworks/

	print_info "iOS build complete!"
fi

# Build for macOS
if [ "$BUILD_MACOS" = true ]; then
	print_info "Building for macOS..."

	# Add macOS targets if not already added
	rustup target add aarch64-apple-darwin x86_64-apple-darwin 2>/dev/null || true

	# Build for Apple Silicon
	print_info "Building for macOS Apple Silicon..."
	cargo build --release --target aarch64-apple-darwin --features flutter

	# Build for Intel
	print_info "Building for macOS Intel..."
	cargo build --release --target x86_64-apple-darwin --features flutter

	# Create universal binary
	print_info "Creating macOS universal binary..."
	mkdir -p flutter/macos/Frameworks
	lipo -create \
		target/aarch64-apple-darwin/release/libkizuna.dylib \
		target/x86_64-apple-darwin/release/libkizuna.dylib \
		-output flutter/macos/Frameworks/libkizuna.dylib

	print_info "macOS build complete!"
fi

# Build for Windows
if [ "$BUILD_WINDOWS" = true ]; then
	print_info "Building for Windows..."

	# Add Windows target if not already added
	rustup target add x86_64-pc-windows-msvc 2>/dev/null || true

	# Build for Windows
	print_info "Building for Windows x86_64..."
	cargo build --release --target x86_64-pc-windows-msvc --features flutter

	# Copy library to Flutter plugin
	print_info "Copying Windows library..."
	mkdir -p flutter/windows
	cp target/x86_64-pc-windows-msvc/release/kizuna.dll flutter/windows/

	print_info "Windows build complete!"
fi

# Build for Linux
if [ "$BUILD_LINUX" = true ]; then
	print_info "Building for Linux..."

	# Add Linux target if not already added
	rustup target add x86_64-unknown-linux-gnu 2>/dev/null || true

	# Build for Linux
	print_info "Building for Linux x86_64..."
	cargo build --release --target x86_64-unknown-linux-gnu --features flutter

	# Copy library to Flutter plugin
	print_info "Copying Linux library..."
	mkdir -p flutter/linux
	cp target/x86_64-unknown-linux-gnu/release/libkizuna.so flutter/linux/

	print_info "Linux build complete!"
fi

print_info "All builds completed successfully!"
print_info "Native libraries are ready in the flutter/ directory"
