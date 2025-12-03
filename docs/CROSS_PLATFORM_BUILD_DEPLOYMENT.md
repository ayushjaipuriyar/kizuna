# Cross-Platform Build and Deployment System

This document describes the automated cross-platform build and deployment pipeline for Kizuna.

## Overview

The cross-platform build and deployment system provides comprehensive automation for building, packaging, and distributing Kizuna across all supported platforms. It includes:

1. **Automated Cross-Platform Build System** - CI/CD integration for all target platforms
2. **Platform-Specific Deployment Automation** - Package generation and code signing
3. **Feature Parity Validation** - Automated testing for feature consistency

## Supported Platforms

### Desktop Platforms
- **Linux**: x86_64, ARM64 (deb, rpm, flatpak, snap)
- **macOS**: x86_64, ARM64 (app bundle, DMG, pkg)
- **Windows**: x86_64, ARM64 (MSI, MSIX, ZIP)

### Mobile Platforms
- **Android**: ARM64, ARM32 (AAB, APK)
- **iOS**: ARM64 (app bundle)

### Web Platform
- **WebAssembly**: wasm32 (WASM module, PWA)

### Container Platform
- **Docker**: multi-arch (linux/amd64, linux/arm64)

## Build System

### GitHub Actions Workflows

#### Cross-Platform Build (`cross-platform-build.yml`)
Automatically builds Kizuna for all supported platforms on every push and pull request.

**Features:**
- Parallel builds for all platforms
- Automated testing on native platforms
- Cross-compilation validation
- Build artifact generation
- Build report generation

**Triggers:**
- Push to `main` or `develop` branches
- Pull requests
- Manual workflow dispatch

#### Release and Deployment (`release-deployment.yml`)
Automates the release process including building, packaging, signing, and distribution.

**Features:**
- Multi-platform package generation
- Code signing (macOS, Windows)
- Container image publishing
- GitHub release creation
- Automated distribution

**Triggers:**
- Git tags matching `v*`
- Manual workflow dispatch

#### Feature Parity Validation (`feature-parity-validation.yml`)
Validates feature consistency across all platforms.

**Features:**
- Feature parity testing
- Platform implementation validation
- Feature matrix generation
- Regression detection
- Automated issue creation

**Triggers:**
- Push to `main` or `develop` branches
- Pull requests
- Weekly schedule (Monday 00:00 UTC)
- Manual workflow dispatch

### Local Build Scripts

#### `scripts/build-all-platforms.sh`
Comprehensive build script for local development.

**Usage:**
```bash
# Build all platforms in release mode
./scripts/build-all-platforms.sh

# Build specific platforms
./scripts/build-all-platforms.sh --platforms linux,macos

# Build in debug mode
./scripts/build-all-platforms.sh --debug

# Skip tests
./scripts/build-all-platforms.sh --skip-tests
```

**Options:**
- `--release` - Build in release mode (default)
- `--debug` - Build in debug mode
- `--skip-tests` - Skip running tests
- `--platforms PLAT` - Comma-separated list of platforms
- `--output DIR` - Output directory for artifacts

#### `scripts/package-all-platforms.sh`
Platform-specific package generation script.

**Usage:**
```bash
# Package all platforms
./scripts/package-all-platforms.sh

# Package with custom version
./scripts/package-all-platforms.sh --version 1.0.0

# Enable code signing
./scripts/package-all-platforms.sh --sign
```

**Options:**
- `--version VERSION` - Package version
- `--build-dir DIR` - Build artifacts directory
- `--package-dir DIR` - Output directory for packages
- `--sign` - Enable package signing

#### `scripts/validate-feature-parity.sh`
Feature parity validation script.

**Usage:**
```bash
# Run feature parity validation
./scripts/validate-feature-parity.sh

# Generate feature matrix
GENERATE_MATRIX=true ./scripts/validate-feature-parity.sh
```

## Deployment System

### Package Formats

#### Linux
- **Debian Package (.deb)**: For Debian, Ubuntu, and derivatives
- **RPM Package (.rpm)**: For Fedora, RHEL, and derivatives
- **Flatpak**: Universal Linux package format
- **Snap**: Ubuntu's universal package format
- **AppImage**: Portable Linux application format

#### macOS
- **App Bundle (.app)**: Native macOS application bundle
- **DMG**: Disk image for easy installation
- **PKG**: macOS installer package

#### Windows
- **MSI**: Windows Installer package
- **MSIX**: Modern Windows app package
- **ZIP**: Portable archive

#### Container
- **Docker Image**: Multi-architecture container image
- **OCI Image**: Open Container Initiative format

### Code Signing

#### macOS
Code signing is performed using the Developer ID certificate:

```bash
codesign --force --deep --sign "Developer ID Application" Kizuna.app
```

**Requirements:**
- Valid Developer ID certificate
- Apple Developer account
- Notarization for distribution

#### Windows
Code signing is performed using signtool:

```bash
signtool sign /f certificate.pfx /p password /t timestamp_url binary.exe
```

**Requirements:**
- Valid code signing certificate
- Timestamp server URL

### Distribution Channels

#### App Stores
- Apple App Store (macOS, iOS)
- Microsoft Store (Windows)
- Google Play Store (Android)

#### Package Repositories
- APT repositories (Debian/Ubuntu)
- YUM repositories (Fedora/RHEL)
- Flatpak repositories
- Snapcraft store
- Homebrew (macOS)
- Chocolatey (Windows)

#### Container Registries
- Docker Hub
- GitHub Container Registry
- Amazon ECR
- Google Container Registry

#### Direct Distribution
- GitHub Releases
- Direct download from website

## Feature Parity

### Required Features
All platforms must support:
- ✅ File Transfer
- ✅ Discovery

### Optional Features
Platform-dependent features:
- Clipboard synchronization
- Screen streaming
- Remote command execution
- System tray integration
- Desktop notifications
- Auto-start on boot
- File type associations

### Feature Matrix

| Feature | Linux | macOS | Windows | Android | iOS | WebAssembly | Container |
|---------|-------|-------|---------|---------|-----|-------------|-----------|
| File Transfer | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Discovery | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Clipboard | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Streaming | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Command Execution | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| System Tray | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| Notifications | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Auto Start | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| File Associations | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |

### Platform Compatibility

#### Desktop Platforms
- **High feature parity** between Linux, macOS, and Windows
- All core features supported
- Platform-specific optimizations available

#### Mobile Platforms
- **Medium feature parity** for Android and iOS
- Core features supported
- Limited UI integration features

#### Web Platform
- **Medium feature parity** for WebAssembly
- Core features supported
- Browser security restrictions apply

#### Container Platform
- **Medium feature parity** for containers
- Core features supported
- Headless operation mode

## Rust Modules

### `src/platform/build_system.rs`
Build system integration and cross-compilation support.

**Key Components:**
- `BuildTarget` - Build target configuration
- `BuildConfig` - Build configuration
- `BuildArtifact` - Build artifact information
- `BuildSystemManager` - Build system manager

### `src/platform/deployment.rs`
Platform-specific deployment automation.

**Key Components:**
- `DeploymentConfig` - Deployment configuration
- `PackageFormat` - Package format enumeration
- `SigningConfig` - Code signing configuration
- `DeploymentPackage` - Deployment package
- `DeploymentManager` - Deployment manager

### `src/platform/feature_parity.rs`
Feature parity validation across platforms.

**Key Components:**
- `FeatureParityValidator` - Feature parity validator
- `FeatureMatrix` - Feature availability matrix
- `CompatibilityMatrix` - Platform compatibility matrix
- `FeatureParityReport` - Validation report

## Testing

### Unit Tests
All modules include comprehensive unit tests:

```bash
# Run all platform tests
cargo test --lib platform

# Run specific module tests
cargo test --lib platform::build_system
cargo test --lib platform::deployment
cargo test --lib platform::feature_parity
```

### Integration Tests
Integration tests validate the complete build and deployment pipeline:

```bash
# Run integration tests
cargo test --test platform_test
```

### Feature Parity Tests
Automated tests ensure feature consistency:

```bash
# Run feature parity tests
cargo test --lib platform::feature_parity

# Generate feature matrix
./scripts/validate-feature-parity.sh
```

## Continuous Integration

### Build Matrix
The CI system builds and tests all platform combinations:

- **Linux**: x86_64, ARM64
- **macOS**: x86_64, ARM64
- **Windows**: x86_64, ARM64
- **WebAssembly**: wasm32
- **Container**: multi-arch

### Validation Steps
1. **Build Validation**: Ensure all platforms build successfully
2. **Test Validation**: Run tests on native platforms
3. **Artifact Validation**: Verify build artifacts
4. **Feature Parity Validation**: Check feature consistency
5. **Regression Detection**: Detect feature regressions

### Artifact Management
- Build artifacts are uploaded to GitHub Actions
- Artifacts are validated for integrity
- Checksums are generated for all artifacts
- Build reports are generated automatically

## Release Process

### Automated Release
1. Create a git tag: `git tag v1.0.0`
2. Push the tag: `git push origin v1.0.0`
3. GitHub Actions automatically:
   - Builds all platforms
   - Generates packages
   - Signs packages (if configured)
   - Creates GitHub release
   - Publishes container images
   - Uploads release artifacts

### Manual Release
1. Run build script: `./scripts/build-all-platforms.sh`
2. Run package script: `./scripts/package-all-platforms.sh --version 1.0.0`
3. Sign packages (if required)
4. Upload to distribution channels

## Configuration

### Environment Variables

#### Build Configuration
- `BUILD_TYPE` - Build type (release or debug)
- `OUTPUT_DIR` - Output directory for artifacts
- `SKIP_TESTS` - Skip tests (true or false)
- `PLATFORMS` - Platforms to build (comma-separated)

#### Deployment Configuration
- `VERSION` - Package version
- `BUILD_DIR` - Build artifacts directory
- `PACKAGE_DIR` - Output directory for packages
- `SIGN_PACKAGES` - Enable package signing (true or false)

#### Code Signing
- `MACOS_SIGNING_IDENTITY` - macOS code signing identity
- `MACOS_CERTIFICATE` - macOS certificate (base64 encoded)
- `MACOS_CERTIFICATE_PWD` - macOS certificate password
- `WINDOWS_CERTIFICATE` - Windows certificate path
- `WINDOWS_CERTIFICATE_PWD` - Windows certificate password

### GitHub Secrets
Required secrets for automated deployment:

- `MACOS_CERTIFICATE` - macOS code signing certificate
- `MACOS_CERTIFICATE_PWD` - macOS certificate password
- `PYPI_API_TOKEN` - PyPI API token (for Python wheels)
- `GITHUB_TOKEN` - GitHub token (automatically provided)

## Troubleshooting

### Build Failures
1. Check build logs in GitHub Actions
2. Verify Rust toolchain is installed
3. Ensure all dependencies are available
4. Check platform-specific requirements

### Package Generation Failures
1. Verify build artifacts exist
2. Check packaging tool availability
3. Ensure proper permissions
4. Verify signing credentials (if applicable)

### Feature Parity Issues
1. Run feature parity validation
2. Check platform-specific implementations
3. Review feature matrix
4. Verify test coverage

## Future Enhancements

- [ ] Automated performance benchmarking
- [ ] Binary size optimization tracking
- [ ] Automated security scanning
- [ ] Multi-language support validation
- [ ] Automated documentation generation
- [ ] Release notes automation
- [ ] Dependency update automation
- [ ] Platform-specific optimization validation

## References

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust Cross-Compilation](https://rust-lang.github.io/rustup/cross-compilation.html)
- [macOS Code Signing](https://developer.apple.com/documentation/security/notarizing_macos_software_before_distribution)
- [Windows Code Signing](https://docs.microsoft.com/en-us/windows/win32/seccrypto/cryptography-tools)
- [Docker Multi-Platform Builds](https://docs.docker.com/build/building/multi-platform/)
