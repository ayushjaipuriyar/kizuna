# Publishing Guide for Kizuna Node.js Bindings

This guide explains how to publish the Kizuna Node.js bindings to npm.

## Prerequisites

1. **npm account**: You need an npm account with publish permissions
2. **napi-rs CLI**: Install with `npm install -g @napi-rs/cli`
3. **Rust toolchain**: Install from https://rustup.rs/
4. **Cross-compilation tools** (for multi-platform builds):
   - Linux: `apt-get install gcc-aarch64-linux-gnu`
   - macOS: Xcode Command Line Tools
   - Windows: Visual Studio Build Tools

## Version Management

### Updating the Version

Use the version management script to ensure consistency:

```bash
cd bindings/nodejs
node scripts/version.js 0.2.0
```

This updates:
- `package.json`
- Root `Cargo.toml`
- `README.md`

### Semantic Versioning

Follow semantic versioning (semver):
- **Major** (1.0.0): Breaking changes
- **Minor** (0.1.0): New features, backward compatible
- **Patch** (0.0.1): Bug fixes, backward compatible

## Building for Multiple Platforms

### Local Build (Current Platform)

```bash
# Release build
npm run build

# Debug build
npm run build:debug
```

### Cross-Platform Builds

#### Linux x64 and ARM64

```bash
# Install cross-compilation tools
sudo apt-get install gcc-aarch64-linux-gnu

# Build for x64
cargo build --release --features nodejs

# Build for ARM64
cargo build --release --features nodejs --target aarch64-unknown-linux-gnu
```

#### macOS x64 and ARM64 (Apple Silicon)

```bash
# Build for x64
cargo build --release --features nodejs --target x86_64-apple-darwin

# Build for ARM64
cargo build --release --features nodejs --target aarch64-apple-darwin

# Create universal binary
lipo -create \
  target/x86_64-apple-darwin/release/libkizuna.dylib \
  target/aarch64-apple-darwin/release/libkizuna.dylib \
  -output target/release/libkizuna.dylib
```

#### Windows

```bash
# Build for x64
cargo build --release --features nodejs --target x86_64-pc-windows-msvc
```

## Testing Before Publishing

### Run Tests

```bash
npm test
```

### Test Installation

Create a test project:

```bash
mkdir test-install
cd test-install
npm init -y
npm install ../path/to/bindings/nodejs
```

Test the installation:

```javascript
// test.js
const { Kizuna } = require('kizuna-node');
const kizuna = new Kizuna();
console.log('Kizuna loaded successfully!');
```

## Publishing to npm

### Prepare for Publishing

1. **Update version** (see Version Management above)

2. **Build all platforms** (or use CI/CD)

3. **Run prepublish script**:
   ```bash
   npm run prepublishOnly
   ```

4. **Review package contents**:
   ```bash
   npm pack --dry-run
   ```

### Publish

#### First-time Setup

```bash
npm login
```

#### Publish Release

```bash
# Publish as latest
npm publish

# Publish as beta
npm publish --tag beta

# Publish as next
npm publish --tag next
```

### Post-Publishing

1. **Verify on npm**: Check https://www.npmjs.com/package/kizuna-node

2. **Test installation**:
   ```bash
   npm install kizuna-node
   ```

3. **Create GitHub release**:
   - Tag: `v0.2.0`
   - Title: `Release 0.2.0`
   - Description: Changelog

## Platform-Specific Packages

For optimal installation experience, publish platform-specific packages:

```bash
# Linux x64
npm publish --tag linux-x64-gnu

# Linux ARM64
npm publish --tag linux-arm64-gnu

# macOS x64
npm publish --tag darwin-x64

# macOS ARM64
npm publish --tag darwin-arm64

# Windows x64
npm publish --tag win32-x64-msvc
```

These are automatically installed as optional dependencies.

## Automated Publishing with CI/CD

The project includes GitHub Actions workflows for automated builds and publishing.

### Trigger Automated Build

Push a tag:

```bash
git tag v0.2.0
git push origin v0.2.0
```

### Configure Secrets

Add to GitHub repository secrets:
- `NPM_TOKEN`: npm authentication token

Get npm token:
```bash
npm token create
```

## Troubleshooting

### Build Failures

**Issue**: Native module not found
```
Solution: Ensure Rust is installed and cargo build succeeds
```

**Issue**: Cross-compilation fails
```
Solution: Install platform-specific toolchains
```

### Publishing Failures

**Issue**: Authentication failed
```
Solution: Run npm login and verify credentials
```

**Issue**: Version already exists
```
Solution: Bump version number
```

**Issue**: Package size too large
```
Solution: Check .npmignore and remove unnecessary files
```

## Best Practices

1. **Test thoroughly** before publishing
2. **Use semantic versioning** consistently
3. **Document breaking changes** in changelog
4. **Provide migration guides** for major versions
5. **Keep dependencies minimal**
6. **Test on all supported platforms**
7. **Monitor npm download stats** and issues

## Rollback

If you need to unpublish or deprecate:

```bash
# Deprecate a version
npm deprecate kizuna-node@0.2.0 "This version has a critical bug"

# Unpublish (within 72 hours)
npm unpublish kizuna-node@0.2.0
```

**Note**: Unpublishing is discouraged. Use deprecation instead.

## Support

For questions or issues:
- GitHub Issues: https://github.com/kizuna/kizuna/issues
- npm: https://www.npmjs.com/package/kizuna-node
- Documentation: https://kizuna.dev/docs/nodejs
