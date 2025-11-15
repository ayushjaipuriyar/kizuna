/// Core API module providing the foundational Rust API
pub mod api;
pub mod config;
pub mod error;
pub mod events;
pub mod runtime;

// Re-export core types
pub use api::{KizunaAPI, KizunaInstance};
pub use config::KizunaConfig;
pub use error::KizunaError;
pub use events::KizunaEvent;
pub use runtime::AsyncRuntime;
