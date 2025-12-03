#!/bin/bash
# Comprehensive cross-platform build script for Kizuna
# Builds for all supported platforms and architectures

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
BUILD_TYPE="${BUILD_TYPE:-release}"
OUTPUT_DIR="${OUTPUT_DIR:-dist}"
SKIP_TESTS="${SKIP_TESTS:-false}"
PLATFORMS="${PLATFORMS:-all}"

# Platform targets
LINUX_TARGETS=("x86_64-unknown-linux-gnu" "aarch64-unknown-linux-gnu")
MACOS_TARGETS=("x86_64-apple-darwin" "aarch64-apple-darwin")
WINDOWS_TARGETS=("x86_64-pc-windows-msvc" "aarch64-pc-windows-msvc")
WASM_TARGET="wasm32-unknown-unknown"

# Helper functions
log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_tool() {
    if ! command -v "$1" &> /dev/null; then
        log_error "$1 is not installed"
        return 1
    fi
    return 0
}

install_target() {
    local target=$1
    log_info "Installing Rust target: $target"
    rustup target add "$target" || {
        log_error "Failed to install target: $target"
        return 1
    }
}

build_target() {
    local target=$1
    local platform=$2
    
    log_info "Building for $platform ($target)..."
    
    # Install target if not already installed
    install_target "$target"
    
    # Build command
    local build_cmd="cargo build --target $target"
    if [ "$BUILD_TYPE" = "release" ]; then
        build_cmd="$build_cmd --release"
    fi
    
    # Execute build
    if $build_cmd; then
        log_info "✓ Build successful for $platform ($target)"
        
        # Copy artifact to output directory
        local binary_name="kizuna"
        if [[ "$target" == *"windows"* ]]; then
            binary_name="kizuna.exe"
        fi
        
        local source_path="target/$target/$BUILD_TYPE/$binary_name"
        local dest_dir="$OUTPUT_DIR/$platform/$target"
        
        mkdir -p "$dest_dir"
        if [ -f "$source_path" ]; then
            cp "$source_path" "$dest_dir/"
            log_info "Artifact copied to: $dest_dir/$binary_name"
        fi
        
        return 0
    else
        log_error "✗ Build failed for $platform ($target)"
        return 1
    fi
}

run_tests() {
    local target=$1
    
    if [ "$SKIP_TESTS" = "true" ]; then
        log_warn "Skipping tests (SKIP_TESTS=true)"
        return 0
    fi
    
    log_info "Running tests for $target..."
    
    local test_cmd="cargo test --target $target"
    if [ "$BUILD_TYPE" = "release" ]; then
        test_cmd="$test_cmd --release"
    fi
    
    if $test_cmd; then
        log_info "✓ Tests passed for $target"
        return 0
    else
        log_error "✗ Tests failed for $target"
        return 1
    fi
}

build_linux() {
    log_info "Building Linux targets..."
    
    local success=0
    local failed=0
    
    for target in "${LINUX_TARGETS[@]}"; do
        if build_target "$target" "linux"; then
            ((success++))
            
            # Run tests only for native architecture
            if [ "$target" = "x86_64-unknown-linux-gnu" ] && [ "$(uname -m)" = "x86_64" ]; then
                run_tests "$target" || ((failed++))
            fi
        else
            ((failed++))
        fi
    done
    
    log_info "Linux builds: $success successful, $failed failed"
    return $failed
}

build_macos() {
    log_info "Building macOS targets..."
    
    if [ "$(uname -s)" != "Darwin" ]; then
        log_warn "Skipping macOS builds (not running on macOS)"
        return 0
    fi
    
    local success=0
    local failed=0
    
    for target in "${MACOS_TARGETS[@]}"; do
        if build_target "$target" "macos"; then
            ((success++))
            
            # Run tests for native architecture
            local current_arch=$(uname -m)
            if { [ "$target" = "x86_64-apple-darwin" ] && [ "$current_arch" = "x86_64" ]; } || \
               { [ "$target" = "aarch64-apple-darwin" ] && [ "$current_arch" = "arm64" ]; }; then
                run_tests "$target" || ((failed++))
            fi
        else
            ((failed++))
        fi
    done
    
    log_info "macOS builds: $success successful, $failed failed"
    return $failed
}

build_windows() {
    log_info "Building Windows targets..."
    
    if [ "$(uname -s)" != "MINGW"* ] && [ "$(uname -s)" != "MSYS"* ]; then
        log_warn "Skipping Windows builds (not running on Windows)"
        log_info "Use cross-compilation or run on Windows for Windows builds"
        return 0
    fi
    
    local success=0
    local failed=0
    
    for target in "${WINDOWS_TARGETS[@]}"; do
        if build_target "$target" "windows"; then
            ((success++))
            
            # Run tests for native architecture
            if [ "$target" = "x86_64-pc-windows-msvc" ]; then
                run_tests "$target" || ((failed++))
            fi
        else
            ((failed++))
        fi
    done
    
    log_info "Windows builds: $success successful, $failed failed"
    return $failed
}

build_wasm() {
    log_info "Building WebAssembly target..."
    
    # Check for wasm-pack
    if ! check_tool "wasm-pack"; then
        log_warn "wasm-pack not found, installing..."
        cargo install wasm-pack || {
            log_error "Failed to install wasm-pack"
            return 1
        }
    fi
    
    # Build WASM
    local wasm_output="$OUTPUT_DIR/wasm"
    mkdir -p "$wasm_output"
    
    if wasm-pack build --target web --out-dir "$wasm_output" --release; then
        log_info "✓ WASM build successful"
        log_info "WASM artifacts in: $wasm_output"
        return 0
    else
        log_error "✗ WASM build failed"
        return 1
    fi
}

build_container() {
    log_info "Building container images..."
    
    # Check for Docker
    if ! check_tool "docker"; then
        log_warn "Docker not found, skipping container build"
        return 0
    fi
    
    # Build multi-arch container
    if docker buildx version &> /dev/null; then
        log_info "Building multi-architecture container..."
        
        if docker buildx build \
            --platform linux/amd64,linux/arm64 \
            --tag kizuna:latest \
            --file Dockerfile \
            .; then
            log_info "✓ Container build successful"
            return 0
        else
            log_error "✗ Container build failed"
            return 1
        fi
    else
        log_warn "Docker buildx not available, building single-arch container"
        
        if docker build -t kizuna:latest .; then
            log_info "✓ Container build successful"
            return 0
        else
            log_error "✗ Container build failed"
            return 1
        fi
    fi
}

generate_build_report() {
    log_info "Generating build report..."
    
    local report_file="$OUTPUT_DIR/build-report.txt"
    
    cat > "$report_file" << EOF
Kizuna Cross-Platform Build Report
===================================
Build Date: $(date)
Build Type: $BUILD_TYPE
Host System: $(uname -s) $(uname -m)

Artifacts:
EOF
    
    # List all artifacts
    find "$OUTPUT_DIR" -type f -exec ls -lh {} \; | awk '{print $9 " - " $5}' >> "$report_file"
    
    log_info "Build report saved to: $report_file"
    cat "$report_file"
}

show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Build Kizuna for all supported platforms

OPTIONS:
    --release           Build in release mode (default)
    --debug             Build in debug mode
    --skip-tests        Skip running tests
    --platforms PLAT    Comma-separated list of platforms to build
                        Options: linux,macos,windows,wasm,container,all
                        Default: all
    --output DIR        Output directory for artifacts (default: dist)
    --help              Show this help message

EXAMPLES:
    # Build all platforms in release mode
    $0

    # Build only Linux targets in debug mode
    $0 --debug --platforms linux

    # Build without running tests
    $0 --skip-tests

    # Build specific platforms
    $0 --platforms linux,wasm

ENVIRONMENT VARIABLES:
    BUILD_TYPE          Build type (release or debug)
    OUTPUT_DIR          Output directory for artifacts
    SKIP_TESTS          Skip tests (true or false)
    PLATFORMS           Platforms to build (comma-separated)

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_TYPE="release"
            shift
            ;;
        --debug)
            BUILD_TYPE="debug"
            shift
            ;;
        --skip-tests)
            SKIP_TESTS="true"
            shift
            ;;
        --platforms)
            PLATFORMS="$2"
            shift 2
            ;;
        --output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Main execution
main() {
    log_info "Starting cross-platform build..."
    log_info "Build type: $BUILD_TYPE"
    log_info "Output directory: $OUTPUT_DIR"
    log_info "Platforms: $PLATFORMS"
    
    # Check prerequisites
    if ! check_tool "cargo"; then
        log_error "Rust/Cargo is not installed"
        exit 1
    fi
    
    if ! check_tool "rustup"; then
        log_error "rustup is not installed"
        exit 1
    fi
    
    # Create output directory
    mkdir -p "$OUTPUT_DIR"
    
    # Track overall success
    local total_failed=0
    
    # Build platforms based on selection
    IFS=',' read -ra PLATFORM_ARRAY <<< "$PLATFORMS"
    
    for platform in "${PLATFORM_ARRAY[@]}"; do
        case "$platform" in
            linux|all)
                build_linux || ((total_failed+=$?))
                ;;
            macos|all)
                if [ "$platform" = "all" ] || [ "$platform" = "macos" ]; then
                    build_macos || ((total_failed+=$?))
                fi
                ;;
            windows|all)
                if [ "$platform" = "all" ] || [ "$platform" = "windows" ]; then
                    build_windows || ((total_failed+=$?))
                fi
                ;;
            wasm|all)
                if [ "$platform" = "all" ] || [ "$platform" = "wasm" ]; then
                    build_wasm || ((total_failed+=$?))
                fi
                ;;
            container|all)
                if [ "$platform" = "all" ] || [ "$platform" = "container" ]; then
                    build_container || ((total_failed+=$?))
                fi
                ;;
            *)
                log_warn "Unknown platform: $platform"
                ;;
        esac
        
        # Break early if we've already processed 'all'
        if [ "$platform" = "all" ]; then
            break
        fi
    done
    
    # Generate build report
    generate_build_report
    
    # Summary
    echo ""
    if [ $total_failed -eq 0 ]; then
        log_info "✓ All builds completed successfully!"
        exit 0
    else
        log_error "✗ Some builds failed (total failures: $total_failed)"
        exit 1
    fi
}

# Run main
main
