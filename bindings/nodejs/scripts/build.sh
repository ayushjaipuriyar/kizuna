#!/bin/bash
# Build script for Kizuna Node.js bindings
# Supports cross-platform builds for npm distribution

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}Building Kizuna Node.js bindings...${NC}\n"

# Check if napi-rs CLI is installed
if ! command -v napi &> /dev/null; then
    echo -e "${YELLOW}napi CLI not found. Installing...${NC}"
    npm install -g @napi-rs/cli
fi

# Determine build mode
BUILD_MODE="${1:-release}"
if [ "$BUILD_MODE" = "debug" ]; then
    BUILD_FLAG=""
    echo -e "${YELLOW}Building in debug mode${NC}"
else
    BUILD_FLAG="--release"
    echo -e "${GREEN}Building in release mode${NC}"
fi

# Navigate to project root
cd "$(dirname "$0")/../../.."

# Build the native module
echo -e "\n${GREEN}Compiling Rust code...${NC}"
cargo build --features nodejs $BUILD_FLAG

# Copy the built library to the bindings directory
echo -e "\n${GREEN}Copying native module...${NC}"
if [ "$BUILD_MODE" = "debug" ]; then
    BUILD_DIR="target/debug"
else
    BUILD_DIR="target/release"
fi

# Determine the library extension based on platform
case "$(uname -s)" in
    Linux*)
        LIB_EXT="so"
        LIB_PREFIX="lib"
        ;;
    Darwin*)
        LIB_EXT="dylib"
        LIB_PREFIX="lib"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        LIB_EXT="dll"
        LIB_PREFIX=""
        ;;
    *)
        echo -e "${RED}Unsupported platform${NC}"
        exit 1
        ;;
esac

# Copy and rename to .node extension
if [ -f "$BUILD_DIR/${LIB_PREFIX}kizuna.$LIB_EXT" ]; then
    cp "$BUILD_DIR/${LIB_PREFIX}kizuna.$LIB_EXT" "bindings/nodejs/kizuna.node"
    echo -e "${GREEN}✓ Native module copied to bindings/nodejs/kizuna.node${NC}"
else
    echo -e "${RED}✗ Native module not found at $BUILD_DIR/${LIB_PREFIX}kizuna.$LIB_EXT${NC}"
    exit 1
fi

echo -e "\n${GREEN}Build complete!${NC}"
echo -e "You can now test the bindings with: ${YELLOW}npm test${NC}"
