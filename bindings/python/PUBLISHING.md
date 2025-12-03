# Publishing Kizuna Python Package to PyPI

This guide covers the process of publishing the Kizuna Python package to PyPI.

## Prerequisites

1. **Install maturin**:
   ```bash
   pip install maturin
   ```

2. **PyPI Account**: Create accounts on:
   - [Test PyPI](https://test.pypi.org/) (for testing)
   - [PyPI](https://pypi.org/) (for production)

3. **API Tokens**: Generate API tokens from your PyPI account settings

4. **Configure credentials**:
   ```bash
   # For PyPI
   export MATURIN_PYPI_TOKEN=your_pypi_token
   
   # For Test PyPI
   export MATURIN_REPOSITORY=testpypi
   export MATURIN_PYPI_TOKEN=your_testpypi_token
   ```

## Pre-Release Checklist

- [ ] Update version in `Cargo.toml`
- [ ] Update version in `pyproject.toml`
- [ ] Update `CHANGELOG.md`
- [ ] Run all tests: `cargo test --features python`
- [ ] Build and test locally: `maturin develop --features python`
- [ ] Test examples work correctly
- [ ] Update documentation if needed
- [ ] Verify type stubs are complete
- [ ] Check license files are included

## Building Wheels

### Local Development Build

```bash
# Build for current platform
maturin build --features python

# Build release version
maturin build --release --features python
```

### Build for All Platforms

Use the provided script:

```bash
./scripts/build-python-wheels.sh --release --all-platforms
```

Or manually:

```bash
# Linux x86_64
maturin build --release --features python --target x86_64-unknown-linux-gnu

# Linux ARM64
maturin build --release --features python --target aarch64-unknown-linux-gnu

# macOS x86_64
maturin build --release --features python --target x86_64-apple-darwin

# macOS ARM64 (M1/M2)
maturin build --release --features python --target aarch64-apple-darwin

# Windows x86_64
maturin build --release --features python --target x86_64-pc-windows-msvc
```

Wheels will be created in `target/wheels/`.

## Testing Before Publishing

### Test Locally

```bash
# Install the wheel
pip install target/wheels/kizuna-*.whl

# Test import
python -c "from kizuna import Kizuna; print('Import successful')"

# Run examples
python bindings/python/examples/basic_usage.py
```

### Test on Test PyPI

```bash
# Publish to Test PyPI
maturin publish --repository testpypi --features python

# Install from Test PyPI
pip install --index-url https://test.pypi.org/simple/ kizuna

# Test the installation
python -c "from kizuna import Kizuna"
```

## Publishing to PyPI

### Automated Publishing (Recommended)

The GitHub Actions workflow automatically publishes to PyPI when you create a release tag:

```bash
# Create and push a version tag
git tag v0.1.0
git push origin v0.1.0
```

The workflow will:
1. Build wheels for all platforms
2. Build source distribution
3. Publish to PyPI automatically

### Manual Publishing

```bash
# Build wheels for all platforms first
./scripts/build-python-wheels.sh --release --all-platforms

# Publish to PyPI
maturin publish --features python

# Or specify wheels directory
maturin upload target/wheels/*
```

### Publishing Specific Wheels

```bash
# Publish only specific wheels
maturin upload target/wheels/kizuna-0.1.0-cp38-abi3-linux_x86_64.whl
```

## Version Management

### Semantic Versioning

Follow [Semantic Versioning](https://semver.org/):
- **MAJOR**: Incompatible API changes
- **MINOR**: New functionality (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Update Version

1. Update `Cargo.toml`:
   ```toml
   [package]
   version = "0.2.0"
   ```

2. Update `pyproject.toml`:
   ```toml
   [project]
   version = "0.2.0"
   ```

3. Update `CHANGELOG.md`:
   ```markdown
   ## [0.2.0] - 2024-01-15
   ### Added
   - New feature X
   ### Changed
   - Improved Y
   ### Fixed
   - Bug Z
   ```

## Post-Release

1. **Verify on PyPI**: Check [pypi.org/project/kizuna](https://pypi.org/project/kizuna)

2. **Test installation**:
   ```bash
   pip install kizuna
   python -c "from kizuna import Kizuna"
   ```

3. **Update documentation**: Update docs with new version

4. **Announce release**: 
   - GitHub Releases
   - Project website
   - Social media

## Troubleshooting

### Build Fails

```bash
# Clean build artifacts
cargo clean
rm -rf target/

# Rebuild
maturin build --release --features python
```

### Upload Fails

```bash
# Check credentials
echo $MATURIN_PYPI_TOKEN

# Verify package
twine check target/wheels/*

# Try with verbose output
maturin publish --features python -vv
```

### Version Already Exists

PyPI doesn't allow re-uploading the same version. You must:
1. Increment the version number
2. Build new wheels
3. Upload again

### Platform-Specific Issues

**Linux**: Use manylinux containers for compatibility:
```bash
docker run --rm -v $(pwd):/io \
  ghcr.io/pyo3/maturin \
  build --release --features python
```

**macOS**: Ensure you have both x86_64 and ARM64 toolchains:
```bash
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

**Windows**: Use Visual Studio Build Tools:
```bash
# Install from: https://visualstudio.microsoft.com/downloads/
```

## CI/CD Integration

### GitHub Actions

The repository includes a workflow at `.github/workflows/python-wheels.yml` that:
- Builds wheels for all platforms
- Runs tests
- Publishes to PyPI on tag push

### GitLab CI

Example `.gitlab-ci.yml`:
```yaml
build-wheels:
  image: ghcr.io/pyo3/maturin
  script:
    - maturin build --release --features python
  artifacts:
    paths:
      - target/wheels/

publish-pypi:
  image: ghcr.io/pyo3/maturin
  script:
    - maturin publish --features python
  only:
    - tags
```

## Security

### API Token Security

- Never commit API tokens to version control
- Use environment variables or CI/CD secrets
- Rotate tokens periodically
- Use scoped tokens (project-specific)

### Package Signing

Consider signing your packages:
```bash
# Generate GPG key
gpg --gen-key

# Sign wheel
gpg --detach-sign -a target/wheels/kizuna-*.whl
```

## Resources

- [Maturin Documentation](https://maturin.rs/)
- [PyPI Publishing Guide](https://packaging.python.org/tutorials/packaging-projects/)
- [PyO3 Guide](https://pyo3.rs/)
- [Python Packaging User Guide](https://packaging.python.org/)

## Support

For publishing issues:
- Check [Maturin Issues](https://github.com/PyO3/maturin/issues)
- Ask on [PyO3 Discord](https://discord.gg/33kcChzH7f)
- Open an issue in the Kizuna repository
