// WebAssembly platform module

pub mod adapter;
pub mod bindings;
pub mod pwa;
pub mod security;

pub use adapter::{WasmAdapter, BrowserCapabilities};
pub use bindings::KizunaWasm;

#[cfg(target_arch = "wasm32")]
pub use bindings::PolyfillManager;
