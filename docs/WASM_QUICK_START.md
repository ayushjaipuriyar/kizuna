# Kizuna WebAssembly Quick Start Guide

## Prerequisites

- Rust toolchain (1.70+)
- wasm-pack (will be installed automatically by build script)
- Python 3 or Node.js (for local testing)

## Quick Start

### 1. Install WASM Target

```bash
rustup target add wasm32-unknown-unknown
```

### 2. Build for WASM

```bash
./scripts/build-wasm.sh
```

This will:
- Install wasm-pack if needed
- Compile Rust code to WebAssembly
- Generate JavaScript bindings
- Create service worker
- Output everything to `www/` directory

### 3. Test Locally

```bash
# Using Python
python3 -m http.server 8080 --directory www

# Or using Node.js
npx http-server www -p 8080
```

Open http://localhost:8080 in your browser.

## Project Structure

```
kizuna/
├── src/platform/wasm/          # WASM platform implementation
│   ├── adapter.rs              # Platform adapter
│   ├── bindings.rs             # JavaScript bindings
│   ├── pwa.rs                  # PWA functionality
│   ├── security.rs             # Security and API restrictions
│   └── mod.rs                  # Module exports
├── www/                        # Web application
│   ├── index.html              # Main page
│   ├── manifest.json           # PWA manifest
│   ├── offline.html            # Offline fallback
│   └── pkg/                    # Generated WASM (after build)
├── scripts/
│   └── build-wasm.sh           # Build script
├── .cargo/config.toml          # Cargo configuration
└── wasm-pack.toml              # wasm-pack configuration
```

## Using the JavaScript API

```javascript
// Import and initialize
import init, { KizunaWasm } from './pkg/kizuna.js';

async function main() {
    // Initialize WASM module
    await init();
    
    // Create Kizuna instance
    const kizuna = new KizunaWasm();
    
    // Initialize the application
    await kizuna.initialize();
    
    // Check browser capabilities
    const capabilities = kizuna.get_capabilities();
    console.log('Notifications:', capabilities.notifications);
    console.log('Service Worker:', capabilities.serviceWorker);
    
    // Show a notification (if supported)
    if (capabilities.notifications) {
        await kizuna.show_notification('Hello', 'Kizuna is running!');
    }
    
    // Use local storage
    kizuna.store_local('myKey', 'myValue');
    const value = kizuna.get_local('myKey');
    console.log('Stored value:', value);
    
    // Get browser information
    const info = kizuna.get_browser_info();
    console.log('Browser:', info);
}

main().catch(console.error);
```

## Browser Capabilities

The WASM build automatically detects and adapts to available browser features:

- ✅ **Always Available:** WebAssembly, DOM, Canvas
- 🔒 **Requires HTTPS:** Service Workers, Notifications, Clipboard API
- 🌐 **Browser Dependent:** WebRTC, IndexedDB, Web Workers
- 🔐 **Requires Headers:** SharedArrayBuffer (cross-origin isolation)

## PWA Installation

Your app can be installed as a Progressive Web App:

1. Visit the site in a supported browser
2. Look for the install prompt or "Install" button
3. Click to add to home screen/app drawer

## Offline Support

The service worker provides offline functionality:

- Cached resources are available offline
- Offline page displays when network is unavailable
- Background sync queues operations when offline

## Development Tips

### Watch Mode

```bash
# Auto-rebuild on changes
cargo watch -s './scripts/build-wasm.sh'
```

### Debug Mode

```bash
# Build without optimizations
wasm-pack build --target web --out-dir www/pkg --dev
```

### Check Bundle Size

```bash
ls -lh www/pkg/kizuna_bg.wasm
```

### Browser DevTools

- Open browser console to see logs
- Check Application tab for Service Worker status
- Use Network tab to verify caching

## Common Issues

### WASM Module Won't Load

**Problem:** Module fails to load with MIME type error

**Solution:** Ensure your server serves `.wasm` files with `application/wasm` MIME type

```python
# Python 3.11+
python3 -m http.server 8080 --directory www
```

### Service Worker Not Registering

**Problem:** Service Worker registration fails

**Solution:** 
- Use HTTPS or localhost
- Check browser console for errors
- Verify `sw.js` is at the root path

### Features Not Working

**Problem:** Certain APIs don't work

**Solution:**
- Check browser compatibility
- Use the capability detection UI
- Review console for security restrictions
- Ensure HTTPS for secure APIs

## Performance Optimization

### Build Optimizations

The release build includes:
- `opt-level = "z"` - Optimize for size
- `lto = true` - Link-time optimization
- `codegen-units = 1` - Better optimization
- `wasm-opt` - Additional WASM optimization

### Runtime Optimizations

- Lazy loading of features
- Efficient memory usage
- Web Workers for parallel processing
- Cache API for resource management

## Deployment

### Static Hosting

Deploy the `www/` directory to any static hosting service:

- GitHub Pages
- Netlify
- Vercel
- Cloudflare Pages
- AWS S3 + CloudFront

### HTTPS Requirement

For full functionality, deploy with HTTPS:

```bash
# Example: Netlify
netlify deploy --dir=www --prod
```

### Headers for Cross-Origin Isolation

For SharedArrayBuffer support, add these headers:

```
Cross-Origin-Embedder-Policy: require-corp
Cross-Origin-Opener-Policy: same-origin
```

## Testing

### Manual Testing

1. Build the project
2. Start local server
3. Open browser to localhost
4. Test each capability
5. Try offline mode
6. Test PWA installation

### Automated Testing

```bash
# Run WASM tests
wasm-pack test --headless --firefox
wasm-pack test --headless --chrome
```

## Resources

- [WebAssembly Documentation](https://webassembly.org/)
- [wasm-pack Guide](https://rustwasm.github.io/wasm-pack/)
- [PWA Documentation](https://web.dev/progressive-web-apps/)
- [Web APIs on MDN](https://developer.mozilla.org/en-US/docs/Web/API)

## Next Steps

1. Customize the UI in `www/index.html`
2. Add your app icons to `www/icons/`
3. Update PWA manifest in `www/manifest.json`
4. Implement additional features in Rust
5. Deploy to production with HTTPS

## Support

For issues or questions:
- Check the implementation summary: `src/platform/wasm/IMPLEMENTATION_SUMMARY.md`
- Review browser console for errors
- Verify browser compatibility
- Check that HTTPS is enabled for production
