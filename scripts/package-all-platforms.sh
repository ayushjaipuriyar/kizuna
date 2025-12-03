#!/bin/bash
# Platform-specific package generation script for Kizuna
# Creates distribution packages for all supported platforms

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
VERSION="${VERSION:-0.1.0}"
BUILD_DIR="${BUILD_DIR:-dist}"
PACKAGE_DIR="${PACKAGE_DIR:-packages}"
SIGN_PACKAGES="${SIGN_PACKAGES:-false}"

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
        return 1
    fi
    return 0
}

# Linux package generation
package_linux_deb() {
    log_info "Creating Debian package..."
    
    local arch=$1
    local deb_arch
    
    case "$arch" in
        x86_64)
            deb_arch="amd64"
            ;;
        aarch64)
            deb_arch="arm64"
            ;;
        *)
            log_error "Unsupported architecture for deb: $arch"
            return 1
            ;;
    esac
    
    local package_name="kizuna_${VERSION}_${deb_arch}"
    local package_dir="$PACKAGE_DIR/linux/deb/$package_name"
    
    # Create package structure
    mkdir -p "$package_dir/DEBIAN"
    mkdir -p "$package_dir/usr/bin"
    mkdir -p "$package_dir/usr/share/applications"
    mkdir -p "$package_dir/usr/share/doc/kizuna"
    
    # Copy binary
    cp "$BUILD_DIR/linux/${arch}-unknown-linux-gnu/kizuna" "$package_dir/usr/bin/"
    chmod +x "$package_dir/usr/bin/kizuna"
    
    # Create control file
    cat > "$package_dir/DEBIAN/control" << EOF
Package: kizuna
Version: $VERSION
Section: net
Priority: optional
Architecture: $deb_arch
Maintainer: Kizuna Team <team@kizuna.dev>
Description: Cross-platform device connectivity and file sharing
 Kizuna provides seamless connectivity between devices with features
 including file transfer, clipboard sync, and remote command execution.
EOF
    
    # Create desktop entry
    cat > "$package_dir/usr/share/applications/kizuna.desktop" << EOF
[Desktop Entry]
Name=Kizuna
Comment=Cross-platform device connectivity
Exec=/usr/bin/kizuna
Icon=kizuna
Terminal=false
Type=Application
Categories=Network;FileTransfer;
EOF
    
    # Build package
    dpkg-deb --build "$package_dir" "$PACKAGE_DIR/linux/deb/${package_name}.deb"
    
    log_info "✓ Debian package created: ${package_name}.deb"
}

package_linux_rpm() {
    log_info "Creating RPM package..."
    
    local arch=$1
    local rpm_arch
    
    case "$arch" in
        x86_64)
            rpm_arch="x86_64"
            ;;
        aarch64)
            rpm_arch="aarch64"
            ;;
        *)
            log_error "Unsupported architecture for rpm: $arch"
            return 1
            ;;
    esac
    
    if ! check_tool "rpmbuild"; then
        log_warn "rpmbuild not found, skipping RPM package"
        return 0
    fi
    
    local rpm_dir="$PACKAGE_DIR/linux/rpm"
    mkdir -p "$rpm_dir"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
    
    # Create spec file
    cat > "$rpm_dir/SPECS/kizuna.spec" << EOF
Name:           kizuna
Version:        $VERSION
Release:        1%{?dist}
Summary:        Cross-platform device connectivity and file sharing

License:        MIT
URL:            https://github.com/kizuna/kizuna
Source0:        kizuna-$VERSION.tar.gz

BuildArch:      $rpm_arch

%description
Kizuna provides seamless connectivity between devices with features
including file transfer, clipboard sync, and remote command execution.

%install
mkdir -p %{buildroot}%{_bindir}
cp %{_sourcedir}/../../../$BUILD_DIR/linux/${arch}-unknown-linux-gnu/kizuna %{buildroot}%{_bindir}/

%files
%{_bindir}/kizuna

%changelog
* $(date "+%a %b %d %Y") Kizuna Team <team@kizuna.dev> - $VERSION-1
- Initial package release
EOF
    
    # Build RPM
    rpmbuild --define "_topdir $rpm_dir" -bb "$rpm_dir/SPECS/kizuna.spec"
    
    # Copy to output directory
    cp "$rpm_dir/RPMS/$rpm_arch/"*.rpm "$PACKAGE_DIR/linux/rpm/"
    
    log_info "✓ RPM package created"
}

package_linux_flatpak() {
    log_info "Creating Flatpak manifest..."
    
    local flatpak_dir="$PACKAGE_DIR/linux/flatpak"
    mkdir -p "$flatpak_dir"
    
    # Create Flatpak manifest
    cat > "$flatpak_dir/dev.kizuna.Kizuna.yml" << EOF
app-id: dev.kizuna.Kizuna
runtime: org.freedesktop.Platform
runtime-version: '23.08'
sdk: org.freedesktop.Sdk
command: kizuna
finish-args:
  - --share=network
  - --share=ipc
  - --socket=x11
  - --socket=wayland
  - --device=dri
  - --filesystem=home
modules:
  - name: kizuna
    buildsystem: simple
    build-commands:
      - install -D kizuna /app/bin/kizuna
    sources:
      - type: file
        path: ../../../$BUILD_DIR/linux/x86_64-unknown-linux-gnu/kizuna
EOF
    
    log_info "✓ Flatpak manifest created: $flatpak_dir/dev.kizuna.Kizuna.yml"
    log_info "  Build with: flatpak-builder build-dir $flatpak_dir/dev.kizuna.Kizuna.yml"
}

# macOS package generation
package_macos_app() {
    log_info "Creating macOS app bundle..."
    
    local arch=$1
    local app_name="Kizuna.app"
    local app_dir="$PACKAGE_DIR/macos/$app_name"
    
    # Create app bundle structure
    mkdir -p "$app_dir/Contents/MacOS"
    mkdir -p "$app_dir/Contents/Resources"
    
    # Copy binary
    cp "$BUILD_DIR/macos/${arch}-apple-darwin/kizuna" "$app_dir/Contents/MacOS/"
    chmod +x "$app_dir/Contents/MacOS/kizuna"
    
    # Create Info.plist
    cat > "$app_dir/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>kizuna</string>
    <key>CFBundleIdentifier</key>
    <string>dev.kizuna.Kizuna</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Kizuna</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSSupportsAutomaticGraphicsSwitching</key>
    <true/>
</dict>
</plist>
EOF
    
    log_info "✓ macOS app bundle created: $app_name"
    
    # Code signing (if enabled)
    if [ "$SIGN_PACKAGES" = "true" ]; then
        sign_macos_app "$app_dir"
    fi
}

sign_macos_app() {
    local app_dir=$1
    
    if [ -z "$MACOS_SIGNING_IDENTITY" ]; then
        log_warn "MACOS_SIGNING_IDENTITY not set, skipping code signing"
        return 0
    fi
    
    log_info "Signing macOS app bundle..."
    
    codesign --force --deep --sign "$MACOS_SIGNING_IDENTITY" "$app_dir"
    
    log_info "✓ macOS app bundle signed"
}

package_macos_dmg() {
    log_info "Creating macOS DMG..."
    
    if ! check_tool "hdiutil"; then
        log_warn "hdiutil not found (not on macOS), skipping DMG creation"
        return 0
    fi
    
    local dmg_name="Kizuna-${VERSION}.dmg"
    local dmg_path="$PACKAGE_DIR/macos/$dmg_name"
    local app_dir="$PACKAGE_DIR/macos/Kizuna.app"
    
    if [ ! -d "$app_dir" ]; then
        log_error "App bundle not found: $app_dir"
        return 1
    fi
    
    # Create temporary DMG directory
    local temp_dmg_dir=$(mktemp -d)
    cp -R "$app_dir" "$temp_dmg_dir/"
    
    # Create DMG
    hdiutil create -volname "Kizuna" -srcfolder "$temp_dmg_dir" -ov -format UDZO "$dmg_path"
    
    # Clean up
    rm -rf "$temp_dmg_dir"
    
    log_info "✓ macOS DMG created: $dmg_name"
}

# Windows package generation
package_windows_msi() {
    log_info "Creating Windows MSI installer..."
    
    local arch=$1
    
    if ! check_tool "wix"; then
        log_warn "WiX Toolset not found, skipping MSI creation"
        log_info "  Install from: https://wixtoolset.org/"
        return 0
    fi
    
    local msi_dir="$PACKAGE_DIR/windows/msi"
    mkdir -p "$msi_dir"
    
    # Create WiX source file
    cat > "$msi_dir/kizuna.wxs" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
  <Product Id="*" Name="Kizuna" Language="1033" Version="$VERSION" 
           Manufacturer="Kizuna Team" UpgradeCode="12345678-1234-1234-1234-123456789012">
    <Package InstallerVersion="200" Compressed="yes" InstallScope="perMachine" />
    
    <MajorUpgrade DowngradeErrorMessage="A newer version is already installed." />
    <MediaTemplate EmbedCab="yes" />
    
    <Feature Id="ProductFeature" Title="Kizuna" Level="1">
      <ComponentGroupRef Id="ProductComponents" />
    </Feature>
  </Product>
  
  <Fragment>
    <Directory Id="TARGETDIR" Name="SourceDir">
      <Directory Id="ProgramFilesFolder">
        <Directory Id="INSTALLFOLDER" Name="Kizuna" />
      </Directory>
    </Directory>
  </Fragment>
  
  <Fragment>
    <ComponentGroup Id="ProductComponents" Directory="INSTALLFOLDER">
      <Component Id="KizunaExe" Guid="*">
        <File Id="KizunaExe" Source="../../../$BUILD_DIR/windows/${arch}-pc-windows-msvc/kizuna.exe" 
              KeyPath="yes" Checksum="yes" />
      </Component>
    </ComponentGroup>
  </Fragment>
</Wix>
EOF
    
    log_info "✓ WiX source file created: $msi_dir/kizuna.wxs"
    log_info "  Build with: candle kizuna.wxs && light kizuna.wixobj"
}

package_windows_msix() {
    log_info "Creating Windows MSIX package..."
    
    local msix_dir="$PACKAGE_DIR/windows/msix"
    mkdir -p "$msix_dir"
    
    # Create AppxManifest.xml
    cat > "$msix_dir/AppxManifest.xml" << EOF
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10">
  <Identity Name="Kizuna" Publisher="CN=Kizuna" Version="$VERSION.0" />
  <Properties>
    <DisplayName>Kizuna</DisplayName>
    <PublisherDisplayName>Kizuna Team</PublisherDisplayName>
    <Logo>Assets\StoreLogo.png</Logo>
  </Properties>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="10.0.17763.0" MaxVersionTested="10.0.19041.0" />
  </Dependencies>
  <Resources>
    <Resource Language="en-us" />
  </Resources>
  <Applications>
    <Application Id="Kizuna" Executable="kizuna.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="Kizuna" Description="Cross-platform device connectivity"
                          BackgroundColor="transparent" Square150x150Logo="Assets\Square150x150Logo.png"
                          Square44x44Logo="Assets\Square44x44Logo.png">
      </uap:VisualElements>
    </Application>
  </Applications>
</Package>
EOF
    
    log_info "✓ MSIX manifest created: $msix_dir/AppxManifest.xml"
    log_info "  Build with: makeappx pack /d $msix_dir /p Kizuna.msix"
}

# Container package generation
package_container() {
    log_info "Packaging container images..."
    
    if ! check_tool "docker"; then
        log_warn "Docker not found, skipping container packaging"
        return 0
    fi
    
    local container_dir="$PACKAGE_DIR/container"
    mkdir -p "$container_dir"
    
    # Save container image
    log_info "Saving container image to tar..."
    docker save kizuna:latest | gzip > "$container_dir/kizuna-${VERSION}-container.tar.gz"
    
    log_info "✓ Container image saved: kizuna-${VERSION}-container.tar.gz"
}

# Main packaging function
package_all() {
    log_info "Starting package generation for all platforms..."
    log_info "Version: $VERSION"
    log_info "Build directory: $BUILD_DIR"
    log_info "Package directory: $PACKAGE_DIR"
    
    # Create package directory
    mkdir -p "$PACKAGE_DIR"
    
    # Linux packages
    if [ -d "$BUILD_DIR/linux" ]; then
        log_info "Packaging Linux builds..."
        
        if [ -f "$BUILD_DIR/linux/x86_64-unknown-linux-gnu/kizuna" ]; then
            package_linux_deb "x86_64" || log_warn "Failed to create x86_64 deb package"
            package_linux_rpm "x86_64" || log_warn "Failed to create x86_64 rpm package"
        fi
        
        if [ -f "$BUILD_DIR/linux/aarch64-unknown-linux-gnu/kizuna" ]; then
            package_linux_deb "aarch64" || log_warn "Failed to create aarch64 deb package"
            package_linux_rpm "aarch64" || log_warn "Failed to create aarch64 rpm package"
        fi
        
        package_linux_flatpak || log_warn "Failed to create Flatpak manifest"
    fi
    
    # macOS packages
    if [ -d "$BUILD_DIR/macos" ]; then
        log_info "Packaging macOS builds..."
        
        if [ -f "$BUILD_DIR/macos/x86_64-apple-darwin/kizuna" ]; then
            package_macos_app "x86_64" || log_warn "Failed to create x86_64 app bundle"
        fi
        
        if [ -f "$BUILD_DIR/macos/aarch64-apple-darwin/kizuna" ]; then
            package_macos_app "aarch64" || log_warn "Failed to create aarch64 app bundle"
        fi
        
        package_macos_dmg || log_warn "Failed to create DMG"
    fi
    
    # Windows packages
    if [ -d "$BUILD_DIR/windows" ]; then
        log_info "Packaging Windows builds..."
        
        if [ -f "$BUILD_DIR/windows/x86_64-pc-windows-msvc/kizuna.exe" ]; then
            package_windows_msi "x86_64" || log_warn "Failed to create x86_64 MSI"
            package_windows_msix || log_warn "Failed to create MSIX manifest"
        fi
    fi
    
    # Container packages
    package_container || log_warn "Failed to package container"
    
    # Generate package manifest
    generate_package_manifest
    
    log_info "✓ Package generation complete!"
    log_info "Packages available in: $PACKAGE_DIR"
}

generate_package_manifest() {
    log_info "Generating package manifest..."
    
    local manifest_file="$PACKAGE_DIR/manifest.json"
    
    cat > "$manifest_file" << EOF
{
  "version": "$VERSION",
  "build_date": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "packages": {
EOF
    
    # Find all packages and add to manifest
    local first=true
    find "$PACKAGE_DIR" -type f \( -name "*.deb" -o -name "*.rpm" -o -name "*.dmg" -o -name "*.msi" -o -name "*.tar.gz" \) | while read -r package; do
        local rel_path=$(realpath --relative-to="$PACKAGE_DIR" "$package")
        local size=$(stat -f%z "$package" 2>/dev/null || stat -c%s "$package")
        local checksum=$(sha256sum "$package" | awk '{print $1}')
        
        if [ "$first" = false ]; then
            echo "," >> "$manifest_file"
        fi
        first=false
        
        cat >> "$manifest_file" << EOF
    "$rel_path": {
      "size": $size,
      "sha256": "$checksum"
    }
EOF
    done
    
    cat >> "$manifest_file" << EOF
  }
}
EOF
    
    log_info "✓ Package manifest created: $manifest_file"
}

show_usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Package Kizuna for all supported platforms

OPTIONS:
    --version VERSION   Package version (default: $VERSION)
    --build-dir DIR     Build artifacts directory (default: $BUILD_DIR)
    --package-dir DIR   Output directory for packages (default: $PACKAGE_DIR)
    --sign              Enable package signing (requires credentials)
    --help              Show this help message

ENVIRONMENT VARIABLES:
    VERSION             Package version
    BUILD_DIR           Build artifacts directory
    PACKAGE_DIR         Output directory for packages
    SIGN_PACKAGES       Enable package signing (true/false)
    MACOS_SIGNING_IDENTITY  macOS code signing identity

EXAMPLES:
    # Package all platforms
    $0

    # Package with custom version
    $0 --version 1.0.0

    # Package with signing enabled
    $0 --sign

EOF
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --build-dir)
            BUILD_DIR="$2"
            shift 2
            ;;
        --package-dir)
            PACKAGE_DIR="$2"
            shift 2
            ;;
        --sign)
            SIGN_PACKAGES="true"
            shift
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

# Run packaging
package_all
