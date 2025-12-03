// WebAssembly/Browser platform - re-export from wasm module

mod wasm;

pub use wasm::{WasmAdapter, BrowserCapabilities, KizunaWasm};

#[cfg(target_arch = "wasm32")]
pub use wasm::PolyfillManager;
