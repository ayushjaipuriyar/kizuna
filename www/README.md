# Kizuna WebAssembly Build

This directory contains the WebAssembly build of Kizuna for running in web browsers.

## Features

- **WebAssembly Compilation**: Rust code compiled to WASM for browser execution
- **Progressive Web App**: Installable app with offline functionality
- **Service Worker**: Caching and offline support
- **Browser API Integration**: Notifications, storage, clipboard, WebRTC
- **Feature Detection**: Automatic detection of browser capabilities
- **Graceful Degradation**: Fallbacks for unsupported features

## Building

To build the WASM module:

```bash
./scripts/build-wasm.sh
```

This will:
1. Compile Rust code to WebAssembly using wasm-pack
2. Generate JavaScript bindings
3. Create service worker for offline functionality
4. Output everything to the `www/` directory

## Testing Locally

To test the WASM build locally:

```bash
# Using Python's built-in HTTP server
python3 -m http.server 8080 --directory www

# Or using Node.js http-server
npx http-server www -p 8080
```

Then open http://localhost:8080 in your browser.

## Browser Requirements

### Minimum Requirements
- Modern browser with WebAssembly support (Chrome 57+, Firefox 52+, Safari 11+, Edge 16+)
- JavaScript enabled

### Optional Features
- **HTTPS**: Required for Service Workers, Notifications, Clipboard API
- **Cross-Origin Isolation**: Required for SharedArrayBuffer (high-performance features)

## Browser Compatibility

| Feature | Chrome | Firefox | Safari | Edge |
|---------|--------|---------|--------|------|
| WebAssembly | 57+ | 52+ | 11+ | 16+ |
| Service Workers | 40+ | 44+ | 11.1+ | 17+ |
| Notifications | 22+ | 22+ | 16+ | 14+ |
| WebRTC | 56+ | 44+ | 11+ | 79+ |
| Clipboard API | 66+ | 63+ | 13.1+ | 79+ |
| IndexedDB | 24+ | 16+ | 10+ | 12+ |

## PWA Installation

The app can be installed as a Progressive Web App:

1. Visit the site in a supported browser
2. Click the "Install" button or use browser's install prompt
3. The app will be added to your home screen/app drawer

## Offline Functionality

When installed as a PWA, the app provides:
- Offline access to cached pages
- Background sync for pending operations
- Push notifications (when granted)
- Local data storage

## Security

The WASM build follows browser security best practices:
- Runs in browser sandbox
- Requires HTTPS for sensitive APIs
- Respects Content Security Policy
- Implements graceful degradation for restricted APIs

## File Structure

```
www/
├── index.html          # Main application page
├── offline.html        # Offline fallback page
├── manifest.json       # PWA manifest
├── sw.js              # Service worker (generated)
├── pkg/               # WASM module and JS bindings (generated)
│   ├── kizuna.js
│   ├── kizuna_bg.wasm
│   └── ...
└── icons/             # App icons (to be added)
    ├── icon-192.png
    └── icon-512.png
```

## Development

For development with hot reload:

```bash
# Watch for changes and rebuild
cargo watch -s './scripts/build-wasm.sh'
```

## Troubleshooting

### WASM module fails to load
- Ensure the server serves `.wasm` files with correct MIME type (`application/wasm`)
- Check browser console for specific errors
- Verify HTTPS is used for production deployments

### Service Worker not registering
- Service Workers require HTTPS (except on localhost)
- Check browser console for registration errors
- Ensure `sw.js` is accessible at the root path

### Features not working
- Check browser compatibility table above
- Use the capability detection UI to see what's available
- Review browser console for API restriction messages

## Performance Optimization

The WASM build is optimized for:
- Small bundle size (using `wasm-opt`)
- Fast startup time
- Efficient memory usage
- Lazy loading of features

## Further Reading

- [WebAssembly Documentation](https://webassembly.org/)
- [wasm-pack Guide](https://rustwasm.github.io/wasm-pack/)
- [PWA Documentation](https://web.dev/progressive-web-apps/)
- [Service Worker API](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API)
