# Kizuna Python Bindings - Development Guide

This guide covers development, building, and testing of the Kizuna Python bindings.

## Prerequisites

- Rust 1.70 or higher
- Python 3.8 or higher
- pip and setuptools

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Install Python Development Tools

```bash
pip install maturin pytest pytest-asyncio mypy
```

## Building from Source

### Development Build

Build the Python extension module for development:

```bash
# From the repository root
maturin develop --features python
```

This builds the extension and installs it in your current Python environment.

### Release Build

Build optimized release version:

```bash
maturin build --release --features python
```

The wheel file will be created in `target/wheels/`.

### Install from Wheel

```bash
pip install target/wheels/kizuna-*.whl
```

## Project Structure

```
bindings/python/
├── kizuna.pyi          # Type stubs for IDE support
├── README.md           # User documentation
├── DEVELOPMENT.md      # This file
└── examples/           # Example scripts
    ├── basic_usage.py
    ├── file_transfer.py
    └── streaming.py
```

## Type Checking

The bindings include comprehensive type hints in `kizuna.pyi`.

### Run Type Checker

```bash
mypy examples/basic_usage.py
```

### Type Stub Validation

```bash
stubtest kizuna
```

## Testing

### Unit Tests

Create tests in `tests/python/`:

```python
# tests/python/test_basic.py
import pytest
from kizuna import Kizuna

@pytest.mark.asyncio
async def test_initialization():
    kizuna = Kizuna()
    assert kizuna is not None
    await kizuna.shutdown()

@pytest.mark.asyncio
async def test_discovery():
    kizuna = Kizuna()
    peers = await kizuna.discover_peers()
    assert isinstance(peers, list)
    await kizuna.shutdown()
```

### Run Tests

```bash
pytest tests/python/
```

### Integration Tests

```bash
pytest tests/python/integration/
```

## Documentation

### Generate API Documentation

Using Sphinx:

```bash
cd docs/python
pip install sphinx sphinx-rtd-theme
make html
```

### Update Type Stubs

When adding new API methods, update `kizuna.pyi`:

1. Add method signature with type hints
2. Add comprehensive docstring
3. Add usage example
4. Run `stubtest` to validate

## Code Style

### Python Code Style

Follow PEP 8 guidelines:

```bash
pip install black isort
black examples/
isort examples/
```

### Rust Code Style

```bash
cargo fmt --all
cargo clippy --features python
```

## Performance Profiling

### Profile Python Code

```python
import cProfile
import asyncio
from kizuna import Kizuna

async def profile_discovery():
    kizuna = Kizuna()
    peers = await kizuna.discover_peers()
    await kizuna.shutdown()

cProfile.run('asyncio.run(profile_discovery())')
```

### Memory Profiling

```bash
pip install memory_profiler
python -m memory_profiler examples/file_transfer.py
```

## Debugging

### Enable Debug Logging

```python
import logging
logging.basicConfig(level=logging.DEBUG)

from kizuna import Kizuna
# Debug output will be printed
```

### Rust Debug Builds

```bash
maturin develop --features python
# This creates a debug build with symbols
```

### GDB Debugging

```bash
gdb python
(gdb) run your_script.py
```

## Common Issues

### Import Error

If you get `ImportError: No module named 'kizuna'`:

```bash
# Rebuild and install
maturin develop --features python
```

### Async Runtime Error

If you get runtime errors with asyncio:

```python
# Make sure to use asyncio.run() or proper event loop
import asyncio

async def main():
    kizuna = Kizuna()
    # ... your code ...
    await kizuna.shutdown()

asyncio.run(main())
```

### Type Checking Errors

If mypy reports errors:

1. Ensure `kizuna.pyi` is in the package
2. Update type stubs if API changed
3. Use `# type: ignore` for known issues

## Contributing

### Adding New Features

1. Implement in Rust (`src/developer_api/bindings/python.rs`)
2. Add Python wrapper if needed
3. Update type stubs (`kizuna.pyi`)
4. Add documentation to README
5. Add examples
6. Add tests
7. Update changelog

### Code Review Checklist

- [ ] Rust code compiles without warnings
- [ ] Python type stubs are complete
- [ ] Documentation is updated
- [ ] Examples are provided
- [ ] Tests pass
- [ ] Performance is acceptable
- [ ] Memory leaks are checked

## Release Process

### Version Bumping

1. Update version in `Cargo.toml`
2. Update version in `pyproject.toml`
3. Update CHANGELOG.md
4. Create git tag

### Building Wheels

Build wheels for multiple platforms:

```bash
# Linux
maturin build --release --features python --target x86_64-unknown-linux-gnu

# macOS
maturin build --release --features python --target x86_64-apple-darwin
maturin build --release --features python --target aarch64-apple-darwin

# Windows
maturin build --release --features python --target x86_64-pc-windows-msvc
```

### Publishing to PyPI

```bash
# Test PyPI first
maturin publish --repository testpypi --features python

# Production PyPI
maturin publish --features python
```

## Resources

- [PyO3 Documentation](https://pyo3.rs/)
- [Maturin Documentation](https://maturin.rs/)
- [Python Type Hints](https://docs.python.org/3/library/typing.html)
- [Asyncio Documentation](https://docs.python.org/3/library/asyncio.html)

## Support

For development questions:
- Open an issue on GitHub
- Join the Kizuna Discord server
- Check the main documentation at kizuna.dev
