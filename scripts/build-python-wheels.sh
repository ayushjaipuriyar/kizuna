#!/bin/bash
# Build Python wheels for Kizuna
# This script builds wheels for the current platform

set -e

echo "Building Kizuna Python wheels..."
echo

# Check if maturin is installed
if ! command -v maturin &>/dev/null; then
	echo "Error: maturin is not installed"
	echo "Install it with: pip install maturin"
	exit 1
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
	echo "Error: Must be run from the repository root"
	exit 1
fi

# Parse command line arguments
BUILD_TYPE="release"
PLATFORM="current"

while [[ $# -gt 0 ]]; do
	case $1 in
	--debug)
		BUILD_TYPE="debug"
		shift
		;;
	--release)
		BUILD_TYPE="release"
		shift
		;;
	--all-platforms)
		PLATFORM="all"
		shift
		;;
	--help)
		echo "Usage: $0 [OPTIONS]"
		echo
		echo "Options:"
		echo "  --debug           Build debug version (default: release)"
		echo "  --release         Build release version"
		echo "  --all-platforms   Build for all supported platforms (requires cross-compilation)"
		echo "  --help            Show this help message"
		exit 0
		;;
	*)
		echo "Unknown option: $1"
		echo "Use --help for usage information"
		exit 1
		;;
	esac
done

# Build command
BUILD_CMD="maturin build --features python"

if [ "$BUILD_TYPE" = "release" ]; then
	BUILD_CMD="$BUILD_CMD --release"
	echo "Building release wheels..."
else
	echo "Building debug wheels..."
fi

# Build for current platform or all platforms
if [ "$PLATFORM" = "all" ]; then
	echo "Building for all platforms..."
	echo

	# Detect current OS
	OS=$(uname -s)

	case "$OS" in
	Linux*)
		echo "Building Linux wheels..."
		$BUILD_CMD --target x86_64-unknown-linux-gnu

		if command -v cross &>/dev/null; then
			echo "Building ARM64 Linux wheels..."
			$BUILD_CMD --target aarch64-unknown-linux-gnu
		else
			echo "Skipping ARM64 Linux (cross not installed)"
		fi
		;;

	Darwin*)
		echo "Building macOS wheels..."
		$BUILD_CMD --target x86_64-apple-darwin
		$BUILD_CMD --target aarch64-apple-darwin
		;;

	MINGW* | MSYS* | CYGWIN*)
		echo "Building Windows wheels..."
		$BUILD_CMD --target x86_64-pc-windows-msvc
		;;

	*)
		echo "Unknown OS: $OS"
		echo "Building for current platform only..."
		$BUILD_CMD
		;;
	esac
else
	echo "Building for current platform..."
	$BUILD_CMD
fi

echo
echo "Build complete!"
echo "Wheels are in: target/wheels/"
echo
echo "To install locally:"
echo "  pip install target/wheels/kizuna-*.whl"
echo
echo "To publish to PyPI:"
echo "  maturin publish --features python"
